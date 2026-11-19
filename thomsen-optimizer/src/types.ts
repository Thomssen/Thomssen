/** Mirrors the serde structs in `core/src` (camelCase). Keep in step. */

export interface AppErrorShape {
  code?: string;
  message: string;
}

export type PlanKind = "powerSaver" | "balanced" | "highPerformance" | "ultimate" | "custom";
export interface PowerPlan {
  guid: string;
  name: string;
  active: boolean;
  kind: PlanKind;
}

// -- settings / profiles ---------------------------------------------------------
export type PowerChoice = "keep" | "balanced" | "highPerformance" | "ultimate";
export interface ProfileConfig {
  id: string;
  name: string;
  description: string;
  builtin: boolean;
  power: PowerChoice;
  gameMode: boolean;
  disableGameDvr: boolean;
  closeApps: string[];
  tradeoffs: string;
}

export interface Settings {
  launchOnStartup: boolean;
  minimizeToTray: boolean;
  startMinimized: boolean;
  checkUpdatesAutomatically: boolean;
  autoDetectGames: boolean;
  autoBoost: boolean;
  defaultProfileId: string;
  restoreWhenGameCloses: boolean;
  refreshMs: number;
  fpsMonitoring: boolean;
  historyRetentionDays: number;
  temperatureMonitoring: boolean;
  confirmChanges: boolean;
  autoRestorePoint: boolean;
  cleanerMinAgeHours: number;
  animations: boolean;
  uiScale: number;
  accentColor: string;
  presentmonPath: string | null;
  profiles: ProfileConfig[];
  onboardingCompleted: boolean;
  lastSeenVersion: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  elevated: boolean;
  dataDir: string;
  selfCpu: number;
  selfMemoryBytes: number;
}

// -- monitoring ----------------------------------------------------------------------
export interface GpuLive {
  name: string | null;
  usage: number | null;
  temperatureC: number | null;
  vramUsedMb: number | null;
  vramTotalMb: number | null;
  source: string | null;
}

export interface Sample {
  timestampMs: number;
  cpuUsage: number;
  cpuTempC: number | null;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  memoryAvailableBytes: number;
  diskReadBps: number;
  diskWriteBps: number;
  netDownBps: number;
  netUpBps: number;
  processCount: number;
  uptimeSecs: number;
  gpu: GpuLive | null;
  selfCpu: number;
  selfMemoryBytes: number;
}

// -- system info --------------------------------------------------------------------------
export interface CpuInfo {
  name: string;
  vendor: string;
  physicalCores: number | null;
  logicalProcessors: number;
  baseMhz: number | null;
  currentMhz: number | null;
  architecture: string;
}
export interface GpuInfo {
  name: string;
  manufacturer: string | null;
  vramBytes: number | null;
  driverVersion: string | null;
}
export interface DriveInfo {
  name: string;
  mount: string;
  fileSystem: string;
  totalBytes: number;
  freeBytes: number;
  kind: string;
  removable: boolean;
}
export interface PhysicalDisk {
  name: string;
  mediaType: string | null;
  busType: string | null;
  health: string | null;
  sizeBytes: number | null;
}
export interface SystemInfo {
  cpu: CpuInfo;
  gpus: GpuInfo[];
  memory: { totalBytes: number; speedMhz: number | null; modules: number | null };
  drives: DriveInfo[];
  physicalDisks: PhysicalDisk[];
  os: { name: string; version: string; build: string; architecture: string; uptimeSecs: number };
}

// -- booster / games -----------------------------------------------------------------------
export interface BoosterStatus {
  active: boolean;
  profileId: string | null;
  profileName: string | null;
  startedAt: string | null;
  games: string[];
  changeCount: number;
  manual: boolean;
}

export interface ApplyOutcome {
  label: string;
  outcome: "applied" | "alreadySet" | "failed";
  message: string | null;
  recordId: string | null;
}

export type BoosterEvent =
  | { type: "activated"; game: string; profileName: string; outcomes: ApplyOutcome[] }
  | { type: "failed"; game: string; message: string }
  | { type: "restored"; restored: number; failed: string[] };

export type EngineEvent =
  | { type: "sample"; data: Sample }
  | { type: "booster"; data: BoosterEvent }
  | { type: "boosterStatus"; data: BoosterStatus };

export type Launcher = "steam" | "epic" | "xbox" | "battleNet" | "custom";
export interface GameSettings {
  profileId: string | null;
  autoBoost: boolean;
  backgroundManagement: boolean;
  memoryMonitoring: boolean;
  hidden: boolean;
}
export interface Game {
  id: string;
  name: string;
  launcher: Launcher;
  exePath: string | null;
  installDir: string | null;
  lastPlayed: string | null;
  settings: GameSettings;
  running: boolean;
  runningPids: number[];
  memoryBytes: number;
}

// -- optimization ----------------------------------------------------------------------------
export type Category = "cpu" | "memory" | "windows" | "storage" | "background" | "gpu";
export type Severity = "attention" | "suggestion" | "info";
export interface Recommendation {
  id: string;
  category: Category;
  severity: Severity;
  title: string;
  evidence: string;
  changes: string;
  why: string;
  downsides: string;
  restartRequired: boolean;
  howToReverse: string;
  action: { type: "changes"; changes: unknown[] } | { type: "navigate"; route: string } | { type: "guidance" };
  safeAuto: boolean;
}
export interface AnalysisReport {
  recommendations: Recommendation[];
  generatedAt: string;
  cpuAvg: number;
}

export interface ClosePlan {
  pid: number;
  name: string;
  startTime: number;
}
export interface ProcGroup {
  name: string;
  exe: string | null;
  count: number;
  cpu: number;
  memoryBytes: number;
  hasWindow: boolean;
  protected: boolean;
  protectedReason: string | null;
  members: ClosePlan[];
}

export interface ResultItem {
  kind: "setting" | "profile" | "process" | "cleanup" | "info";
  label: string;
  ok: boolean;
  detail: string | null;
}
export interface OptimizationResult {
  id: string;
  timestamp: string;
  title: string;
  items: ResultItem[];
  applied: number;
  alreadySet: number;
  failed: number;
  bytesFreed: number;
}
export interface OptimizeRequest {
  recommendationIds: string[];
  close: ClosePlan[];
  cleanScanId: string | null;
  cleanCategories: string[];
}

// -- cleaner / startup -------------------------------------------------------------------------
export interface CategoryScan {
  id: string;
  name: string;
  description: string;
  risk: "low" | "medium";
  defaultSelected: boolean;
  warning: string | null;
  bytes: number;
  files: number;
  largest: { path: string; size: number }[];
  locations: string[];
  truncated: boolean;
}
export interface ScanSummary {
  scanId: string;
  categories: CategoryScan[];
  totalBytes: number;
}
export interface CleanReport {
  categories: { id: string; name: string; deletedFiles: number; bytesFreed: number; skippedFiles: number; error: string | null }[];
  totalDeleted: number;
  totalFreed: number;
  totalSkipped: number;
}
export interface StartupEntry {
  id: string;
  name: string;
  command: string;
  exePath: string | null;
  publisher: string | null;
  location: string;
  source: string;
  enabled: boolean;
  protected: boolean;
  protectedReason: string | null;
  impact: string | null;
}

// -- restore ---------------------------------------------------------------------------------------
export interface ChangeRecord {
  id: string;
  createdAt: string;
  label: string;
  detail: string;
  scope: string;
  kind: { type: "registry" | "powerPlan" | "irreversible" };
  status: "pending" | "applied" | "restored" | "restoreFailed" | "irreversible";
  restoredAt: string | null;
  error: string | null;
}
export interface RestoreReport {
  restored: number;
  failed: string[];
  nothingToRestore: boolean;
}

// -- dashboard / FPS ------------------------------------------------------------------------------------
export interface Dashboard {
  powerPlan: PowerPlan | null;
  booster: BoosterStatus;
  runningGames: string[];
  restorableChanges: number;
  fps: number | null;
  elevated: boolean;
}
export interface FpsStatus {
  presentmonPath: string | null;
  presentmonOk: boolean;
  capturing: boolean;
  elevated: boolean;
}
export interface LiveFps {
  capturing: boolean;
  gameName: string | null;
  frames: number;
  elapsedSecs: number;
  recent: number[];
  currentFps: number | null;
  waitingForFrames: boolean;
  error: string | null;
}
export interface FrameStats {
  frames: number;
  durationSecs: number;
  avgFps: number;
  onePercentLowFps: number | null;
  avgFrametimeMs: number;
  p99FrametimeMs: number;
  maxFrametimeMs: number;
  stddevMs: number;
  stabilityPct: number;
}
export interface FpsSession {
  id: string;
  gameName: string;
  processName: string;
  startedAt: string;
  stats: FrameStats;
  frameTimes: number[];
  label: string;
  profileId: string | null;
  notes: string;
}
export interface Comparison {
  a: FpsSession;
  b: FpsSession;
  avgFpsChangePct: number | null;
  lowFpsChangePct: number | null;
  stabilityChangePts: number;
  sameGame: boolean;
  disclaimer: string;
}

export interface UpdateInfo {
  configured: boolean;
  available: boolean;
  currentVersion: string;
  version: string | null;
  notes: string | null;
  date: string | null;
}
