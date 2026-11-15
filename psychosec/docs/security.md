# Security model

**Server-side authority.** The executable holds no secret. Discord OAuth2 (state + PKCE to Discord) happens on the backend.
The CLI starts a browser login carrying its own PKCE-style verifier; only the holder of that verifier can redeem the
approved login, once, for an application session. Membership, role, revocation and bans are re-evaluated on **every**
request (`lib/sessions.ts`), including at redemption.

**Sessions.** Opaque `psy_…` tokens, only SHA-256 hashes stored. App sessions slide 12 h to an absolute 7 days; admin
sessions are 8 h absolute. Tokens are never returned by any admin endpoint. CLI stores its token in Windows Credential Manager.

**Admin.** Deny by default: separate `admin` session kind via HTTP-only cookie + a row in `admin_users` checked per request.
Bearer tokens are rejected. Mutations need Origin allow-list + HMAC CSRF token + explicit `confirm:true`. Admins can't act on
themselves; only super admins can act on admins. All actions go to an append-only audit table (a trigger blocks UPDATE/DELETE).
Initial super admins come from `ADMIN_DISCORD_IDS` only.

**IP logging.** IPs come from `x-vercel-forwarded-for` (set by Vercel's edge, not client-controllable). Client-supplied IPs are
ignored (tested). Country is the edge's approximate value, labelled as such. Retention is configurable and purged daily.
Disclosure: `apps/website/src/app/privacy/page.tsx`, installer privacy page.

**Local data.** Case data is encrypted field-by-field (ChaCha20-Poly1305); key in Credential Manager. Not sent to the backend.

**Updates.** Ed25519-signed manifest + SHA-256 + HTTPS required; unsigned/unverifiable updates are never executed.

**Rate limiting.** Postgres fixed-window limiter (`rate_limit_hit`) on login start/callback/poll, session, heartbeat, version.

## Source protection — honest limits
Keep the repository private, ship only the installer, release builds are stripped with remapped paths, and all authorization
is on the server. A native executable can still be disassembled or patched by a determined person; that is why nothing
valuable or secret lives in it and why a patched client gains nothing the backend doesn't grant.

## Responsible use
Username/web modules make unauthenticated, rate-limited public requests and do not bypass logins or anti-abuse systems.
`web inspect` sends a single passive GET. Only assess systems you own or are authorized to test.
