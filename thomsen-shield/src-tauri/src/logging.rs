//! Events & Logs - a flat, append-only history capped at a sane size so it
//! never grows without bound on a machine that's been protected for months.

use crate::error::AppResult;
use crate::store::models::{LogCategory, LogEntry, LogExportFormat};
use crate::store::{load_or_default, paths, save_atomic};

const MAX_ENTRIES: usize = 5000;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct LogFile {
    entries: Vec<LogEntry>,
}

pub fn append(category: LogCategory, message: impl Into<String>) -> AppResult<()> {
    let path = paths::logs_path()?;
    let mut file: LogFile = load_or_default(&path)?;
    file.entries.push(LogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now(),
        category,
        message: message.into(),
    });
    if file.entries.len() > MAX_ENTRIES {
        let overflow = file.entries.len() - MAX_ENTRIES;
        file.entries.drain(0..overflow);
    }
    save_atomic(&path, &file)
}

pub fn list(category: Option<LogCategory>) -> AppResult<Vec<LogEntry>> {
    let file: LogFile = load_or_default(&paths::logs_path()?)?;
    let mut entries = file.entries;
    if let Some(cat) = category {
        entries.retain(|e| e.category == cat);
    }
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(entries)
}

pub fn clear() -> AppResult<()> {
    save_atomic(&paths::logs_path()?, &LogFile::default())
}

pub fn export(format: LogExportFormat, category: Option<LogCategory>) -> AppResult<std::path::PathBuf> {
    let entries = list(category)?;
    let dir = paths::exports_dir()?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");

    match format {
        LogExportFormat::Json => {
            let path = dir.join(format!("thomsen-shield-events-{stamp}.json"));
            let json = serde_json::to_vec_pretty(&entries)?;
            std::fs::write(&path, json)?;
            Ok(path)
        }
        LogExportFormat::Txt => {
            let path = dir.join(format!("thomsen-shield-events-{stamp}.txt"));
            let mut text = String::new();
            for e in &entries {
                text.push_str(&format!("{} [{:?}] {}\n", e.timestamp.format("%Y-%m-%d %H:%M:%S"), e.category, e.message));
            }
            std::fs::write(&path, text)?;
            Ok(path)
        }
    }
}
