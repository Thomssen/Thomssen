pub mod allowlist;
pub mod blocklist;
pub mod models;
pub mod paths;
pub mod settings;

use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};

use crate::error::AppResult;

/// Reads a JSON file if it exists, or returns `T::default()` if it doesn't
/// (first run) - never errors just because nothing has been saved yet.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> AppResult<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(T::default());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

/// Writes via a temp file + rename so a crash mid-write can never leave a
/// half-written, corrupt file behind - the safety guarantee the spec asks
/// for ("do not leave broken ... state every time the application
/// restarts") applied to every store, not just the firewall ledger.
pub fn save_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let json = serde_json::to_vec_pretty(value)?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Shallow-merges a JSON patch object's top-level keys into `current`,
/// then re-parses the result back into `T`. The frontend always sends
/// complete nested objects rather than partially-nested ones (e.g. a
/// changed `protection` field carries the *whole* `ProtectionSettings`,
/// not one field of it - see `SettingsPage.tsx`), so a shallow merge is
/// exactly right and a full recursive merge would be unnecessary.
pub fn merge_patch<T: Serialize + DeserializeOwned>(current: &T, patch: serde_json::Value) -> AppResult<T> {
    let mut current_value = serde_json::to_value(current)?;
    if let (Some(current_obj), Some(patch_obj)) = (current_value.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj {
            current_obj.insert(k.clone(), v.clone());
        }
    }
    Ok(serde_json::from_value(current_value)?)
}

pub fn seed_default_allowlist() -> Vec<models::AllowlistEntry> {
    let now = chrono::Utc::now();
    models::BUILT_IN_ALLOWLIST
        .iter()
        .map(|(value, label)| models::AllowlistEntry {
            id: uuid::Uuid::new_v4().to_string(),
            value: value.to_string(),
            label: Some(label.to_string()),
            is_built_in: true,
            created_at: now,
        })
        .collect()
}
