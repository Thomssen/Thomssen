import { invoke } from "@tauri-apps/api/core";

export type Style = "gaussian" | "pixelate" | "solid";
export type Method = "hybrid" | "uia_only" | "ocr_only";
export type Scope = "all_windows" | "foreground_only";
export type AppMode = "all" | "selected";
export type EntryKind = "email" | "name" | "address" | "phone" | "username" | "keyword" | "regex";

export interface Settings {
  filters: {
    emails: boolean; phones: boolean; names: boolean; addresses: boolean; ips: boolean;
    financial: boolean; usernames: boolean; dob: boolean; passwords: boolean; custom: boolean;
  };
  appearance: {
    style: Style; intensity: number; blur_radius: number; pixel_size: number;
    opacity: number; padding: number; safe_minimum: boolean;
  };
  apps: { mode: AppMode; selected: string[]; excluded: string[]; protect_browsers: boolean; protect_self: boolean };
  performance: { interval_ms: number; method: Method; scope: Scope; ocr_scale: number; hold_ms: number };
  general: { autostart: boolean; start_minimized: boolean; minimize_to_tray: boolean; protect_on_launch: boolean };
  shortcuts: { toggle: string; open_settings: string; panic: string };
  notifications: { on_toggle: boolean; on_panic: boolean; on_error: boolean };
  capture: { overlay_in_captures: boolean };
  monitors: { disabled: string[] };
}

export interface Entry { id: string; kind: EntryKind; value: string }

export interface Status {
  running: boolean;
  panic: boolean;
  engine: {
    active_regions: number;
    by_category: Record<string, number>;
    total_detected: number;
    apps_with_detections: { exe: string; count: number }[];
    visible_apps: string[];
    ocr: string; ocr_ok: boolean; uia_ok: boolean;
    last_cycle_ms: number; last_detect_ms: number;
    last_error: string | null;
  };
  monitors: { name: string; x: number; y: number; width: number; height: number; scale_percent: number; primary: boolean; enabled: boolean }[];
  capture_apps: string[];
  warning: string | null;
}

export interface CaptureResult {
  id: string; name: string; overlay_visible: boolean | null; verified: boolean; detail: string;
}

export const api = {
  status: () => invoke<Status>("get_status"),
  settings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<{ settings: Settings; warnings: string[] }>("save_settings", { settings }),
  entries: () => invoke<Entry[]>("get_entries"),
  addEntry: (kind: EntryKind, value: string) => invoke<Entry>("add_entry", { kind, value }),
  updateEntry: (id: string, kind: EntryKind, value: string) => invoke<Entry>("update_entry", { id, kind, value }),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  setProtection: (on: boolean) => invoke<void>("set_protection_cmd", { on }),
  setPanic: (on: boolean) => invoke<void>("set_panic_cmd", { on }),
  listApps: () => invoke<{ exe: string; running: boolean; browser: boolean }[]>("list_apps"),
  testText: (text: string) => invoke<{ start: number; end: number; category: string }[]>("test_text", { text }),
  previewStyle: async (appearance: Settings["appearance"]) => {
    const buf = await invoke<ArrayBuffer>("preview_style", { appearance });
    const dv = new DataView(buf);
    return { w: dv.getUint32(0, true), h: dv.getUint32(4, true), px: new Uint8ClampedArray(buf, 8) };
  },
  captureTest: (holdSecs: number) => invoke<CaptureResult[]>("run_capture_test", { holdSecs }),
  dataFolder: () => invoke<string>("open_log_folder"),
  minimize: () => invoke<void>("window_minimize"),
  toggleMaximize: () => invoke<void>("window_toggle_maximize"),
  close: () => invoke<void>("window_close"),
  quit: () => invoke<void>("quit"),
};

export const CATEGORY_LABEL: Record<string, string> = {
  email: "Email", phone: "Phone", name: "Name", address: "Address", ip: "IP address",
  credit_card: "Credit card", bank: "Bank / IBAN", username: "Username", dob: "Date of birth",
  secret: "Password / token", custom: "Custom",
};
