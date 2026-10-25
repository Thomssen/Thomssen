//! Licensing. The client never trusts a bare local boolean: activation and
//! periodic re-validation both go through the license API
//! (`thomsen-shield-license-api/`) and come back as an Ed25519-signed
//! token (`token.rs`) that's cached locally, encrypted at rest via DPAPI
//! (`store.rs`). Startup only ever checks that *cached* token's signature
//! and expiry - no network call - so licensing can never make startup
//! slow; a background task (`spawn_revalidation_loop`) is what keeps the
//! cached token fresh, and its `expires_at` is what gives an offline PC a
//! grace period rather than being cut off the moment connectivity drops.

pub mod client;
pub mod device_id;
pub mod store;
pub mod token;

use crate::error::{AppError, AppResult};
use crate::store::models::{LicenseInfo, LicenseProviderKind, LicenseStatus};

use token::TokenPayload;

pub struct LicenseManager {
    device_id: String,
}

impl LicenseManager {
    pub fn new() -> Self {
        Self { device_id: device_id::stable_device_id() }
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// Local-only: verifies the cached token's signature and expiry.
    /// Never makes a network call - this is what keeps startup fast.
    pub fn info(&self) -> LicenseInfo {
        match self.cached_payload() {
            // The device_id check matters on its own, not just as a
            // signature-adjacent formality: without it, a cache file valid
            // for a *different* machine (moved/restored by hand, or a
            // future multi-profile scenario) would read as licensed here
            // purely on expiry, even though no server ever bound it to
            // this device.
            Some(payload) if payload.expires_at > chrono::Utc::now() && payload.device_id == self.device_id => LicenseInfo {
                status: LicenseStatus::Active,
                key: Some(payload.license_key),
                activated_at: Some(payload.issued_at),
                provider: LicenseProviderKind::Remote,
            },
            // Either no cached token, it failed to verify, or the offline
            // grace period has lapsed - all three land the user back on
            // the activation screen. See the module doc for why this is
            // safe rather than harsh: a background revalidation succeeding
            // even once refreshes `expires_at` well before this happens in
            // ordinary use.
            _ => LicenseInfo { status: LicenseStatus::Unlicensed, key: None, activated_at: None, provider: LicenseProviderKind::Remote },
        }
    }

    fn cached_payload(&self) -> Option<TokenPayload> {
        let token = store::load_token().ok().flatten()?;
        token.verify(&token::license_signing_public_key()).ok()
    }

    pub async fn activate(&self, license_key: &str) -> AppResult<LicenseInfo> {
        let token = client::activate(license_key, &self.device_id).await?;
        // Verify locally before trusting/caching it, even though it just
        // came from the server over TLS - defense in depth, and it's the
        // exact same check `info()` will run on every future read.
        token.verify(&token::license_signing_public_key())?;
        store::save_token(&token)?;
        Ok(self.info())
    }

    pub async fn deactivate(&self) -> AppResult<()> {
        if let Some(token) = store::load_token()? {
            if let Some(payload) = token.verify(&token::license_signing_public_key()).ok() {
                client::deactivate(&payload.license_key, &self.device_id, &token).await?;
            }
        }
        store::clear_token()
    }

    /// One background revalidation attempt. A *transport* failure (offline,
    /// server unreachable, a timeout) is deliberately not something this
    /// needs to act on - the cached token's existing `expires_at` is the
    /// grace period, and it's simply left as-is until an attempt succeeds.
    ///
    /// An *explicit* rejection is different and must not wait for that
    /// expiry: `/api/license/validate` answers 403 the instant a license is
    /// disabled or refunded, and 409 the instant it's bound to a different
    /// device - see `lib/license-repo.ts` and that endpoint in the API
    /// project. Treating that the same as "server unreachable" would mean a
    /// disabled/refunded license keeps working locally for up to its
    /// token's full 14-day validity, even while online and being told no on
    /// every single check-in - a revocation-latency bug, not a grace
    /// period. `client::post_json` attaches the response's status code
    /// precisely so this can tell the two apart.
    pub async fn revalidate_once(&self) {
        let Ok(Some(token)) = store::load_token() else { return };
        let Ok(payload) = token.verify(&token::license_signing_public_key()) else { return };

        match client::validate(&payload.license_key, &self.device_id, &token).await {
            Ok(fresh) => {
                if fresh.verify(&token::license_signing_public_key()).is_ok() {
                    let _ = store::save_token(&fresh);
                }
            }
            Err(AppError { code: Some(code), .. }) if matches!(code.as_str(), "401" | "403" | "404" | "409") => {
                let _ = store::clear_token();
            }
            Err(_) => {
                // Transport-level failure (offline, DNS, timeout, 5xx) -
                // leave the cached token's own expiry as the grace period.
            }
        }
    }
}

impl Default for LicenseManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawns the periodic background check-in described in the module doc.
/// Fires once shortly after startup (so a freshly activated license is
/// confirmed quickly) and then on a multi-hour interval - never on
/// startup's own critical path.
pub fn spawn_revalidation_loop(manager: std::sync::Arc<LicenseManager>) {
    // tauri::async_runtime::spawn, not a raw tokio::spawn - see the
    // comment on the same fix in monitor::spawn for why.
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        loop {
            manager.revalidate_once().await;
            tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}
