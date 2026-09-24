//! Shared data model - mirrors `src/types.ts` field-for-field. Every struct
//! here is `rename_all = "camelCase"` so the JSON sent to the frontend needs
//! no translation layer on either side.

use serde::{Deserialize, Serialize};

// -- Dashboard ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub protection_active: bool,
    pub firewall_active: bool,
    pub lockdown_active: bool,
    pub active_connections: u32,
    pub blocked_ips: u32,
    pub suspicious_connections: u32,
    pub activity_points: Vec<u32>,
}

// -- Live connections -------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionProtocol {
    #[serde(rename = "TCP")]
    Tcp,
    #[serde(rename = "UDP")]
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Established,
    Listening,
    SynSent,
    SynReceived,
    FinWait1,
    FinWait2,
    TimeWait,
    CloseWait,
    Closing,
    LastAck,
    DeleteTcb,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveConnection {
    pub id: String,
    pub app_name: String,
    pub process_id: u32,
    pub process_path: Option<String>,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: String,
    pub remote_port: u16,
    pub protocol: ConnectionProtocol,
    pub status: ConnectionStatus,
    pub is_allowlisted: bool,
    pub is_suspicious: bool,
}

// -- Protection ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensitivityLevel {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockDuration {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "24h")]
    TwentyFourHours,
    #[serde(rename = "permanent")]
    Permanent,
}

impl BlockDuration {
    /// `None` means permanent (never expires).
    pub fn to_chrono_duration(self) -> Option<chrono::Duration> {
        match self {
            BlockDuration::FiveMinutes => Some(chrono::Duration::minutes(5)),
            BlockDuration::ThirtyMinutes => Some(chrono::Duration::minutes(30)),
            BlockDuration::OneHour => Some(chrono::Duration::hours(1)),
            BlockDuration::TwentyFourHours => Some(chrono::Duration::hours(24)),
            BlockDuration::Permanent => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectionSettings {
    pub enabled: bool,
    pub detect_connection_spikes: bool,
    pub detect_repeated_connections: bool,
    pub auto_block_suspicious_ips: bool,
    pub block_duration: BlockDuration,
    pub sensitivity: SensitivityLevel,
}

impl Default for ProtectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_connection_spikes: true,
            detect_repeated_connections: true,
            auto_block_suspicious_ips: true,
            block_duration: BlockDuration::ThirtyMinutes,
            sensitivity: SensitivityLevel::Normal,
        }
    }
}

// -- Firewall -----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirewallRuleDirection {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirewallRuleAction {
    Block,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirewallRuleTargetType {
    Ip,
    Application,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallRule {
    pub id: String,
    pub name: String,
    pub target_type: FirewallRuleTargetType,
    pub target: String,
    pub direction: FirewallRuleDirection,
    pub action: FirewallRuleAction,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallProfilesEnabled {
    pub domain: bool,
    pub private: bool,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirewallStatus {
    pub active: bool,
    pub profiles_enabled: FirewallProfilesEnabled,
}

// -- Blocklist ------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    Manual,
    ConnectionSpike,
    RepeatedConnections,
    Lockdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockStatus {
    Active,
    Expired,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedEntry {
    pub id: String,
    pub ip: String,
    pub reason: BlockReason,
    pub note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: BlockStatus,
}

// -- Allowlist ----------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistEntry {
    pub id: String,
    pub value: String,
    pub label: Option<String>,
    pub is_built_in: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Seeded on first run - see `store::seed_default_allowlist`. Never
/// removable from the UI; kept out of the persisted file so a future
/// update to this list doesn't require a migration.
pub const BUILT_IN_ALLOWLIST: &[(&str, &str)] = &[
    ("127.0.0.1", "Loopback"),
    ("10.0.0.0/8", "Private network"),
    ("172.16.0.0/12", "Private network"),
    ("192.168.0.0/16", "Private network"),
];

// -- Events / logs ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogCategory {
    Protection,
    Firewall,
    Blocked,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub category: LogCategory,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogExportFormat {
    Txt,
    Json,
}

// -- Licensing ------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseStatus {
    Unlicensed,
    Active,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseProviderKind {
    Dev,
    Remote,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseInfo {
    pub status: LicenseStatus,
    pub key: Option<String>,
    pub activated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub provider: LicenseProviderKind,
}

// -- Setup wizard / settings --------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionMode {
    Standard,
    Strict,
    MonitoringOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub onboarding_completed: bool,
    pub protection_mode: ProtectionMode,

    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,

    pub protection: ProtectionSettings,
    pub lockdown_mode: bool,

    pub notify_on_blocked_ip: bool,
    pub notify_on_suspicious_activity: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarding_completed: false,
            protection_mode: ProtectionMode::Standard,
            start_with_windows: false,
            start_minimized: false,
            minimize_to_tray: true,
            protection: ProtectionSettings::default(),
            lockdown_mode: false,
            notify_on_blocked_ip: true,
            notify_on_suspicious_activity: true,
        }
    }
}
