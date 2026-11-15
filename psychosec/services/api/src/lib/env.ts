// Central, validated access to configuration. Nothing here has a secret default.
function req(name: string): string {
  const v = process.env[name];
  if (!v || !v.trim()) throw new Error(`Missing required environment variable ${name}`);
  return v.trim();
}
const opt = (name: string, d = "") => (process.env[name] ?? d).trim();
const list = (name: string) => opt(name).split(",").map((s) => s.trim()).filter(Boolean);
const int = (name: string, d: number) => {
  const n = Number.parseInt(opt(name), 10);
  return Number.isFinite(n) && n > 0 ? n : d;
};

export const env = {
  get dbUrl() { return req("DATABASE_URL"); },
  get discordClientId() { return req("DISCORD_CLIENT_ID"); },
  get discordClientSecret() { return req("DISCORD_CLIENT_SECRET"); },
  get discordGuildId() { return req("DISCORD_GUILD_ID"); },
  get discordRequiredRoleId() { return opt("DISCORD_REQUIRED_ROLE_ID"); },
  get discordRedirectUri() { return req("DISCORD_REDIRECT_URI"); },
  /** Discord IDs allowed to become super-administrators on first admin login. */
  get adminDiscordIds() { return list("ADMIN_DISCORD_IDS"); },
  get adminOrigin() { return opt("ADMIN_ORIGIN", "https://admin.psychosec.org"); },
  get siteOrigin() { return opt("SITE_ORIGIN", "https://psychosec.org"); },
  get allowedOrigins() { return [this.adminOrigin, this.siteOrigin, ...list("EXTRA_ALLOWED_ORIGINS")]; },
  get cookieDomain() { return opt("COOKIE_DOMAIN"); }, // e.g. .psychosec.org ; empty for localhost
  get insecureCookies() { return opt("INSECURE_COOKIES") === "1"; }, // local dev only
  get sessionTtlHours() { return int("SESSION_TTL_HOURS", 12); },
  get sessionMaxDays() { return int("SESSION_MAX_DAYS", 7); },
  get adminSessionHours() { return int("ADMIN_SESSION_HOURS", 8); },
  get loginIpRetentionDays() { return int("LOGIN_IP_RETENTION_DAYS", 90); },
  get trustedProxy() { return opt("TRUSTED_PROXY", "vercel"); }, // vercel | none
};
