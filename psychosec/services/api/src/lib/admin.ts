import { sql } from "./db";
import { env } from "./env";
import { authenticateToken, bearer, type AuthedUser } from "./sessions";
import { forbidden, parseCookies, unauthorized, clientIp, err, deny } from "./http";
import { safeEqual, randomToken } from "./crypto";

export const ADMIN_COOKIE = "psy_admin";
export const WEB_COOKIE = "psy_web";
export const CSRF_COOKIE = "psy_csrf";

export interface AdminCtx { user: AuthedUser; sessionId: string; role: "admin" | "super_admin"; ip: string | null }

/**
 * Admin guard (deny by default): cookie session of kind 'admin' AND a row in admin_users,
 * both verified server-side on every call. Mutations additionally need CSRF + Origin checks.
 */
export async function requireAdmin(req: Request, opts: { mutating?: boolean } = {}): Promise<AdminCtx | Response> {
  if (bearer(req)) return forbidden("Admin API does not accept bearer tokens.");
  const token = parseCookies(req)[ADMIN_COOKIE];
  if (!token) return unauthorized("Admin sign-in required.");
  const a = await authenticateToken(token, "admin");
  if (!a.ok) return deny(a);
  const row = (await sql()`select role from admin_users where user_id = ${a.user.id}`)[0];
  if (!row) return forbidden("Not an administrator.");
  if (opts.mutating) {
    const origin = req.headers.get("origin");
    if (!origin || !env.allowedOrigins.includes(origin)) return err(403, "bad_origin", "Cross-site request blocked.");
    const header = req.headers.get("x-csrf-token") ?? "";
    const cookie = parseCookies(req)[CSRF_COOKIE] ?? "";
    if (!header || !safeEqual(header, cookie) || !safeEqual(header, csrfFor(token))) return err(403, "bad_csrf", "CSRF check failed.");
  }
  return { user: a.user, sessionId: a.session.id, role: row.role, ip: clientIp(req) };
}
export const isResponse = (x: unknown): x is Response => x instanceof Response;

import { createHmac } from "node:crypto";
/** CSRF token bound to the admin session token (HMAC keyed by the client secret). */
export function csrfFor(sessionToken: string): string {
  return createHmac("sha256", env.discordClientSecret).update("csrf:" + sessionToken).digest("base64url");
}
export const newNonce = () => randomToken(8);

export async function audit(ctx: AdminCtx, action: string, targetUserId: string | null, reason?: string | null, detail?: unknown) {
  await sql()`insert into admin_audit_logs (admin_user_id, action, target_user_id, reason, detail, ip_address)
    values (${ctx.user.id}, ${action}, ${targetUserId}, ${reason ?? null}, ${detail ? JSON.stringify(detail) : null}::jsonb, ${ctx.ip}::inet)`;
}
