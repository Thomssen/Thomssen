import { z } from "zod";
import { sql } from "@/lib/db";
import { audit, isResponse, requireAdmin } from "@/lib/admin";
import { badRequest, err, json, readJson, safe } from "@/lib/http";
import { logActivity } from "@/lib/sessions";

/** Never returns token material (only the opaque session id). */
export const GET = safe(async (req) => {
  const a = await requireAdmin(req);
  if (isResponse(a)) return a;
  const state = new URL(req.url).searchParams.get("state") === "all" ? "all" : "active";
  const rows = await sql()`
    select s.id, s.kind, s.created_at, s.last_seen_at, s.expires_at, s.revoked_at, s.revoked_reason, s.application_version,
           u.id as user_id, u.username, u.display_name, u.avatar_url,
           case when s.revoked_at is not null then 'revoked' when s.expires_at <= now() then 'expired' else 'active' end as status
      from sessions s join users u on u.id = s.user_id
     where ${state === "all" ? sql()`true` : sql()`s.revoked_at is null and s.expires_at > now()`}
     order by s.last_seen_at desc limit 200`;
  return json({ sessions: rows });
});

const body = z.object({ id: z.string().uuid(), confirm: z.literal(true) });
export const POST = safe(async (req) => {
  const a = await requireAdmin(req, { mutating: true });
  if (isResponse(a)) return a;
  const b = await readJson(req, body);
  if (!b) return badRequest();
  if (b.id === a.sessionId) return err(403, "self_session", "Use sign out to end your own session.");
  const r = await sql()`update sessions set revoked_at = now(), revoked_reason = 'terminated by administrator' where id = ${b.id} and revoked_at is null returning user_id`;
  if (!r.length) return err(404, "not_found", "Active session not found.");
  await logActivity(r[0].user_id, "session_revoked");
  await audit(a, "terminate_session", r[0].user_id, null, { session: b.id });
  return json({ ok: true });
});
