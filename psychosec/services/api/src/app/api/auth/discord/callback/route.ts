import { completeOAuth } from "@/lib/login";
import { ADMIN_COOKIE, CSRF_COOKIE, WEB_COOKIE, csrfFor } from "@/lib/admin";
import { clientCountry, clientIp, esc, html, rateLimit, safe, setCookie, tooMany } from "@/lib/http";
import { env } from "@/lib/env";

const page = (title: string, body: string, ok: boolean, status = 200, headers: Record<string, string> = {}) =>
  html(`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>PSYCHOSEC</title>
<body style="background:#08090B;color:#F2F2F2;font:15px/1.6 ui-monospace,Consolas,monospace;display:grid;place-items:center;height:100vh;margin:0">
<div style="max-width:420px;padding:24px;border:1px solid #292B2F;border-radius:8px;text-align:center">
<div style="letter-spacing:.3em;color:#87BDBB;font-weight:700">PSYCHOSEC</div>
<h1 style="font-size:16px;color:${ok ? "#91C5A5" : "#D78585"}">${esc(title)}</h1><p style="color:#858585">${esc(body)}</p></div></body>`, status, headers);

export const GET = safe(async (req) => {
  const ip = clientIp(req);
  if (!(await rateLimit(`login-cb:${ip ?? "unknown"}`, 60, 30))) return tooMany();
  const u = new URL(req.url);
  const r = await completeOAuth({ code: u.searchParams.get("code"), state: u.searchParams.get("state"), ip, country: clientCountry(req) });
  if (r.kind === "error") return page("Sign-in failed", r.message, false, 400);
  if (r.kind === "denied") return page("Access denied", r.message, false, 403);
  if (r.kind === "approved") return page("Signed in", "You can close this tab and return to the PSYCHOSEC terminal.", true);
  // admin / web: cookie session then redirect to the right site
  const maxAge = Math.max(60, Math.floor((new Date(r.expires_at).getTime() - Date.now()) / 1000));
  const cookie = r.client === "admin" ? ADMIN_COOKIE : WEB_COOKIE;
  const headers = new Headers({ location: r.client === "admin" ? env.adminOrigin : env.siteOrigin + "/members", "cache-control": "no-store" });
  headers.append("set-cookie", setCookie(cookie, r.token, { maxAgeSec: maxAge }));
  if (r.client === "admin") headers.append("set-cookie", setCookie(CSRF_COOKIE, csrfFor(r.token), { maxAgeSec: maxAge, httpOnly: false }));
  return new Response(null, { status: 302, headers });
});
