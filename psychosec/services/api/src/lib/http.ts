import { env } from "./env";
import { sql } from "./db";

export const SECURITY_HEADERS: Record<string, string> = {
  "cache-control": "no-store",
  "x-content-type-options": "nosniff",
  "x-frame-options": "DENY",
  "referrer-policy": "no-referrer",
  "strict-transport-security": "max-age=63072000; includeSubDomains; preload",
  "content-security-policy": "default-src 'none'; frame-ancestors 'none'; base-uri 'none'",
  "cross-origin-resource-policy": "same-site",
};

export function json(body: unknown, status = 200, extra: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", ...SECURITY_HEADERS, ...extra } });
}
export const err = (status: number, error: string, message: string, extra: Record<string, string> = {}) => json({ error, message }, status, extra);
export const unauthorized = (m = "Session is invalid or expired.") => err(401, "session_invalid", m);
export const deny = (a: { status: number; code: string; message: string }) => err(a.status, a.code, a.message);
export const forbidden = (m = "Access denied.", code = "forbidden") => err(403, code, m);

export function html(body: string, status = 200, extra: Record<string, string> = {}): Response {
  return new Response(body, {
    status,
    headers: { "content-type": "text/html; charset=utf-8", ...SECURITY_HEADERS, "content-security-policy": "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'", ...extra },
  });
}
export const esc = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));

/**
 * The client IP is taken ONLY from the trusted edge header. Client-supplied values
 * (x-forwarded-for on arbitrary hosts, JSON bodies) are never authoritative.
 */
export function clientIp(req: Request): string | null {
  const h = req.headers;
  const cand = env.trustedProxy === "vercel" ? h.get("x-vercel-forwarded-for") ?? h.get("x-real-ip") : env.trustedProxy === "test" ? h.get("x-test-ip") : null;
  if (!cand) return null;
  const ip = cand.split(",")[0].trim();
  return /^[0-9a-fA-F:.]{3,45}$/.test(ip) ? ip : null;
}
/** Approximate country from the edge network (not authoritative). */
export function clientCountry(req: Request): string | null {
  const c = req.headers.get("x-vercel-ip-country");
  return c && /^[A-Z]{2}$/.test(c) ? c : null;
}

export async function rateLimit(key: string, windowSec: number, max: number): Promise<boolean> {
  const r = await sql()`select rate_limit_hit(${key}, ${windowSec}, ${max}) as ok`;
  return r[0].ok as boolean;
}
export const tooMany = () => err(429, "rate_limited", "Too many requests. Try again shortly.", { "Retry-After": "60" });

export async function readJson<T>(req: Request, schema: { safeParse: (v: unknown) => { success: true; data: T } | { success: false } }): Promise<T | null> {
  const len = Number(req.headers.get("content-length") ?? 0);
  if (len > 16_384) return null;
  try {
    const p = schema.safeParse(await req.json());
    return p.success ? p.data : null;
  } catch { return null; }
}
export const badRequest = (m = "Invalid request.") => err(400, "invalid_request", m);

/** Wrap a handler so unexpected errors never leak internals. */
export function safe(handler: (req: Request, ctx: any) => Promise<Response>) {
  return async (req: Request, ctx?: any): Promise<Response> => {
    try { return await handler(req, ctx); }
    catch (e) {
      console.error("handler error", (e as Error).message);
      return err(500, "server_error", "Something went wrong.");
    }
  };
}

export function parseCookies(req: Request): Record<string, string> {
  const out: Record<string, string> = {};
  for (const part of (req.headers.get("cookie") ?? "").split(";")) {
    const i = part.indexOf("=");
    if (i > 0) out[part.slice(0, i).trim()] = decodeURIComponent(part.slice(i + 1).trim());
  }
  return out;
}
export function setCookie(name: string, value: string, opts: { maxAgeSec: number; httpOnly?: boolean }): string {
  const parts = [`${name}=${encodeURIComponent(value)}`, "Path=/", `Max-Age=${opts.maxAgeSec}`, "SameSite=Lax"];
  if (opts.httpOnly !== false) parts.push("HttpOnly");
  if (!env.insecureCookies) parts.push("Secure");
  if (env.cookieDomain) parts.push(`Domain=${env.cookieDomain}`);
  return parts.join("; ");
}
export const clearCookie = (name: string, httpOnly = true) => setCookie(name, "", { maxAgeSec: 0, httpOnly });
