import { sql } from "@/lib/db";
import { authenticateToken, bearer, logActivity } from "@/lib/sessions";
import { clientIp, json, rateLimit, safe, tooMany } from "@/lib/http";

/** Latest non-withdrawn release. Public (the website shows it); authenticated callers are logged. */
export const GET = safe(async (req) => {
  if (!(await rateLimit(`ver:${clientIp(req) ?? "unknown"}`, 60, 60))) return tooMany();
  const rows = await sql()`select version, download_url, checksum_sha256, signature, notes, published_at from application_releases
    where withdrawn_at is null order by string_to_array(version, '.')::int[] desc limit 1`;
  const tok = bearer(req);
  if (tok) {
    const a = await authenticateToken(tok, "app");
    if (a.ok) await logActivity(a.user.id, "update_checked", req.headers.get("x-psychosec-version"));
  }
  return json({ release: rows[0] ?? null }, 200);
});
