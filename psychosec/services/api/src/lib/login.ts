import { sql } from "./db";
import { env } from "./env";
import { randomToken, sha256b64url } from "./crypto";
import { activeBan, logActivity, createSession, revokeAllSessions } from "./sessions";
import * as discord from "./discord";

export type Client = "cli" | "admin" | "web";
export const LOGIN_REQUEST_MINUTES = 10;

export async function beginLogin(opts: { client: Client; loginId: string; challenge: string | null; ip: string | null; country: string | null; version: string | null }) {
  const state = randomToken(24);
  const verifier = randomToken(48);
  await sql()`
    insert into login_requests (login_id, client, challenge, oauth_state, oauth_verifier, ip_address, ip_country, application_version, expires_at)
    values (${opts.loginId}, ${opts.client}, ${opts.challenge}, ${state}, ${verifier}, ${opts.ip}::inet, ${opts.country}, ${opts.version},
            now() + make_interval(mins => ${LOGIN_REQUEST_MINUTES}))`;
  return discord.authorizeUrl(state, sha256b64url(verifier));
}

export interface Decision { allowed: boolean; result: string; message: string; userId?: string }

/** Evaluate a Discord identity against membership, role, revocation and bans. */
export async function evaluate(du: discord.DiscordUser, roles: string[] | null, client: Client): Promise<Decision> {
  const isAdminListed = env.adminDiscordIds.includes(du.id);
  const row = await upsertUser(du, roles !== null, roles !== null && (!env.discordRequiredRoleId || roles.includes(env.discordRequiredRoleId)));
  const uid = row.id as string;
  if (roles === null) return { allowed: false, userId: uid, result: "failed_membership", message: "You are not a member of the PSYCHOSEC Discord server." };
  if (env.discordRequiredRoleId && !roles.includes(env.discordRequiredRoleId))
    return { allowed: false, userId: uid, result: "failed_role", message: "Your Discord account does not have the required PSYCHOSEC role." };
  if (row.access_status !== "authorized") return { allowed: false, userId: uid, result: "failed_revoked", message: "Your access to PSYCHOSEC has been revoked." };
  if (await activeBan(uid)) return { allowed: false, userId: uid, result: "failed_banned", message: "Your account is suspended." };
  if (client === "admin") {
    if (isAdminListed) await sql()`insert into admin_users (user_id, role) values (${uid}, 'super_admin') on conflict (user_id) do nothing`;
    const a = await sql()`select 1 from admin_users where user_id = ${uid}`;
    if (!a.length) return { allowed: false, userId: uid, result: "failed_admin_denied", message: "This account is not an administrator." };
  }
  return { allowed: true, userId: uid, result: "success", message: "ok" };
}

async function upsertUser(du: discord.DiscordUser, member: boolean, roleOk: boolean) {
  const r = await sql()`
    insert into users (discord_id, username, display_name, avatar_url, guild_member, role_verified)
    values (${du.id}, ${du.username}, ${du.global_name}, ${discord.avatarUrl(du)}, ${member}, ${roleOk})
    on conflict (discord_id) do update set username = excluded.username, display_name = excluded.display_name,
      avatar_url = excluded.avatar_url, guild_member = excluded.guild_member, role_verified = excluded.role_verified
    returning id, access_status`;
  return r[0];
}

export async function recordLogin(p: { userId?: string; discordId?: string; ip: string | null; country: string | null; result: string; client: Client; version: string | null; sessionId?: string | null }) {
  await sql()`insert into login_events (user_id, discord_id, ip_address, ip_country, result, session_id, application_version, client)
    values (${p.userId ?? null}, ${p.discordId ?? null}, ${p.ip}::inet, ${p.country}, ${p.result}::login_result, ${p.sessionId ?? null}, ${p.version}, ${p.client})`;
}

/** Handles the Discord redirect. Returns what the callback route should do next. */
export async function completeOAuth(opts: { code: string | null; state: string | null; ip: string | null; country: string | null }) {
  if (!opts.state) return { kind: "error" as const, message: "Missing state." };
  const rows = await sql()`select * from login_requests where oauth_state = ${opts.state} and status = 'pending' and expires_at > now()`;
  const lr = rows[0];
  if (!lr) {
    await recordLogin({ ip: opts.ip, country: opts.country, result: "failed_state", client: "cli", version: null });
    return { kind: "error" as const, message: "This sign-in link is invalid or expired. Start again from the application." };
  }
  // The state is single use regardless of outcome.
  await sql()`update login_requests set oauth_state = null where id = ${lr.id}`;
  const client = lr.client as Client;
  const version = (lr.application_version as string | null) ?? null;
  if (!opts.code) {
    await sql()`update login_requests set status = 'denied', message = 'Authorization was cancelled.' where id = ${lr.id}`;
    await recordLogin({ ip: opts.ip, country: opts.country, result: "failed_oauth", client, version });
    return { kind: "denied" as const, client, message: "Authorization was cancelled." };
  }
  let du: discord.DiscordUser, roles: string[] | null;
  try {
    const at = await discord.exchangeCode(opts.code, lr.oauth_verifier);
    du = await discord.fetchUser(at);
    roles = await discord.fetchGuildRoles(at);
  } catch (e) {
    console.error("oauth error", (e as Error).message);
    await sql()`update login_requests set status = 'denied', message = 'Discord sign-in failed.' where id = ${lr.id}`;
    await recordLogin({ ip: opts.ip, country: opts.country, result: "failed_oauth", client, version });
    return { kind: "denied" as const, client, message: "Discord sign-in failed. Try again." };
  }
  const d = await evaluate(du, roles, client);
  if (!d.allowed) {
    await sql()`update login_requests set status = 'denied', message = ${d.message}, user_id = ${d.userId ?? null} where id = ${lr.id}`;
    await recordLogin({ userId: d.userId, discordId: du.id, ip: opts.ip, country: opts.country, result: d.result, client, version });
    await logActivity(d.userId ?? null, d.result === "failed_membership" || d.result === "failed_role" ? "membership_failed" : "login_failed", version, d.result);
    return { kind: "denied" as const, client, message: d.message };
  }
  await sql()`update users set last_login_at = now(), last_activity_at = now() where id = ${d.userId ?? null}`;
  if (client === "cli") {
    // Session is created when the CLI redeems its verifier; log the successful authorization now.
    await sql()`update login_requests set status = 'approved', user_id = ${d.userId ?? null} where id = ${lr.id}`;
    await recordLogin({ userId: d.userId, discordId: du.id, ip: opts.ip, country: opts.country, result: "success", client, version });
    await logActivity(d.userId!, "login_succeeded", version);
    return { kind: "approved" as const, client };
  }
  const s = await createSession({ userId: d.userId!, kind: client === "admin" ? "admin" : "web", ip: opts.ip, version });
  await sql()`update login_requests set status = 'consumed', user_id = ${d.userId ?? null} where id = ${lr.id}`;
  await recordLogin({ userId: d.userId, discordId: du.id, ip: opts.ip, country: opts.country, result: "success", client, version, sessionId: s.id });
  await logActivity(d.userId!, "login_succeeded", version, client);
  return { kind: "session" as const, client, token: s.token, expires_at: s.expires_at };
}

/** CLI redeems its approved login with the PKCE verifier and receives a session. */
export async function redeemCliLogin(loginId: string, verifier: string, ip: string | null, version: string | null) {
  const found = await sql()`select * from login_requests where login_id = ${loginId} and client = 'cli'`;
  const lr = found[0];
  if (!lr) return { status: "expired" as const, message: "Unknown sign-in request." };
  if (new Date(lr.expires_at) <= new Date() && lr.status !== "consumed") return { status: "expired" as const, message: "The sign-in request expired." };
  if (lr.status === "pending") return { status: "pending" as const };
  if (lr.status === "denied") return { status: "denied" as const, message: lr.message ?? "Access denied." };
  if (lr.status === "consumed") return { status: "expired" as const, message: "This sign-in was already used." };
  if (sha256b64url(verifier) !== lr.challenge) return { status: "denied" as const, message: "Sign-in verification failed." };
  const consumed = await sql()`select * from consume_login_request(${loginId})`;
  if (!consumed[0]?.id) return { status: "expired" as const, message: "This sign-in was already used." };
  // Re-check the account at redemption time (it may have been banned while the user was authorizing).
  const u = (await sql()`select id, access_status, discord_id, username, display_name, avatar_url, role_verified from users where id = ${lr.user_id}`)[0];
  if (!u || u.access_status !== "authorized" || (await activeBan(u.id))) return { status: "denied" as const, message: "Your access is not currently permitted." };
  const s = await createSession({ userId: u.id, kind: "app", ip, version });
  await sql()`update login_events set session_id = ${s.id} where id = (select id from login_events where user_id = ${u.id} and client = 'cli' and result = 'success' and session_id is null order by created_at desc limit 1)`;
  return {
    status: "approved" as const,
    token: s.token,
    session: { user: { discord_id: u.discord_id, username: u.username, display_name: u.display_name, avatar_url: u.avatar_url }, expires_at: s.expires_at.toISOString(), role_verified: u.role_verified },
  };
}

export { revokeAllSessions };
