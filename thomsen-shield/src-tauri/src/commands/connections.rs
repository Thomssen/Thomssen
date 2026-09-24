use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::store::models::{BlockReason, BlockStatus, BlockedEntry, FirewallRuleDirection, LiveConnection, LogCategory};
use crate::store::{allowlist, blocklist};
use crate::{logging, network};

#[tauri::command]
pub async fn get_connections(state: State<'_, Arc<AppState>>) -> Result<Vec<LiveConnection>, AppError> {
    let allowlist_values: Vec<String> = allowlist::list()?.into_iter().map(|e| e.value).collect();
    let suspicious = state.detection.lock().await.suspicious_ips();
    Ok(network::connections::snapshot(&allowlist_values, &suspicious)?)
}

#[tauri::command]
pub async fn block_connection_ip(ip: String, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    blocklist::add(BlockedEntry {
        id: uuid::Uuid::new_v4().to_string(),
        ip: ip.clone(),
        reason: BlockReason::Manual,
        note: None,
        created_at: chrono::Utc::now(),
        expires_at: None,
        status: BlockStatus::Active,
    })?;
    state.firewall.create_ip_rule(&ip, FirewallRuleDirection::In).await?;
    logging::append(LogCategory::Blocked, format!("{ip} manually blocked"))?;
    Ok(())
}

#[tauri::command]
pub async fn block_connection_app(path: String, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.firewall.create_app_rule(&path, FirewallRuleDirection::Out).await?;
    logging::append(LogCategory::Firewall, format!("{path} blocked"))?;
    Ok(())
}
