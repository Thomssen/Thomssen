use crate::error::AppError;
use crate::logging;
use crate::store::models::{LogCategory, LogEntry, LogExportFormat};

#[tauri::command]
pub async fn list_logs(category: Option<LogCategory>) -> Result<Vec<LogEntry>, AppError> {
    Ok(logging::list(category)?)
}

#[tauri::command]
pub async fn export_logs(format: LogExportFormat, category: Option<LogCategory>) -> Result<String, AppError> {
    let path = logging::export(format, category)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn clear_logs() -> Result<(), AppError> {
    logging::clear()?;
    Ok(())
}
