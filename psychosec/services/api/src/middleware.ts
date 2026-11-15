import { NextRequest, NextResponse } from "next/server";

// CORS for the admin dashboard and website only; everything else is same-origin/no-CORS.
export function middleware(req: NextRequest) {
  const allowed = [
    process.env.ADMIN_ORIGIN ?? "https://admin.psychosec.org",
    process.env.SITE_ORIGIN ?? "https://psychosec.org",
    ...(process.env.EXTRA_ALLOWED_ORIGINS ?? "").split(",").map((s) => s.trim()).filter(Boolean),
  ];
  const origin = req.headers.get("origin");
  const ok = origin !== null && allowed.includes(origin);
  if (req.method === "OPTIONS") {
    const h = new Headers();
    if (ok) {
      h.set("access-control-allow-origin", origin!);
      h.set("access-control-allow-credentials", "true");
      h.set("access-control-allow-methods", "GET,POST,DELETE,OPTIONS");
      h.set("access-control-allow-headers", "content-type,x-csrf-token");
      h.set("access-control-max-age", "600");
      h.set("vary", "origin");
    }
    return new NextResponse(null, { status: ok ? 204 : 403, headers: h });
  }
  const res = NextResponse.next();
  if (ok) {
    res.headers.set("access-control-allow-origin", origin!);
    res.headers.set("access-control-allow-credentials", "true");
    res.headers.set("vary", "origin");
  }
  return res;
}
export const config = { matcher: "/api/:path*" };
