//! Local OCR through the Windows.Media.Ocr engine that ships with Windows 10/11.
//! No network access, no model download; languages come from the user's installed language packs.

use privacy_core::locate::Word;
use privacy_core::Rect;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

pub struct Ocr {
    engine: OcrEngine,
    pub language: String,
    max_dim: u32,
}

impl Ocr {
    pub fn new() -> Result<Self, String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| {
            "No OCR language available. Add a language in Windows Settings → Time & language → Language & region."
                .to_string()
        })?;
        let language = engine.RecognizerLanguage().ok().and_then(|l| l.LanguageTag().ok()).map(|t| t.to_string()).unwrap_or_default();
        let max_dim = OcrEngine::MaxImageDimension().unwrap_or(10240).max(1024);
        Ok(Self { engine, language, max_dim })
    }

    pub fn max_dimension(&self) -> u32 {
        self.max_dim
    }

    /// Recognise opaque-or-not BGRA pixels. Returns lines of words in image pixel coordinates.
    pub fn recognize(&self, bgra: &[u8], w: u32, h: u32) -> Result<Vec<Vec<Word>>, String> {
        if w < 8 || h < 8 || bgra.len() < (w * h * 4) as usize {
            return Ok(vec![]);
        }
        let err = |e: windows::core::Error| e.to_string();
        let writer = DataWriter::new().map_err(err)?;
        writer.WriteBytes(&bgra[..(w * h * 4) as usize]).map_err(err)?;
        let buf = writer.DetachBuffer().map_err(err)?;
        let bmp = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(&buf, BitmapPixelFormat::Bgra8, w as i32, h as i32, BitmapAlphaMode::Ignore)
            .map_err(err)?;
        let res = self.engine.RecognizeAsync(&bmp).map_err(err)?.get().map_err(err)?;
        let mut lines = vec![];
        for line in res.Lines().map_err(err)? {
            let mut words = vec![];
            for wd in line.Words().map_err(err)? {
                let r = wd.BoundingRect().map_err(err)?;
                let text = wd.Text().map_err(err)?.to_string();
                words.push(Word {
                    text,
                    rect: Rect::new(r.X.floor() as i32, r.Y.floor() as i32, (r.Width.ceil() as i32).max(1), (r.Height.ceil() as i32).max(1)),
                });
            }
            if !words.is_empty() {
                lines.push(words);
            }
        }
        Ok(lines)
    }
}

/// Copy `region` of a `src_w`-wide BGRA image, scaled by `scale_pct`/100 with box averaging.
/// Returns `(pixels, width, height)`.
pub fn crop_scaled(src: &[u8], src_w: usize, region: Rect, scale_pct: u32) -> (Vec<u8>, u32, u32) {
    let (rw, rh) = (region.w as usize, region.h as usize);
    if scale_pct >= 100 {
        let mut out = Vec::with_capacity(rw * rh * 4);
        for y in 0..rh {
            let s = ((region.y as usize + y) * src_w + region.x as usize) * 4;
            out.extend_from_slice(&src[s..s + rw * 4]);
        }
        return (out, rw as u32, rh as u32);
    }
    let f = scale_pct as f32 / 100.0;
    let (ow, oh) = (((rw as f32 * f) as usize).max(1), ((rh as f32 * f) as usize).max(1));
    let mut out = vec![255u8; ow * oh * 4];
    for oy in 0..oh {
        let y0 = oy * rh / oh;
        let y1 = (((oy + 1) * rh).div_ceil(oh)).max(y0 + 1).min(rh);
        for ox in 0..ow {
            let x0 = ox * rw / ow;
            let x1 = (((ox + 1) * rw).div_ceil(ow)).max(x0 + 1).min(rw);
            let mut acc = [0u32; 3];
            for y in y0..y1 {
                let base = ((region.y as usize + y) * src_w + region.x as usize) * 4;
                for x in x0..x1 {
                    let p = base + x * 4;
                    acc[0] += src[p] as u32;
                    acc[1] += src[p + 1] as u32;
                    acc[2] += src[p + 2] as u32;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            let o = (oy * ow + ox) * 4;
            out[o] = (acc[0] / n) as u8;
            out[o + 1] = (acc[1] / n) as u8;
            out[o + 2] = (acc[2] / n) as u8;
        }
    }
    (out, ow as u32, oh as u32)
}
