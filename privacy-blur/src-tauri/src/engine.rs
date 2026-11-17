//! Protection engine. Three worker threads:
//!
//! * **render** – captures every enabled monitor, diffs it against the previous frame,
//!   forwards changed frames to the detector and draws the overlays.
//! * **detect** – OCR on the changed screen bands, merges UI Automation results, applies the
//!   per-application rules, stabilises the regions over time.
//! * **uia**    – walks the accessibility tree of visible windows (slow, so it runs apart).
//!
//! No thread ever keeps recognised text: detection returns `(rectangle, category)` only.

use crate::win::{
    capture::{self, MonitorCapture},
    monitors::{self, MonitorInfo},
    ocr::{self, Ocr},
    overlay::{pump_messages, OverlayPool, PanicCover},
    uia::Uia,
    winlist::{self, WinInfo},
};
use parking_lot::{Mutex, RwLock};
use privacy_core::effects::{self, Params};
use privacy_core::locate::detect_line;
use privacy_core::settings::{AppMode, Category, Method, Scope, Settings};
use privacy_core::track::{Region, Tracker};
use privacy_core::{Detector, Entry, Rect};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

const BROWSERS: &[&str] = &[
    "chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "vivaldi.exe", "arc.exe",
    "chromium.exe", "iexplore.exe", "librewolf.exe", "waterfox.exe", "zen.exe", "thorium.exe",
];
/// Apps that capture or stream the screen; shown on the dashboard together with the capture caveats.
const CAPTURE_APPS: &[&str] = &[
    "obs64.exe", "obs32.exe", "discord.exe", "teams.exe", "ms-teams.exe", "zoom.exe", "slack.exe",
    "streamlabs desktop.exe", "xsplit.core.exe", "skype.exe", "webex.exe", "bandicam.exe", "screenrec.exe",
];

pub fn is_browser(exe: &str) -> bool {
    BROWSERS.contains(&exe)
}

#[derive(Clone, Default, Serialize)]
pub struct AppCount {
    pub exe: String,
    pub count: u32,
}

#[derive(Clone, Default, Serialize)]
pub struct EngineStatus {
    pub active_regions: u32,
    pub by_category: BTreeMap<String, u32>,
    pub total_detected: u64,
    pub apps_with_detections: Vec<AppCount>,
    pub visible_apps: Vec<String>,
    pub ocr: String,
    pub ocr_ok: bool,
    pub uia_ok: bool,
    pub last_cycle_ms: u32,
    pub last_detect_ms: u32,
    pub last_error: Option<String>,
}

struct Output {
    seq: u64,
    regions: Vec<Region>,
}

struct UiaOut {
    at: Instant,
    hits: Vec<crate::win::uia::UiaHit>,
}

pub struct Shared {
    pub settings: RwLock<Arc<Settings>>,
    detector: RwLock<Arc<Detector>>,
    generation: AtomicU64,
    running: AtomicBool,
    panic: AtomicBool,
    shutdown: AtomicBool,
    status: Mutex<EngineStatus>,
    output: Mutex<Output>,
    uia_out: Mutex<UiaOut>,
    frame_seq: AtomicU64,
    own_exe: String,
}

struct MonFrame {
    info: MonitorInfo,
    w: usize,
    pixels: Arc<Vec<u8>>,
    /// Changed area (monitor-local) since the previous frame the detector received.
    dirty: Option<Rect>,
}

struct FrameSet {
    monitors: Vec<MonFrame>,
    windows: Vec<WinInfo>,
}

pub struct Engine {
    pub shared: Arc<Shared>,
}

fn now_ms(t0: &Instant) -> u64 {
    t0.elapsed().as_millis() as u64
}

impl Shared {
    fn allowed(&self, exe: &str, s: &Settings) -> bool {
        let exe = exe.to_lowercase();
        if s.apps.excluded.iter().any(|e| *e == exe) {
            return false;
        }
        if exe == self.own_exe && !s.apps.protect_self {
            return false;
        }
        if is_browser(&exe) && !s.apps.protect_browsers {
            return false;
        }
        match s.apps.mode {
            AppMode::All => true,
            AppMode::Selected => s.apps.selected.iter().any(|e| *e == exe),
        }
    }
    fn report_error(&self, msg: impl Into<String>) {
        self.status.lock().last_error = Some(msg.into());
    }
}

impl Engine {
    pub fn new(settings: Settings, entries: Vec<Entry>) -> Self {
        let own_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .unwrap_or_else(|| "privacy-blur.exe".into());
        let det = Detector::new(&settings.filters, &entries);
        let shared = Arc::new(Shared {
            settings: RwLock::new(Arc::new(settings)),
            detector: RwLock::new(Arc::new(det)),
            generation: AtomicU64::new(1),
            running: AtomicBool::new(false),
            panic: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            status: Mutex::new(EngineStatus::default()),
            output: Mutex::new(Output { seq: 0, regions: vec![] }),
            uia_out: Mutex::new(UiaOut { at: Instant::now(), hits: vec![] }),
            frame_seq: AtomicU64::new(0),
            own_exe,
        });
        let (tx, rx) = sync_channel::<Arc<FrameSet>>(1);
        let s1 = shared.clone();
        std::thread::Builder::new().name("pb-render".into()).spawn(move || render_loop(s1, tx)).expect("spawn render thread");
        let s2 = shared.clone();
        std::thread::Builder::new().name("pb-detect".into()).spawn(move || detect_loop(s2, rx)).expect("spawn detect thread");
        let s3 = shared.clone();
        std::thread::Builder::new().name("pb-uia".into()).spawn(move || uia_loop(s3)).expect("spawn uia thread");
        Engine { shared }
    }

    pub fn apply(&self, settings: Settings, entries: &[Entry]) {
        let det = Detector::new(&settings.filters, entries);
        *self.shared.detector.write() = Arc::new(det);
        *self.shared.settings.write() = Arc::new(settings);
        self.shared.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set_running(&self, on: bool) {
        self.shared.running.store(on, Ordering::SeqCst);
        if !on {
            self.shared.generation.fetch_add(1, Ordering::SeqCst);
            // drop stale detections so a restart can never flash old overlays
            let mut o = self.shared.output.lock();
            o.seq += 1;
            o.regions.clear();
            let mut s = self.shared.status.lock();
            s.active_regions = 0;
            s.by_category.clear();
            s.apps_with_detections.clear();
            s.visible_apps.clear();
        }
    }
    pub fn running(&self) -> bool {
        self.shared.running.load(Ordering::SeqCst)
    }
    pub fn set_panic(&self, on: bool) {
        self.shared.panic.store(on, Ordering::SeqCst);
    }
    pub fn panic(&self) -> bool {
        self.shared.panic.load(Ordering::SeqCst)
    }
    pub fn status(&self) -> EngineStatus {
        self.shared.status.lock().clone()
    }
    pub fn shutdown(&self) {
        self.shared.shutdown.store(true, Ordering::SeqCst);
    }
}

// ───────────────────────────── render thread ─────────────────────────────

fn render_loop(sh: Arc<Shared>, tx: SyncSender<Arc<FrameSet>>) {
    let mut pool = OverlayPool::new(false);
    let mut cover = PanicCover::default();
    let mut caps: Vec<MonitorCapture> = vec![];
    let mut snapshots: Vec<Arc<Vec<u8>>> = vec![];
    let mut pending_dirty: Vec<Option<Rect>> = vec![];
    let mut mon_check = Instant::now() - Duration::from_secs(60);
    let mut mon_list: Vec<MonitorInfo> = vec![];
    let mut last_full_send = Instant::now();
    let mut last_raise = Instant::now();
    let mut last_seq_drawn = u64::MAX;
    let mut last_disabled: Vec<String> = vec![];
    let mut buf: Vec<u8> = vec![];
    let mut crop: Vec<u8> = vec![];
    let screen = capture::screen_dc();

    while !sh.shutdown.load(Ordering::Relaxed) {
        let cycle = Instant::now();
        pump_messages();
        let st = sh.settings.read().clone();
        pool.set_excluded(!st.capture.overlay_in_captures);

        if monitors_changed(&mut mon_check, &mut mon_list) {
            caps.clear();
            snapshots.clear();
            pending_dirty.clear();
            last_seq_drawn = u64::MAX;
        }
        // Panic cover works even while protection is stopped.
        let want_cover = sh.panic.load(Ordering::Relaxed);
        if want_cover && !cover.visible() {
            cover.show(&mon_list.iter().map(|m| m.rect).collect::<Vec<_>>());
        } else if !want_cover && cover.visible() {
            cover.hide();
        } else if want_cover && last_raise.elapsed() > Duration::from_millis(800) {
            cover.raise();
        }

        if !sh.running.load(Ordering::Relaxed) {
            pool.hide_from(0);
            pool.trim(0);
            if !caps.is_empty() {
                caps.clear(); // free the capture buffers while idle
                snapshots.clear();
                pending_dirty.clear();
                last_seq_drawn = u64::MAX;
            }
            std::thread::sleep(Duration::from_millis(120));
            continue;
        }

        if caps.is_empty() || last_disabled != st.monitors.disabled {
            last_disabled = st.monitors.disabled.clone();
            caps = mon_list.iter().filter(|m| !st.monitors.disabled.contains(&m.name)).filter_map(|m| MonitorCapture::new(m.clone())).collect();
            snapshots = caps.iter().map(|_| Arc::new(Vec::new())).collect();
            pending_dirty = caps.iter().map(|_| None).collect();
            last_seq_drawn = u64::MAX;
        }

        // 1. capture + change detection
        let mut any_change = false;
        for (i, c) in caps.iter_mut().enumerate() {
            if !c.grab(screen) {
                continue;
            }
            if let Some(d) = c.diff() {
                any_change = true;
                snapshots[i] = Arc::new(c.pixels().to_vec());
                pending_dirty[i] = Some(pending_dirty[i].map_or(d, |p| p.union(&d)));
            }
        }
        if any_change {
            sh.frame_seq.fetch_add(1, Ordering::Relaxed);
        }
        // 2. hand over to the detector (drop the frame if it is still busy; dirty areas accumulate)
        if any_change || last_full_send.elapsed() > Duration::from_millis(2500) {
            let fs = FrameSet {
                monitors: caps
                    .iter()
                    .enumerate()
                    .map(|(i, c)| MonFrame { info: c.info.clone(), w: c.w, pixels: snapshots[i].clone(), dirty: pending_dirty[i] })
                    .collect(),
                windows: winlist::list_windows(),
            };
            if tx.try_send(Arc::new(fs)).is_ok() {
                last_full_send = Instant::now();
                for d in pending_dirty.iter_mut() {
                    *d = None;
                }
            }
        }

        // 3. draw overlays from the latest detections and the freshest pixels
        let (seq, regions) = {
            let o = sh.output.lock();
            (o.seq, o.regions.clone())
        };
        if seq != last_seq_drawn || any_change {
            last_seq_drawn = seq;
            let params = Params::from(&st.appearance);
            let pad = st.appearance.padding as i32;
            let mut used = 0usize;
            for r in &regions {
                let (cx, cy) = r.rect.center();
                let Some(mi) = caps.iter().position(|c| c.info.rect.contains_point(cx, cy)) else { continue };
                let mon = &caps[mi].info.rect;
                let want = r.rect.inflate(pad, pad / 2 + 1);
                let Some(abs) = want.intersect(mon) else { continue };
                let local = Rect::new(abs.x - mon.x, abs.y - mon.y, abs.w, abs.h);
                let px = &snapshots[mi];
                if px.len() < caps[mi].w * caps[mi].h * 4 {
                    continue;
                }
                let (sw, w, h) = (caps[mi].w, local.w as usize, local.h as usize);
                crop.clear();
                for y in 0..h {
                    let s = ((local.y as usize + y) * sw + local.x as usize) * 4;
                    crop.extend_from_slice(&px[s..s + w * 4]);
                }
                effects::render(&params, &crop, w, h, r.rect.h.max(1) as usize, &mut buf);
                pool.present(used, abs.x, abs.y, abs.w, abs.h, &buf);
                used += 1;
            }
            pool.hide_from(used);
            pool.trim(used);
        }
        if last_raise.elapsed() > Duration::from_millis(800) {
            last_raise = Instant::now();
            pool.raise_all();
        }

        let spent = cycle.elapsed();
        sh.status.lock().last_cycle_ms = spent.as_millis() as u32;
        let target = Duration::from_millis(st.performance.interval_ms as u64);
        if spent < target {
            // sleep in small slices so shutdown/panic stay responsive
            let mut left = target - spent;
            while left > Duration::ZERO && !sh.shutdown.load(Ordering::Relaxed) {
                let step = left.min(Duration::from_millis(25));
                std::thread::sleep(step);
                pump_messages();
                if sh.panic.load(Ordering::Relaxed) != cover.visible() {
                    break;
                }
                left -= step;
            }
        }
    }
    capture::release_screen_dc(screen);
}

fn monitors_changed(last_check: &mut Instant, list: &mut Vec<MonitorInfo>) -> bool {
    if last_check.elapsed() < Duration::from_millis(1500) && !list.is_empty() {
        return false;
    }
    *last_check = Instant::now();
    let now = monitors::list();
    if now != *list {
        *list = now;
        return true;
    }
    false
}

// ───────────────────────────── detect thread ─────────────────────────────

struct Cached {
    monitor: String,
    region: Region,
}

fn detect_loop(sh: Arc<Shared>, rx: Receiver<Arc<FrameSet>>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let t0 = Instant::now();
    let mut tracker = Tracker::new();
    let mut cache: Vec<Cached> = vec![];
    let mut ocr_engine: Option<Ocr> = None;
    let mut ocr_retry = Instant::now() - Duration::from_secs(60);
    let mut last_full: HashMap<String, Instant> = HashMap::new();
    let mut gen_seen = 0u64;
    let mut seq = 0u64;

    while !sh.shutdown.load(Ordering::Relaxed) {
        let frame = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(f) => f,
            Err(_) => continue,
        };
        // drain: only the newest frame matters
        let mut frame = frame;
        loop {
            match rx.try_recv() {
                Ok(f) => frame = f,
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        let started = Instant::now();
        let st = sh.settings.read().clone();
        let det = sh.detector.read().clone();
        let gen = sh.generation.load(Ordering::SeqCst);
        let mut force_full = false;
        if gen != gen_seen {
            gen_seen = gen;
            cache.clear();
            tracker.clear();
            last_full.clear();
            force_full = true;
        }
        if !sh.running.load(Ordering::Relaxed) || det.is_noop() {
            if !tracker.regions().is_empty() || seq == 0 {
                tracker.clear();
                cache.clear();
                seq += 1;
                *sh.output.lock() = Output { seq, regions: vec![] };
            }
            let mut s = sh.status.lock();
            s.active_regions = 0;
            s.by_category.clear();
            s.apps_with_detections.clear();
            continue;
        }

        let use_ocr = st.performance.method != Method::UiaOnly;
        let use_uia = st.performance.method != Method::OcrOnly;

        if use_ocr && ocr_engine.is_none() && ocr_retry.elapsed() > Duration::from_secs(10) {
            ocr_retry = Instant::now();
            match Ocr::new() {
                Ok(o) => {
                    let mut s = sh.status.lock();
                    s.ocr = format!("Ready ({})", o.language);
                    s.ocr_ok = true;
                    ocr_engine = Some(o);
                }
                Err(e) => {
                    let mut s = sh.status.lock();
                    s.ocr = e;
                    s.ocr_ok = false;
                }
            }
        }
        if !use_ocr {
            let mut s = sh.status.lock();
            s.ocr = "Disabled".into();
            s.ocr_ok = false;
        }

        let fg = frame.windows.iter().find(|w| w.foreground).cloned();
        let full_interval = Duration::from_millis((st.performance.interval_ms as u64 * 8).max(3000));

        // ── OCR on the changed bands ──
        if let (true, Some(engine)) = (use_ocr, ocr_engine.as_ref()) {
            for mf in &frame.monitors {
                if mf.pixels.len() < mf.w * 4 {
                    continue;
                }
                let mon = mf.info.rect;
                let h = mf.pixels.len() / 4 / mf.w;
                let need_full = force_full || last_full.get(&mf.info.name).map_or(true, |t| t.elapsed() > full_interval);
                let mut band = if need_full {
                    Some(Rect::new(0, 0, mf.w as i32, h as i32))
                } else {
                    mf.dirty.map(|d| {
                        // text lines run horizontally: always scan full rows, with a margin
                        let y0 = (d.y - 40).max(0);
                        let y1 = (d.bottom() + 40).min(h as i32);
                        Rect::new(0, y0, mf.w as i32, y1 - y0)
                    })
                };
                if st.performance.scope == Scope::ForegroundOnly {
                    band = match (band, &fg) {
                        (Some(b), Some(f)) => {
                            let local = Rect::new(f.rect.x - mon.x, f.rect.y - mon.y, f.rect.w, f.rect.h);
                            b.intersect(&local)
                        }
                        _ => None,
                    };
                }
                let Some(band) = band.filter(|b| b.h >= 8 && b.w >= 8) else { continue };
                if need_full {
                    last_full.insert(mf.info.name.clone(), Instant::now());
                }

                // downscale if requested or if the engine's size limit demands it
                let mut scale = st.performance.ocr_scale;
                let max = engine.max_dimension() as f32;
                let needed = (max / band.w.max(band.h) as f32 * 100.0).floor() as u32;
                scale = scale.min(needed.max(10));
                let (img, iw, ih) = ocr::crop_scaled(&mf.pixels, mf.w, band, scale);
                let k = iw as f32 / band.w as f32;
                match engine.recognize(&img, iw, ih) {
                    Ok(lines) => {
                        let mut fresh: Vec<Cached> = vec![];
                        for line in lines {
                            let words: Vec<_> = line
                                .into_iter()
                                .map(|mut w| {
                                    w.rect = Rect::new(
                                        mon.x + band.x + (w.rect.x as f32 / k) as i32,
                                        mon.y + band.y + (w.rect.y as f32 / k) as i32,
                                        ((w.rect.w as f32 / k).ceil() as i32).max(1),
                                        ((w.rect.h as f32 / k).ceil() as i32).max(1),
                                    );
                                    w
                                })
                                .collect();
                            for (rect, category) in detect_line(&det, &words) {
                                fresh.push(Cached { monitor: mf.info.name.clone(), region: Region { rect, category } });
                            }
                        }
                        let abs_band = Rect::new(mon.x + band.x, mon.y + band.y, band.w, band.h);
                        cache.retain(|c| !(c.monitor == mf.info.name && c.region.rect.intersect(&abs_band).is_some()));
                        cache.extend(fresh);
                        sh.status.lock().last_error = None;
                    }
                    Err(e) => {
                        // never include pixel/text content here, only the OS error text
                        sh.report_error(format!("OCR failed: {e}"));
                    }
                }
            }
            // forget cached regions of monitors that no longer exist or were disabled
            let alive: Vec<&str> = frame.monitors.iter().map(|m| m.info.name.as_str()).collect();
            cache.retain(|c| alive.contains(&c.monitor.as_str()));
        } else {
            cache.clear();
        }

        // ── merge with UI Automation ──
        let mut all: Vec<(Region, Option<isize>)> = cache.iter().map(|c| (c.region, None)).collect();
        if use_uia {
            let u = sh.uia_out.lock();
            if u.at.elapsed() < Duration::from_millis(2500) {
                for h in &u.hits {
                    // the element must belong to the window that is actually visible there
                    let (cx, cy) = h.rect.center();
                    if winlist::owner_at(&frame.windows, cx, cy).map_or(false, |w| w.hwnd == h.hwnd) {
                        all.push((Region { rect: h.rect, category: h.category }, Some(h.hwnd)));
                    }
                }
            }
        }

        // ── application rules ──
        let mut kept: Vec<(Region, String)> = vec![];
        for (r, _) in all {
            let (cx, cy) = r.rect.center();
            let owner = winlist::owner_at(&frame.windows, cx, cy);
            let exe = owner.map(|w| w.exe.clone()).unwrap_or_else(|| "explorer.exe".into());
            if st.performance.scope == Scope::ForegroundOnly && !owner.map_or(false, |w| w.foreground) {
                continue;
            }
            if sh.allowed(&exe, &st) {
                kept.push((r, exe));
            }
        }
        let merged = merge_regions(kept);
        let regs: Vec<Region> = merged.iter().map(|(r, _)| *r).collect();
        let changed = tracker.update(now_ms(&t0), st.performance.hold_ms as u64, &regs);

        let active = tracker.regions();
        if changed || seq == 0 {
            seq += 1;
            *sh.output.lock() = Output { seq, regions: active.clone() };
        }
        // statistics (categories and executable names only)
        let mut by_cat: BTreeMap<String, u32> = BTreeMap::new();
        for r in &active {
            *by_cat.entry(category_label(r.category).to_string()).or_default() += 1;
        }
        let mut per_app: HashMap<String, u32> = HashMap::new();
        for (r, exe) in &merged {
            if active.iter().any(|a| a.rect == r.rect) {
                *per_app.entry(exe.clone()).or_default() += 1;
            }
        }
        let mut visible: Vec<String> = frame.windows.iter().filter(|w| sh.allowed(&w.exe, &st)).map(|w| w.exe.clone()).collect();
        visible.sort();
        visible.dedup();
        let mut s = sh.status.lock();
        s.active_regions = active.len() as u32;
        s.by_category = by_cat;
        s.total_detected = tracker.total_seen;
        s.visible_apps = visible;
        s.apps_with_detections = {
            let mut v: Vec<AppCount> = per_app.into_iter().map(|(exe, count)| AppCount { exe, count }).collect();
            v.sort_by(|a, b| b.count.cmp(&a.count).then(a.exe.cmp(&b.exe)));
            v
        };
        s.last_detect_ms = started.elapsed().as_millis() as u32;
    }
    unsafe {
        CoUninitialize();
    }
}

/// Merge same-category regions that overlap heavily (OCR and UIA finding the same text).
fn merge_regions(mut v: Vec<(Region, String)>) -> Vec<(Region, String)> {
    let mut out: Vec<(Region, String)> = vec![];
    v.sort_by_key(|(r, _)| (r.rect.y, r.rect.x));
    'next: for (r, exe) in v {
        for (o, _) in out.iter_mut() {
            if let Some(i) = o.rect.intersect(&r.rect) {
                let small = o.rect.area().min(r.rect.area());
                if small > 0 && i.area() * 2 >= small {
                    o.rect = o.rect.union(&r.rect);
                    continue 'next;
                }
            }
        }
        out.push((r, exe));
    }
    out
}

// ───────────────────────────── UI Automation thread ─────────────────────────────

fn uia_loop(sh: Arc<Shared>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut uia: Option<Uia> = None;
    let mut last_run = Instant::now() - Duration::from_secs(60);
    let mut last_seq = u64::MAX;
    let mut last_fg = 0isize;
    let mut gen_seen = 0u64;

    while !sh.shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
        let st = sh.settings.read().clone();
        if !sh.running.load(Ordering::Relaxed) || st.performance.method == Method::OcrOnly {
            let mut o = sh.uia_out.lock();
            if !o.hits.is_empty() {
                o.hits.clear();
            }
            let mut s = sh.status.lock();
            s.uia_ok = false;
            continue;
        }
        if uia.is_none() {
            match Uia::new() {
                Ok(u) => uia = Some(u),
                Err(e) => {
                    sh.report_error(format!("UI Automation unavailable: {e}"));
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
            }
        }
        let seq = sh.frame_seq.load(Ordering::Relaxed);
        let gen = sh.generation.load(Ordering::SeqCst);
        let fg = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize };
        let min_gap = Duration::from_millis((st.performance.interval_ms as u64 * 2).max(600));
        let stale = last_run.elapsed() > Duration::from_secs(4);
        if last_run.elapsed() < min_gap || (seq == last_seq && fg == last_fg && gen == gen_seen && !stale) {
            continue;
        }
        last_run = Instant::now();
        last_seq = seq;
        last_fg = fg;
        gen_seen = gen;

        let det = sh.detector.read().clone();
        let mut wins: Vec<WinInfo> = winlist::list_windows().into_iter().filter(|w| sh.allowed(&w.exe, &st)).collect();
        if st.performance.scope == Scope::ForegroundOnly {
            wins.retain(|w| w.foreground);
        }
        wins.truncate(6);
        let hits = uia.as_ref().map(|u| u.scan(&det, &wins)).unwrap_or_default();
        *sh.uia_out.lock() = UiaOut { at: Instant::now(), hits };
        sh.status.lock().uia_ok = true;
    }
    unsafe {
        CoUninitialize();
    }
}

pub fn capture_apps_running() -> Vec<String> {
    let procs = winlist::running_process_names();
    CAPTURE_APPS.iter().filter(|a| procs.iter().any(|p| p == *a)).map(|s| s.to_string()).collect()
}

pub fn category_label(c: Category) -> &'static str {
    match c {
        Category::Email => "email",
        Category::Phone => "phone",
        Category::Name => "name",
        Category::Address => "address",
        Category::Ip => "ip",
        Category::CreditCard => "credit_card",
        Category::Bank => "bank",
        Category::Username => "username",
        Category::Dob => "dob",
        Category::Secret => "secret",
        Category::Custom => "custom",
    }
}
