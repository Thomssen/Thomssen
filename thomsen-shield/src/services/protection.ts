import { invoke } from "./ipc";
import type { ProtectionSettings } from "@/types";

export function getProtectionSettings(): Promise<ProtectionSettings> {
  return invoke<ProtectionSettings>("get_protection_settings");
}

export function saveProtectionSettings(patch: Partial<ProtectionSettings>): Promise<ProtectionSettings> {
  return invoke<ProtectionSettings>("save_protection_settings", { patch });
}

export function setLockdownMode(enabled: boolean): Promise<void> {
  return invoke<void>("set_lockdown_mode", { enabled });
}
