# Deployment

Nothing here has been deployed from this repository yet. Each step below needs an account/credential only you have.
Do not paste secrets into GitHub, chat or source files.

## 1. Supabase
1. Create (or pick) a project at supabase.com.
2. SQL editor → run `database/migrations/0001_init.sql` (or `supabase db push`). All tables have RLS enabled with
   **no policies**, so the public `anon`/`authenticated` keys can read nothing; only the backend's direct database
   connection is used.
3. Project Settings → Database → copy the **Transaction pooler** connection string (port 6543). This is `DATABASE_URL`.
4. Retention: the API's daily Vercel cron (`/api/cron/purge`) calls `purge_old_records`. Optionally also schedule it with
   `pg_cron`: `select cron.schedule('purge','17 3 * * *','select purge_old_records(90,30,180)');`

## 2. Discord application
1. discord.com/developers/applications → New Application → OAuth2.
2. Add redirect `https://api.psychosec.org/api/auth/discord/callback` (exact match).
3. Copy Client ID / Client Secret into Vercel. Scopes used: `identify guilds.members.read` (no bot token needed).
4. Enable Developer Mode in Discord → right-click your server → *Copy Server ID* → `DISCORD_GUILD_ID`.
5. Optional: right-click a role → *Copy Role ID* → `DISCORD_REQUIRED_ROLE_ID`.
6. Your own user ID (right-click yourself → *Copy User ID*) → `ADMIN_DISCORD_IDS`. That user becomes super
   administrator the first time they sign in to the dashboard. There is no default admin and no admin password.

## 3. Vercel (three projects from the same repository)
| Project | Root directory | Domain | Env vars |
|---|---|---|---|
| psychosec-api | `services/api` | `api.psychosec.org` | all API vars in `.env.example` |
| psychosec-admin | `apps/admin` | `admin.psychosec.org` | `NEXT_PUBLIC_API_URL` |
| psychosec-site | `apps/website` | `psychosec.org` | `NEXT_PUBLIC_API_URL`, `NEXT_PUBLIC_LOGIN_IP_RETENTION_DAYS` |

Set "Include source files outside of the Root Directory" (needed for the `@psychosec/shared` workspace package) and
use the default Next.js build command. Install command: `npm install` at the repository root.

**Existing psychosec.org site:** this repository does not replace it. Before attaching `psychosec.org` to
`psychosec-site`, inspect the current deployment (Vercel dashboard → Domains). To keep an existing site, deploy
`apps/website` on a preview domain first and move over only the pages you want (`/download`, `/privacy`, `/members`).

## 4. DNS (at your registrar / DNS host)
Add the records that Vercel shows in *Project → Settings → Domains* for each domain. Typically:
- `psychosec.org` → `A 76.76.21.21` (or the value Vercel displays)
- `admin.psychosec.org` → `CNAME cname.vercel-dns.com`
- `api.psychosec.org` → `CNAME cname.vercel-dns.com`

Use the exact values Vercel displays; they can differ per project. The domains are **not verified working** until Vercel
shows "Valid Configuration" and you complete a test sign-in (see checklist).

## 5. First-run checklist
1. `curl https://api.psychosec.org/api/client/version` → `{"release":null}`.
2. Open `https://admin.psychosec.org`, sign in with Discord as an `ADMIN_DISCORD_IDS` user.
3. Build/install the CLI (docs/releasing.md), launch it, complete Discord login, confirm the session and your IP
   appear in the dashboard (Sessions, Users → your profile).
4. Ban/unban a test account and confirm the CLI locks out within one heartbeat (≤5 min) or immediately on its next API call.

## Cookies and subdomains
Admin/web sessions use HTTP-only cookies on `COOKIE_DOMAIN=.psychosec.org` (SameSite=Lax, Secure). The admin site reads
only the non-secret CSRF cookie. For local development set `INSECURE_COOKIES=1` and an empty `COOKIE_DOMAIN`.
