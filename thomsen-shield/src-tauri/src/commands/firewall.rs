use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::store::models::{FirewallRule, FirewallRuleDirection, FirewallStatus, LogCategory};
use crate::{logging, netutil};

#[tauri::command]
pub async fn get_firewall_status(state: State<'_, Arc<AppState>>) -> Result<FirewallStatus, AppError> {
    Ok(state.firewall.status().await?)
}

#[tauri::command]
pub async fn list_firewall_rules(state: State<'_, Arc<AppState>>) -> Result<Vec<FirewallRule>, AppError> {
    Ok(state.firewall.list_rules().await?)
}

#[tauri::command]
pub async fn create_ip_block_rule(ip: String, direction: FirewallRuleDirection, state: State<'_, Arc<AppState>>) -> Result<FirewallRule, AppError> {
    if let Some(reason) = netutil::reject_unsafe_block_target(&ip) {
        return Err(AppError::new(reason));
    }
    let rule = state.firewall.create_ip_rule(&ip, direction).await?;
    logging::append(LogCategory::Firewall, format!("Created rule {}", rule.name))?;
    Ok(rule)
}

#[tauri::command]
pub async fn create_app_block_rule(path: String, direction: FirewallRuleDirection, state: State<'_, Arc<AppState>>) -> Result<FirewallRule, AppError> {
    let rule = state.firewall.create_app_rule(&path, direction).await?;
    logging::append(LogCategory::Firewall, format!("Created rule {}", rule.name))?;
    Ok(rule)
}

#[tauri::command]
pub async fn remove_firewall_rule(id: String, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.firewall.remove_rule(&id).await?;
    logging::append(LogCategory::Firewall, "Removed a firewall rule")?;
    Ok(())
}

#[tauri::command]
pub async fn set_firewall_rule_enabled(id: String, enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.firewall.set_rule_enabled(&id, enabled).await?;
    logging::append(LogCategory::Firewall, if enabled { "Enabled a firewall rule" } else { "Disabled a firewall rule" })?;
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetResult {
    pub removed: usize,
}

#[tauri::command]
pub async fn reset_thomsen_shield_rules(state: State<'_, Arc<AppState>>) -> Result<ResetResult, AppError> {
    let removed = state.firewall.reset_all().await?;
    logging::append(LogCategory::Firewall, format!("Reset: removed {removed} Thomsen Shield rule(s)"))?;
    Ok(ResetResult { removed })
}
