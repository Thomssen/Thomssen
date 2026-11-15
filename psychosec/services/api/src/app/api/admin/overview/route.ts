import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const db = sql();
  const [stats] = await db`
    select
      (select count(*) from users)::int as total_users,
      (select count(*) from users where last_activity_at > now() - interval '7 days')::int as active_users_7d,
      (select count(*) from sessions where kind = 'app' and revoked_at is null and expires_at > now())::int as valid_sessions,
      (select count(*) from login_events where result = 'success' and client = 'cli' and created_at >= date_trunc('day', now()))::int as logins_today,
      (select count(*) from login_events where result <> 'success' and created_at >= date_trunc('day', now()))::int as failed_logins_today,
      (select count(*) from activity_events where type = 'app_launched' and created_at >= date_trunc('day', now()))::int as cli_launches_today,
      (select count(*) from bans where revoked_at is null and (expires_at is null or expires_at > now()))::int as banned_accounts`;
  const series = await db`
    select to_char(d, 'YYYY-MM-DD') as day,
           coalesce(sum((e.result = 'success')::int), 0)::int as success,
           coalesce(sum((e.result <> 'success')::int), 0)::int as failed
      from generate_series(date_trunc('day', now()) - interval '13 days', date_trunc('day', now()), interval '1 day') d
      left join login_events e on e.created_at >= d and e.created_at < d + interval '1 day'
     group by d order by d`;
  return json({ ...stats, series });
});
