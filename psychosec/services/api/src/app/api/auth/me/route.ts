import { ADMIN_COOKIE, CSRF_COOKIE, WEB_COOKIE } from "@/lib/admin";
import { authenticateToken } from "@/lib/sessions";
import { sql } from "@/lib/db";
import { json, parseCookies, safe, unauthorized, forbidden, deny } from "@/lib/http";

/** Who am I? For the website (?kind=web) and admin dashboard (?kind=admin), via HTTP-only cookie. */
export const GET = safe(async (req) => {
  const kind = new URL(req.url).searchParams.get("kind") === "admin" ? "admin" : "web";
  const ck = parseCookies(req);
  const a = await authenticateToken(ck[kind === "admin" ? ADMIN_COOKIE : WEB_COOKIE], kind);
  if (!a.ok) return deny(a);
  const base = { discord_id: a.user.discord_id, username: a.user.username, display_name: a.user.display_name, avatar_url: a.user.avatar_url };
  if (kind === "web") return json(base);
  const role = (await sql()`select role from admin_users where user_id = ${a.user.id}`)[0]?.role;
  if (!role) return forbidden("Not an administrator.");
  return json({ ...base, role, csrf: ck[CSRF_COOKIE] ?? "" });
});
