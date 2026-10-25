use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// `%PROGRAMDATA%\ThomsenShield` - one shared home for all persisted state
/// (settings, blocklist, allowlist, the firewall rules ledger, logs, the
/// cached license). Writable by any user by default on Windows, so none of
/// this requires elevation - only the actual firewall API calls do. Shared
/// across Windows accounts deliberately: Thomsen Shield protects the PC,
/// not a single user's profile.
pub fn data_dir() -> AppResult<PathBuf> {
    let base = std::env::var("PROGRAMDATA")
        .map_err(|_| AppError::new("Couldn't resolve %PROGRAMDATA% - is this running on Windows?"))?;
    let dir = PathBuf::from(base).join("ThomsenShield");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn settings_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("settings.json"))
}

pub fn blocklist_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("blocklist.json"))
}

pub fn allowlist_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("allowlist.json"))
}

pub fn rules_ledger_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("firewall_rules.json"))
}

pub fn logs_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("events.json"))
}

pub fn license_cache_path() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("license.dat"))
}

pub fn exports_dir() -> AppResult<PathBuf> {
    let dir = data_dir()?.join("exports");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
