import { vi } from "vitest";
import { createHash, randomBytes } from "node:crypto";
import { sql } from "@/lib/db";

/** Fake Discord: code format  "<discordId>:<member|nomember|norole>" */
export function mockDiscord() {
  const real = globalThis.fetch;
  vi.stubGlobal("fetch", async (input: any, init?: any) => {
    const url = String(input);
    if (!url.startsWith("https://discord.com/api/v10")) return real(input, init);
    const J = (b: unknown, s = 200) => new Response(JSON.stringify(b), { status: s });
    if (url.endsWith("/oauth2/token")) {
      const p = new URLSearchParams(String(init.body));
      if (p.get("code") === "bad") return J({ error: "invalid_grant" }, 400);
      return J({ access_token: "at|" + p.get("code") });
    }
    const [id, mode] = String(init.headers.authorization).replace("Bearer at|", "").split(":");
    if (url.endsWith("/users/@me")) return J({ id, username: "user" + id.slice(-3), global_name: "User " + id.slice(-3), avatar: null });
    if (url.includes("/member")) {
      if (mode === "nomember") return J({ message: "Unknown Guild Member", code: 10007 }, 404);
      return J({ roles: mode === "norole" ? ["1"] : ["555"] });
    }
    return J({}, 404);
  });
}

export const b64u = (b: Buffer) => b.toString("base64url");
export function pkce() {
  const verifier = b64u(randomBytes(48));
  return { verifier, challenge: b64u(createHash("sha256").update(verifier).digest()), loginId: b64u(randomBytes(24)) };
}
export const req = (url: string, init: RequestInit & { ip?: string } = {}) => {
  const headers = new Headers(init.headers);
  headers.set("x-test-ip", init.ip ?? "203.0.113.7");
  return new Request("https://api.test" + url, { ...init, headers });
};
export const post = (url: string, body: unknown, headers: Record<string, string> = {}, ip?: string) =>
  req(url, { method: "POST", body: JSON.stringify(body), headers: { "content-type": "application/json", ...headers }, ip });

export async function resetDb() {
  await sql()`truncate rate_limits, login_requests, login_events, activity_events, bans, admin_users, sessions, users cascade`;
}
