import { beforeAll, beforeEach, afterAll, describe, expect, it } from "vitest";
import { sql, closeDb } from "@/lib/db";
import { GET as start } from "@/app/api/auth/discord/start/route";
import { GET as callback } from "@/app/api/auth/discord/callback/route";
import { POST as poll } from "@/app/api/auth/cli/poll/route";
import { GET as sessionGet } from "@/app/api/auth/session/route";
import { GET as me } from "@/app/api/auth/me/route";
import { GET as overview } from "@/app/api/admin/overview/route";
import { GET as users } from "@/app/api/admin/users/route";
import { GET as userGet, POST as userAct } from "@/app/api/admin/users/[id]/route";
import { GET as sessionsList, POST as sessionKill } from "@/app/api/admin/sessions/route";
import { GET as activity } from "@/app/api/admin/activity/route";
import { GET as bansList } from "@/app/api/admin/bans/route";
import { GET as auditList } from "@/app/api/admin/audit-logs/route";
import { GET as security } from "@/app/api/admin/security/route";
import { GET as settings } from "@/app/api/admin/settings/route";
import { mockDiscord, pkce, post, req, resetDb } from "./helpers";

beforeAll(mockDiscord);
beforeEach(resetDb);
afterAll(closeDb);

const ADMIN_ID = "10000000001"; // in ADMIN_DISCORD_IDS
const ORIGIN = "https://admin.test";

async function webLogin(client: "admin" | "web", id: string) {
  const s = await start(req(`/api/auth/discord/start?client=${client}`));
  const state = new URL(s.headers.get("location")!).searchParams.get("state")!;
  const cb = await callback(req(`/api/auth/discord/callback?code=${id}:member&state=${state}`));
  const cookies = Object.fromEntries(cb.headers.getSetCookie().map((c) => c.split(";")[0].split("=") as [string, string]));
  return { cb, cookies };
}
async function adminSession() {
  const { cb, cookies } = await webLogin("admin", ADMIN_ID);
  expect(cb.status).toBe(302);
  return { cookie: `psy_admin=${cookies.psy_admin}; psy_csrf=${cookies.psy_csrf}`, csrf: decodeURIComponent(cookies.psy_csrf) };
}
async function appUser(id: string) {
  const p = pkce();
  const s = await start(req(`/api/auth/discord/start?client=cli&login_id=${p.loginId}&challenge=${p.challenge}`));
  const state = new URL(s.headers.get("location")!).searchParams.get("state")!;
  await callback(req(`/api/auth/discord/callback?code=${id}:member&state=${state}`));
  const j: any = await (await poll(post("/api/auth/cli/poll", { login_id: p.loginId, verifier: p.verifier }))).json();
  return j.token as string;
}
const ctx = (id: string) => ({ params: Promise.resolve({ id }) });
const mut = (url: string, body: unknown, a: { cookie: string; csrf: string }, o: Record<string, string> = {}) =>
  post(url, body, { cookie: a.cookie, "x-csrf-token": a.csrf, origin: ORIGIN, ...o });
const adminGet = (url: string, a: { cookie: string }) => req(url, { headers: { cookie: a.cookie } });

const READ_ROUTES: [string, (r: Request) => Promise<Response>][] = [
  ["/api/admin/overview", overview], ["/api/admin/users", users], ["/api/admin/sessions", sessionsList], ["/api/admin/activity", activity],
  ["/api/admin/bans", bansList], ["/api/admin/audit-logs", auditList], ["/api/admin/security", security], ["/api/admin/settings", settings],
];

describe("admin permission boundaries", () => {
  it("denies unauthenticated callers on every admin endpoint", async () => {
    for (const [u, h] of READ_ROUTES) expect((await h(req(u))).status, u).toBe(401);
  });

  it("denies ordinary members: app bearer tokens and web-member cookies", async () => {
    const tok = await appUser("20000000021");
    const web = await webLogin("web", "20000000021");
    for (const [u, h] of READ_ROUTES) {
      expect((await h(req(u, { headers: { authorization: `Bearer ${tok}` } }))).status, u + " bearer").toBe(403);
      expect((await h(req(u, { headers: { cookie: `psy_admin=${web.cookies.psy_web}` } }))).status, u + " web cookie as admin").toBe(401);
    }
  });

  it("refuses admin sign-in for Discord members who are not on the allowlist", async () => {
    const { cb } = await webLogin("admin", "20000000022");
    expect(cb.status).toBe(403);
    expect((await sql()`select count(*)::int c from sessions where kind = 'admin'`)[0].c).toBe(0);
    expect((await sql()`select result from login_events where discord_id = '20000000022'`)[0].result).toBe("failed_admin_denied");
  });

  it("allows allow-listed admins to read, and does not expose token material", async () => {
    await appUser("20000000023");
    const a = await adminSession();
    for (const [u, h] of READ_ROUTES) expect((await h(adminGet(u, a))).status, u).toBe(200);
    const raw = await (await sessionsList(adminGet("/api/admin/sessions", a))).text();
    expect(raw).not.toMatch(/psy_|token_hash/);
    const ov: any = await (await overview(adminGet("/api/admin/overview", a))).json();
    expect(ov.total_users).toBe(2);
    expect(ov.valid_sessions).toBe(1);
    expect(ov.series).toHaveLength(14);
  });

  it("reports the admin's identity with a CSRF token", async () => {
    const a = await adminSession();
    const j: any = await (await me(req("/api/auth/me?kind=admin", { headers: { cookie: a.cookie } }))).json();
    expect(j).toMatchObject({ discord_id: ADMIN_ID, role: "super_admin" });
    expect(j.csrf).toBeTruthy();
    expect((await me(req("/api/auth/me?kind=admin"))).status).toBe(401);
  });

  it("enforces CSRF and origin on every mutation", async () => {
    const a = await adminSession();
    const uid = (await sql()`select id from users where discord_id = ${ADMIN_ID}`)[0].id;
    const body = { action: "unban", confirm: true };
    expect((await userAct(post(`/api/admin/users/${uid}`, body, { cookie: a.cookie, origin: ORIGIN }), ctx(uid))).status).toBe(403); // no csrf
    expect((await userAct(mut(`/api/admin/users/${uid}`, body, a, { "x-csrf-token": "wrong" }), ctx(uid))).status).toBe(403);
    expect((await userAct(mut(`/api/admin/users/${uid}`, body, a, { origin: "https://evil.example" }), ctx(uid))).status).toBe(403);
  });

  it("refuses sessions that were terminated or whose admin rights were removed", async () => {
    const a = await adminSession();
    await sql()`delete from admin_users`;
    expect((await overview(adminGet("/api/admin/overview", a))).status).toBe(403);
    await sql()`update sessions set revoked_at = now()`;
    expect((await overview(adminGet("/api/admin/overview", a))).status).toBe(401);
  });
});

describe("admin user management", () => {
  it("bans, enforces the ban on live sessions and new logins, audits, then unbans", async () => {
    const tok = await appUser("20000000031");
    const a = await adminSession();
    const target = (await sql()`select id from users where discord_id = '20000000031'`)[0].id;

    // confirmation + reason are mandatory
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "ban", reason: "abuse" }, a), ctx(target))).status).toBe(400);
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "ban", confirm: true }, a), ctx(target))).status).toBe(400);
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "ban", reason: "abuse of service", confirm: true }, a), ctx(target))).status).toBe(200);

    const r = await sessionGet(req("/api/auth/session", { headers: { authorization: `Bearer ${tok}` } }));
    expect([401, 403]).toContain(r.status);
    // new login attempt is refused
    const p = pkce();
    const s = await start(req(`/api/auth/discord/start?client=cli&login_id=${p.loginId}&challenge=${p.challenge}`));
    const state = new URL(s.headers.get("location")!).searchParams.get("state")!;
    expect((await callback(req(`/api/auth/discord/callback?code=20000000031:member&state=${state}`))).status).toBe(403);
    expect((await sql()`select result from login_events where discord_id = '20000000031' order by id desc limit 1`)[0].result).toBe("failed_banned");

    const detail: any = await (await userGet(adminGet(`/api/admin/users/${target}`, a), ctx(target))).json();
    expect(detail.bans).toHaveLength(1);
    expect(detail.logins[0].ip).toBeTruthy();
    expect(detail.ip_country_note).toMatch(/approximate/i);

    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "unban", confirm: true }, a), ctx(target))).status).toBe(200);
    const audit: any = await (await auditList(adminGet("/api/admin/audit-logs", a))).json();
    expect(audit.logs.map((l: any) => l.action)).toEqual(expect.arrayContaining(["ban", "unban"]));
  });

  it("revokes access, all sessions, and terminates a single session", async () => {
    const tok = await appUser("20000000032");
    const a = await adminSession();
    const target = (await sql()`select id from users where discord_id = '20000000032'`)[0].id;
    const sid = (await sql()`select id from sessions where kind = 'app'`)[0].id;
    expect((await sessionKill(mut("/api/admin/sessions", { id: sid, confirm: true }, a))).status).toBe(200);
    expect((await sessionGet(req("/api/auth/session", { headers: { authorization: `Bearer ${tok}` } }))).status).toBe(401);
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "revoke", reason: "left the team", confirm: true }, a), ctx(target))).status).toBe(200);
    expect((await sql()`select access_status s from users where id = ${target}`)[0].s).toBe("revoked");
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "allow", confirm: true }, a), ctx(target))).status).toBe(200);
    expect((await userAct(mut(`/api/admin/users/${target}`, { action: "revoke_sessions", confirm: true }, a), ctx(target))).status).toBe(200);
  });

  it("blocks self-targeting, malformed ids and cross-admin actions by regular admins", async () => {
    const a = await adminSession();
    const self = (await sql()`select id from users where discord_id = ${ADMIN_ID}`)[0].id;
    expect((await userAct(mut(`/api/admin/users/${self}`, { action: "ban", reason: "nope nope", confirm: true }, a), ctx(self))).status).toBe(403);
    expect((await userAct(mut(`/api/admin/users/not-a-uuid`, { action: "allow", confirm: true }, a), ctx("not-a-uuid"))).status).toBe(400);
    expect((await userGet(adminGet(`/api/admin/users/00000000-0000-0000-0000-000000000000`, a), ctx("00000000-0000-0000-0000-000000000000"))).status).toBe(404);
    // downgrade caller to a regular admin; super_admin target is now off limits
    await appUser("20000000033");
    const other = (await sql()`select id from users where discord_id = '20000000033'`)[0].id;
    await sql()`insert into admin_users (user_id, role) values (${other}, 'admin')`;
    await sql()`update admin_users set role = 'admin' where user_id = ${self}`;
    await sql()`update admin_users set role = 'super_admin' where user_id = ${other}`;
    expect((await userAct(mut(`/api/admin/users/${other}`, { action: "ban", reason: "no no no", confirm: true }, a), ctx(other))).status).toBe(403);
  });

  it("searches users safely (wildcards and SQL metacharacters are inert)", async () => {
    await appUser("20000000034");
    const a = await adminSession();
    const find = async (q: string) => ((await (await users(adminGet(`/api/admin/users?q=${encodeURIComponent(q)}`, a))).json()) as any).users;
    expect(await find("20000000034")).toHaveLength(1);
    expect(await find("%")).toHaveLength(0);
    expect(await find("'; drop table users; --")).toHaveLength(0);
    expect((await find("")).length).toBe(2);
  });

  it("audit log is append-only", async () => {
    const a = await adminSession();
    const t = (await sql()`select id from users`)[0].id;
    await userAct(mut(`/api/admin/users/${t}`, { action: "unban", confirm: true }, a), ctx(t)).catch(() => {});
    await sql()`insert into admin_audit_logs (admin_user_id, action) values (${t}, 'x')`;
    await expect(sql()`update admin_audit_logs set action = 'y'`).rejects.toThrow(/append-only/);
    await expect(sql()`delete from admin_audit_logs`).rejects.toThrow(/append-only/);
  });
});
