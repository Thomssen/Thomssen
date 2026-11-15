# Testing

## Automated
| Suite | Command | Covers |
|---|---|---|
| CLI (Rust) | `cargo test -p psychosec` | command parser, encryption, case store, report escaping/CSV injection, update signature verification (incl. a manifest signed by the real Node script), PKCE vector, parsers for DNS/RDAP/headers/EXIF helpers, UI rendering on a test backend |
| API (Vitest, real PostgreSQL) | `npm test -w @psychosec/api` | Discord login (member / non-member / missing role / replayed state / bad code), trusted-IP logging, PKCE redemption, session expiry, refresh cap, logout, revocation, bans, **admin permission boundaries** (anonymous, app token, web-member cookie, non-admin, removed admin, CSRF, origin), self-action, input validation, SQL-metacharacter search, rate limiting, append-only audit |
| Migrations | `PGURL=... scripts/validate-migrations.sh` | applies all SQL to a scratch database |

API tests need a PostgreSQL server: set `TEST_DATABASE_URL=postgres://user:pass@host:port/psytest` (the test DB is dropped/recreated).
Discord is stubbed; no real Discord/Supabase call is made.

## Not tested in the development environment (verify before launch)
- **Windows build, installer, uninstaller, upgrade**: built on Linux; Windows target compile and Inno Setup need a Windows host (CI runs them on `windows-latest`).
- **Windows Credential Manager** storage and **Windows Terminal / conhost rendering** (rendering is verified on a virtual terminal backend only).
- **Live OSINT sources** (Cloudflare DoH, rdap.org, crt.sh, ipwho.is, platform endpoints): response parsing is unit-tested, but live calls were not exercised from the build sandbox. Platform endpoints change; re-check the username site list (`apps/cli/src/osint/username.rs`) periodically.
- **Real Discord OAuth**, Supabase hosting, Vercel deployment, custom domains/DNS, edge IP headers: need your credentials.
- **Authenticode signing**: needs your certificate.
