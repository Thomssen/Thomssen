//! Talks to the license API - `thomsen-shield-license-api/` in this repo,
//! deployed separately (see that project's README). The desktop app only
//! ever calls these three endpoints; it never touches Supabase or Stripe
//! directly, and never sees a secret key - see that project's README for
//! why.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

use super::token::SignedToken;

/// Overridable via the `THOMSEN_SHIELD_LICENSE_API` environment variable
/// for local development against `vercel dev` or a preview deployment -
/// see the license API project's README ("DEVELOPMENT MODE"). Production
/// builds need nothing set.
fn base_url() -> String {
    std::env::var("THOMSEN_SHIELD_LICENSE_API").unwrap_or_else(|_| "https://api.thomssen.dev".to_string())
}

#[derive(Serialize)]
struct ActivateRequest<'a> {
    license_key: &'a str,
    device_id: &'a str,
}

#[derive(Serialize)]
struct ValidateRequest<'a> {
    license_key: &'a str,
    device_id: &'a str,
    token: &'a SignedToken,
}

#[derive(Serialize)]
struct DeactivateRequest<'a> {
    license_key: &'a str,
    device_id: &'a str,
    token: &'a SignedToken,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: SignedToken,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    error: String,
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::new(format!("Couldn't set up the license connection: {e}")))
}

async fn post_json<Req: Serialize, Resp: for<'de> Deserialize<'de>>(path: &str, body: &Req) -> AppResult<Resp> {
    let url = format!("{}{}", base_url(), path);
    let response = client()?.post(&url).json(body).send().await?;

    if response.status().is_success() {
        Ok(response.json::<Resp>().await?)
    } else {
        let status = response.status();
        let message = response
            .json::<ApiErrorBody>()
            .await
            .map(|b| b.error)
            .unwrap_or_else(|_| format!("The license server returned an error ({status})."));
        Err(AppError::new(message))
    }
}

pub async fn activate(license_key: &str, device_id: &str) -> AppResult<SignedToken> {
    let resp: TokenResponse = post_json("/api/license/activate", &ActivateRequest { license_key, device_id }).await?;
    Ok(resp.token)
}

pub async fn validate(license_key: &str, device_id: &str, token: &SignedToken) -> AppResult<SignedToken> {
    let resp: TokenResponse = post_json("/api/license/validate", &ValidateRequest { license_key, device_id, token }).await?;
    Ok(resp.token)
}

pub async fn deactivate(license_key: &str, device_id: &str, token: &SignedToken) -> AppResult<()> {
    let _: serde_json::Value = post_json("/api/license/deactivate", &DeactivateRequest { license_key, device_id, token }).await?;
    Ok(())
}
