import type { VercelRequest, VercelResponse } from "@vercel/node";
import { timingSafeEqual } from "node:crypto";
import { requireEnv } from "./env.js";

/**
 * A single shared secret (`ADMIN_API_TOKEN`), checked as a bearer token.
 * That's a deliberately simple - but not weak - choice for a solo-owner
 * internal tool: combined with HTTPS and a long random token (see
 * `scripts/generate-admin-token.ts`), it's a legitimate way to keep the
 * admin endpoints from being usable by anyone who doesn't have the token,
 * without standing up a full auth system for one user. Never logged,
 * never sent anywhere but this one header.
 */
export function isAuthorizedAdmin(req: VercelRequest): boolean {
  const header = req.headers.authorization;
  if (!header?.startsWith("Bearer ")) return false;
  const provided = Buffer.from(header.slice("Bearer ".length));
  const expected = Buffer.from(requireEnv("ADMIN_API_TOKEN"));
  if (provided.length !== expected.length) return false;
  return timingSafeEqual(provided, expected);
}

export function requireAdmin(req: VercelRequest, res: VercelResponse): boolean {
  if (!isAuthorizedAdmin(req)) {
    res.status(401).json({ error: "Missing or invalid admin token." });
    return false;
  }
  return true;
}
