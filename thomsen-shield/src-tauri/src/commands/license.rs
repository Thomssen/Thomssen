use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use crate::store::models::LicenseInfo;

#[tauri::command]
pub async fn get_license_info(state: State<'_, Arc<AppState>>) -> Result<LicenseInfo, AppError> {
    Ok(state.license.info())
}

#[tauri::command]
pub async fn activate_license(key: String, state: State<'_, Arc<AppState>>) -> Result<LicenseInfo, AppError> {
    state.license.activate(&key).await
}

#[tauri::command]
pub async fn deactivate_license(state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
    state.license.deactivate().await
}
