import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const db = sql();
  const [failed, noisyIps, multiIp] = await Promise.all([
    db`select e.id, e.result, e.created_at, host(e.ip_address) as ip, e.ip_country, e.client, e.discord_id, u.username
         from login_events e left join users u on u.id = e.user_id where e.result <> 'success' order by e.created_at desc limit 100`,
    db`select host(ip_address) as ip, count(*)::int as failures, max(created_at) as last_seen
         from login_events where result <> 'success' and ip_address is not null and created_at > now() - interval '24 hours'
         group by ip_address having count(*) >= 3 order by failures desc limit 25`,
    db`select u.id as user_id, u.username, count(distinct e.ip_address)::int as ips
         from login_events e join users u on u.id = e.user_id where e.result = 'success' and e.created_at > now() - interval '7 days'
         group by u.id, u.username having count(distinct e.ip_address) >= 4 order by ips desc limit 25`,
  ]);
  return json({ failed_logins: failed, noisy_ips: noisyIps, many_ips_7d: multiIp });
});
