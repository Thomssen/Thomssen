//! Application state and the actions shared by the UI, the tray menu and keyboard shortcuts.

use crate::engine::Engine;
use crate::store::Store;
use parking_lot::Mutex;
use privacy_core::settings::Shortcuts;
use privacy_core::{Entry, Settings};
use std::str::FromStr;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub toggle: MenuItem<Wry>,
    pub panic: MenuItem<Wry>,
}

pub struct AppState {
    pub engine: Engine,
    pub store: Store,
    pub settings: Mutex<Settings>,
    pub entries: Mutex<Vec<Entry>>,
    pub warning: Mutex<Option<String>>,
    pub tray: Mutex<Option<TrayItems>>,
}

pub fn notify(app: &AppHandle, kind: NotifyKind, title: &str, body: &str) {
    let st = app.state::<AppState>();
    let n = st.settings.lock().notifications.clone();
    let on = match kind {
        NotifyKind::Toggle => n.on_toggle,
        NotifyKind::Panic => n.on_panic,
        NotifyKind::Error => n.on_error,
    };
    if on {
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

#[derive(Clone, Copy)]
pub enum NotifyKind {
    Toggle,
    Panic,
    Error,
}

pub fn refresh_tray(app: &AppHandle) {
    let st = app.state::<AppState>();
    let running = st.engine.running();
    let panic = st.engine.panic();
    if let Some(t) = st.tray.lock().as_ref() {
        let status = if panic { "Status: PANIC cover active" } else if running { "Status: Protection active" } else { "Status: Protection paused" };
        let _ = t.status.set_text(status);
        let _ = t.toggle.set_text(if running { "Pause protection" } else { "Start protection" });
        let _ = t.panic.set_text(if panic { "Remove panic cover" } else { "Panic: cover screen" });
    }
    let _ = app.emit("state-changed", ());
}

pub fn set_protection(app: &AppHandle, on: bool) {
    let st = app.state::<AppState>();
    st.engine.set_running(on);
    refresh_tray(app);
    log::info!("protection {}", if on { "started" } else { "stopped" });
    notify(app, NotifyKind::Toggle, "Privacy Blur", if on { "Protection started" } else { "Protection paused" });
}

pub fn toggle_protection(app: &AppHandle) {
    let on = !app.state::<AppState>().engine.running();
    set_protection(app, on);
}

pub fn set_panic(app: &AppHandle, on: bool) {
    let st = app.state::<AppState>();
    st.engine.set_panic(on);
    refresh_tray(app);
    log::info!("panic cover {}", if on { "on" } else { "off" });
    notify(app, NotifyKind::Panic, "Privacy Blur", if on { "Panic cover enabled: screen is covered" } else { "Panic cover removed" });
}

pub fn toggle_panic(app: &AppHandle) {
    let on = !app.state::<AppState>().engine.panic();
    set_panic(app, on);
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn open_settings(app: &AppHandle) {
    show_main(app);
    let _ = app.emit("navigate", "settings");
}

/// Register the three global shortcuts. Returns human readable problems (e.g. combination already taken).
pub fn register_shortcuts(app: &AppHandle, s: &Shortcuts) -> Vec<String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut problems = vec![];
    let items: [(&str, &str, fn(&AppHandle)); 3] = [
        ("Toggle protection", &s.toggle, toggle_protection),
        ("Open settings", &s.open_settings, open_settings),
        ("Panic cover", &s.panic, toggle_panic),
    ];
    for (name, text, action) in items {
        if text.trim().is_empty() {
            continue;
        }
        match Shortcut::from_str(text.trim()) {
            Ok(sc) => {
                let r = gs.on_shortcut(sc, move |app, _, ev| {
                    if ev.state == ShortcutState::Pressed {
                        action(app);
                    }
                });
                if let Err(e) = r {
                    problems.push(format!("{name}: '{text}' could not be registered ({e}). It may be used by another program."));
                }
            }
            Err(_) => problems.push(format!("{name}: '{text}' is not a valid shortcut.")),
        }
    }
    problems
}

pub fn sync_autostart(app: &AppHandle, want: bool) -> Result<(), String> {
    let al = app.autolaunch();
    let cur = al.is_enabled().map_err(|e| e.to_string())?;
    if want && !cur {
        al.enable().map_err(|e| e.to_string())?;
    } else if !want && cur {
        al.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}
