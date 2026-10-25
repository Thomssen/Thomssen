import { invoke } from "./ipc";
import type { DashboardSummary } from "@/types";

export function getDashboardSummary(): Promise<DashboardSummary> {
  return invoke<DashboardSummary>("get_dashboard_summary");
}

export interface AppInfo {
  version: string;
  loadWarnings: string[];
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}
