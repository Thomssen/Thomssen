import { z } from "zod";
import { sql } from "@/lib/db";
import { audit, isResponse, requireAdmin } from "@/lib/admin";
import { badRequest, err, forbidden, json, readJson, safe } from "@/lib/http";
import { logActivity, revokeAllSessions } from "@/lib/sessions";
import { ADMIN_ACTIONS } from "@psychosec/shared";

const uuid = z.string().uuid();

export const GET = safe(async (req, ctx) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const { id } = await ctx.params;
  if (!uuid.safeParse(id).success) return badRequest();
  const db = sql();
  const u = (await db`select id, discord_id, username, display_name, avatar_url, access_status, guild_member, role_verified, created_at, last_login_at, last_activity_at from users where id = ${id}`)[0];
  if (!u) return err(404, "not_found", "User not found.");
  const [sessions, logins, bans, audits, admin] = await Promise.all([
    db`select id, kind, created_at, last_seen_at, expires_at, revoked_at, revoked_reason, application_version, host(ip_address) as ip from sessions where user_id = ${id} order by created_at desc limit 50`,
    db`select id, host(ip_address) as ip, ip_country, result, created_at, session_id, application_version, client from login_events where user_id = ${id} order by created_at desc limit 200`,
    db`select b.id, b.reason, b.created_at, b.expires_at, b.revoked_at, c.username as created_by from bans b join users c on c.id = b.created_by where b.user_id = ${id} order by b.created_at desc`,
    db`select l.id, l.action, l.reason, l.created_at, a.username as admin from admin_audit_logs l join users a on a.id = l.admin_user_id where l.target_user_id = ${id} order by l.created_at desc limit 50`,
    db`select role from admin_users where user_id = ${id}`,
  ]);
  return json({
    user: u, is_admin: admin.length > 0, sessions, logins, bans, audit: audits,
    ip_country_note: "Country is approximate and derived from the edge network.",
  });
});

const body = z.object({
  action: z.enum(ADMIN_ACTIONS),
  reason: z.string().trim().min(3).max(500).optional(),
  expires_at: z.string().datetime().optional(),
  confirm: z.literal(true), // destructive actions need explicit confirmation from the UI
});

export const POST = safe(async (req, ctx) => {
  const a = await requireAdmin(req, { mutating: true });
  if (isResponse(a)) return a;
  const { id } = await ctx.params;
  if (!uuid.safeParse(id).success) return badRequest();
  const b = await readJson(req, body);
  if (!b) return badRequest("Invalid action. A confirmation flag is required.");
  if (id === a.user.id) return forbidden("Administrators cannot act on their own account.", "self_action");
  const target = (await sql()`select id from users where id = ${id}`)[0];
  if (!target) return err(404, "not_found", "User not found.");
  const targetAdmin = (await sql()`select role from admin_users where user_id = ${id}`)[0];
  if (targetAdmin && a.role !== "super_admin") return forbidden("Only a super administrator can act on another administrator.", "admin_target");
  if (["revoke", "ban"].includes(b.action) && !b.reason) return badRequest("A reason is required.");
  const db = sql();

  switch (b.action) {
    case "allow":
      await db`update users set access_status = 'authorized' where id = ${id}`;
      await logActivity(id, "access_allowed");
      break;
    case "revoke": {
      await db`update users set access_status = 'revoked' where id = ${id}`;
      const n = await revokeAllSessions(id, "access revoked");
      await logActivity(id, "access_revoked", null, `${n} sessions ended`);
      break;
    }
    case "ban": {
      await db`update bans set revoked_at = now(), revoked_by = ${a.user.id} where user_id = ${id} and revoked_at is null`;
      await db`insert into bans (user_id, reason, created_by, expires_at) values (${id}, ${b.reason!}, ${a.user.id}, ${b.expires_at ?? null})`;
      await revokeAllSessions(id, "banned");
      await logActivity(id, "user_banned");
      break;
    }
    case "unban":
      await db`update bans set revoked_at = now(), revoked_by = ${a.user.id} where user_id = ${id} and revoked_at is null`;
      await logActivity(id, "user_unbanned");
      break;
    case "revoke_sessions": {
      const n = await revokeAllSessions(id, "revoked by administrator");
      await logActivity(id, "session_revoked", null, `${n} sessions`);
      break;
    }
  }
  await audit(a, b.action, id, b.reason ?? null, b.expires_at ? { expires_at: b.expires_at } : undefined);
  return json({ ok: true });
});
