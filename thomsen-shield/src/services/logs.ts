import { invoke } from "./ipc";
import type { LogCategory, LogEntry, LogExportFormat } from "@/types";

/** The backend's `category` param is `Option<LogCategory>` - "all" is a
 * frontend-only filter value and must never be sent across the bridge. */
function toBackendCategory(category?: LogCategory | "all"): LogCategory | undefined {
  return category === "all" ? undefined : category;
}

export function listLogs(category?: LogCategory | "all"): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("list_logs", { category: toBackendCategory(category) });
}

/** Resolves to the path the export was written to. */
export function exportLogs(format: LogExportFormat, category?: LogCategory | "all"): Promise<string> {
  return invoke<string>("export_logs", { format, category: toBackendCategory(category) });
}

export function clearLogs(): Promise<void> {
  return invoke<void>("clear_logs");
}
