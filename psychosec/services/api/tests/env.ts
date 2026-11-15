// Test configuration only. No real credentials.
const url = process.env.TEST_DATABASE_URL ?? "postgres://postgres@localhost:55432/psytest";
Object.assign(process.env, {
  DATABASE_URL: url, DISCORD_CLIENT_ID: "111", DISCORD_CLIENT_SECRET: "test-secret", DISCORD_GUILD_ID: "999",
  DISCORD_REQUIRED_ROLE_ID: "555", DISCORD_REDIRECT_URI: "https://api.test/api/auth/discord/callback",
  ADMIN_DISCORD_IDS: "10000000001", ADMIN_ORIGIN: "https://admin.test", SITE_ORIGIN: "https://site.test",
  TRUSTED_PROXY: "test", INSECURE_COOKIES: "1",
});
