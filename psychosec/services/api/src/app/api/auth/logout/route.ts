import { authenticateToken, bearer, logActivity } from "@/lib/sessions";
import { ADMIN_COOKIE, CSRF_COOKIE, WEB_COOKIE } from "@/lib/admin";
import { sql } from "@/lib/db";
import { clearCookie, json, parseCookies, safe } from "@/lib/http";
import { env } from "@/lib/env";

export const POST = safe(async (req) => {
  const ck = parseCookies(req);
  const candidates: [string | null | undefined, "app" | "admin" | "web"][] = [[bearer(req), "app"], [ck[ADMIN_COOKIE], "admin"], [ck[WEB_COOKIE], "web"]];
  // Cookie-authenticated logout must come from an allowed origin (CSRF).
  const origin = req.headers.get("origin");
  for (const [tok, kind] of candidates) {
    if (!tok) continue;
    if (kind !== "app" && (!origin || !env.allowedOrigins.includes(origin))) continue;
    const a = await authenticateToken(tok, kind);
    if (a.ok) {
      await sql()`update sessions set revoked_at = now(), revoked_reason = 'logout' where id = ${a.session.id}`;
      await logActivity(a.user.id, "logout");
    }
  }
  const h = new Headers();
  h.append("set-cookie", clearCookie(ADMIN_COOKIE));
  h.append("set-cookie", clearCookie(WEB_COOKIE));
  h.append("set-cookie", clearCookie(CSRF_COOKIE, false));
  const res = json({ ok: true });
  for (const c of h.getSetCookie()) res.headers.append("set-cookie", c);
  return res;
});
