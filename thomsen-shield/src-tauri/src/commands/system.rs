use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::store::models::{BlockStatus, DashboardSummary};
use crate::store::{blocklist, settings};

#[tauri::command]
pub async fn get_dashboard_summary(state: State<'_, Arc<AppState>>) -> Result<DashboardSummary, AppError> {
    let current_settings = settings::load()?;
    let firewall_status = state.firewall.status().await?;
    let blocked_ips = blocklist::list()?.into_iter().filter(|e| e.status == BlockStatus::Active).count() as u32;

    // Reuses the background monitor loop's own numbers rather than taking
    // a second, redundant connection snapshot every time the Dashboard
    // polls - see monitor.rs.
    let active_connections = *state.activity_points.lock().await.back().unwrap_or(&0);
    let suspicious_connections = state.detection.lock().await.suspicious_ips().len() as u32;
    let activity_points: Vec<u32> = state.activity_points.lock().await.iter().copied().collect();

    Ok(DashboardSummary {
        protection_active: current_settings.protection.enabled,
        firewall_active: firewall_status.active,
        lockdown_active: current_settings.lockdown_mode,
        active_connections,
        blocked_ips,
        suspicious_connections,
        activity_points,
    })
}

#[derive(serde::Serialize)]
pub struct AppInfo {
    pub version: String,
    pub load_warnings: Vec<String>,
}

#[tauri::command]
pub async fn get_app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo { version: env!("CARGO_PKG_VERSION").to_string(), load_warnings: Vec::new() })
}
