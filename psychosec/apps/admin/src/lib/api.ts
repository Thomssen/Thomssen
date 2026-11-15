"use client";
export const API = process.env.NEXT_PUBLIC_API_URL ?? "https://api.psychosec.org";

export class ApiError extends Error {
  constructor(public status: number, public code: string, message: string) { super(message); }
}
function csrf(): string {
  const m = document.cookie.split("; ").find((c) => c.startsWith("psy_csrf="));
  return m ? decodeURIComponent(m.slice(9)) : "";
}
export async function api<T>(path: string, opts: { method?: "GET" | "POST"; body?: unknown } = {}): Promise<T> {
  const method = opts.method ?? "GET";
  const r = await fetch(API + path, {
    method, credentials: "include",
    headers: { ...(opts.body ? { "content-type": "application/json" } : {}), ...(method !== "GET" ? { "x-csrf-token": csrf() } : {}) },
    body: opts.body ? JSON.stringify(opts.body) : undefined,
  });
  const j = await r.json().catch(() => ({}));
  if (!r.ok) throw new ApiError(r.status, j.error ?? "error", j.message ?? `Request failed (${r.status})`);
  return j as T;
}
export const loginUrl = `${API}/api/auth/discord/start?client=admin`;

export const fmt = (s: string | null | undefined) => (s ? new Date(s).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" }) : "—");
export function ago(s: string | null | undefined) {
  if (!s) return "never";
  const d = (Date.now() - new Date(s).getTime()) / 1000;
  if (d < 60) return "just now";
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  if (d < 86400) return `${Math.floor(d / 3600)}h ago`;
  return `${Math.floor(d / 86400)}d ago`;
}
