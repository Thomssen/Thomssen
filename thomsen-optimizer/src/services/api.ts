/** Typed wrappers over every backend command. One place, no stringly-typed calls in pages. */
import { invoke } from "./ipc";
import type {
  AnalysisReport, AppInfo, ApplyOutcome, BoosterEvent, BoosterStatus, ChangeRecord, CleanReport, ClosePlan, Comparison, Dashboard, FpsSession, FpsStatus, Game, GameSettings,
  LiveFps, OptimizationResult, OptimizeRequest, PowerPlan, ProcGroup, RestoreReport, Sample, ScanSummary, Settings, StartupEntry, SystemInfo, UpdateInfo,
} from "@/types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  setUiState: (visible: boolean, monitoring: boolean) => invoke<void>("set_ui_state", { visible, monitoring }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  autostartEnabled: () => invoke<boolean>("autostart_enabled"),

  systemInfo: () => invoke<SystemInfo | null>("system_info"),
  latestSample: () => invoke<Sample | null>("latest_sample"),
  dashboard: () => invoke<Dashboard>("dashboard"),
  powerPlans: () => invoke<PowerPlan[]>("power_plans"),
  setPowerPlan: (guid: string) => invoke<void>("set_power_plan", { guid }),

  analyze: () => invoke<AnalysisReport>("analyze"),
  optimizeNow: () => invoke<OptimizationResult>("optimize_now"),
  optimizeSelected: (request: OptimizeRequest) => invoke<OptimizationResult>("optimize_selected", { request }),
  applyRecommendation: (id: string) => invoke<OptimizationResult>("apply_recommendation", { id }),
  results: () => invoke<OptimizationResult[]>("optimization_results"),
  processes: () => invoke<ProcGroup[]>("list_processes"),
  closeProcesses: (plans: ClosePlan[]) => invoke<OptimizationResult>("close_processes", { plans }),

  games: (rescan = false) => invoke<Game[]>("list_games", { rescan }),
  addGame: (name: string, exePath: string) => invoke<void>("add_game", { name, exePath }),
  removeGame: (id: string) => invoke<void>("remove_game", { id }),
  updateGame: (id: string, settings: GameSettings) => invoke<GameSettings>("update_game", { id, settings }),
  setGpuPreference: (gameId: string, high: boolean) => invoke<ApplyOutcome[]>("set_gpu_preference", { gameId, high }),
  activateProfile: (profileId: string, closeApps: boolean) => invoke<OptimizationResult>("activate_profile", { profileId, closeApps }),
  deactivateProfile: () => invoke<BoosterEvent>("deactivate_profile"),
  boosterStatus: () => invoke<BoosterStatus>("booster_status"),

  cleanerScan: () => invoke<ScanSummary>("cleaner_scan"),
  cleanerClean: (scanId: string, categories: string[]) => invoke<CleanReport>("cleaner_clean", { scanId, categories }),
  startupList: () => invoke<StartupEntry[]>("startup_list"),
  startupSet: (id: string, enabled: boolean) => invoke<void>("startup_set", { id, enabled }),

  history: () => invoke<ChangeRecord[]>("history_list"),
  restoreChange: (id: string) => invoke<void>("restore_change", { id }),
  restoreAll: () => invoke<RestoreReport>("restore_all"),
  createRestorePoint: () => invoke<void>("create_restore_point"),

  fpsStatus: () => invoke<FpsStatus>("fps_status"),
  fpsStart: (gameId: string) => invoke<void>("fps_start", { gameId }),
  fpsLive: () => invoke<LiveFps>("fps_live"),
  fpsStop: (save: boolean, label: string) => invoke<FpsSession | null>("fps_stop", { save, label }),
  fpsSessions: () => invoke<FpsSession[]>("fps_sessions"),
  fpsCompare: (a: string, b: string) => invoke<Comparison>("fps_compare", { a, b }),
  fpsUpdateSession: (id: string, label: string, notes: string) => invoke<void>("fps_update_session", { id, label, notes }),
  fpsDeleteSession: (id: string) => invoke<void>("fps_delete_session", { id }),
  fpsClear: () => invoke<void>("fps_clear_sessions"),

  pickExecutable: (title: string) => invoke<string | null>("pick_executable", { title }),
  openExternal: (target: "presentmon" | "website" | "github" | "releases" | "gameMode" | "graphics" | "power" | "dataFolder") => invoke<void>("open_external", { target }),
  relaunchElevated: () => invoke<void>("relaunch_elevated"),

  updateCheck: () => invoke<UpdateInfo>("update_check"),
  updateInstall: () => invoke<void>("update_install"),
};
