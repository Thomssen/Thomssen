import { z } from "zod";
import { beginLogin } from "@/lib/login";
import { badRequest, clientCountry, clientIp, rateLimit, safe, tooMany } from "@/lib/http";
import { randomToken } from "@/lib/crypto";

const q = z.object({
  client: z.enum(["cli", "admin", "web"]).default("web"),
  login_id: z.string().regex(/^[A-Za-z0-9_-]{16,64}$/).optional(),
  challenge: z.string().regex(/^[A-Za-z0-9_-]{43}$/).optional(),
  v: z.string().regex(/^[0-9]+\.[0-9]+\.[0-9]+$/).optional(),
});

export const GET = safe(async (req) => {
  const ip = clientIp(req);
  if (!(await rateLimit(`login-start:${ip ?? "unknown"}`, 60, 20))) return tooMany();
  const p = q.safeParse(Object.fromEntries(new URL(req.url).searchParams));
  if (!p.success) return badRequest("Invalid sign-in link.");
  const { client, login_id, challenge, v } = p.data;
  if (client === "cli" && (!login_id || !challenge)) return badRequest("Invalid sign-in link.");
  const url = await beginLogin({
    client, loginId: login_id ?? randomToken(18), challenge: challenge ?? null, ip, country: clientCountry(req), version: v ?? null,
  });
  return new Response(null, { status: 302, headers: { location: url, "cache-control": "no-store", "referrer-policy": "no-referrer" } });
});
