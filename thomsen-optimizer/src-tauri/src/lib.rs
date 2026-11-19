//! Thomsen Optimizer app shell: window, tray, updater and the command layer
//! over `thomsen-optimizer-core`. No optimization logic lives here.

mod commands;
mod state;
mod tray;
mod updater;

use state::AppState;
use tauri::{Emitter, Manager, WindowEvent};
use thomsen_optimizer_core::engine::{Engine, EngineEvent};
use thomsen_optimizer_core::store::DataDir;

pub const EVENT: &str = "engine://event";

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            let data = DataDir::new(&dir).map_err(|e| e.to_string())?;
            thomsen_optimizer_core::logging::init(data.path("app.log"));
            log::info!("Thomsen Optimizer {} starting", env!("CARGO_PKG_VERSION"));
            let engine = Engine::new(data, thomsen_optimizer_core::sys::platform_ops());
            app.manage(AppState::new(engine.clone()));

            // Hardware facts take a moment to gather (PowerShell/CIM): never block startup.
            let e = engine.clone();
            std::thread::Builder::new().name("hwinfo".into()).spawn(move || e.refresh_system_info()).ok();

            // Background engine loop: adaptive monitoring + game detection.
            let handle = app.handle().clone();
            let e = engine.clone();
            std::thread::Builder::new()
                .name("engine".into())
                .spawn(move || {
                    e.run(|ev: EngineEvent| {
                        let _ = handle.emit(EVENT, ev);
                    })
                })
                .map_err(|e| e.to_string())?;

            tray::build(app.handle())?;

            let started_hidden = std::env::args().any(|a| a == "--minimized") && engine.settings().start_minimized;
            if started_hidden {
                engine.set_window_hidden(true);
            } else if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let state = window.state::<AppState>();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if state.engine.settings().minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                        state.engine.set_window_hidden(true);
                    }
                }
                WindowEvent::Resized(_) => {
                    let min = window.is_minimized().unwrap_or(false);
                    state.engine.set_window_hidden(min);
                }
                WindowEvent::Focused(true) => state.engine.set_window_hidden(false),
                _ => {}
            }
        })
        .invoke_handler(commands::handler())
        .build(tauri::generate_context!())
        .expect("error while building Thomsen Optimizer");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            // Leave Windows as we found it: end any active gaming session.
            if let Some(state) = handle.try_state::<AppState>() {
                state.engine.stop.store(true, std::sync::atomic::Ordering::Relaxed);
                state.engine.deactivate_profile();
            }
        }
    });
}
