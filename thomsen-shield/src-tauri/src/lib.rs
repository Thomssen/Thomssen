pub mod commands;
pub mod detection;
pub mod elevation;
pub mod error;
pub mod firewall;
pub mod license;
pub mod logging;
pub mod monitor;
pub mod netutil;
pub mod network;
pub mod state;
pub mod store;

use std::sync::Arc;

use tauri::Manager;

use state::AppState;

/// TEMPORARY: release builds have no console and no crash reporting at
/// all, so a startup failure on a real machine (as opposed to CI, which
/// never reproduced this) is completely invisible - not even a "thread
/// panicked" line, since `panic = "abort"` (Cargo.toml) turns any panic on
/// any thread into an immediate process-wide abort. This writes a plain
/// trail of how far startup got, plus the panic hook below, so the next
/// failure actually leaves evidence. Remove once the real cause is fixed
/// and confirmed.
fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("thomsen-shield-startup.log")) {
        let _ = writeln!(f, "[{}] {msg}", chrono::Utc::now().to_rfc3339());
    }
}

pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        debug_log(&format!("PANIC: {info}"));
    }));
    debug_log("run() entered");

    // This same executable is re-launched with this flag, elevated, to
    // perform exactly one batch of firewall writes - see `elevation.rs`
    // and `firewall::elevated_ops`. That path never touches Tauri/WebView2
    // at all, so it's handled and exited before anything else here runs.
    if elevation::is_elevated_child_invocation() {
        debug_log("is_elevated_child_invocation() true - taking the elevated-child early-exit path instead of starting the GUI");
        firewall::elevated_ops::run_elevated_child();
    }
    debug_log("not an elevated-child invocation, continuing to Tauri Builder");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch (double-clicking the shortcut while already
            // running) should surface the existing window, not start a
            // second monitoring engine and a second set of firewall
            // bookkeeping racing the first.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_dialog::init())
        .manage({
            debug_log("building AppState (COM/firewall worker thread starts here)");
            let state = Arc::new(AppState::new());
            debug_log("AppState built OK");
            state
        })
        .setup(|app| {
            debug_log("setup() entered");
            // The *same* `Arc` commands receive via `State<'_, Arc<AppState>>`
            // - background tasks and command handlers must observe each
            // other's writes (a rule the monitor loop creates has to show up
            // in `list_firewall_rules`, a connection it flags has to show up
            // as suspicious in `get_connections`), so there is deliberately
            // only ever one `AppState` instance for the whole process.
            let shared = app.state::<Arc<AppState>>().inner().clone();

            let reconcile_target = shared.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = reconcile_target.firewall.reconcile_on_startup().await {
                    let _ = logging::append(store::models::LogCategory::Warning, format!("Startup firewall reconciliation failed: {e}"));
                }
            });

            monitor::spawn(shared.clone());
            license::spawn_revalidation_loop(shared.license.clone());

            debug_log(&format!("setup() got main window: {}", app.get_webview_window("main").is_some()));
            if let Some(window) = app.get_webview_window("main") {
                let shown = window.show();
                debug_log(&format!("window.show() result: {shown:?}"));
            }
            debug_log("setup() returning Ok");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_dashboard_summary,
            commands::system::get_app_info,
            commands::connections::get_connections,
            commands::connections::block_connection_ip,
            commands::connections::block_connection_app,
            commands::protection::get_protection_settings,
            commands::protection::save_protection_settings,
            commands::protection::set_lockdown_mode,
            commands::firewall::get_firewall_status,
            commands::firewall::list_firewall_rules,
            commands::firewall::create_ip_block_rule,
            commands::firewall::create_app_block_rule,
            commands::firewall::remove_firewall_rule,
            commands::firewall::set_firewall_rule_enabled,
            commands::firewall::reset_thomsen_shield_rules,
            commands::blocklist::list_blocklist,
            commands::blocklist::add_manual_block,
            commands::blocklist::unblock_entry,
            commands::blocklist::make_block_permanent,
            commands::blocklist::clear_block_history,
            commands::allowlist::list_allowlist,
            commands::allowlist::add_allowlist_entry,
            commands::allowlist::remove_allowlist_entry,
            commands::logs::list_logs,
            commands::logs::export_logs,
            commands::logs::clear_logs,
            commands::license::get_license_info,
            commands::license::activate_license,
            commands::license::deactivate_license,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::complete_onboarding,
        ])
        .build(tauri::generate_context!())
        .map(|app| {
            debug_log("builder.build() succeeded, entering app.run() (blocks until exit)");
            app.run(|_app_handle, event| {
                if let tauri::RunEvent::ExitRequested { .. } = event {
                    debug_log("ExitRequested");
                }
            });
            debug_log("app.run() returned - process is about to exit normally");
        })
        .unwrap_or_else(|e| {
            debug_log(&format!("builder.build() FAILED: {e:?}"));
            panic!("error while running the Thomsen Shield application: {e}");
        });
}
