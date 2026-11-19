//! Updates via Tauri's official updater. Only signed artifacts are installed
//! (the updater verifies the minisign signature against the public key in the
//! app configuration); an unconfigured/empty key disables updating entirely
//! rather than falling back to anything unsigned.

use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;
use thomsen_optimizer_core::Error;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub configured: bool,
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
}

fn pubkey_configured(app: &AppHandle) -> bool {
    app.config().plugins.0.get("updater").and_then(|u| u.get("pubkey")).and_then(|k| k.as_str()).is_some_and(|k| !k.trim().is_empty())
}

#[tauri::command]
pub async fn update_check(app: AppHandle, state: State<'_, AppState>) -> Result<UpdateInfo, Error> {
    let current = app.package_info().version.to_string();
    if !pubkey_configured(&app) {
        return Ok(UpdateInfo { configured: false, available: false, current_version: current, version: None, notes: None, date: None });
    }
    let updater = app.updater().map_err(|e| Error::Command(format!("Updates are unavailable: {e}")))?;
    let found = updater.check().await.map_err(|e| Error::Command(format!("Could not check for updates: {e}")))?;
    let info = match &found {
        Some(u) => UpdateInfo { configured: true, available: true, current_version: current, version: Some(u.version.clone()), notes: u.body.clone(), date: u.date.map(|d| d.to_string()) },
        None => UpdateInfo { configured: true, available: false, current_version: current, version: None, notes: None, date: None },
    };
    *state.pending_update.lock().unwrap() = found;
    Ok(info)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
    stage: &'static str,
}

/// Downloads, verifies the signature and installs the update found by the
/// last check. The UI must have asked the user to confirm first.
#[tauri::command]
pub async fn update_install(app: AppHandle, state: State<'_, AppState>) -> Result<(), Error> {
    let update = state.pending_update.lock().unwrap().take().ok_or_else(|| Error::Invalid("Check for updates first.".into()))?;
    // Never replace the app while a gaming session has changed Windows settings.
    state.engine.deactivate_profile();
    let mut downloaded = 0u64;
    let a = app.clone();
    let b = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = a.emit("update://progress", Progress { downloaded, total, stage: "downloading" });
            },
            move || {
                let _ = b.emit("update://progress", Progress { downloaded: 0, total: None, stage: "installing" });
            },
        )
        .await
        .map_err(|e| Error::Command(format!("The update could not be installed: {e}. Your current version was left unchanged.")))?;
    app.restart();
}
