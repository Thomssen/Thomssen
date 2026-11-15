import { sql } from "./db";
import { env } from "./env";
import { newSessionToken, sha256hex } from "./crypto";

export type SessionKind = "app" | "admin" | "web";
export interface AuthedUser { id: string; discord_id: string; username: string; display_name: string | null; avatar_url: string | null; role_verified: boolean }
export interface AuthedSession { id: string; kind: SessionKind; expires_at: Date; absolute_expires_at: Date }
export type AuthResult =
  | { ok: true; user: AuthedUser; session: AuthedSession }
  | { ok: false; status: 401 | 403; code: string; message: string };

export async function createSession(opts: { userId: string; kind: SessionKind; ip: string | null; version?: string | null }) {
  const token = newSessionToken();
  const hours = opts.kind === "admin" ? env.adminSessionHours : env.sessionTtlHours;
  const absHours = opts.kind === "admin" ? env.adminSessionHours : env.sessionMaxDays * 24;
  const rows = await sql()`
    insert into sessions (user_id, kind, token_hash, expires_at, absolute_expires_at, application_version, ip_address)
    values (${opts.userId}, ${opts.kind}, ${sha256hex(token)}, now() + make_interval(hours => ${hours}),
            now() + make_interval(hours => ${absHours}), ${opts.version ?? null}, ${opts.ip}::inet)
    returning id, expires_at`;
  return { token, id: rows[0].id as string, expires_at: rows[0].expires_at as Date };
}

/** Is the user currently banned? (an unexpired, unrevoked ban exists) */
export async function activeBan(userId: string) {
  const r = await sql()`select id, reason, expires_at from bans where user_id = ${userId} and revoked_at is null and (expires_at is null or expires_at > now()) limit 1`;
  return r[0] as { id: string; reason: string; expires_at: Date | null } | undefined;
}

/**
 * Authoritative session check, executed on every request. Looks the token up by hash,
 * then re-evaluates expiry, revocation, access status and bans server-side.
 */
export async function authenticateToken(token: string | null | undefined, kind: SessionKind): Promise<AuthResult> {
  if (!token || !token.startsWith("psy_") || token.length > 100) return { ok: false, status: 401, code: "session_invalid", message: "Session is invalid or expired." };
  const rows = await sql()`
    select s.id, s.kind, s.expires_at, s.absolute_expires_at, s.revoked_at,
           u.id as user_id, u.discord_id, u.username, u.display_name, u.avatar_url, u.access_status, u.role_verified
      from sessions s join users u on u.id = s.user_id
     where s.token_hash = ${sha256hex(token)}`;
  const r = rows[0];
  if (!r || r.kind !== kind) return { ok: false, status: 401, code: "session_invalid", message: "Session is invalid or expired." };
  if (r.revoked_at) return { ok: false, status: 401, code: "session_revoked", message: "This session was ended. Sign in again." };
  if (new Date(r.expires_at) <= new Date()) {
    await sql()`update sessions set revoked_at = now(), revoked_reason = 'expired' where id = ${r.id} and revoked_at is null`;
    await sql()`insert into activity_events (user_id, type) values (${r.user_id}, 'session_expired')`;
    return { ok: false, status: 401, code: "session_expired", message: "Session expired. Sign in again." };
  }
  if (r.access_status !== "authorized") return { ok: false, status: 403, code: "access_revoked", message: "Your access to PSYCHOSEC has been revoked." };
  const ban = await activeBan(r.user_id);
  if (ban) return { ok: false, status: 403, code: "banned", message: "Your account is suspended." };
  await sql()`update sessions set last_seen_at = now() where id = ${r.id} and last_seen_at < now() - interval '30 seconds'`;
  return {
    ok: true,
    user: { id: r.user_id, discord_id: r.discord_id, username: r.username, display_name: r.display_name, avatar_url: r.avatar_url, role_verified: r.role_verified },
    session: { id: r.id, kind: r.kind, expires_at: r.expires_at, absolute_expires_at: r.absolute_expires_at },
  };
}

export const bearer = (req: Request) => {
  const h = req.headers.get("authorization") ?? "";
  return h.toLowerCase().startsWith("bearer ") ? h.slice(7).trim() : null;
};

export async function revokeAllSessions(userId: string, reason: string, kind?: SessionKind) {
  const r = await sql()`update sessions set revoked_at = now(), revoked_reason = ${reason}
    where user_id = ${userId} and revoked_at is null ${kind ? sql()`and kind = ${kind}` : sql()``} returning id`;
  return r.length;
}

export async function logActivity(userId: string | null, type: string, version?: string | null, detail?: string | null) {
  await sql()`insert into activity_events (user_id, type, application_version, detail) values (${userId}, ${type}, ${version ?? null}, ${detail ?? null})`;
}
