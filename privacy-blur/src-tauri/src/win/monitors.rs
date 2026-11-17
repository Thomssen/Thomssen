use privacy_core::Rect;
use serde::Serialize;
use std::mem::size_of;
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonitorInfo {
    /// Stable device name such as `\\.\DISPLAY1`
    pub name: String,
    /// Position in virtual-screen physical pixels
    pub rect: Rect,
    pub scale_percent: u32,
    pub primary: bool,
}

unsafe extern "system" fn cb(hm: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
    let v = &mut *(lp.0 as *mut Vec<MonitorInfo>);
    let mut mi = MONITORINFOEXW::default();
    mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hm, &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool() {
        let r = mi.monitorInfo.rcMonitor;
        let len = mi.szDevice.iter().position(|&c| c == 0).unwrap_or(mi.szDevice.len());
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(hm, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        v.push(MonitorInfo {
            name: String::from_utf16_lossy(&mi.szDevice[..len]),
            rect: Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top),
            scale_percent: dx * 100 / 96,
            primary: mi.monitorInfo.dwFlags & 1 != 0,
        });
    }
    BOOL(1)
}

pub fn list() -> Vec<MonitorInfo> {
    let mut v: Vec<MonitorInfo> = vec![];
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut v as *mut Vec<MonitorInfo> as isize));
    }
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}
