import { env } from "./env";

const API = "https://discord.com/api/v10";

export function authorizeUrl(state: string, codeChallenge: string): string {
  const u = new URL("https://discord.com/oauth2/authorize");
  u.searchParams.set("client_id", env.discordClientId);
  u.searchParams.set("response_type", "code");
  u.searchParams.set("redirect_uri", env.discordRedirectUri);
  u.searchParams.set("scope", "identify guilds.members.read");
  u.searchParams.set("state", state);
  u.searchParams.set("code_challenge", codeChallenge);
  u.searchParams.set("code_challenge_method", "S256");
  u.searchParams.set("prompt", "none");
  return u.toString();
}

export async function exchangeCode(code: string, verifier: string): Promise<string> {
  const r = await fetch(`${API}/oauth2/token`, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      client_id: env.discordClientId, client_secret: env.discordClientSecret, grant_type: "authorization_code",
      code, redirect_uri: env.discordRedirectUri, code_verifier: verifier,
    }),
  });
  if (!r.ok) throw new Error(`discord token exchange failed (${r.status})`);
  const j = (await r.json()) as { access_token?: string };
  if (!j.access_token) throw new Error("discord returned no access token");
  return j.access_token;
}

export interface DiscordUser { id: string; username: string; global_name: string | null; avatar: string | null }
export async function fetchUser(token: string): Promise<DiscordUser> {
  const r = await fetch(`${API}/users/@me`, { headers: { authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`discord user lookup failed (${r.status})`);
  return (await r.json()) as DiscordUser;
}

/** Returns the guild member's role IDs, or null when the user is not in the guild. */
export async function fetchGuildRoles(token: string): Promise<string[] | null> {
  const r = await fetch(`${API}/users/@me/guilds/${env.discordGuildId}/member`, { headers: { authorization: `Bearer ${token}` } });
  if (r.status === 404 || r.status === 403 || r.status === 10007) return null;
  if (!r.ok) throw new Error(`discord membership lookup failed (${r.status})`);
  const j = (await r.json()) as { roles?: string[] };
  return j.roles ?? [];
}

export const avatarUrl = (u: DiscordUser) =>
  u.avatar ? `https://cdn.discordapp.com/avatars/${u.id}/${u.avatar}.png?size=128` : null;
