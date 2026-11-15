//! Client for the PSYCHOSEC backend. The backend makes every authorization decision;
//! the CLI only presents what the server tells it.
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub discord_id: String,
    pub username: String,
    pub display_name: Option<String>,
    #[allow(dead_code)]
    pub avatar_url: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub user: UserInfo,
    pub expires_at: String,
    #[serde(default)]
    pub role_verified: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Release {
    pub version: String,
    pub download_url: String,
    pub checksum_sha256: String,
    pub signature: Option<String>,
    #[allow(dead_code)]
    pub published_at: Option<String>,
    #[allow(dead_code)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ApiError {
    Network(String),
    /// 401: session unknown/expired/revoked.
    Unauthorized(String),
    /// 403: access revoked, banned, or no longer a member.
    Forbidden(String),
    Server(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(m) => write!(f, "cannot reach the PSYCHOSEC service: {m}"),
            ApiError::Unauthorized(m) | ApiError::Forbidden(m) | ApiError::Server(m) => write!(f, "{m}"),
        }
    }
}

#[derive(Deserialize)]
struct ErrBody {
    message: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PollResponse {
    pub status: String,
    pub token: Option<String>,
    pub session: Option<SessionInfo>,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct Api {
    http: reqwest::Client,
    base: String,
}

impl Api {
    pub fn new() -> Result<Self, String> {
        let base = crate::config::api_base();
        if !base.starts_with("https://") && !cfg!(debug_assertions) {
            return Err("API URL must use HTTPS".into());
        }
        let http = reqwest::Client::builder()
            .user_agent(format!("Psychosec/{}", crate::config::VERSION))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { http, base })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn login_url(&self, login_id: &str, challenge: &str) -> String {
        format!(
            "{}/api/auth/discord/start?client=cli&login_id={}&challenge={}&v={}",
            self.base,
            login_id,
            challenge,
            crate::config::VERSION
        )
    }

    async fn send<T: for<'de> Deserialize<'de>>(&self, rb: reqwest::RequestBuilder) -> Result<T, ApiError> {
        let resp = rb
            .header("x-psychosec-version", crate::config::VERSION)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.without_url().to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return resp.json::<T>().await.map_err(|e| ApiError::Server(format!("unexpected server response: {e}")));
        }
        let body: Option<ErrBody> = resp.json().await.ok();
        let msg = body
            .as_ref()
            .and_then(|b| b.message.clone().or(b.error.clone()))
            .unwrap_or_else(|| format!("server returned {status}"));
        Err(match status.as_u16() {
            401 => ApiError::Unauthorized(msg),
            403 => ApiError::Forbidden(msg),
            429 => ApiError::Server("too many requests, try again shortly".into()),
            _ => ApiError::Server(msg),
        })
    }

    pub async fn session(&self, token: &str) -> Result<SessionInfo, ApiError> {
        self.send(self.http.get(format!("{}/api/auth/session", self.base)).bearer_auth(token)).await
    }

    pub async fn refresh(&self, token: &str) -> Result<SessionInfo, ApiError> {
        self.send(self.http.post(format!("{}/api/auth/session", self.base)).bearer_auth(token)).await
    }

    pub async fn logout(&self, token: &str) -> Result<serde_json::Value, ApiError> {
        self.send(self.http.post(format!("{}/api/auth/logout", self.base)).bearer_auth(token)).await
    }

    pub async fn heartbeat(&self, token: &str, launch: bool) -> Result<serde_json::Value, ApiError> {
        #[derive(Serialize)]
        struct B {
            launch: bool,
            version: &'static str,
        }
        self.send(
            self.http
                .post(format!("{}/api/client/heartbeat", self.base))
                .bearer_auth(token)
                .json(&B { launch, version: crate::config::VERSION }),
        )
        .await
    }

    pub async fn poll_login(&self, login_id: &str, verifier: &str) -> Result<PollResponse, ApiError> {
        self.send(
            self.http
                .post(format!("{}/api/auth/cli/poll", self.base))
                .json(&serde_json::json!({ "login_id": login_id, "verifier": verifier, "version": crate::config::VERSION })),
        )
        .await
    }

    /// Latest release metadata. Sends the session token when available so the check is
    /// recorded as an operational event.
    pub async fn latest_release(&self, token: Option<&str>) -> Result<Option<Release>, ApiError> {
        #[derive(Deserialize)]
        struct R {
            release: Option<Release>,
        }
        let mut rb = self.http.get(format!("{}/api/client/version", self.base));
        if let Some(t) = token {
            rb = rb.bearer_auth(t);
        }
        Ok(self.send::<R>(rb).await?.release)
    }
}
