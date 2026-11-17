//! Privacy Blur — local, real-time privacy protection for Windows.

#[cfg(not(windows))]
compile_error!("Privacy Blur is a Windows application. Use `cargo test -p privacy-core` on other platforms.");

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod capture_test;
#[cfg(windows)]
mod commands;
#[cfg(windows)]
mod engine;
#[cfg(windows)]
mod logging;
#[cfg(windows)]
mod preview;
#[cfg(windows)]
mod store;
#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use run_impl::run;

#[cfg(windows)]
mod run_impl {
    use crate::app::*;
    use crate::{commands as c, engine::Engine, logging, store::Store, win};
    use parking_lot::Mutex;
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::{Manager, WindowEvent};

    pub fn run() {
        win::init_dpi_awareness();
        let result = tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
            .setup(|app| {
                let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
                logging::init(dir.clone());
                log::info!("Privacy Blur {} starting", env!("CARGO_PKG_VERSION"));
                let store = Store::new(dir);
                let (settings, w1) = store.load_settings();
                let (entries, w2) = store.load_entries();
                let warning = w1.or(w2);
                if let Some(w) = &warning {
                    log::warn!("{w}");
                }
                let engine = Engine::new(settings.clone(), entries.clone());
                app.manage(AppState {
                    engine,
                    store,
                    settings: Mutex::new(settings.clone()),
                    entries: Mutex::new(entries),
                    warning: Mutex::new(warning),
                    tray: Mutex::new(None),
                });

                // tray
                let status = MenuItem::with_id(app, "status", "Status: Protection paused", false, None::<&str>)?;
                let toggle = MenuItem::with_id(app, "toggle", "Start protection", true, None::<&str>)?;
                let panic = MenuItem::with_id(app, "panic", "Panic: cover screen", true, None::<&str>)?;
                let open = MenuItem::with_id(app, "open", "Open Privacy Blur", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
                let sep1 = PredefinedMenuItem::separator(app)?;
                let sep2 = PredefinedMenuItem::separator(app)?;
                let menu = Menu::with_items(app, &[&status, &sep1, &toggle, &panic, &open, &sep2, &quit])?;
                TrayIconBuilder::with_id("main")
                    .icon(app.default_window_icon().cloned().ok_or("missing icon")?)
                    .tooltip("Privacy Blur")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, ev| match ev.id.as_ref() {
                        "toggle" => toggle_protection(app),
                        "panic" => toggle_panic(app),
                        "open" => show_main(app),
                        "quit" => c::quit(app.clone()),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, ev| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                            show_main(tray.app_handle());
                        }
                    })
                    .build(app)?;
                *app.state::<AppState>().tray.lock() = Some(TrayItems { status, toggle, panic });

                let handle = app.handle().clone();
                for p in register_shortcuts(&handle, &settings.shortcuts) {
                    log::warn!("{p}");
                }
                if let Err(e) = sync_autostart(&handle, settings.general.autostart) {
                    log::warn!("autostart sync failed: {e}");
                }

                let minimized = std::env::args().any(|a| a == "--minimized") || settings.general.start_minimized;
                if !minimized {
                    show_main(&handle);
                }
                if settings.general.protect_on_launch {
                    set_protection(&handle, true);
                }
                refresh_tray(&handle);

                // surface engine problems (OCR missing, capture failure…) once each; messages never contain screen content
                let h2 = handle.clone();
                std::thread::spawn(move || {
                    let mut last: Option<String> = None;
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        let cur = h2.state::<AppState>().engine.status().last_error;
                        if cur != last {
                            if let Some(e) = &cur {
                                log::error!("{e}");
                                notify(&h2, NotifyKind::Error, "Privacy Blur problem", e);
                            }
                            last = cur;
                        }
                    }
                });
                Ok(())
            })
            .on_window_event(|window, event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let st = window.app_handle().state::<AppState>();
                    if st.settings.lock().general.minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        st.engine.shutdown();
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                c::get_status,
                c::get_settings,
                c::save_settings,
                c::get_entries,
                c::add_entry,
                c::update_entry,
                c::delete_entry,
                c::set_protection_cmd,
                c::set_panic_cmd,
                c::list_apps,
                c::test_text,
                c::preview_style,
                c::run_capture_test,
                c::open_log_folder,
                c::window_minimize,
                c::window_toggle_maximize,
                c::window_close,
                c::quit,
            ])
            .run(tauri::generate_context!());
        if let Err(e) = result {
            log::error!("fatal: {e}");
            eprintln!("Privacy Blur failed to start: {e}");
        }
    }
}
