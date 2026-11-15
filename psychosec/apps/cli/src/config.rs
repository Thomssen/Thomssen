use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const APP_NAME: &str = "PSYCHOSEC";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Backend base URL. Fixed at build time. Runtime override is honoured only in
/// debug builds so a release binary cannot be silently redirected.
pub fn api_base() -> String {
    let compiled = option_env!("PSYCHOSEC_API_URL").unwrap_or("https://api.psychosec.org");
    if cfg!(debug_assertions) {
        if let Ok(v) = std::env::var("PSYCHOSEC_API_URL") {
            return v.trim_end_matches('/').to_string();
        }
    }
    compiled.trim_end_matches('/').to_string()
}

/// Ed25519 public key (base64) used to verify signed release manifests.
pub fn update_pubkey() -> Option<&'static str> {
    option_env!("PSYCHOSEC_UPDATE_PUBKEY").filter(|s| !s.is_empty())
}

pub fn data_dir() -> PathBuf {
    let d = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PSYCHOSEC");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn reports_dir() -> PathBuf {
    let d = dirs::document_dir().unwrap_or_else(data_dir).join("PSYCHOSEC").join("reports");
    let _ = std::fs::create_dir_all(&d);
    d
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Check for updates on startup.
    pub update_check: bool,
    /// Download (never install) updates automatically when one is found.
    pub auto_download: bool,
    /// Per-request network timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { update_check: true, auto_download: false, timeout_secs: 12 }
    }
}

impl Settings {
    fn path() -> PathBuf {
        data_dir().join("settings.json")
    }
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(), s).map_err(|e| e.to_string())
    }
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let b = |v: &str| match v.to_ascii_lowercase().as_str() {
            "on" | "true" | "yes" | "1" => Ok(true),
            "off" | "false" | "no" | "0" => Ok(false),
            _ => Err(format!("'{v}' is not a valid on/off value")),
        };
        match key {
            "update_check" => self.update_check = b(value)?,
            "auto_download" => self.auto_download = b(value)?,
            "timeout_secs" => {
                let n: u64 = value.parse().map_err(|_| "timeout_secs must be a number".to_string())?;
                if !(3..=120).contains(&n) {
                    return Err("timeout_secs must be between 3 and 120".into());
                }
                self.timeout_secs = n;
            }
            _ => return Err(format!("unknown setting '{key}' (update_check, auto_download, timeout_secs)")),
        }
        self.save()
    }
}

/// Optional third-party API keys are read from the environment only.
pub fn env_key(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}
