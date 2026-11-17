//! Censoring renderers. They operate on a BGRA crop of the screen and produce a
//! premultiplied BGRA bitmap for a layered window. Pure CPU code, no OS calls.
//!
//! Every style *replaces* the pixels below (Gaussian blur and pixelation are computed
//! from the real screen content; they are real filters, not a translucent tint).
//! The overlay is a derived image — with the "safe minimum" enabled the strength is
//! raised so the text cannot be read back from it.

use crate::settings::{Appearance, Style};

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub style: Style,
    pub intensity: u8,
    pub blur_radius: u32,
    pub pixel_size: u32,
    pub opacity: u8,
    pub safe_minimum: bool,
}

impl From<&Appearance> for Params {
    fn from(a: &Appearance) -> Self {
        Self {
            style: a.style,
            intensity: a.intensity,
            blur_radius: a.blur_radius,
            pixel_size: a.pixel_size,
            opacity: a.opacity,
            safe_minimum: a.safe_minimum,
        }
    }
}

/// Effective Gaussian radius for a text height.
pub fn gaussian_radius(p: &Params, text_h: usize) -> usize {
    let user = 1.0 + (p.blur_radius.saturating_sub(1)) as f32 * (p.intensity as f32 / 100.0);
    let r = if p.safe_minimum { user.max(text_h as f32 * 0.6) } else { user };
    r.round().max(1.0) as usize
}

/// Effective pixelation block size for a text height.
pub fn pixel_block(p: &Params, text_h: usize) -> usize {
    let user = p.pixel_size as f32;
    let b = if p.safe_minimum { user.max(text_h as f32 * 0.55) } else { user };
    b.round().max(2.0) as usize
}

fn box_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
    let (lines, len, step, line_step) = if horizontal { (h, w, 4, w * 4) } else { (w, h, w * 4, 4) };
    let win = (2 * r + 1) as u32;
    for line in 0..lines {
        let base = line * line_step;
        for ch in 0..3 {
            let at = |i: isize| -> u32 {
                let c = i.clamp(0, len as isize - 1) as usize;
                src[base + c * step + ch] as u32
            };
            let mut sum: u32 = 0;
            for k in -(r as isize)..=(r as isize) {
                sum += at(k);
            }
            for i in 0..len {
                dst[base + i * step + ch] = (sum / win) as u8;
                sum = sum + at(i as isize + r as isize + 1) - at(i as isize - r as isize);
            }
        }
        for i in 0..len {
            dst[base + i * step + 3] = 255;
        }
    }
}

fn gaussian(src: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let mut a = src.to_vec();
    let mut b = vec![0u8; src.len()];
    for _ in 0..3 {
        box_pass(&a, &mut b, w, h, r, true);
        box_pass(&b, &mut a, w, h, r, false);
    }
    a
}

fn pixelate(src: &[u8], w: usize, h: usize, blk: usize) -> Vec<u8> {
    let mut out = vec![255u8; src.len()];
    let mut by = 0;
    while by < h {
        let bh = blk.min(h - by);
        let mut bx = 0;
        while bx < w {
            let bw = blk.min(w - bx);
            let mut acc = [0u32; 3];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let i = (y * w + x) * 4;
                    acc[0] += src[i] as u32;
                    acc[1] += src[i + 1] as u32;
                    acc[2] += src[i + 2] as u32;
                }
            }
            let n = (bw * bh) as u32;
            let avg = [(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let i = (y * w + x) * 4;
                    out[i..i + 3].copy_from_slice(&avg);
                }
            }
            bx += blk;
        }
        by += blk;
    }
    out
}

/// Render the censored version of `src` (BGRA, `w*h*4` bytes) into `out` as premultiplied BGRA.
/// `text_h` is the height of the detected text in pixels (used for the safe minimum).
pub fn render(p: &Params, src: &[u8], w: usize, h: usize, text_h: usize, out: &mut Vec<u8>) {
    assert_eq!(src.len(), w * h * 4);
    let mut px = match p.style {
        Style::Gaussian => gaussian(src, w, h, gaussian_radius(p, text_h)),
        Style::Pixelate => pixelate(src, w, h, pixel_block(p, text_h)),
        Style::Solid => {
            let mut v = vec![0u8; w * h * 4];
            for a in v.chunks_exact_mut(4) {
                a[3] = 255;
            }
            v
        }
    };
    let alpha = (p.opacity as u32 * 255 / 100).min(255);
    if alpha < 255 {
        for a in px.chunks_exact_mut(4) {
            for c in &mut a[..3] {
                *c = (*c as u32 * alpha / 255) as u8;
            }
            a[3] = alpha as u8;
        }
    }
    *out = px;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stripes(w: usize, h: usize) -> Vec<u8> {
        let mut v = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let c = if (x / 2 + y / 2) % 2 == 0 { 255 } else { 0 };
                v[i..i + 4].copy_from_slice(&[c, c, c, 255]);
            }
        }
        v
    }
    fn contrast(v: &[u8]) -> u32 {
        let mut mn = 255;
        let mut mx = 0;
        for p in v.chunks_exact(4) {
            mn = mn.min(p[0] as u32);
            mx = mx.max(p[0] as u32);
        }
        mx - mn
    }
    fn p(style: Style) -> Params {
        Params { style, intensity: 80, blur_radius: 14, pixel_size: 10, opacity: 100, safe_minimum: false }
    }

    #[test]
    fn gaussian_destroys_detail() {
        let src = stripes(120, 30);
        let mut out = vec![];
        render(&p(Style::Gaussian), &src, 120, 30, 20, &mut out);
        // interior (ignore clamped edges): high-frequency text-like pattern is flattened
        let mut mid: Vec<u8> = vec![];
        for y in 10..20 {
            for x in 40..80 {
                mid.extend_from_slice(&out[(y * 120 + x) * 4..(y * 120 + x) * 4 + 4]);
            }
        }
        assert!(contrast(&mid) < 30, "contrast {}", contrast(&mid));
        assert_eq!(out.len(), src.len());
    }

    #[test]
    fn safe_minimum_raises_strength() {
        let mut q = p(Style::Gaussian);
        q.intensity = 0;
        q.safe_minimum = false;
        assert_eq!(gaussian_radius(&q, 40), 1);
        q.safe_minimum = true;
        assert!(gaussian_radius(&q, 40) >= 24);
        q.style = Style::Pixelate;
        q.pixel_size = 3;
        assert!(pixel_block(&q, 40) >= 22);
    }

    #[test]
    fn pixelate_is_blocky() {
        let src = stripes(40, 40);
        let mut out = vec![];
        render(&p(Style::Pixelate), &src, 40, 40, 20, &mut out);
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(out[(y * 40 + x) * 4], out[0]);
            }
        }
    }

    #[test]
    fn solid_is_black_and_opacity_premultiplies() {
        let src = stripes(8, 8);
        let mut out = vec![];
        render(&p(Style::Solid), &src, 8, 8, 8, &mut out);
        assert!(out.chunks_exact(4).all(|a| a == [0, 0, 0, 255]));
        let mut q = p(Style::Solid);
        q.opacity = 50;
        render(&q, &src, 8, 8, 8, &mut out);
        assert!(out.chunks_exact(4).all(|a| a[3] == 127));
        let mut g = p(Style::Gaussian);
        g.opacity = 50;
        render(&g, &src, 8, 8, 8, &mut out);
        assert!(out.chunks_exact(4).all(|a| a[0] <= a[3]), "premultiplied invariant");
    }
}
