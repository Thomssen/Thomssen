use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::store::models::{BlockReason, BlockStatus, BlockedEntry, FirewallRuleDirection, LogCategory};
use crate::store::blocklist;
use crate::logging;

#[tauri::command]
pub async fn list_blocklist() -> Result<Vec<BlockedEntry>, AppError> {
    Ok(blocklist::list()?)
}

#[tauri::command]
pub async fn add_manual_block(ip: String, note: Option<String>, state: State<'_, Arc<AppState>>) -> Result<BlockedEntry, AppError> {
    let entry = blocklist::add(BlockedEntry {
        id: uuid::Uuid::new_v4().to_string(),
        ip: ip.clone(),
        reason: BlockReason::Manual,
        note,
        created_at: chrono::Utc::now(),
        expires_at: None,
        status: BlockStatus::Active,
    })?;
    state.firewall.create_ip_rule(&ip, FirewallRuleDirection::In).await?;
    logging::append(LogCategory::Blocked, format!("{ip} manually blocked"))?;
    Ok(entry)
}

#[tauri::command]
pub async fn unblock_entry(id: String, state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    let entries = blocklist::list()?;
    let Some(mut entry) = entries.into_iter().find(|e| e.id == id) else { return Ok(()) };
    entry.status = BlockStatus::Removed;
    let ip = entry.ip.clone();
    blocklist::update(entry)?;

    let rules = state.firewall.list_rules().await?;
    if let Some(rule) = rules.into_iter().find(|r| r.target == ip) {
        state.firewall.remove_rule(&rule.id).await?;
    }
    logging::append(LogCategory::Blocked, format!("{ip} unblocked"))?;
    Ok(())
}

#[tauri::command]
pub async fn make_block_permanent(id: String) -> Result<(), AppError> {
    let entries = blocklist::list()?;
    let Some(mut entry) = entries.into_iter().find(|e| e.id == id) else { return Ok(()) };
    entry.expires_at = None;
    let ip = entry.ip.clone();
    blocklist::update(entry)?;
    logging::append(LogCategory::Blocked, format!("{ip} is now permanently blocked"))?;
    Ok(())
}

#[tauri::command]
pub async fn clear_block_history() -> Result<(), AppError> {
    blocklist::clear_inactive()?;
    Ok(())
}
