import { sql } from "@/lib/db";
import { isResponse, requireAdmin } from "@/lib/admin";
import { json, safe } from "@/lib/http";

export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const rows = await sql()`
    select l.id, l.action, l.reason, l.created_at, adm.username as admin, tgt.username as target, tgt.id as target_id
      from admin_audit_logs l join users adm on adm.id = l.admin_user_id left join users tgt on tgt.id = l.target_user_id
     order by l.created_at desc limit 300`;
  return json({ logs: rows });
});
