/**
 * Shared domain types - the contract between the React UI and the Rust
 * backend. Field names are camelCase on both sides (the Rust structs carry
 * `#[serde(rename_all = "camelCase")]`), so these mirror `src-tauri/src`
 * exactly.
 */

// -- Dashboard ----------------------------------------------------------------

export interface DashboardSummary {
  protectionActive: boolean;
  firewallActive: boolean;
  lockdownActive: boolean;
  activeConnections: number;
  blockedIps: number;
  suspiciousConnections: number;
  /** Recent connection-count samples, oldest first, for the small live graph. */
  activityPoints: number[];
}

// -- Live connections -----------------------------------------------------------

export type ConnectionProtocol = "TCP" | "UDP";

export type ConnectionStatus =
  | "Established"
  | "Listening"
  | "SynSent"
  | "SynReceived"
  | "FinWait1"
  | "FinWait2"
  | "TimeWait"
  | "CloseWait"
  | "Closing"
  | "LastAck"
  | "DeleteTcb"
  | "Closed"
  | "Unknown";

export interface LiveConnection {
  id: string;
  appName: string;
  processId: number;
  processPath: string | null;
  localAddress: string;
  localPort: number;
  remoteAddress: string;
  remotePort: number;
  protocol: ConnectionProtocol;
  status: ConnectionStatus;
  isAllowlisted: boolean;
  isSuspicious: boolean;
}

// -- Protection -----------------------------------------------------------------

export type SensitivityLevel = "low" | "normal" | "high";
export type BlockDuration = "5m" | "30m" | "1h" | "24h" | "permanent";

export interface ProtectionSettings {
  enabled: boolean;
  detectConnectionSpikes: boolean;
  detectRepeatedConnections: boolean;
  autoBlockSuspiciousIps: boolean;
  blockDuration: BlockDuration;
  sensitivity: SensitivityLevel;
}

export const BLOCK_DURATION_LABELS: Record<BlockDuration, string> = {
  "5m": "5 minutes",
  "30m": "30 minutes",
  "1h": "1 hour",
  "24h": "24 hours",
  permanent: "Permanent",
};

export const SENSITIVITY_LABELS: Record<SensitivityLevel, string> = {
  low: "Low",
  normal: "Normal",
  high: "High",
};

// -- Firewall ---------------------------------------------------------------------

export type FirewallRuleDirection = "in" | "out";
export type FirewallRuleAction = "block" | "allow";
export type FirewallRuleTargetType = "ip" | "application";

export interface FirewallRule {
  id: string;
  name: string;
  targetType: FirewallRuleTargetType;
  target: string;
  direction: FirewallRuleDirection;
  action: FirewallRuleAction;
  enabled: boolean;
  createdAt: string;
}

export interface FirewallStatus {
  active: boolean;
  profilesEnabled: {
    domain: boolean;
    private: boolean;
    public: boolean;
  };
}

// -- Blocklist ------------------------------------------------------------------

export type BlockReason =
  | "manual"
  | "connection_spike"
  | "repeated_connections"
  | "lockdown";

export const BLOCK_REASON_LABELS: Record<BlockReason, string> = {
  manual: "Manually blocked",
  connection_spike: "Connection spike detected",
  repeated_connections: "Repeated connections detected",
  lockdown: "Blocked by Lockdown Mode",
};

export type BlockStatus = "active" | "expired" | "removed";

export interface BlockedEntry {
  id: string;
  ip: string;
  reason: BlockReason;
  note: string | null;
  createdAt: string;
  expiresAt: string | null;
  status: BlockStatus;
}

// -- Allowlist --------------------------------------------------------------------

export interface AllowlistEntry {
  id: string;
  value: string;
  label: string | null;
  isBuiltIn: boolean;
  createdAt: string;
}

// -- Events / logs ----------------------------------------------------------------

export type LogCategory = "protection" | "firewall" | "blocked" | "warning";

export const LOG_CATEGORY_LABELS: Record<LogCategory, string> = {
  protection: "Protection",
  firewall: "Firewall",
  blocked: "Blocked",
  warning: "Warnings",
};

export interface LogEntry {
  id: string;
  timestamp: string;
  category: LogCategory;
  message: string;
}

export type LogExportFormat = "txt" | "json";

// -- Licensing --------------------------------------------------------------------

export type LicenseStatus = "unlicensed" | "active" | "invalid";

export interface LicenseInfo {
  status: LicenseStatus;
  key: string | null;
  activatedAt: string | null;
  /** Which provider validated this license - see src-tauri/src/license. The
   * DEV provider is a local stand-in; `remote` is the real backend, wired in
   * later without changing this shape. */
  provider: "dev" | "remote";
}

// -- Setup wizard / settings --------------------------------------------------------

export type ProtectionMode = "standard" | "strict" | "monitoring_only";

export interface Settings {
  onboardingCompleted: boolean;
  protectionMode: ProtectionMode;

  startWithWindows: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;

  protection: ProtectionSettings;
  lockdownMode: boolean;

  notifyOnBlockedIp: boolean;
  notifyOnSuspiciousActivity: boolean;
}

// -- Errors -------------------------------------------------------------------------

export interface AppErrorShape {
  message: string;
  code?: string;
}
