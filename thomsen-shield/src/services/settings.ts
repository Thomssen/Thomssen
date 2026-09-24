import { invoke } from "./ipc";
import type { Settings } from "@/types";

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function saveSettings(patch: Partial<Settings>): Promise<Settings> {
  return invoke<Settings>("save_settings", { patch });
}

export function completeOnboarding(patch: Partial<Settings>): Promise<Settings> {
  return invoke<Settings>("complete_onboarding", { patch });
}
