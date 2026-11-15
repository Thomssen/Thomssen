import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const rows = await sql()`
    select b.id, b.reason, b.created_at, b.expires_at, b.revoked_at, u.id as user_id, u.username, u.avatar_url, c.username as created_by,
           (b.revoked_at is null and (b.expires_at is null or b.expires_at > now())) as active
      from bans b join users u on u.id = b.user_id join users c on c.id = b.created_by
     order by b.created_at desc limit 200`;
  return json({ bans: rows });
});
