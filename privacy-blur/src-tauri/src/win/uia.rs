//! Text discovery through Windows UI Automation.
//!
//! For controls that implement the TextPattern (browsers' documents, editors, terminals,
//! Office) we ask for the *visible* text ranges and then for the exact bounding rectangles of
//! each match, which is pixel accurate. For plain controls (labels, links, list items,
//! buttons) UIA only gives the element's rectangle and its text, so the match position is
//! estimated proportionally; the OCR pass provides the precise rectangle there.
//!
//! UIA never reads secure/password edit fields' content (the OS refuses), and it cannot see
//! into elevated windows from a non-elevated process (UIPI) — OCR covers those.

use super::winlist::WinInfo;
use privacy_core::locate::{detect_line, words_in_rect};
use privacy_core::settings::Category;
use privacy_core::{Detector, Rect};
use std::ffi::c_void;
use std::time::{Duration, Instant};
use std::mem::ManuallyDrop;
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_BOOL, VT_BSTR, VT_I4};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Ole::{SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData};
use windows::Win32::UI::Accessibility::*;

#[allow(dead_code)]
pub struct UiaHit {
    pub rect: Rect,
    pub category: Category,
    pub hwnd: isize,
    /// true when the rectangle is an estimate (no per-character geometry available)
    pub approx: bool,
}

fn variant_i32(v: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 { vt: VT_I4, wReserved1: 0, wReserved2: 0, wReserved3: 0, Anonymous: VARIANT_0_0_0 { lVal: v } }),
        },
    }
}

fn variant_bool(v: bool) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_BOOL,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { boolVal: VARIANT_BOOL(if v { -1 } else { 0 }) },
            }),
        },
    }
}

fn variant_string(v: &VARIANT) -> Option<String> {
    unsafe {
        let inner = &v.Anonymous.Anonymous;
        if inner.vt == VT_BSTR {
            Some(inner.Anonymous.bstrVal.to_string())
        } else {
            None
        }
    }
}

pub struct Uia {
    auto: IUIAutomation,
    cond: IUIAutomationCondition,
    cache: IUIAutomationCacheRequest,
}

const MAX_ELEMENTS: i32 = 4000;
const MAX_TEXT: i32 = 20_000;
const WINDOW_BUDGET: Duration = Duration::from_millis(500);

impl Uia {
    /// COM must already be initialised (MTA) on the calling thread.
    pub fn new() -> Result<Self, String> {
        let e = |x: windows::core::Error| x.to_string();
        unsafe {
            let auto: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(e)?;
            let mut cond: IUIAutomationCondition =
                auto.CreatePropertyCondition(UIA_IsTextPatternAvailablePropertyId, &variant_bool(true)).map_err(e)?;
            for ct in [
                UIA_TextControlTypeId,
                UIA_HyperlinkControlTypeId,
                UIA_EditControlTypeId,
                UIA_ButtonControlTypeId,
                UIA_ListItemControlTypeId,
                UIA_TabItemControlTypeId,
                UIA_TreeItemControlTypeId,
                UIA_DataItemControlTypeId,
                UIA_MenuItemControlTypeId,
                UIA_DocumentControlTypeId,
            ] {
                let c = auto.CreatePropertyCondition(UIA_ControlTypePropertyId, &variant_i32(ct.0)).map_err(e)?;
                cond = auto.CreateOrCondition(&cond, &c).map_err(e)?;
            }
            let cache = auto.CreateCacheRequest().map_err(e)?;
            cache.AddProperty(UIA_NamePropertyId).map_err(e)?;
            cache.AddProperty(UIA_BoundingRectanglePropertyId).map_err(e)?;
            cache.AddProperty(UIA_ControlTypePropertyId).map_err(e)?;
            cache.AddProperty(UIA_IsOffscreenPropertyId).map_err(e)?;
            Ok(Self { auto, cond, cache })
        }
    }

    pub fn scan(&self, det: &Detector, windows: &[WinInfo]) -> Vec<UiaHit> {
        let mut out = vec![];
        for w in windows {
            self.scan_window(det, w, &mut out);
        }
        out
    }

    fn scan_window(&self, det: &Detector, w: &WinInfo, out: &mut Vec<UiaHit>) {
        unsafe {
            let Ok(root) = self.auto.ElementFromHandle(HWND(w.hwnd as *mut c_void)) else { return };
            let Ok(all) = root.FindAllBuildCache(TreeScope_Descendants, &self.cond, &self.cache) else { return };
            let n = all.Length().unwrap_or(0).min(MAX_ELEMENTS);
            let started = Instant::now();
            for i in 0..n {
                if started.elapsed() > WINDOW_BUDGET {
                    break;
                }
                let Ok(el) = all.GetElement(i) else { continue };
                if el.CachedIsOffscreen().map(|b| b.as_bool()).unwrap_or(true) {
                    continue;
                }
                let Ok(r) = el.CachedBoundingRectangle() else { continue };
                let rect = Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top);
                if rect.w < 6 || rect.h < 6 || w.rect.intersect(&rect).is_none() {
                    continue;
                }
                // Precise path: TextPattern with visible ranges.
                if self.scan_text_pattern(det, &el, w.hwnd, out) {
                    continue;
                }
                // Estimated path: label text of the element.
                let ct = el.CachedControlType().unwrap_or_default();
                let mut text = el.CachedName().map(|b| b.to_string()).unwrap_or_default();
                if ct == UIA_EditControlTypeId {
                    if let Ok(v) = el.GetCurrentPropertyValue(UIA_ValueValuePropertyId) {
                        if let Some(s) = variant_string(&v) {
                            text.push(' ');
                            text.push_str(&s);
                        }
                    }
                }
                let text = text.trim();
                if text.is_empty() || text.chars().count() > 600 {
                    continue;
                }
                self.scan_label(det, text, rect, w.hwnd, out);
            }
        }
    }

    fn scan_label(&self, det: &Detector, text: &str, rect: Rect, hwnd: isize, out: &mut Vec<UiaHit>) {
        let single_line = rect.h <= 44 && !text.contains('\n');
        if single_line {
            let words = words_in_rect(text, rect);
            for (r, category) in detect_line(det, &words) {
                // proportional estimates are off for proportional fonts: widen a little
                let pad = (r.h / 4).max(3);
                out.push(UiaHit { rect: r.inflate(pad, 1), category, hwnd, approx: true });
            }
        } else {
            // Multi-line label: we cannot place the match, so cover the element only when the
            // match makes up most of it.
            let hits = det.detect(text);
            let covered: usize = hits.iter().map(|h| h.end - h.start).sum();
            if let Some(h) = hits.first() {
                if covered * 2 >= text.len() {
                    out.push(UiaHit { rect, category: h.category, hwnd, approx: true });
                }
            }
        }
    }

    /// Returns true when the element exposed a TextPattern (even if nothing was found).
    unsafe fn scan_text_pattern(&self, det: &Detector, el: &IUIAutomationElement, hwnd: isize, out: &mut Vec<UiaHit>) -> bool {
        let Ok(pat) = el.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) else { return false };
        let Ok(ranges) = pat.GetVisibleRanges() else { return true };
        let n = ranges.Length().unwrap_or(0).min(64);
        for i in 0..n {
            let Ok(range) = ranges.GetElement(i) else { continue };
            let Ok(bs) = range.GetText(MAX_TEXT) else { continue };
            let text = bs.to_string();
            if text.trim().is_empty() {
                continue;
            }
            for hit in det.detect(&text).into_iter().take(40) {
                let cs = text[..hit.start].encode_utf16().count() as i32;
                let len = text[hit.start..hit.end].encode_utf16().count() as i32;
                let Some(rects) = sub_range_rects(&range, cs, len, &text[hit.start..hit.end]) else { continue };
                for r in rects {
                    out.push(UiaHit { rect: r, category: hit.category, hwnd, approx: false });
                }
            }
        }
        true
    }
}

/// Bounding rectangles (one per line) of the characters `[start, start+len)` of `range`.
/// Returns `None` if UIA's idea of the text differs from the expected one (surrogates, CRLF…),
/// in which case the OCR pass is relied on instead of guessing.
unsafe fn sub_range_rects(range: &IUIAutomationTextRange, start: i32, len: i32, expected: &str) -> Option<Vec<Rect>> {
    let sub = range.Clone().ok()?;
    sub.MoveEndpointByRange(TextPatternRangeEndpoint_End, &sub, TextPatternRangeEndpoint_Start).ok()?;
    sub.MoveEndpointByUnit(TextPatternRangeEndpoint_End, TextUnit_Character, start + len).ok()?;
    sub.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, start).ok()?;
    let got = sub.GetText(len + 8).ok()?.to_string();
    if got.trim() != expected.trim() {
        return None;
    }
    let sa = sub.GetBoundingRectangles().ok()?;
    let vals = read_f64_array(sa)?;
    let rects: Vec<Rect> = vals
        .chunks_exact(4)
        .map(|c| Rect::new(c[0].floor() as i32, c[1].floor() as i32, c[2].ceil() as i32, c[3].ceil() as i32))
        .filter(|r| !r.is_empty())
        .collect();
    if rects.is_empty() {
        None
    } else {
        Some(rects)
    }
}

unsafe fn read_f64_array(sa: *mut windows::Win32::System::Com::SAFEARRAY) -> Option<Vec<f64>> {
    if sa.is_null() {
        return None;
    }
    let lb = SafeArrayGetLBound(sa, 1).ok()?;
    let ub = SafeArrayGetUBound(sa, 1).ok()?;
    let n = (ub - lb + 1).max(0) as usize;
    let mut p: *mut c_void = std::ptr::null_mut();
    let mut v = None;
    if SafeArrayAccessData(sa, &mut p).is_ok() && !p.is_null() {
        v = Some(std::slice::from_raw_parts(p as *const f64, n).to_vec());
        let _ = SafeArrayUnaccessData(sa);
    }
    let _ = SafeArrayDestroy(sa);
    v
}
