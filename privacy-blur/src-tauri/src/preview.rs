//! Renders the live style preview with the *same* effect code the overlay uses, so what the
//! Appearance page shows is exactly what the screen will show.

use privacy_core::effects::{self, Params};
use privacy_core::settings::Appearance;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Gdi::*;

pub const W: usize = 560;
pub const H: usize = 116;

/// Returns `[w: u32 LE][h: u32 LE][RGBA pixels]`.
pub fn render(a: &Appearance) -> Vec<u8> {
    let mut px = vec![0u8; W * H * 4];
    let mut regions: Vec<(i32, i32, i32, i32)> = vec![];
    let mut text_h = 22usize;
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = W as i32;
        bi.bmiHeader.biHeight = -(H as i32);
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;
        let mut bits = std::ptr::null_mut();
        if let Ok(bmp) = CreateDIBSection(Some(screen), &bi, DIB_RGB_COLORS, &mut bits, None, 0) {
            let old = SelectObject(dc, bmp.into());
            let buf = std::slice::from_raw_parts_mut(bits as *mut u8, W * H * 4);
            for p in buf.chunks_exact_mut(4) {
                p.copy_from_slice(&[27, 24, 24, 255]); // BGRA #181818
            }
            let font = CreateFontW(-22, 0, 0, 0, 400, 0, 0, 0, FONT_CHARSET(1), FONT_OUTPUT_PRECISION(0), FONT_CLIP_PRECISION(0), FONT_QUALITY(5), 0, w!("Segoe UI"));
            let oldf = SelectObject(dc, font.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00EE_EEEE));
            let mut tm = TEXTMETRICW::default();
            if GetTextMetricsW(dc, &mut tm).as_bool() {
                text_h = tm.tmHeight.max(8) as usize;
            }
            let lines: [(&str, &str, &str); 2] = [
                ("Signed in as ", "jane.doe@example.com", ""),
                ("Call ", "+1 (555) 012-3456", " about order #4821"),
            ];
            for (i, (a_txt, b_txt, c_txt)) in lines.iter().enumerate() {
                let y = 18 + i as i32 * 44;
                let mut x = 20;
                for (k, part) in [a_txt, b_txt, c_txt].iter().enumerate() {
                    if part.is_empty() {
                        continue;
                    }
                    let wide: Vec<u16> = part.encode_utf16().collect();
                    let mut sz = SIZE::default();
                    let _ = GetTextExtentPoint32W(dc, &wide, &mut sz);
                    let _ = TextOutW(dc, x, y, &wide);
                    if k == 1 {
                        regions.push((x, y, sz.cx, sz.cy));
                    }
                    x += sz.cx;
                }
            }
            SelectObject(dc, oldf);
            let _ = DeleteObject(font.into());
            px.copy_from_slice(buf);
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
        }
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
    }
    for p in px.chunks_exact_mut(4) {
        p[3] = 255;
    }
    // apply the real effect to each region (with the configured padding)
    let params = Params::from(a);
    let pad = a.padding as i32;
    let mut out = vec![];
    for (x, y, w, h) in regions {
        let x0 = (x - pad).max(0);
        let y0 = (y - pad / 2 - 1).max(0);
        let x1 = (x + w + pad).min(W as i32);
        let y1 = (y + h + pad / 2 + 1).min(H as i32);
        let (cw, ch) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut crop = Vec::with_capacity(cw * ch * 4);
        for row in 0..ch {
            let s = ((y0 as usize + row) * W + x0 as usize) * 4;
            crop.extend_from_slice(&px[s..s + cw * 4]);
        }
        effects::render(&params, &crop, cw, ch, text_h, &mut out);
        // compose premultiplied result over the opaque background
        for row in 0..ch {
            for col in 0..cw {
                let s = (row * cw + col) * 4;
                let d = ((y0 as usize + row) * W + x0 as usize + col) * 4;
                let inv = 255 - out[s + 3] as u32;
                for c in 0..3 {
                    px[d + c] = (out[s + c] as u32 + px[d + c] as u32 * inv / 255).min(255) as u8;
                }
            }
        }
    }
    let mut res = Vec::with_capacity(8 + W * H * 4);
    res.extend_from_slice(&(W as u32).to_le_bytes());
    res.extend_from_slice(&(H as u32).to_le_bytes());
    for p in px.chunks_exact(4) {
        res.extend_from_slice(&[p[2], p[1], p[0], 255]);
    }
    res
}
