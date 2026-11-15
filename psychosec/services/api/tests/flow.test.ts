import { beforeAll, beforeEach, afterAll, describe, expect, it } from "vitest";
import { sql, closeDb } from "@/lib/db";
import { GET as start } from "@/app/api/auth/discord/start/route";
import { GET as callback } from "@/app/api/auth/discord/callback/route";
import { POST as poll } from "@/app/api/auth/cli/poll/route";
import { GET as sessionGet, POST as sessionRefresh } from "@/app/api/auth/session/route";
import { POST as logout } from "@/app/api/auth/logout/route";
import { POST as heartbeat } from "@/app/api/client/heartbeat/route";
import { GET as versionGet } from "@/app/api/client/version/route";
import { mockDiscord, pkce, post, req, resetDb } from "./helpers";

beforeAll(mockDiscord);
beforeEach(resetDb);
afterAll(closeDb);

async function cliLogin(discordCode: string, ip = "203.0.113.7") {
  const p = pkce();
  const s = await start(req(`/api/auth/discord/start?client=cli&login_id=${p.loginId}&challenge=${p.challenge}&v=1.0.0`, { ip }));
  expect(s.status).toBe(302);
  const loc = new URL(s.headers.get("location")!);
  expect(loc.host).toBe("discord.com");
  expect(loc.searchParams.get("code_challenge_method")).toBe("S256");
  const state = loc.searchParams.get("state")!;
  const cb = await callback(req(`/api/auth/discord/callback?code=${encodeURIComponent(discordCode)}&state=${state}`, { ip }));
  return { p, cb, state };
}
const redeem = async (p: ReturnType<typeof pkce>, verifier = p.verifier, ip = "198.51.100.9") =>
  (await poll(post("/api/auth/cli/poll", { login_id: p.loginId, verifier, version: "1.0.0" }, {}, ip))).json() as Promise<any>;

describe("discord login", () => {
  it("authorizes a guild member with the role, records the trusted IP and issues a session", async () => {
    const { p, cb } = await cliLogin("20000000001:member", "203.0.113.50");
    expect(cb.status).toBe(200);
    expect((await redeem(p, "x".repeat(43))).status).toBe("denied"); // wrong verifier
    const ok = await redeem(p);
    expect(ok.status).toBe("approved");
    expect(ok.token).toMatch(/^psy_/);
    const ev = await sql()`select host(ip_address) as ip, result, session_id from login_events order by id`;
    expect(ev[0]).toMatchObject({ ip: "203.0.113.50", result: "success" });
    const tokenRow = await sql()`select token_hash from sessions`;
    expect(tokenRow[0].token_hash).not.toContain(ok.token); // only a hash is stored
    const me = await sessionGet(req("/api/auth/session", { headers: { authorization: `Bearer ${ok.token}` } }));
    expect(me.status).toBe(200);
    expect((await me.json()).user.discord_id).toBe("20000000001");
    // single use
    expect((await redeem(p)).status).toBe("expired");
  });

  it("does not let the client choose its logged IP", async () => {
    const p = pkce();
    const s = await start(req(`/api/auth/discord/start?client=cli&login_id=${p.loginId}&challenge=${p.challenge}`, { ip: "203.0.113.99", headers: { "x-forwarded-for": "6.6.6.6" } }));
    const state = new URL(s.headers.get("location")!).searchParams.get("state")!;
    await callback(req(`/api/auth/discord/callback?code=20000000009:member&state=${state}`, { ip: "203.0.113.99", headers: { "x-forwarded-for": "6.6.6.6" } }));
    expect((await sql()`select host(ip_address) as ip from login_events`)[0].ip).toBe("203.0.113.99");
  });

  it("rejects non-members and users without the required role, and logs the failure", async () => {
    for (const [code, result] of [["20000000002:nomember", "failed_membership"], ["20000000003:norole", "failed_role"]] as const) {
      const { p, cb } = await cliLogin(code);
      expect(cb.status).toBe(403);
      expect((await redeem(p)).status).toBe("denied");
      const r = await sql()`select result from login_events where discord_id = ${code.split(":")[0]}`;
      expect(r[0].result).toBe(result);
    }
    expect((await sql()`select count(*)::int c from sessions`)[0].c).toBe(0);
  });

  it("rejects replayed or forged state and bad OAuth codes", async () => {
    const { state } = await cliLogin("20000000004:member");
    const replay = await callback(req(`/api/auth/discord/callback?code=20000000004:member&state=${state}`));
    expect(replay.status).toBe(400);
    expect((await callback(req(`/api/auth/discord/callback?code=x&state=forged`))).status).toBe(400);
    const { cb } = await cliLogin("bad");
    expect(cb.status).toBe(403);
  });

  it("validates start parameters", async () => {
    expect((await start(req("/api/auth/discord/start?client=cli"))).status).toBe(400);
    expect((await start(req("/api/auth/discord/start?client=evil"))).status).toBe(400);
    expect((await poll(post("/api/auth/cli/poll", { login_id: "short" }))).status).toBe(400);
  });
});

describe("sessions", () => {
  async function session(id = "20000000010") {
    const { p } = await cliLogin(`${id}:member`);
    return (await redeem(p)).token as string;
  }
  const auth = (t: string) => ({ authorization: `Bearer ${t}` });

  it("rejects missing, malformed and unknown tokens", async () => {
    expect((await sessionGet(req("/api/auth/session"))).status).toBe(401);
    expect((await sessionGet(req("/api/auth/session", { headers: auth("psy_nope") }))).status).toBe(401);
    expect((await sessionGet(req("/api/auth/session", { headers: auth("garbage") }))).status).toBe(401);
  });

  it("expires sessions server-side", async () => {
    const t = await session();
    await sql()`update sessions set expires_at = now() - interval '1 minute'`;
    const r = await sessionGet(req("/api/auth/session", { headers: auth(t) }));
    expect(r.status).toBe(401);
    expect((await r.json()).error).toBe("session_expired");
    expect((await sql()`select count(*)::int c from activity_events where type = 'session_expired'`)[0].c).toBe(1);
  });

  it("refresh slides expiry but never past the absolute maximum", async () => {
    const t = await session();
    await sql()`update sessions set expires_at = now() + interval '1 minute', absolute_expires_at = now() + interval '2 hours'`;
    const r = await sessionRefresh(req("/api/auth/session", { method: "POST", headers: auth(t) }));
    expect(r.status).toBe(200);
    const row = (await sql()`select expires_at <= absolute_expires_at as ok from sessions`)[0];
    expect(row.ok).toBe(true);
  });

  it("logout revokes the session", async () => {
    const t = await session();
    expect((await logout(req("/api/auth/logout", { method: "POST", headers: auth(t) }))).status).toBe(200);
    const r = await sessionGet(req("/api/auth/session", { headers: auth(t) }));
    expect(r.status).toBe(401);
    expect((await r.json()).error).toBe("session_revoked");
  });

  it("revoked access and bans are enforced by the backend", async () => {
    const t = await session();
    await sql()`update users set access_status = 'revoked'`;
    expect((await sessionGet(req("/api/auth/session", { headers: auth(t) }))).status).toBe(403);
    await sql()`update users set access_status = 'authorized'`;
    expect((await sessionGet(req("/api/auth/session", { headers: auth(t) }))).status).toBe(200);
    const u = (await sql()`select id from users`)[0].id;
    await sql()`insert into bans (user_id, reason, created_by) values (${u}, 'test', ${u})`;
    const r = await sessionGet(req("/api/auth/session", { headers: auth(t) }));
    expect(r.status).toBe(403);
    expect((await r.json()).error).toBe("banned");
    await sql()`update bans set expires_at = now() - interval '1 second' where revoked_at is null`; // expired ban no longer applies
    expect((await sessionGet(req("/api/auth/session", { headers: auth(t) }))).status).toBe(200);
  });

  it("heartbeat validates input, updates version, records launches", async () => {
    const t = await session();
    expect((await heartbeat(post("/api/client/heartbeat", { launch: "yes", version: "x" }, auth(t)))).status).toBe(400);
    expect((await heartbeat(post("/api/client/heartbeat", { launch: true, version: "1.2.3" }, auth(t)))).status).toBe(200);
    expect((await sql()`select application_version v from sessions`)[0].v).toBe("1.2.3");
    expect((await sql()`select count(*)::int c from activity_events where type = 'app_launched'`)[0].c).toBe(1);
    expect((await heartbeat(post("/api/client/heartbeat", { launch: false, version: "1.2.3" }))).status).toBe(401);
  });

  it("serves the latest release metadata", async () => {
    await sql()`insert into application_releases (version, download_url, checksum_sha256, signature) values
      ('1.9.0','https://x/a.exe',${"a".repeat(64)},'sig'), ('1.10.0','https://x/b.exe',${"b".repeat(64)},'sig')`;
    const j: any = await (await versionGet(req("/api/client/version"))).json();
    expect(j.release.version).toBe("1.10.0"); // numeric, not lexical, ordering
  });

  it("rate limits login starts", async () => {
    let last = 0;
    for (let i = 0; i < 22; i++) {
      const p = pkce();
      last = (await start(req(`/api/auth/discord/start?client=cli&login_id=${p.loginId}&challenge=${p.challenge}`, { ip: "192.0.2.1" }))).status;
    }
    expect(last).toBe(429);
  });
});
