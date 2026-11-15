const csp = [
  "default-src 'self'", "script-src 'self' 'unsafe-inline'", "style-src 'self' 'unsafe-inline'", "img-src 'self' data: https://cdn.discordapp.com",
  `connect-src 'self' ${process.env.NEXT_PUBLIC_API_URL ?? "https://api.psychosec.org"}`, "frame-ancestors 'none'", "base-uri 'none'", "form-action 'self'",
].join("; ");
/** @type {import('next').NextConfig} */
export default {
  transpilePackages: ["@psychosec/shared"],
  poweredByHeader: false,
  productionBrowserSourceMaps: false,
  async headers() {
    return [{ source: "/(.*)", headers: [
      { key: "Content-Security-Policy", value: csp },
      { key: "X-Frame-Options", value: "DENY" },
      { key: "X-Content-Type-Options", value: "nosniff" },
      { key: "Referrer-Policy", value: "no-referrer" },
      { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
      { key: "X-Robots-Tag", value: "noindex, nofollow" },
    ] }];
  },
};
