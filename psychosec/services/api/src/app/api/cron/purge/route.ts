import { sql } from "@/lib/db";
import { env } from "@/lib/env";
import { safeEqual } from "@/lib/crypto";
import { err, json, safe } from "@/lib/http";

/** Retention job, called by Vercel Cron with `Authorization: Bearer $CRON_SECRET`. */
export const GET = safe(async (req) => {
  const secret = process.env.CRON_SECRET ?? "";
  const got = (req.headers.get("authorization") ?? "").replace(/^Bearer /i, "");
  if (!secret || secret.length < 16 || !safeEqual(got, secret)) return err(401, "unauthorized", "Unauthorized.");
  await sql()`select purge_old_records(${env.loginIpRetentionDays}, 30, 180)`;
  return json({ ok: true });
});
