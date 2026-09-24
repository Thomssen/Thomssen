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

pub fn run() {
    // This same executable is re-launched with this flag, elevated, to
    // perform exactly one batch of firewall writes - see `elevation.rs`
    // and `firewall::elevated_ops`. That path never touches Tauri/WebView2
    // at all, so it's handled and exited before anything else here runs.
    if elevation::is_elevated_child_invocation() {
        firewall::elevated_ops::run_elevated_child();
    }

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
        .manage(Arc::new(AppState::new()))
        .setup(|app| {
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

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }

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
        .run(tauri::generate_context!())
        .expect("error while running the Thomsen Shield application");
}
