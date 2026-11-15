import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const sp = new URL(req.url).searchParams;
  const q = (sp.get("q") ?? "").trim().slice(0, 64);
  const page = Math.max(0, Number.parseInt(sp.get("page") ?? "0", 10) || 0);
  const like = `%${q.replace(/[%_\\]/g, "\\$&")}%`;
  const rows = await sql()`
    select u.id, u.discord_id, u.username, u.display_name, u.avatar_url, u.access_status, u.created_at, u.last_login_at, u.last_activity_at,
      exists (select 1 from bans b where b.user_id = u.id and b.revoked_at is null and (b.expires_at is null or b.expires_at > now())) as banned,
      (select host(le.ip_address) from login_events le where le.user_id = u.id and le.result = 'success' order by le.created_at desc limit 1) as last_login_ip,
      (select count(*) from sessions s where s.user_id = u.id and s.kind = 'app' and s.revoked_at is null and s.expires_at > now())::int as active_sessions
    from users u
    where ${q === "" ? sql()`true` : sql()`(u.username ilike ${like} or u.display_name ilike ${like} or u.discord_id ilike ${like})`}
    order by u.last_activity_at desc nulls last, u.created_at desc
    limit 50 offset ${page * 50}`;
  return json({ users: rows, page });
});
