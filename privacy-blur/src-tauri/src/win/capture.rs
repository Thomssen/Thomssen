//! Screen capture through GDI `BitBlt` plus cheap tile-based change detection.
//!
//! Why GDI and not Windows.Graphics.Capture: our own click-through overlays are
//! `WS_EX_LAYERED` windows, and `BitBlt(SRCCOPY)` from the screen DC leaves layered windows
//! out. That gives the detector an *overlay-free* picture of the desktop without hiding the
//! overlay (no flicker) and without per-window exclusion tricks. `capture_test` verifies this
//! assumption on the user's machine instead of trusting it.

use super::monitors::MonitorInfo;
use privacy_core::Rect;
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;

pub const TILE: usize = 64;

pub struct MonitorCapture {
    pub info: MonitorInfo,
    pub w: usize,
    pub h: usize,
    hdc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    tiles: Vec<u64>,
    tiles_x: usize,
}

// The raw pointer is only ever used from the thread that owns the struct.
unsafe impl Send for MonitorCapture {}

impl MonitorCapture {
    pub fn new(info: MonitorInfo) -> Option<Self> {
        let (w, h) = (info.rect.w.max(1) as usize, info.rect.h.max(1) as usize);
        unsafe {
            let screen = GetDC(None);
            let hdc = CreateCompatibleDC(Some(screen));
            let mut bi = BITMAPINFO::default();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w as i32;
            bi.bmiHeader.biHeight = -(h as i32); // top-down
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bmp = CreateDIBSection(Some(screen), &bi, DIB_RGB_COLORS, &mut bits, None, 0);
            ReleaseDC(None, screen);
            let bmp = bmp.ok()?;
            if bits.is_null() {
                let _ = DeleteObject(bmp.into());
                let _ = DeleteDC(hdc);
                return None;
            }
            let old = SelectObject(hdc, bmp.into());
            let tiles_x = w.div_ceil(TILE);
            Some(Self { info, w, h, hdc, bmp, old, bits: bits as *mut u8, tiles: vec![], tiles_x })
        }
    }

    /// Copy the monitor's current content (without layered windows) into the DIB.
    pub fn grab(&mut self, screen: HDC) -> bool {
        unsafe {
            BitBlt(self.hdc, 0, 0, self.w as i32, self.h as i32, Some(screen), self.info.rect.x, self.info.rect.y, SRCCOPY).is_ok()
        }
    }

    pub fn pixels(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.bits, self.w * self.h * 4) }
    }

    /// Compare against the previous call. `None` = identical, `Some(rect)` = bounding box of
    /// all changed tiles in monitor-local pixels (the first call reports the whole monitor).
    pub fn diff(&mut self) -> Option<Rect> {
        let tiles_y = self.h.div_ceil(TILE);
        let px = unsafe { std::slice::from_raw_parts(self.bits, self.w * self.h * 4) };
        let first = self.tiles.is_empty();
        if first {
            self.tiles = vec![0; self.tiles_x * tiles_y];
        }
        let mut bbox: Option<Rect> = None;
        for ty in 0..tiles_y {
            for tx in 0..self.tiles_x {
                let x0 = tx * TILE;
                let x1 = (x0 + TILE).min(self.w);
                let mut hsh: u64 = 0xcbf29ce484222325;
                for y in ty * TILE..((ty + 1) * TILE).min(self.h) {
                    let row = &px[(y * self.w + x0) * 4..(y * self.w + x1) * 4];
                    let mut chunks = row.chunks_exact(8);
                    for c in &mut chunks {
                        hsh = (hsh ^ u64::from_le_bytes(c.try_into().unwrap())).wrapping_mul(0x100000001b3).rotate_left(5);
                    }
                    for &b in chunks.remainder() {
                        hsh = (hsh ^ b as u64).wrapping_mul(0x100000001b3);
                    }
                }
                let slot = &mut self.tiles[ty * self.tiles_x + tx];
                if first || *slot != hsh {
                    *slot = hsh;
                    let r = Rect::new(x0 as i32, (ty * TILE) as i32, (x1 - x0) as i32, (TILE.min(self.h - ty * TILE)) as i32);
                    bbox = Some(bbox.map_or(r, |b| b.union(&r)));
                }
            }
        }
        bbox
    }
}

impl Drop for MonitorCapture {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.hdc);
        }
    }
}

pub fn screen_dc() -> HDC {
    unsafe { GetDC(Some(HWND::default())) }
}

pub fn release_screen_dc(dc: HDC) {
    unsafe {
        ReleaseDC(Some(HWND::default()), dc);
    }
}
