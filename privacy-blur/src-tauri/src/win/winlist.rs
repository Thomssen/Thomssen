//! Visible top-level windows in z-order, their owning executables, and process names.

use parking_lot::Mutex;
use privacy_core::Rect;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Debug, Serialize)]
pub struct WinInfo {
    pub hwnd: isize,
    pub rect: Rect,
    pub pid: u32,
    /// lower-case executable file name, e.g. `discord.exe`
    pub exe: String,
    pub foreground: bool,
}

static EXE_CACHE: LazyLock<Mutex<HashMap<u32, String>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn exe_of_pid(pid: u32) -> String {
    if let Some(s) = EXE_CACHE.lock().get(&pid) {
        return s.clone();
    }
    let mut name = String::new();
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                name = full.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
            }
            let _ = CloseHandle(h);
        }
    }
    let mut c = EXE_CACHE.lock();
    if c.len() > 1000 {
        c.clear(); // pids get reused; keep the cache small and fresh
    }
    c.insert(pid, name.clone());
    name
}

struct Ctx {
    out: Vec<WinInfo>,
    fg: isize,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lp: LPARAM) -> BOOL {
    let ctx = &mut *(lp.0 as *mut Ctx);
    if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
        return BOOL(1);
    }
    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    // Click-through layered windows are overlays (ours, screen annotators, game HUDs): never "content".
    if ex & WS_EX_LAYERED.0 != 0 && ex & WS_EX_TRANSPARENT.0 != 0 {
        return BOOL(1);
    }
    let mut cloaked = 0u32;
    let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
    if cloaked != 0 {
        return BOOL(1);
    }
    let mut r = RECT::default();
    if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut RECT as *mut _, std::mem::size_of::<RECT>() as u32).is_err() {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    let rect = Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top);
    if rect.w < 8 || rect.h < 8 {
        return BOOL(1);
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    ctx.out.push(WinInfo { hwnd: hwnd.0 as isize, rect, pid, exe: exe_of_pid(pid), foreground: hwnd.0 as isize == ctx.fg });
    BOOL(1)
}

/// Visible, non-minimised, non-cloaked windows from top to bottom of the z-order.
pub fn list_windows() -> Vec<WinInfo> {
    let mut ctx = Ctx { out: vec![], fg: unsafe { GetForegroundWindow().0 as isize } };
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.out
}

/// First (top-most) window containing the point.
pub fn owner_at(wins: &[WinInfo], x: i32, y: i32) -> Option<&WinInfo> {
    wins.iter().find(|w| w.rect.contains_point(x, y))
}

/// Executable names of all running processes (lower-case, deduplicated).
pub fn running_process_names() -> Vec<String> {
    let mut out = vec![];
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                out.push(String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase());
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out.sort();
    out.dedup();
    out
}
