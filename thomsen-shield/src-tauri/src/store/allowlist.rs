use crate::error::AppResult;
use crate::store::models::AllowlistEntry;

use super::{load_or_default, paths, save_atomic, seed_default_allowlist};

pub fn list() -> AppResult<Vec<AllowlistEntry>> {
    let path = paths::allowlist_path()?;
    let mut entries: Vec<AllowlistEntry> = load_or_default(&path)?;
    if entries.is_empty() {
        entries = seed_default_allowlist();
        save_atomic(&path, &entries)?;
    }
    Ok(entries)
}

fn save_all(entries: &[AllowlistEntry]) -> AppResult<()> {
    save_atomic(&paths::allowlist_path()?, &entries)
}

pub fn add(value: String, label: Option<String>) -> AppResult<AllowlistEntry> {
    let mut entries = list()?;
    let entry = AllowlistEntry { id: uuid::Uuid::new_v4().to_string(), value, label, is_built_in: false, created_at: chrono::Utc::now() };
    entries.push(entry.clone());
    save_all(&entries)?;
    Ok(entry)
}

/// Built-in entries are silently kept even if a caller asks to remove one -
/// the local network ranges are never user-removable, see the spec.
pub fn remove(id: &str) -> AppResult<()> {
    let mut entries = list()?;
    entries.retain(|e| e.id != id || e.is_built_in);
    save_all(&entries)
}
