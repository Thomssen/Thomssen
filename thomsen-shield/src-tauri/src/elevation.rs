//! Runs *this same executable*, re-invoked with a hidden CLI flag, as
//! administrator for exactly one batch of work, then reads its result back
//! from a temp file. The main GUI process is never itself elevated - only
//! this narrow, short-lived, non-windowed re-invocation is, and only when a
//! protected action actually needs it. See `main.rs` for the other half
//! (the `--elevated-op` early-exit path this launches into) and
//! `firewall::elevated_ops` for what actually runs there.

use base64::Engine;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::error::{AppError, AppResult};

const ELEVATED_OP_FLAG: &str = "--elevated-op";
const ELEVATED_OUT_FLAG: &str = "--elevated-out";

pub fn is_elevated_child_invocation() -> bool {
    std::env::args().any(|a| a == ELEVATED_OP_FLAG)
}

/// Parses this process's own argv for the payload the parent encoded -
/// called only from the `main.rs` early-exit path.
pub fn parse_elevated_request<T: serde::de::DeserializeOwned>() -> AppResult<(T, std::path::PathBuf)> {
    let args: Vec<String> = std::env::args().collect();
    let payload = find_flag_value(&args, ELEVATED_OP_FLAG).ok_or_else(|| AppError::new("Missing elevated request payload"))?;
    let out_path = find_flag_value(&args, ELEVATED_OUT_FLAG).ok_or_else(|| AppError::new("Missing elevated response path"))?;

    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| AppError::new(format!("Malformed elevated request: {e}")))?;
    let request: T = serde_json::from_slice(&json)?;
    Ok((request, std::path::PathBuf::from(out_path)))
}

fn find_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

/// Runs `payload`, serialized to JSON, through an elevated re-invocation of
/// this executable and returns the response it wrote back. Fails clearly
/// (never silently) if the user declines the UAC prompt.
pub fn run_elevated<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(payload: &Req) -> AppResult<Resp> {
    let exe = std::env::current_exe().map_err(|e| AppError::new(format!("Couldn't locate Thomsen Shield's own executable: {e}")))?;

    let json = serde_json::to_vec(payload)?;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(json);

    let out_path = std::env::temp_dir().join(format!("thomsen-shield-elevated-{}.json", uuid::Uuid::new_v4()));

    let params = format!("{ELEVATED_OP_FLAG} {encoded} {ELEVATED_OUT_FLAG} \"{}\"", out_path.display());

    let exe_wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let params_wide: Vec<u16> = params.encode_utf16().chain(std::iter::once(0)).collect();
    let verb_wide: Vec<u16> = "runas\0".encode_utf16().collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: HWND::default(),
        lpVerb: PCWSTR(verb_wide.as_ptr()),
        lpFile: PCWSTR(exe_wide.as_ptr()),
        lpParameters: PCWSTR(params_wide.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
        // ERROR_CANCELLED is exactly "the user clicked No" on the UAC
        // prompt - the one case worth a friendlier message than the raw
        // HRESULT text.
        if e.code().0 == windows_error_cancelled() {
            AppError::new("Administrator permission is required for this action, and wasn't granted.")
        } else {
            AppError::new(format!("Couldn't start the elevated helper: {e}"))
        }
    })?;

    if info.hProcess == HANDLE::default() {
        return Err(AppError::new("Couldn't start the elevated helper."));
    }

    unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    unsafe { CloseHandle(info.hProcess) }.ok();

    let bytes = std::fs::read(&out_path).map_err(|_| AppError::new("The elevated action didn't report a result. It may not have completed."))?;
    let _ = std::fs::remove_file(&out_path);
    Ok(serde_json::from_slice(&bytes)?)
}

/// Called from the elevated child right before it exits: writes the result
/// the parent is waiting to read.
pub fn write_elevated_response<T: serde::Serialize>(out_path: &std::path::Path, response: &T) -> AppResult<()> {
    let json = serde_json::to_vec(response)?;
    std::fs::write(out_path, json)?;
    Ok(())
}

fn windows_error_cancelled() -> i32 {
    // HRESULT_FROM_WIN32(ERROR_CANCELLED)
    -2147023673 // 0x800704C7
}

use std::os::windows::ffi::OsStrExt;
