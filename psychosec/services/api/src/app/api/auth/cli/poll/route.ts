import { z } from "zod";
import { redeemCliLogin } from "@/lib/login";
import { badRequest, clientIp, json, rateLimit, readJson, safe, tooMany } from "@/lib/http";

const body = z.object({
  login_id: z.string().regex(/^[A-Za-z0-9_-]{16,64}$/),
  verifier: z.string().regex(/^[A-Za-z0-9_-]{43,128}$/),
  version: z.string().regex(/^[0-9]+\.[0-9]+\.[0-9]+$/).optional(),
});

export const POST = safe(async (req) => {
  const ip = clientIp(req);
  if (!(await rateLimit(`cli-poll:${ip ?? "unknown"}`, 60, 60))) return tooMany();
  const b = await readJson(req, body);
  if (!b) return badRequest();
  return json(await redeemCliLogin(b.login_id, b.verifier, ip, b.version ?? null));
});
