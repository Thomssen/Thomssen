import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const type = (new URL(req.url).searchParams.get("type") ?? "").slice(0, 40);
  const rows = await sql()`
    select e.id, e.type, e.created_at, e.application_version, e.detail, u.id as user_id, u.username, u.avatar_url
      from activity_events e left join users u on u.id = e.user_id
     where ${type ? sql()`e.type = ${type}` : sql()`true`}
     order by e.created_at desc limit 200`;
  return json({ events: rows });
});
