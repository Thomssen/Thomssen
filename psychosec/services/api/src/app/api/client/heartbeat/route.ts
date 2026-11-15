import { z } from "zod";
import { authenticateToken, bearer, logActivity } from "@/lib/sessions";
import { sql } from "@/lib/db";
import { badRequest, clientIp, forbidden, json, rateLimit, readJson, safe, tooMany, unauthorized, deny } from "@/lib/http";

const body = z.object({ launch: z.boolean(), version: z.string().regex(/^[0-9]+\.[0-9]+\.[0-9]+$/) });

export const POST = safe(async (req) => {
  if (!(await rateLimit(`hb:${clientIp(req) ?? "unknown"}`, 60, 60))) return tooMany();
  const a = await authenticateToken(bearer(req), "app");
  if (!a.ok) return deny(a);
  const b = await readJson(req, body);
  if (!b) return badRequest();
  await sql()`update sessions set application_version = ${b.version}, last_seen_at = now() where id = ${a.session.id}`;
  await sql()`update users set last_activity_at = now() where id = ${a.user.id}`;
  if (b.launch) await logActivity(a.user.id, "app_launched", b.version);
  return json({ ok: true });
});
