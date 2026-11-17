use crate::app::*;
use crate::capture_test::{self, PathResult};
use crate::engine::{self, EngineStatus};
use crate::win::{monitors, winlist};
use parking_lot::Mutex;
use privacy_core::entries::{validate, Entry, EntryKind};
use privacy_core::settings::{Appearance, Settings};
use privacy_core::Detector;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};

type Res<T> = Result<T, String>;

#[derive(Serialize)]
pub struct MonitorDto {
    name: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    scale_percent: u32,
    primary: bool,
    enabled: bool,
}

#[derive(Serialize)]
pub struct StatusDto {
    running: bool,
    panic: bool,
    engine: EngineStatus,
    monitors: Vec<MonitorDto>,
    capture_apps: Vec<String>,
    warning: Option<String>,
}

static CAPTURE_CACHE: LazyLock<Mutex<(Instant, Vec<String>)>> =
    LazyLock::new(|| Mutex::new((Instant::now() - Duration::from_secs(60), vec![])));

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> StatusDto {
    let s = state.settings.lock().clone();
    let capture_apps = {
        let mut c = CAPTURE_CACHE.lock();
        if c.0.elapsed() > Duration::from_secs(5) {
            *c = (Instant::now(), engine::capture_apps_running());
        }
        c.1.clone()
    };
    StatusDto {
        running: state.engine.running(),
        panic: state.engine.panic(),
        engine: state.engine.status(),
        monitors: monitors::list()
            .into_iter()
            .map(|m| MonitorDto {
                enabled: !s.monitors.disabled.contains(&m.name),
                name: m.name,
                x: m.rect.x,
                y: m.rect.y,
                width: m.rect.w,
                height: m.rect.h,
                scale_percent: m.scale_percent,
                primary: m.primary,
            })
            .collect(),
        capture_apps,
        warning: state.warning.lock().clone(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().clone()
}

#[derive(Serialize)]
pub struct SaveResult {
    settings: Settings,
    warnings: Vec<String>,
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, mut settings: Settings) -> Res<SaveResult> {
    settings.sanitize();
    state.store.save_settings(&settings)?;
    let mut warnings = vec![];
    if let Err(e) = sync_autostart(&app, settings.general.autostart) {
        warnings.push(format!("Start with Windows could not be changed: {e}"));
    }
    warnings.extend(register_shortcuts(&app, &settings.shortcuts));
    *state.settings.lock() = settings.clone();
    state.engine.apply(settings.clone(), &state.entries.lock());
    refresh_tray(&app);
    Ok(SaveResult { settings, warnings })
}

#[tauri::command]
pub fn get_entries(state: State<'_, AppState>) -> Vec<Entry> {
    state.entries.lock().clone()
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
fn new_id() -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:x}{:x}", n, ID_COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn commit_entries(state: &AppState, list: Vec<Entry>) -> Res<()> {
    state.store.save_entries(&list)?;
    let s = state.settings.lock().clone();
    state.engine.apply(s, &list);
    *state.entries.lock() = list;
    Ok(())
}

#[tauri::command]
pub fn add_entry(state: State<'_, AppState>, kind: EntryKind, value: String) -> Res<Entry> {
    let value = value.trim().to_string();
    validate(kind, &value)?;
    let mut list = state.entries.lock().clone();
    if list.iter().any(|e| e.kind == kind && e.value.eq_ignore_ascii_case(&value)) {
        return Err("This entry already exists.".into());
    }
    if list.len() >= 500 {
        return Err("Entry limit (500) reached.".into());
    }
    let e = Entry { id: new_id(), kind, value };
    list.push(e.clone());
    commit_entries(&state, list)?;
    Ok(e)
}

#[tauri::command]
pub fn update_entry(state: State<'_, AppState>, id: String, kind: EntryKind, value: String) -> Res<Entry> {
    let value = value.trim().to_string();
    validate(kind, &value)?;
    let mut list = state.entries.lock().clone();
    let e = list.iter_mut().find(|e| e.id == id).ok_or("Entry not found.")?;
    e.kind = kind;
    e.value = value;
    let out = e.clone();
    commit_entries(&state, list)?;
    Ok(out)
}

#[tauri::command]
pub fn delete_entry(state: State<'_, AppState>, id: String) -> Res<()> {
    let mut list = state.entries.lock().clone();
    list.retain(|e| e.id != id);
    commit_entries(&state, list)
}

#[tauri::command]
pub fn set_protection_cmd(app: AppHandle, on: bool) {
    set_protection(&app, on);
}

#[tauri::command]
pub fn set_panic_cmd(app: AppHandle, on: bool) {
    set_panic(&app, on);
}

#[derive(Serialize)]
pub struct AppDto {
    exe: String,
    running: bool,
    browser: bool,
}

/// Executables with a visible window right now, plus the ones already listed in the rules.
#[tauri::command]
pub fn list_apps(state: State<'_, AppState>) -> Vec<AppDto> {
    let s = state.settings.lock().clone();
    let mut names: Vec<(String, bool)> = winlist::list_windows().into_iter().filter(|w| !w.exe.is_empty()).map(|w| (w.exe, true)).collect();
    for e in s.apps.selected.iter().chain(s.apps.excluded.iter()) {
        names.push((e.clone(), false));
    }
    names.sort();
    let mut out: Vec<AppDto> = vec![];
    for (exe, running) in names {
        if let Some(last) = out.last_mut() {
            if last.exe == exe {
                last.running |= running;
                continue;
            }
        }
        out.push(AppDto { browser: engine::is_browser(&exe), exe, running });
    }
    out
}

#[derive(Serialize)]
pub struct HitDto {
    start: usize,
    end: usize,
    category: String,
}

/// Run the detector on text typed into the "Test detection" box. Offsets are UTF-16 (JS strings).
#[tauri::command]
pub fn test_text(state: State<'_, AppState>, text: String) -> Vec<HitDto> {
    let s = state.settings.lock().clone();
    let det = Detector::new(&s.filters, &state.entries.lock());
    det.detect(&text.chars().take(5000).collect::<String>())
        .into_iter()
        .map(|h| HitDto {
            start: text[..h.start].encode_utf16().count(),
            end: text[..h.end].encode_utf16().count(),
            category: engine::category_label(h.category).to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn preview_style(appearance: Appearance) -> tauri::ipc::Response {
    let mut a = appearance;
    let mut s = Settings { appearance: a.clone(), ..Default::default() };
    s.sanitize();
    a = s.appearance;
    tauri::ipc::Response::new(crate::preview::render(&a))
}

#[tauri::command]
pub async fn run_capture_test(app: AppHandle, hold_secs: u32) -> Res<Vec<PathResult>> {
    let hwnd = app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
    let in_caps = app.state::<AppState>().settings.lock().capture.overlay_in_captures;
    tauri::async_runtime::spawn_blocking(move || capture_test::run(hwnd, in_caps, hold_secs.min(60))).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_log_folder(state: State<'_, AppState>) -> Res<String> {
    Ok(state.store.dir().display().to_string())
}

#[tauri::command]
pub fn window_minimize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
}

#[tauri::command]
pub fn window_toggle_maximize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_maximized().unwrap_or(false) {
            let _ = w.unmaximize();
        } else {
            let _ = w.maximize();
        }
    }
}

#[tauri::command]
pub fn window_close(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if app.state::<AppState>().settings.lock().general.minimize_to_tray {
            let _ = w.hide();
        } else {
            quit(app);
        }
    }
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.state::<AppState>().engine.shutdown();
    app.exit(0);
}
