use crate::error::AppError;
use crate::store::models::{LogCategory, ProtectionSettings};
use crate::store::{merge_patch, settings};
use crate::logging;

#[tauri::command]
pub async fn get_protection_settings() -> Result<ProtectionSettings, AppError> {
    Ok(settings::load()?.protection)
}

#[tauri::command]
pub async fn save_protection_settings(patch: serde_json::Value) -> Result<ProtectionSettings, AppError> {
    let mut current = settings::load()?;
    current.protection = merge_patch(&current.protection, patch)?;
    settings::save(&current)?;
    Ok(current.protection)
}

#[tauri::command]
pub async fn set_lockdown_mode(enabled: bool) -> Result<(), AppError> {
    let mut current = settings::load()?;
    current.lockdown_mode = enabled;
    settings::save(&current)?;
    logging::append(LogCategory::Protection, if enabled { "Lockdown Mode enabled" } else { "Lockdown Mode disabled" })?;
    Ok(())
}
