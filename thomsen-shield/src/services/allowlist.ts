import { invoke } from "./ipc";
import type { AllowlistEntry } from "@/types";

export function listAllowlist(): Promise<AllowlistEntry[]> {
  return invoke<AllowlistEntry[]>("list_allowlist");
}

export function addAllowlistEntry(value: string, label?: string): Promise<AllowlistEntry> {
  return invoke<AllowlistEntry>("add_allowlist_entry", { value, label });
}

export function removeAllowlistEntry(id: string): Promise<void> {
  return invoke<void>("remove_allowlist_entry", { id });
}
