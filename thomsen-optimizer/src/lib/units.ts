/** Number formatting helpers (binary units for memory/storage, decimal for rates). */

export function formatBytes1024(bytes: number | null | undefined, digits = 1): string {
  if (bytes == null || !Number.isFinite(bytes)) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = Math.max(0, bytes);
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${i === 0 ? v.toFixed(0) : v.toFixed(v >= 100 ? 0 : digits)} ${units[i]}`;
}

export function formatRate(bytesPerSec: number | null | undefined): string {
  if (bytesPerSec == null) return "-";
  return `${formatBytes1024(bytesPerSec)}/s`;
}

export function formatPercent(v: number | null | undefined, digits = 0): string {
  return v == null || !Number.isFinite(v) ? "-" : `${v.toFixed(digits)}%`;
}

export function formatTemp(v: number | null | undefined): string {
  return v == null ? "Unavailable" : `${Math.round(v)} °C`;
}

export function formatDuration(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${Math.floor(secs % 60)}s`;
  return `${Math.floor(secs)}s`;
}

export function formatMhz(mhz: number | null | undefined): string {
  if (mhz == null) return "-";
  return mhz >= 1000 ? `${(mhz / 1000).toFixed(2)} GHz` : `${mhz} MHz`;
}

/** Percent change with an explicit sign; `null` -> "n/a". */
export function formatChange(pct: number | null | undefined): string {
  if (pct == null || !Number.isFinite(pct)) return "n/a";
  return `${pct > 0 ? "+" : ""}${pct.toFixed(1)}%`;
}

export function fpsText(v: number | null | undefined): string {
  return v == null || !Number.isFinite(v) ? "Unavailable" : v.toFixed(1);
}
