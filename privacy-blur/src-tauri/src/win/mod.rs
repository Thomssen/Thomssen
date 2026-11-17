//! Windows-specific platform layer. Everything that touches Win32/WinRT lives here.

pub mod capture;
pub mod dpapi;
pub mod monitors;
pub mod ocr;
pub mod overlay;
pub mod uia;
pub mod winlist;

use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

/// Make every coordinate we deal with a physical pixel on every monitor and scaling factor.
/// Must run before any window is created. Failure just means the manifest already did it.
pub fn init_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
