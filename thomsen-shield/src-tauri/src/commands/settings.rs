use tauri_plugin_autostart::ManagerExt;

use crate::error::AppError;
use crate::logging;
use crate::store::models::LogCategory;
use crate::store::{merge_patch, settings};

fn sync_autostart(app: &tauri::AppHandle, enabled: bool) {
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    if let Err(e) = result {
        let _ = logging::append(LogCategory::Warning, format!("Couldn't update the Windows startup entry: {e}"));
    }
}

#[tauri::command]
pub async fn get_settings() -> Result<crate::store::models::Settings, AppError> {
    Ok(settings::load()?)
}

#[tauri::command]
pub async fn save_settings(patch: serde_json::Value, app: tauri::AppHandle) -> Result<crate::store::models::Settings, AppError> {
    let current = settings::load()?;
    let next = merge_patch(&current, patch)?;
    if next.start_with_windows != current.start_with_windows {
        sync_autostart(&app, next.start_with_windows);
    }
    settings::save(&next)?;
    Ok(next)
}

#[tauri::command]
pub async fn complete_onboarding(patch: serde_json::Value, app: tauri::AppHandle) -> Result<crate::store::models::Settings, AppError> {
    let current = settings::load()?;
    let mut next = merge_patch(&current, patch)?;
    next.onboarding_completed = true;
    if next.start_with_windows != current.start_with_windows {
        sync_autostart(&app, next.start_with_windows);
    }
    settings::save(&next)?;
    logging::append(LogCategory::Protection, "First-run setup completed")?;
    Ok(next)
}
