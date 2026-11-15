import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";
import { env } from "@/lib/env";

/** Read-only view of non-secret configuration plus the administrator list. */
export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const admins = await sql()`select u.id, u.username, u.discord_id, u.avatar_url, a.role, a.created_at from admin_users a join users u on u.id = a.user_id order by a.created_at`;
  const releases = await sql()`select version, published_at, withdrawn_at from application_releases order by published_at desc limit 10`;
  return json({
    config: {
      guild_id: env.discordGuildId,
      required_role_configured: Boolean(env.discordRequiredRoleId),
      session_ttl_hours: env.sessionTtlHours,
      session_max_days: env.sessionMaxDays,
      admin_session_hours: env.adminSessionHours,
      login_ip_retention_days: env.loginIpRetentionDays,
      admin_allowlist_size: env.adminDiscordIds.length,
    },
    admins, releases, you: { role: a.role },
  });
});
