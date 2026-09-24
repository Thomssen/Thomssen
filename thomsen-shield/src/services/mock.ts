/**
 * In-browser stand-in for the Rust backend, used only when the UI runs
 * outside Tauri (`npm run dev` in a plain browser). Lets the interface be
 * built and reviewed without a compiled native shell. Never shipped logic -
 * the real behavior lives entirely in `src-tauri/src`.
 */
import type {
  AllowlistEntry,
  BlockedEntry,
  DashboardSummary,
  FirewallRule,
  FirewallStatus,
  LicenseInfo,
  LiveConnection,
  LogEntry,
  ProtectionSettings,
  Settings,
} from "@/types";

let settings: Settings = {
  onboardingCompleted: true,
  protectionMode: "standard",
  startWithWindows: true,
  startMinimized: false,
  minimizeToTray: true,
  protection: {
    enabled: true,
    detectConnectionSpikes: true,
    detectRepeatedConnections: true,
    autoBlockSuspiciousIps: true,
    blockDuration: "30m",
    sensitivity: "normal",
  },
  lockdownMode: false,
  notifyOnBlockedIp: true,
  notifyOnSuspiciousActivity: true,
};

let license: LicenseInfo = {
  status: "unlicensed",
  key: null,
  activatedAt: null,
  provider: "dev",
};

const now = () => new Date().toISOString();
const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();
const minutesFromNow = (m: number) => new Date(Date.now() + m * 60_000).toISOString();

const connections: LiveConnection[] = [
  { id: "c1", appName: "Discord.exe", processId: 4821, processPath: "C:\\Users\\kristian\\AppData\\Local\\Discord\\app\\Discord.exe", localAddress: "192.168.1.42", localPort: 51422, remoteAddress: "162.159.135.234", remotePort: 443, protocol: "TCP", status: "Established", isAllowlisted: false, isSuspicious: false },
  { id: "c2", appName: "chrome.exe", processId: 9014, processPath: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe", localAddress: "192.168.1.42", localPort: 51500, remoteAddress: "142.250.premium.14", remotePort: 443, protocol: "TCP", status: "Established", isAllowlisted: false, isSuspicious: false },
  { id: "c3", appName: "steam.exe", processId: 2210, processPath: "C:\\Program Files (x86)\\Steam\\steam.exe", localAddress: "192.168.1.42", localPort: 27036, remoteAddress: "23.213.88.11", remotePort: 27017, protocol: "UDP", status: "Established", isAllowlisted: false, isSuspicious: false },
  { id: "c4", appName: "svchost.exe", processId: 1204, processPath: "C:\\Windows\\System32\\svchost.exe", localAddress: "192.168.1.42", localPort: 123, remoteAddress: "51.137.137.111", remotePort: 123, protocol: "UDP", status: "Established", isAllowlisted: true, isSuspicious: false },
  { id: "c5", appName: "unknown_process.exe", processId: 7788, processPath: "C:\\Users\\kristian\\AppData\\Local\\Temp\\unknown_process.exe", localAddress: "192.168.1.42", localPort: 51890, remoteAddress: "185.220.101.4", remotePort: 8080, protocol: "TCP", status: "Established", isAllowlisted: false, isSuspicious: true },
];

let blocklist: BlockedEntry[] = [
  { id: "b1", ip: "185.220.101.4", reason: "connection_spike", note: null, createdAt: minutesAgo(12), expiresAt: minutesFromNow(18), status: "active" },
  { id: "b2", ip: "45.155.204.19", reason: "repeated_connections", note: null, createdAt: minutesAgo(90), expiresAt: minutesAgo(60), status: "expired" },
  { id: "b3", ip: "203.0.113.55", reason: "manual", note: "Known scanner", createdAt: minutesAgo(1440), expiresAt: null, status: "active" },
];

let allowlist: AllowlistEntry[] = [
  { id: "a1", value: "127.0.0.1", label: "Loopback", isBuiltIn: true, createdAt: minutesAgo(999999) },
  { id: "a2", value: "10.0.0.0/8", label: "Private network", isBuiltIn: true, createdAt: minutesAgo(999999) },
  { id: "a3", value: "172.16.0.0/12", label: "Private network", isBuiltIn: true, createdAt: minutesAgo(999999) },
  { id: "a4", value: "192.168.0.0/16", label: "Private network", isBuiltIn: true, createdAt: minutesAgo(999999) },
];

let rules: FirewallRule[] = [
  { id: "r1", name: "ThomsenShield_Block_185.220.101.4", targetType: "ip", target: "185.220.101.4", direction: "in", action: "block", enabled: true, createdAt: minutesAgo(12) },
  { id: "r2", name: "ThomsenShield_Block_203.0.113.55", targetType: "ip", target: "203.0.113.55", direction: "in", action: "block", enabled: true, createdAt: minutesAgo(1440) },
];

let logs: LogEntry[] = [
  { id: "l1", timestamp: minutesAgo(12), category: "warning", message: "Suspicious connection rate detected from 185.220.101.4" },
  { id: "l2", timestamp: minutesAgo(12), category: "blocked", message: "185.220.101.4 temporarily blocked for 30 minutes" },
  { id: "l3", timestamp: minutesAgo(60), category: "blocked", message: "Block expired for 45.155.204.19" },
  { id: "l4", timestamp: minutesAgo(180), category: "protection", message: "Protection sensitivity changed to Normal" },
  { id: "l5", timestamp: minutesAgo(1440), category: "firewall", message: "Created rule ThomsenShield_Block_203.0.113.55" },
  { id: "l6", timestamp: minutesAgo(2000), category: "protection", message: "Protection enabled" },
];

const activityPoints = Array.from({ length: 30 }, (_, i) => Math.max(0, Math.round(20 + Math.sin(i / 3) * 8 + Math.random() * 6)));

function delay<T>(value: T, ms = 220): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), ms));
}

function uid(): string {
  return Math.random().toString(36).slice(2, 10);
}

export async function mockBackend<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  switch (command) {
    case "get_dashboard_summary": {
      const point = Math.max(0, Math.round(20 + Math.sin(Date.now() / 4000) * 8 + Math.random() * 6));
      activityPoints.push(point);
      if (activityPoints.length > 30) activityPoints.shift();
      const summary: DashboardSummary = {
        protectionActive: settings.protection.enabled,
        firewallActive: true,
        lockdownActive: settings.lockdownMode,
        activeConnections: connections.length,
        blockedIps: blocklist.filter((b) => b.status === "active").length,
        suspiciousConnections: connections.filter((c) => c.isSuspicious).length,
        activityPoints: [...activityPoints],
      };
      return delay(summary as T);
    }

    case "get_connections":
      return delay(connections as T);

    case "block_connection_ip": {
      const ip = args?.ip as string;
      blocklist = [{ id: uid(), ip, reason: "manual", note: null, createdAt: now(), expiresAt: null, status: "active" }, ...blocklist];
      logs = [{ id: uid(), timestamp: now(), category: "blocked", message: `${ip} manually blocked` }, ...logs];
      return delay(undefined as T);
    }

    case "block_connection_app":
      return delay(undefined as T);

    case "get_protection_settings":
      return delay(settings.protection as T);

    case "save_protection_settings": {
      settings = { ...settings, protection: { ...settings.protection, ...(args?.patch as Partial<ProtectionSettings>) } };
      return delay(settings.protection as T);
    }

    case "set_lockdown_mode": {
      settings = { ...settings, lockdownMode: Boolean(args?.enabled) };
      logs = [{ id: uid(), timestamp: now(), category: "protection", message: settings.lockdownMode ? "Lockdown Mode enabled" : "Lockdown Mode disabled" }, ...logs];
      return delay(undefined as T);
    }

    case "get_firewall_status":
      return delay({ active: true, profilesEnabled: { domain: true, private: true, public: true } } as FirewallStatus as T);

    case "list_firewall_rules":
      return delay(rules as T);

    case "create_ip_block_rule": {
      const ip = args?.ip as string;
      const rule: FirewallRule = { id: uid(), name: `ThomsenShield_Block_${ip}`, targetType: "ip", target: ip, direction: "in", action: "block", enabled: true, createdAt: now() };
      rules = [rule, ...rules];
      return delay(rule as T);
    }

    case "create_app_block_rule": {
      const path = args?.path as string;
      const fname = path.split(/[\\/]/).pop() ?? path;
      const rule: FirewallRule = { id: uid(), name: `ThomsenShield_Block_${fname}`, targetType: "application", target: path, direction: "out", action: "block", enabled: true, createdAt: now() };
      rules = [rule, ...rules];
      return delay(rule as T);
    }

    case "remove_firewall_rule": {
      rules = rules.filter((r) => r.id !== args?.id);
      return delay(undefined as T);
    }

    case "set_firewall_rule_enabled": {
      rules = rules.map((r) => (r.id === args?.id ? { ...r, enabled: Boolean(args?.enabled) } : r));
      return delay(undefined as T);
    }

    case "reset_thomsen_shield_rules": {
      const removed = rules.length;
      rules = [];
      logs = [{ id: uid(), timestamp: now(), category: "firewall", message: `Reset: removed ${removed} Thomsen Shield rule(s)` }, ...logs];
      return delay({ removed } as T);
    }

    case "list_blocklist":
      return delay(blocklist as T);

    case "add_manual_block": {
      const ip = args?.ip as string;
      const entry: BlockedEntry = { id: uid(), ip, reason: "manual", note: (args?.note as string) || null, createdAt: now(), expiresAt: null, status: "active" };
      blocklist = [entry, ...blocklist];
      return delay(entry as T);
    }

    case "unblock_entry": {
      blocklist = blocklist.map((b) => (b.id === args?.id ? { ...b, status: "removed" as const } : b));
      return delay(undefined as T);
    }

    case "make_block_permanent": {
      blocklist = blocklist.map((b) => (b.id === args?.id ? { ...b, expiresAt: null } : b));
      return delay(undefined as T);
    }

    case "list_allowlist":
      return delay(allowlist as T);

    case "add_allowlist_entry": {
      const entry: AllowlistEntry = { id: uid(), value: args?.value as string, label: (args?.label as string) || null, isBuiltIn: false, createdAt: now() };
      allowlist = [...allowlist, entry];
      return delay(entry as T);
    }

    case "remove_allowlist_entry": {
      allowlist = allowlist.filter((a) => a.id !== args?.id);
      return delay(undefined as T);
    }

    case "list_logs": {
      const category = args?.category as string | undefined;
      const result = category && category !== "all" ? logs.filter((l) => l.category === category) : logs;
      return delay(result as T);
    }

    case "export_logs":
      return delay("C:\\Users\\kristian\\Documents\\ThomsenShield\\logs-export.txt" as T);

    case "clear_logs": {
      logs = [];
      return delay(undefined as T);
    }

    case "clear_block_history": {
      blocklist = blocklist.filter((b) => b.status === "active");
      return delay(undefined as T);
    }

    case "get_license_info":
      return delay(license as T);

    case "activate_license": {
      const key = String(args?.key ?? "");
      const valid = /^TSHIELD-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/.test(key);
      if (!valid) {
        license = { status: "invalid", key, activatedAt: null, provider: "dev" };
        throw { message: "That license key doesn't look right. Check it and try again." };
      }
      license = { status: "active", key, activatedAt: now(), provider: "dev" };
      return delay(license as T);
    }

    case "deactivate_license": {
      license = { status: "unlicensed", key: null, activatedAt: null, provider: "dev" };
      return delay(undefined as T);
    }

    case "get_settings":
      return delay(settings as T);

    case "save_settings": {
      settings = { ...settings, ...(args?.patch as Partial<Settings>) };
      return delay(settings as T);
    }

    case "complete_onboarding": {
      settings = { ...settings, ...(args?.patch as Partial<Settings>), onboardingCompleted: true };
      return delay(settings as T);
    }

    case "get_app_info":
      return delay({ version: "1.0.0", loadWarnings: [] as string[] } as T);

    default:
      return Promise.reject({ message: `Unhandled mock command: ${command}` });
  }
}
