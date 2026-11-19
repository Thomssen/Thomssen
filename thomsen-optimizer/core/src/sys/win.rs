//! Small Windows-only helpers with harmless fallbacks on other platforms.

use crate::error::{Error, Result};
use std::collections::HashSet;
use std::path::Path;

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::{SHEmptyRecycleBinW, SHQueryRecycleBinW, ShellExecuteW, SHQUERYRBINFO};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindow, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, GW_OWNER, SW_SHOWNORMAL, WM_CLOSE};

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Visible, titled, un-owned top-level windows: what the user would call
    /// "an open application window".
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = &mut *(lparam as *mut Vec<(HWND, u32)>);
        if IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() && GetWindowTextLengthW(hwnd) > 0 {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            out.push((hwnd, pid));
        }
        1
    }

    fn windows() -> Vec<(HWND, u32)> {
        let mut v: Vec<(HWND, u32)> = Vec::new();
        unsafe {
            EnumWindows(Some(collect), &mut v as *mut _ as LPARAM);
        }
        v
    }

    pub fn visible_window_pids() -> HashSet<u32> {
        windows().into_iter().map(|(_, p)| p).collect()
    }

    pub fn request_close(pid: u32) -> usize {
        let mut n = 0;
        for (hwnd, p) in windows() {
            if p == pid && unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) } != 0 {
                n += 1;
            }
        }
        n
    }

    pub fn recycle_bin_info() -> Option<(u64, u64)> {
        let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
        let hr = unsafe { SHQueryRecycleBinW(null(), &mut info) };
        (hr >= 0).then(|| (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
    }

    pub fn empty_recycle_bin() -> Result<()> {
        // SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND
        let hr = unsafe { SHEmptyRecycleBinW(null_mut(), null(), 0x7) };
        // S_FALSE (1) is returned when the bin was already empty.
        if hr >= 0 { Ok(()) } else { Err(Error::Command(format!("Windows could not empty the Recycle Bin (0x{:08x}).", hr as u32))) }
    }

    pub fn company_name(path: &Path) -> Option<String> {
        let w: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        unsafe {
            let mut handle = 0u32;
            let size = GetFileVersionInfoSizeW(w.as_ptr(), &mut handle);
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            if GetFileVersionInfoW(w.as_ptr(), 0, size, buf.as_mut_ptr().cast()) == 0 {
                return None;
            }
            let mut p: *mut core::ffi::c_void = null_mut();
            let mut len = 0u32;
            let trans = wide("\\VarFileInfo\\Translation");
            let (lang, cp) = if VerQueryValueW(buf.as_ptr().cast(), trans.as_ptr(), &mut p, &mut len) != 0 && len >= 4 {
                let t = std::slice::from_raw_parts(p as *const u16, 2);
                (t[0], t[1])
            } else {
                (0x0409, 0x04b0)
            };
            let q = wide(&format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\CompanyName"));
            if VerQueryValueW(buf.as_ptr().cast(), q.as_ptr(), &mut p, &mut len) != 0 && len > 1 {
                let s = std::slice::from_raw_parts(p as *const u16, (len - 1) as usize);
                let name = String::from_utf16_lossy(s).trim().to_string();
                return (!name.is_empty()).then_some(name);
            }
        }
        None
    }

    /// True when running on battery (AC line status "offline").
    pub fn on_battery() -> bool {
        unsafe {
            let mut st: SYSTEM_POWER_STATUS = std::mem::zeroed();
            GetSystemPowerStatus(&mut st) != 0 && st.ACLineStatus == 0
        }
    }

    pub fn is_elevated() -> bool {
        unsafe {
            let mut token = null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut el = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut ret = 0u32;
            let ok = GetTokenInformation(token, TokenElevation, (&mut el as *mut TOKEN_ELEVATION).cast(), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut ret);
            CloseHandle(token);
            ok != 0 && el.TokenIsElevated != 0
        }
    }

    /// Starts `exe` elevated (UAC prompt). Returns Ok if the user accepted.
    pub fn shell_runas(exe: &Path, args: &str) -> Result<()> {
        let verb = wide("runas");
        let file: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let params = wide(args);
        let r = unsafe { ShellExecuteW(null_mut(), verb.as_ptr(), file.as_ptr(), params.as_ptr(), null(), SW_SHOWNORMAL as i32) };
        if (r as isize) > 32 { Ok(()) } else { Err(Error::PermissionDenied("Administrator approval was declined or unavailable.".into())) }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn visible_window_pids() -> HashSet<u32> {
        HashSet::new()
    }
    pub fn request_close(_: u32) -> usize {
        0
    }
    pub fn recycle_bin_info() -> Option<(u64, u64)> {
        None
    }
    pub fn empty_recycle_bin() -> Result<()> {
        Err(Error::Unsupported("The Recycle Bin is only available on Windows.".into()))
    }
    pub fn company_name(_: &Path) -> Option<String> {
        None
    }
    pub fn on_battery() -> bool {
        false
    }
    pub fn is_elevated() -> bool {
        false
    }
    pub fn shell_runas(_: &Path, _: &str) -> Result<()> {
        Err(Error::Unsupported("Elevation is only available on Windows.".into()))
    }
}

pub use imp::*;
