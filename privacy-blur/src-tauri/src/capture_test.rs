//! Measures — on this very machine — which capture paths record our overlay.
//! Nothing here is assumed: each result comes from a real capture of a real test patch.

use crate::win::overlay::{pump_messages, OverlayPool};
use serde::Serialize;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

#[derive(Serialize)]
pub struct PathResult {
    pub id: &'static str,
    pub name: &'static str,
    /// `Some(true)` = the overlay (and therefore the blur) appeared in this capture.
    pub overlay_visible: Option<bool>,
    pub verified: bool,
    pub detail: String,
}

const PW_RENDERFULLCONTENT: u32 = 2;

fn is_magenta(b: u8, g: u8, r: u8) -> bool {
    r > 200 && b > 200 && g < 70
}

unsafe fn grab_center(screen: HDC, x: i32, y: i32, w: i32, h: i32, flags: ROP_CODE) -> Option<bool> {
    let dc = CreateCompatibleDC(Some(screen));
    let bmp = CreateCompatibleBitmap(screen, w, h);
    let old = SelectObject(dc, bmp.into());
    let ok = BitBlt(dc, 0, 0, w, h, Some(screen), x, y, flags).is_ok();
    let c = if ok { Some(GetPixel(dc, w / 2, h / 2)) } else { None };
    SelectObject(dc, old);
    let _ = DeleteObject(bmp.into());
    let _ = DeleteDC(dc);
    c.map(|c| {
        let v = c.0;
        is_magenta((v >> 16) as u8, (v >> 8) as u8, v as u8)
    })
}

unsafe fn print_window_center(screen: HDC, target: HWND, px: i32, py: i32) -> Option<bool> {
    let mut r = windows::Win32::Foundation::RECT::default();
    GetWindowRect(target, &mut r).ok()?;
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w <= 0 || h <= 0 {
        return None;
    }
    let dc = CreateCompatibleDC(Some(screen));
    let bmp = CreateCompatibleBitmap(screen, w, h);
    let old = SelectObject(dc, bmp.into());
    let ok = PrintWindow(target, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)).as_bool();
    let res = if ok { Some(GetPixel(dc, px - r.left, py - r.top)) } else { None };
    SelectObject(dc, old);
    let _ = DeleteObject(bmp.into());
    let _ = DeleteDC(dc);
    res.map(|c| {
        let v = c.0;
        is_magenta((v >> 16) as u8, (v >> 8) as u8, v as u8)
    })
}

fn pump_for(d: Duration) {
    let t = Instant::now();
    while t.elapsed() < d {
        pump_messages();
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// `main_hwnd`: our own settings window, used as the "window capture" subject.
/// `hold_secs`: keep the patch on screen afterwards so the user can look at it in OBS/Discord/…
pub fn run(main_hwnd: Option<isize>, overlay_in_captures: bool, hold_secs: u32) -> Vec<PathResult> {
    let mut out = vec![];
    unsafe {
        let screen = GetDC(None);
        let (mut px, mut py) = (60, 60);
        let mut win: Option<HWND> = None;
        if let Some(h) = main_hwnd {
            let hw = HWND(h as *mut _);
            let mut r = windows::Win32::Foundation::RECT::default();
            if windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(hw).as_bool() && GetWindowRect(hw, &mut r).is_ok() && r.right - r.left > 200 {
                px = r.left + 80;
                py = r.top + 80;
                win = Some(hw);
            }
        }
        let (w, h) = (160, 90);
        let patch: Vec<u8> = std::iter::repeat([255u8, 0, 255, 255]).take((w * h) as usize).flatten().collect();
        let mut pool = OverlayPool::new(!overlay_in_captures);
        pool.present(0, px, py, w, h, &patch);
        pump_for(Duration::from_millis(500)); // let DWM compose it

        let (cx, cy) = (px, py);
        let a = grab_center(screen, cx, cy, w, h, SRCCOPY | CAPTUREBLT);
        out.push(PathResult {
            id: "gdi_display",
            name: "Display capture via GDI (BitBlt with CAPTUREBLT)",
            overlay_visible: a,
            verified: a.is_some(),
            detail: "Same composition level as screen-recording tools that copy the whole desktop.".into(),
        });
        let b = grab_center(screen, cx, cy, w, h, SRCCOPY);
        out.push(PathResult {
            id: "gdi_plain",
            name: "Privacy Blur's own detector capture (BitBlt, layered windows excluded)",
            overlay_visible: b,
            verified: b.is_some(),
            detail: "Must be 'not visible': otherwise the detector would see its own blur and flicker.".into(),
        });
        match win {
            Some(hw) => {
                let c = print_window_center(screen, hw, px + w / 2, py + h / 2);
                out.push(PathResult {
                    id: "window_capture",
                    name: "Single-window capture (PrintWindow of the Privacy Blur window)",
                    overlay_visible: c,
                    verified: c.is_some(),
                    detail: "Window-based capture (Discord/Teams 'Window', OBS 'Window Capture') renders only that window, so overlays above it are not included.".into(),
                });
            }
            None => out.push(PathResult {
                id: "window_capture",
                name: "Single-window capture",
                overlay_visible: None,
                verified: false,
                detail: "Not tested: the Privacy Blur window must be visible on screen to act as the test subject.".into(),
            }),
        }
        for (id, name) in [
            ("dxgi_wgc", "DXGI Desktop Duplication / Windows Graphics Capture of a monitor (OBS 'Display Capture', Discord/Teams 'Screen')"),
            ("apps", "OBS, Discord, Teams, Zoom"),
        ] {
            out.push(PathResult {
                id,
                name,
                overlay_visible: None,
                verified: false,
                detail: "Not measured by the built-in test. Use the on-screen test patch (hold option) in that application and look at its preview before relying on it.".into(),
            });
        }
        if hold_secs > 0 {
            pump_for(Duration::from_secs(hold_secs.min(120) as u64));
        }
        pool.hide_from(0);
        drop(pool);
        ReleaseDC(None, screen);
    }
    out
}
