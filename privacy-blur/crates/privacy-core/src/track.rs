//! Temporal stabilisation of detected regions: prevents flicker when OCR misses a
//! word for a single frame, and removes regions once their text is really gone.

use crate::geometry::Rect;
use crate::settings::Category;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub rect: Rect,
    pub category: Category,
}

struct Item {
    region: Region,
    last_seen_ms: u64,
}

#[derive(Default)]
pub struct Tracker {
    items: Vec<Item>,
    /// Number of distinct regions ever started (statistics only).
    pub total_seen: u64,
}

impl Tracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn regions(&self) -> Vec<Region> {
        self.items.iter().map(|i| i.region).collect()
    }

    /// Feed one complete detection pass. Returns `true` if the visible set changed.
    pub fn update(&mut self, now_ms: u64, hold_ms: u64, detections: &[Region]) -> bool {
        let before = self.regions();
        let mut used = vec![false; self.items.len()];
        let mut fresh: Vec<Item> = vec![];
        for d in detections {
            let mut best: Option<(usize, f32)> = None;
            for (i, it) in self.items.iter().enumerate() {
                if used[i] {
                    continue;
                }
                let iou = it.region.rect.iou(&d.rect);
                // Same line, mostly overlapping: treat as the same object that moved/resized a bit.
                if iou >= 0.3 && best.map_or(true, |b| iou > b.1) {
                    best = Some((i, iou));
                }
            }
            match best {
                Some((i, iou)) => {
                    used[i] = true;
                    let it = &mut self.items[i];
                    if iou < 0.9 {
                        it.region.rect = d.rect; // ignore sub-pixel jitter, follow real movement
                    }
                    it.region.category = d.category;
                    it.last_seen_ms = now_ms;
                }
                None => {
                    self.total_seen += 1;
                    fresh.push(Item { region: *d, last_seen_ms: now_ms });
                }
            }
        }
        self.items.extend(fresh);
        self.items.retain(|i| now_ms.saturating_sub(i.last_seen_ms) <= hold_ms);
        self.regions() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reg(x: i32) -> Region {
        Region { rect: Rect::new(x, 0, 100, 20), category: Category::Email }
    }
    #[test]
    fn holds_then_expires() {
        let mut t = Tracker::new();
        assert!(t.update(0, 500, &[reg(0)]));
        assert!(!t.update(100, 500, &[reg(1)])); // jitter: unchanged
        assert!(!t.update(300, 500, &[])); // missed once: still there
        assert_eq!(t.regions().len(), 1);
        assert!(t.update(900, 500, &[])); // gone
        assert!(t.regions().is_empty());
        assert_eq!(t.total_seen, 1);
    }
    #[test]
    fn follows_scroll() {
        let mut t = Tracker::new();
        t.update(0, 500, &[Region { rect: Rect::new(0, 0, 100, 20), category: Category::Ip }]);
        assert!(t.update(100, 500, &[Region { rect: Rect::new(0, 5, 100, 20), category: Category::Ip }]));
        assert_eq!(t.regions()[0].rect.y, 5);
        assert_eq!(t.total_seen, 1);
    }
    #[test]
    fn large_jump_shows_new_immediately_and_old_expires() {
        let mut t = Tracker::new();
        t.update(0, 500, &[reg(0)]);
        t.update(100, 500, &[reg(400)]);
        assert_eq!(t.regions().len(), 2); // new one is covered at once, old one held briefly
        t.update(700, 500, &[reg(400)]);
        assert_eq!(t.regions().len(), 1);
    }
    #[test]
    fn new_region_counts() {
        let mut t = Tracker::new();
        t.update(0, 500, &[reg(0), reg(500)]);
        assert_eq!(t.total_seen, 2);
    }
}
