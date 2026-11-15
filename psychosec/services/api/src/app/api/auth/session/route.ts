import { authenticateToken, bearer } from "@/lib/sessions";
import { sql } from "@/lib/db";
import { env } from "@/lib/env";
import { clientIp, err, forbidden, json, rateLimit, safe, tooMany, unauthorized, deny } from "@/lib/http";

async function auth(req: Request) {
  if (!(await rateLimit(`session:${clientIp(req) ?? "unknown"}`, 60, 120))) return { res: tooMany() };
  const a = await authenticateToken(bearer(req), "app");
  if (!a.ok) return { res: deny(a) };
  return { a };
}
const view = (a: any, expires: Date) => ({
  user: { discord_id: a.user.discord_id, username: a.user.username, display_name: a.user.display_name, avatar_url: a.user.avatar_url },
  expires_at: expires.toISOString(), role_verified: a.user.role_verified,
});

/** Validate the current session. */
export const GET = safe(async (req) => {
  const { a, res } = await auth(req);
  if (!a) return res!;
  return json(view(a, a.session.expires_at));
});

/** Refresh: slide the expiry, never beyond the absolute maximum. */
export const POST = safe(async (req) => {
  const { a, res } = await auth(req);
  if (!a) return res!;
  const r = await sql()`update sessions set expires_at = least(now() + make_interval(hours => ${env.sessionTtlHours}), absolute_expires_at)
    where id = ${a.session.id} returning expires_at, absolute_expires_at`;
  if (new Date(r[0].absolute_expires_at) <= new Date()) return err(401, "session_expired", "Session reached its maximum age. Sign in again.");
  return json(view(a, r[0].expires_at));
});
