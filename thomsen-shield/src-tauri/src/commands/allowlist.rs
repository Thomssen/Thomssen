use crate::error::AppError;
use crate::store::allowlist;
use crate::store::models::AllowlistEntry;

#[tauri::command]
pub async fn list_allowlist() -> Result<Vec<AllowlistEntry>, AppError> {
    Ok(allowlist::list()?)
}

#[tauri::command]
pub async fn add_allowlist_entry(value: String, label: Option<String>) -> Result<AllowlistEntry, AppError> {
    Ok(allowlist::add(value, label)?)
}

#[tauri::command]
pub async fn remove_allowlist_entry(id: String) -> Result<(), AppError> {
    allowlist::remove(&id)?;
    Ok(())
}
