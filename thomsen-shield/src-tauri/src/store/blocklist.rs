use crate::error::AppResult;
use crate::store::models::BlockedEntry;

use super::{load_or_default, paths, save_atomic};

pub fn list() -> AppResult<Vec<BlockedEntry>> {
    load_or_default(&paths::blocklist_path()?)
}

fn save_all(entries: &[BlockedEntry]) -> AppResult<()> {
    save_atomic(&paths::blocklist_path()?, &entries)
}

pub fn add(entry: BlockedEntry) -> AppResult<BlockedEntry> {
    let mut entries = list()?;
    entries.insert(0, entry.clone());
    save_all(&entries)?;
    Ok(entry)
}

/// Replaces the entry with the same id - used for status/expiry
/// transitions (active -> expired/removed, adding a permanence flag).
pub fn update(updated: BlockedEntry) -> AppResult<()> {
    let mut entries = list()?;
    if let Some(slot) = entries.iter_mut().find(|e| e.id == updated.id) {
        *slot = updated;
    }
    save_all(&entries)
}

pub fn clear_inactive() -> AppResult<()> {
    let entries = list()?;
    let kept: Vec<_> = entries.into_iter().filter(|e| e.status == crate::store::models::BlockStatus::Active).collect();
    save_all(&kept)
}

pub fn clear_all() -> AppResult<()> {
    save_all(&[])
}
