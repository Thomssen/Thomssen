/**
 * DESIGN-PREVIEW ONLY. Used when the UI runs in a plain browser (`npm run
 * dev`) with no Tauri backend, so screens can be styled and screenshotted.
 * The shipped app never uses this: `invoke()` only falls back here when
 * `__TAURI_INTERNALS__` is absent, and the UI shows a "Design preview"
 * banner whenever that is the case. Values are placeholders, not readings.
 */
import { DEFAULT_PROFILES } from "@/config/profiles";

let tick = 0;
const settings = {
  launchOnStartup: false, minimizeToTray: true, startMinimized: false, checkUpdatesAutomatically: true, autoDetectGames: true, autoBoost: true, defaultProfileId: "balanced",
  restoreWhenGameCloses: true, refreshMs: 1000, fpsMonitoring: true, historyRetentionDays: 90, temperatureMonitoring: true, confirmChanges: true, autoRestorePoint: false,
  cleanerMinAgeHours: 1, animations: true, uiScale: 1, accentColor: "#ffffff", presentmonPath: null as string | null, profiles: DEFAULT_PROFILES, onboardingCompleted: true, lastSeenVersion: "1.0.0",
};

function sample() {
  tick++;
  const w = (n: number, a: number, p: number) => Math.max(1, Math.min(99, n + a * Math.sin(tick / p)));
  return {
    timestampMs: Date.now(), cpuUsage: w(22, 12, 3), cpuTempC: null, memoryUsedBytes: 11.2e9, memoryTotalBytes: 32e9, memoryAvailableBytes: 20.8e9,
    diskReadBps: 4e6 + 3e6 * Math.sin(tick / 4), diskWriteBps: 1.2e6, netDownBps: 1.4e6 + 1e6 * Math.sin(tick / 5), netUpBps: 2e5, processCount: 214, uptimeSecs: 93_000 + tick,
    gpu: { name: "Preview GPU", usage: w(38, 20, 4), temperatureC: 58, vramUsedMb: 3100, vramTotalMb: 12288, source: "Design preview" }, selfCpu: 0.3, selfMemoryBytes: 61e6,
  };
}

const info = {
  cpu: { name: "Preview CPU @ 4.2 GHz", vendor: "Preview", physicalCores: 8, logicalProcessors: 16, baseMhz: 3600, currentMhz: 4200, architecture: "x86_64" },
  gpus: [{ name: "Preview GPU 12 GB", manufacturer: "Preview", vramBytes: 12 * 1024 ** 3, driverVersion: "1.0.0.0" }],
  memory: { totalBytes: 32 * 1024 ** 3, speedMhz: 3600, modules: 2 },
  drives: [{ name: "System", mount: "C:\\", fileSystem: "NTFS", totalBytes: 1e12, freeBytes: 4.1e11, kind: "SSD", removable: false }],
  physicalDisks: [{ name: "Preview NVMe 1TB", mediaType: "SSD", busType: "NVMe", health: "Healthy", sizeBytes: 1e12 }],
  os: { name: "Windows 11 Pro", version: "23H2", build: "22631.3007", architecture: "x86_64", uptimeSecs: 93000 },
};

const rec = (id: string, severity: string, title: string, evidence: string, kind = "changes", safeAuto = true) => ({
  id, category: "windows", severity, title, evidence, changes: "Preview.", why: "Preview.", downsides: "Preview.", restartRequired: false, howToReverse: "Restore from History.",
  action: kind === "changes" ? { type: "changes", changes: [] } : { type: "navigate", route: "cleaner" }, safeAuto,
});

const scan = {
  scanId: "preview", totalBytes: 2.6e9,
  categories: [
    { id: "user-temp", name: "User temporary files", description: "Temporary files left behind by apps and installers.", risk: "low", defaultSelected: true, warning: null, bytes: 1.4e9, files: 3120, largest: [{ path: "C:\\Users\\you\\AppData\\Local\\Temp\\setup.tmp", size: 4.2e8 }], locations: ["C:\\Users\\you\\AppData\\Local\\Temp"], truncated: false },
    { id: "web-cache", name: "Browser and Windows web cache", description: "Cached web content.", risk: "low", defaultSelected: false, warning: "Pages may load slower once afterwards.", bytes: 9.1e8, files: 1480, largest: [], locations: ["C:\\Users\\you\\AppData\\Local\\Microsoft\\Windows\\INetCache"], truncated: false },
    { id: "shader-cache", name: "Shader caches (optional)", description: "Rebuilt automatically as you play.", risk: "medium", defaultSelected: false, warning: "Not recommended: games must recompile shaders and may stutter.", bytes: 3.2e8, files: 260, largest: [], locations: ["C:\\Users\\you\\AppData\\Local\\D3DSCache"], truncated: false },
  ],
};

const games = [
  { id: "steam-1", name: "Counter-Strike 2", launcher: "steam", exePath: "D:\\Steam\\steamapps\\common\\Counter-Strike Global Offensive\\game\\bin\\win64\\cs2.exe", installDir: null, lastPlayed: new Date(Date.now() - 3600e3).toISOString(), settings: { profileId: "competitive", autoBoost: true, backgroundManagement: true, memoryMonitoring: false, hidden: false }, running: false, runningPids: [], memoryBytes: 0 },
  { id: "epic-1", name: "Fortnite", launcher: "epic", exePath: "E:\\Epic\\Fortnite\\FortniteGame\\Binaries\\Win64\\FortniteClient-Win64-Shipping.exe", installDir: null, lastPlayed: null, settings: { profileId: null, autoBoost: false, backgroundManagement: false, memoryMonitoring: false, hidden: false }, running: false, runningPids: [], memoryBytes: 0 },
  { id: "custom-1", name: "Minecraft", launcher: "custom", exePath: "C:\\Games\\Minecraft\\Minecraft.exe", installDir: null, lastPlayed: null, settings: { profileId: "balanced", autoBoost: true, backgroundManagement: false, memoryMonitoring: true, hidden: false }, running: true, runningPids: [1], memoryBytes: 3.1e9 },
];

const booster = { active: false, profileId: null, profileName: null, startedAt: null, games: [], changeCount: 0, manual: false };

const frameStats = { frames: 18000, durationSecs: 150, avgFps: 120, onePercentLowFps: 88, avgFrametimeMs: 8.3, p99FrametimeMs: 11.4, maxFrametimeMs: 24, stddevMs: 1.4, stabilityPct: 83 };
const session = (id: string, label: string, mul: number) => ({ id, gameName: "Counter-Strike 2", processName: "cs2.exe", startedAt: new Date().toISOString(), stats: { ...frameStats, avgFps: 120 * mul }, frameTimes: Array.from({ length: 200 }, (_, i) => 8 / mul + Math.sin(i / 7)), label, profileId: null, notes: "" });

export async function mockBackend<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 40));
  const out = ((): unknown => {
    switch (command) {
      case "app_info": return { name: "Thomsen Optimizer", version: "1.0.0", elevated: false, dataDir: "(preview)", selfCpu: 0.3, selfMemoryBytes: 6.1e7 };
      case "get_settings": return settings;
      case "save_settings": Object.assign(settings, (args as { settings: object }).settings); return settings;
      case "system_info": return info;
      case "latest_sample": return sample();
      case "dashboard": return { powerPlan: { guid: "x", name: "Balanced", active: true, kind: "balanced" }, booster, runningGames: ["Minecraft"], restorableChanges: 0, fps: null, elevated: false };
      case "power_plans": return [{ guid: "a", name: "Balanced", active: true, kind: "balanced" }, { guid: "b", name: "High performance", active: false, kind: "highPerformance" }, { guid: "c", name: "Power saver", active: false, kind: "powerSaver" }];
      case "analyze": return { generatedAt: new Date().toISOString(), cpuAvg: 21, recommendations: [rec("game-mode", "suggestion", "Turn on Windows Game Mode", "Game Mode is switched off for your account."), rec("temp-files", "suggestion", "Remove temporary files", "About 1.4 GB of old temporary files found.", "navigate", false)] };
      case "optimize_now": case "optimize_selected": case "apply_recommendation": case "close_processes": case "activate_profile":
        return { id: "r", timestamp: new Date().toISOString(), title: "Optimization completed", applied: 1, alreadySet: 0, failed: 0, bytesFreed: 0, items: [{ kind: "setting", label: "Windows Game Mode: on", ok: true, detail: null }] };
      case "optimization_results": return [];
      case "list_processes": return [{ name: "chrome.exe", exe: null, count: 14, cpu: 2.1, memoryBytes: 2.4e9, hasWindow: true, protected: false, protectedReason: null, members: [] }, { name: "svchost.exe", exe: null, count: 60, cpu: 0.4, memoryBytes: 8e8, hasWindow: false, protected: true, protectedReason: "Windows or driver component", members: [] }];
      case "list_games": return games;
      case "booster_status": return booster;
      case "cleaner_scan": return scan;
      case "cleaner_clean": return { categories: [{ id: "user-temp", name: "User temporary files", deletedFiles: 3000, bytesFreed: 1.3e9, skippedFiles: 120, error: null }], totalDeleted: 3000, totalFreed: 1.3e9, totalSkipped: 120 };
      case "startup_list": return [{ id: "a", name: "Discord", command: "\"C:\\Users\\you\\AppData\\Local\\Discord\\Update.exe\" --processStart Discord.exe", exePath: null, publisher: "Discord Inc.", location: "Registry - Current user (Run)", source: "hkcuRun", enabled: true, protected: false, protectedReason: null, impact: null }, { id: "b", name: "SecurityHealth", command: "%windir%\\system32\\SecurityHealthSystray.exe", exePath: null, publisher: "Microsoft Corporation", location: "Registry - All users (Run)", source: "hklmRun", enabled: true, protected: true, protectedReason: "Windows security component", impact: null }];
      case "history_list": return [];
      case "fps_status": return { presentmonPath: null, presentmonOk: false, capturing: false, elevated: false };
      case "fps_live": return { capturing: false, gameName: null, frames: 0, elapsedSecs: 0, recent: [], currentFps: null, waitingForFrames: false, error: null };
      case "fps_sessions": return [session("s1", "Before optimization", 1), session("s2", "After optimization", 1.06)];
      case "fps_compare": return { a: session("s1", "Before optimization", 1), b: session("s2", "After optimization", 1.06), avgFpsChangePct: 6, lowFpsChangePct: 4.1, stabilityChangePts: 1.2, sameGame: true, disclaimer: "Preview data." };
      case "update_check": return { configured: false, available: false, currentVersion: "1.0.0", version: null, notes: null, date: null };
      case "restore_all": return { restored: 0, failed: [], nothingToRestore: true };
      case "deactivate_profile": return { type: "restored", restored: 0, failed: [] };
      case "pick_executable": return null;
      default: return null;
    }
  })();
  return out as T;
}
