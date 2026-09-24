# Thomsen Shield - License API

The backend for Thomsen Shield's $19.99 lifetime license: Stripe Checkout
for payment, a Supabase table as the license database, and a small set of
Vercel serverless functions in between. The desktop app never talks to
Stripe or Supabase directly - only to this API, and only over these
endpoints.

Deployed separately from [thomssen.dev](https://thomssen.dev), which stays
exactly where it already is (Hostinger). This exists because Hostinger's
shared hosting isn't a reliable place to run a payment webhook handler;
everything else about the site is untouched. See **Hosting** below.

```
Customer clicks "Buy - $19.99 Lifetime"
        |
        v
GET /buy (= /api/stripe/checkout) --> redirects to Stripe Checkout
        |
   payment succeeds
        v
POST /api/stripe/webhook  (signature verified, idempotent)
        |
        v
license row created in Supabase, key shown at /success
        |
        v
customer pastes the key into Thomsen Shield
        |
        v
POST /api/license/activate --> Supabase check --> signed token back
```

## Endpoints

| Endpoint | Method | Who calls it |
|---|---|---|
| `/buy` (`/api/stripe/checkout`) | GET | The website's Buy button - a plain link, no JS |
| `/api/stripe/webhook` | POST | Stripe only |
| `/success` (`/api/purchase/success`) | GET | The browser, right after payment |
| `/api/license/activate` | POST | The desktop app |
| `/api/license/validate` | POST | The desktop app (periodic background check-in) |
| `/api/license/deactivate` | POST | The desktop app ("Deactivate" in Settings) |
| `/admin` (`/api/admin/*`) | GET/POST | You, with the admin token |

## Licensing model

- One license = one row in `licenses` (see `supabase/schema.sql`).
- One active device at a time, enforced by a `device_id` column that's
  either null (free) or bound to one device. `device_id` is a hash of the
  Windows install's own MachineGuid, computed client-side - not a
  fingerprint of anything more invasive.
- Activation and validation return a token signed with Ed25519
  (`lib/signing.ts`), never a bare boolean. The desktop app verifies that
  signature locally with an embedded public key and caches the token -
  see `thomsen-shield/src-tauri/src/license/` for the client half of this.
- A token is valid for 14 days from when it was issued. The client
  re-validates every few hours in the background; if that keeps failing
  (offline), the *previous* token just keeps working until its own
  14-day window runs out. That's the offline grace period - there's no
  separate timer or config for it, it falls out of how the token expiry
  works.

## Setup

### 1. Supabase

Create a project at [supabase.com](https://supabase.com) (the free tier
is enough), then run `supabase/schema.sql` in its SQL Editor. Copy the
Project URL and the **service_role** key (Settings -> API) into your env
- see `.env.example`.

### 2. Stripe

1. Create a product ("Thomsen Shield") with a one-time $19.99 price.
   Copy its price id (`price_...`).
2. Developers -> Webhooks -> Add endpoint, pointing at
   `https://api.thomssen.dev/api/stripe/webhook`, subscribed to
   `checkout.session.completed`. Copy the signing secret (`whsec_...`).
3. Copy your secret key (`sk_...`) from Developers -> API keys.

Use test-mode keys everywhere until you're ready to go live - see
**Development mode** below.

### 3. Signing keypair

**Required even for local testing** - there is no default/shared keypair
anywhere in either repo, on purpose: a private key that ships in a public
repo isn't private. You generate your own for every environment
(local, and again, separately, for production):

```
npm install
npm run generate-keypair
```

Paste the two printed values into your env as
`LICENSE_SIGNING_PRIVATE_KEY_B64` / `LICENSE_SIGNING_PUBLIC_KEY_B64`.
Then take the public key value and paste it into `DEV_PUBLIC_KEY` in
`thomsen-shield/src-tauri/src/license/token.rs` (it's a byte array - the
comment there shows the format), so the desktop app you're testing
against verifies tokens *this* backend actually issues. Run
`generate-keypair` again for your production Vercel environment and
repeat the same `DEV_PUBLIC_KEY` update before you ship - production and
development must never share a keypair.

### 4. Admin token

```
npm run generate-admin-token
```

Paste the result in as `ADMIN_API_TOKEN`.

### 5. Deploy to Vercel

```
npm i -g vercel   # if you don't have it
vercel link
vercel env add SUPABASE_URL production
vercel env add SUPABASE_SERVICE_ROLE_KEY production
vercel env add STRIPE_SECRET_KEY production
vercel env add STRIPE_WEBHOOK_SECRET production
vercel env add STRIPE_PRICE_ID production
vercel env add LICENSE_SIGNING_PRIVATE_KEY_B64 production
vercel env add LICENSE_SIGNING_PUBLIC_KEY_B64 production
vercel env add ADMIN_API_TOKEN production
vercel --prod
```

Then point `api.thomssen.dev` at the deployment: in Vercel, add
`api.thomssen.dev` as a domain on this project, and in Hostinger's hPanel
DNS settings for thomssen.dev, add the CNAME record Vercel gives you
(typically `api` -> `cname.vercel-dns.com`). The main site's own DNS
records are untouched.

## Development mode

Everything above works identically against Stripe **test mode** so you
can run the full flow without spending real money:

1. `stripe listen --forward-to localhost:3000/api/stripe/webhook` (needs
   the [Stripe CLI](https://stripe.com/docs/stripe-cli)) - prints a
   `whsec_...` for your local `.env`.
2. `npm run dev` (runs `vercel dev`).
3. Visit `http://localhost:3000/buy`, pay with Stripe's test card
   `4242 4242 4242 4242`, any future expiry, any CVC.
4. Confirm: the webhook fires (check the `stripe listen` terminal), a row
   appears in Supabase, `/success?session_id=...` shows the key.
5. In the desktop app (`THOMSEN_SHIELD_LICENSE_API=http://localhost:3000`
   env var - see the Rust client's `license::client::base_url`), activate
   with that key.
6. Restart the app - it should still show as licensed (local token,
   verified offline, no network call).
7. Call `/api/license/validate` again (the app does this on its own after
   ~15s and then every 6h) and confirm `last_validated_at` updates in
   Supabase.
8. Use `/admin` (with your `ADMIN_API_TOKEN`) to reset the device, then
   confirm activation succeeds again from a "different" device id.

Keep production and development entirely separate: a different Supabase
project (or at least willingness to wipe test rows), Stripe test-mode
keys, and ideally a separate signing keypair, so nothing generated while
testing can ever be mistaken for a real customer's license.

## Email

Not implemented for v1 - the customer retrieves their key from the
`/success` page, which is the one thing that has to work. Stripe already
emails its own payment receipt automatically. To add license-key email
delivery later, the natural place is right after the `licenses` row is
created in `api/stripe/webhook.ts`'s `createLicenseForSession` - call
whatever provider you pick there; nothing else in this project needs to
change.

## Security notes

- `SUPABASE_SERVICE_ROLE_KEY` and `STRIPE_SECRET_KEY` exist only in this
  project's server-side environment variables. Nothing in
  `thomsen-shield/` (the desktop app) or any browser-facing code ever
  sees them.
- The webhook handler verifies Stripe's signature
  (`stripe.webhooks.constructEvent`) before doing anything else, against
  the *raw* request body - see `lib/raw-body.ts` for why that needs its
  own body-parsing opt-out.
- A license is only ever created from a `checkout.session.completed`
  event with `payment_status === "paid"`, and the unique constraint on
  `stripe_checkout_session_id` makes a retried webhook delivery a no-op
  instead of a second license.
- The `/success` page looks a license up only by Stripe's own
  unguessable `session_id` - never by email or any listing - so it can
  never show a stranger someone else's key.
