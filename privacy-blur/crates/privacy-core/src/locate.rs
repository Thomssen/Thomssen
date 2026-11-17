//! Maps text hits back to pixel rectangles.

use crate::detect::Detector;
use crate::geometry::Rect;
use crate::settings::Category;

/// A word (or any run of text) together with its on-screen rectangle.
#[derive(Clone, Debug)]
pub struct Word {
    pub text: String,
    pub rect: Rect,
}

/// Detect sensitive spans in one visual line made of `words` (left to right) and
/// return one rectangle per hit. Partially covered words are cut proportionally by
/// character count and widened by half a character on the cut sides.
pub fn detect_line(det: &Detector, words: &[Word]) -> Vec<(Rect, Category)> {
    if words.is_empty() {
        return vec![];
    }
    let mut text = String::new();
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        let s = text.len();
        text.push_str(&w.text);
        spans.push((s, text.len()));
    }
    let mut out = vec![];
    for h in det.detect(&text) {
        let mut rect: Option<Rect> = None;
        for (w, &(ws, we)) in words.iter().zip(&spans) {
            let (s, e) = (h.start.max(ws), h.end.min(we));
            if s >= e || w.rect.is_empty() {
                continue;
            }
            let n = w.text.chars().count().max(1) as f32;
            let cs = w.text[..s - ws].chars().count() as f32;
            let ce = w.text[..e - ws].chars().count() as f32;
            let cw = w.rect.w as f32 / n;
            let mut x0 = w.rect.x as f32 + cw * cs;
            let mut x1 = w.rect.x as f32 + cw * ce;
            if cs > 0.0 {
                x0 -= cw * 0.5;
            }
            if ce < n {
                x1 += cw * 0.5;
            }
            let x0 = x0.max(w.rect.x as f32);
            let x1 = x1.min(w.rect.right() as f32);
            let r = Rect::new(x0.floor() as i32, w.rect.y, (x1.ceil() - x0.floor()).max(1.0) as i32, w.rect.h);
            rect = Some(rect.map_or(r, |a| a.union(&r)));
        }
        if let Some(r) = rect {
            out.push((r, h.category));
        }
    }
    out
}

/// Split `text` that occupies `rect` on one line into words with estimated rectangles
/// (proportional by character count). Used for UI Automation elements that do not
/// expose per-character geometry.
pub fn words_in_rect(text: &str, rect: Rect) -> Vec<Word> {
    let total = text.chars().count().max(1) as f32;
    let cw = rect.w as f32 / total;
    let mut out = vec![];
    let mut idx = 0usize;
    let mut cur: Option<(usize, String)> = None;
    for ch in text.chars().chain(std::iter::once(' ')) {
        if ch.is_whitespace() {
            if let Some((start, w)) = cur.take() {
                let n = w.chars().count();
                out.push(Word {
                    text: w,
                    rect: Rect::new(rect.x + (cw * start as f32) as i32, rect.y, ((cw * n as f32).ceil() as i32).max(1), rect.h),
                });
            }
        } else {
            cur.get_or_insert_with(|| (idx, String::new())).1.push(ch);
        }
        idx += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Filters;

    fn w(t: &str, x: i32, wd: i32) -> Word {
        Word { text: t.into(), rect: Rect::new(x, 10, wd, 20) }
    }

    #[test]
    fn multi_word_hit_unions_rects() {
        let det = Detector::new(&Filters::default(), &[]);
        let words = vec![w("call", 0, 40), w("+45", 50, 30), w("12", 90, 20), w("34", 120, 20), w("56", 150, 20), w("78", 180, 20), w("now", 210, 30)];
        let r = detect_line(&det, &words);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, Category::Phone);
        assert_eq!(r[0].0.x, 50);
        assert_eq!(r[0].0.right(), 200);
        assert_eq!(r[0].0.h, 20);
    }

    #[test]
    fn partial_word_is_cut_proportionally() {
        let det = Detector::new(&Filters::default(), &[]);
        // "(a@b.com)" — only the email part is sensitive; word is 9 chars over 90px.
        let words = vec![w("(a@b.com)", 100, 90)];
        let r = detect_line(&det, &words);
        assert_eq!(r.len(), 1);
        let rc = r[0].0;
        assert!(rc.x >= 100 && rc.x <= 106, "{rc:?}");
        assert!(rc.right() >= 180 && rc.right() <= 190, "{rc:?}");
    }

    #[test]
    fn estimated_words() {
        let ws = words_in_rect("ab cd", Rect::new(0, 0, 50, 10));
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[1].rect.x, 30);
    }
}
