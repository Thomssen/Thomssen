//! Click-through, always-on-top overlay windows.
//!
//! One small layered window per censored region (pooled and reused). Each window shows a
//! pre-rendered premultiplied-alpha bitmap through `UpdateLayeredWindow`, so there is no
//! WebView, no GPU surface and no full-screen buffer: memory is proportional to the area
//! actually censored. `WS_EX_TRANSPARENT | WS_EX_LAYERED` makes mouse input fall through to the
//! window below, even across processes; `WS_EX_NOACTIVATE` keeps keyboard focus where it was.

use std::ffi::c_void;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const OVERLAY_CLASS: PCWSTR = w!("PrivacyBlurOverlay");
const PANIC_CLASS: PCWSTR = w!("PrivacyBlurPanic");
const WDA_EXCLUDEFROMCAPTURE: WINDOW_DISPLAY_AFFINITY = WINDOW_DISPLAY_AFFINITY(0x11);

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCHITTEST {
        return LRESULT(-1); // HTTRANSPARENT
    }
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(3); // MA_NOACTIVATE
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

fn register_classes() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let hinst = HINSTANCE(GetModuleHandleW(None).map(|m| m.0).unwrap_or_default());
        for (name, brush) in [(OVERLAY_CLASS, HBRUSH::default()), (PANIC_CLASS, HBRUSH(GetStockObject(BLACK_BRUSH).0))] {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wndproc),
                hInstance: hinst,
                lpszClassName: name,
                hbrBackground: brush,
                hCursor: HCURSOR::default(),
                ..Default::default()
            };
            RegisterClassExW(&wc);
        }
    });
}

fn create_window(class: PCWSTR) -> Option<HWND> {
    register_classes();
    unsafe {
        let hinst = HINSTANCE(GetModuleHandleW(None).ok()?.0);
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(hinst),
            None,
        )
        .ok()
    }
}

fn set_affinity(hwnd: HWND, excluded: bool) {
    unsafe {
        let _ = SetWindowDisplayAffinity(hwnd, if excluded { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE });
    }
}

/// Process pending window messages for the calling thread (windows must be pumped by their creator).
pub fn pump_messages() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

struct Overlay {
    hwnd: HWND,
    mem: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    w: i32,
    h: i32,
    visible: bool,
}

impl Overlay {
    fn new(excluded: bool) -> Option<Self> {
        let hwnd = create_window(OVERLAY_CLASS)?;
        set_affinity(hwnd, excluded);
        unsafe {
            let screen = GetDC(None);
            let mem = CreateCompatibleDC(Some(screen));
            ReleaseDC(None, screen);
            Some(Self { hwnd, mem, bmp: HBITMAP::default(), old: HGDIOBJ::default(), bits: std::ptr::null_mut(), w: 0, h: 0, visible: false })
        }
    }

    fn ensure_bitmap(&mut self, w: i32, h: i32) -> bool {
        if self.w == w && self.h == h && !self.bits.is_null() {
            return true;
        }
        unsafe {
            if !self.bmp.is_invalid() {
                SelectObject(self.mem, self.old);
                let _ = DeleteObject(self.bmp.into());
                self.bmp = HBITMAP::default();
                self.bits = std::ptr::null_mut();
            }
            let mut bi = BITMAPINFO::default();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -h;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;
            let mut bits: *mut c_void = std::ptr::null_mut();
            let Ok(bmp) = CreateDIBSection(Some(self.mem), &bi, DIB_RGB_COLORS, &mut bits, None, 0) else { return false };
            if bits.is_null() {
                return false;
            }
            self.old = SelectObject(self.mem, bmp.into());
            self.bmp = bmp;
            self.bits = bits as *mut u8;
            self.w = w;
            self.h = h;
        }
        true
    }

    /// `px` must be premultiplied BGRA, `w*h*4` bytes.
    fn present(&mut self, x: i32, y: i32, w: i32, h: i32, px: &[u8]) {
        if w <= 0 || h <= 0 || px.len() < (w * h * 4) as usize || !self.ensure_bitmap(w, h) {
            return;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(px.as_ptr(), self.bits, (w * h * 4) as usize);
            if !self.visible {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                self.visible = true;
            }
            let pos = POINT { x, y };
            let size = SIZE { cx: w, cy: h };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = UpdateLayeredWindow(self.hwnd, None, Some(&pos), Some(&size), Some(self.mem), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        }
    }

    fn hide(&mut self) {
        if self.visible {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
            self.visible = false;
        }
    }

    fn raise(&self) {
        if self.visible {
            unsafe {
                let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            if !self.bmp.is_invalid() {
                SelectObject(self.mem, self.old);
                let _ = DeleteObject(self.bmp.into());
            }
            let _ = DeleteDC(self.mem);
        }
    }
}

/// Pool of region overlays. Must be used from the thread that created it.
pub struct OverlayPool {
    wins: Vec<Overlay>,
    excluded: bool,
}

impl OverlayPool {
    pub fn new(excluded: bool) -> Self {
        Self { wins: vec![], excluded }
    }

    /// `true` hides the overlays from every capture API (WDA_EXCLUDEFROMCAPTURE).
    pub fn set_excluded(&mut self, excluded: bool) {
        if self.excluded != excluded {
            self.excluded = excluded;
            for o in &self.wins {
                set_affinity(o.hwnd, excluded);
            }
        }
    }

    pub fn present(&mut self, idx: usize, x: i32, y: i32, w: i32, h: i32, px: &[u8]) {
        while self.wins.len() <= idx {
            match Overlay::new(self.excluded) {
                Some(o) => self.wins.push(o),
                None => return,
            }
        }
        self.wins[idx].present(x, y, w, h, px);
    }

    pub fn hide_from(&mut self, first: usize) {
        for o in self.wins.iter_mut().skip(first) {
            o.hide();
        }
    }

    pub fn raise_all(&self) {
        for o in &self.wins {
            o.raise();
        }
    }

    /// Release surplus windows so an idle app holds almost no memory.
    pub fn trim(&mut self, keep: usize) {
        if self.wins.len() > keep + 8 {
            self.wins.truncate(keep + 4);
        }
    }
}

/// Opaque black cover over every monitor (panic button). Click-through so the user can
/// still reach the tray / press the shortcut again; always part of screen captures.
#[derive(Default)]
pub struct PanicCover {
    wins: Vec<HWND>,
}

impl PanicCover {
    pub fn visible(&self) -> bool {
        !self.wins.is_empty()
    }

    pub fn show(&mut self, rects: &[privacy_core::Rect]) {
        self.hide();
        for r in rects {
            let Some(hwnd) = create_window(PANIC_CLASS) else { continue };
            unsafe {
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
                set_affinity(hwnd, false);
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), r.x, r.y, r.w, r.h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            }
            self.wins.push(hwnd);
        }
    }

    pub fn hide(&mut self) {
        for h in self.wins.drain(..) {
            unsafe {
                let _ = DestroyWindow(h);
            }
        }
    }

    pub fn raise(&self) {
        for &h in &self.wins {
            unsafe {
                let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
}

impl Drop for PanicCover {
    fn drop(&mut self) {
        self.hide();
    }
}
