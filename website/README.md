# website/

The Thomsen Shield product site, staged here because the connected repo has
no other home for it — copy this folder's contents into wherever
thomssen.dev is actually served from, preserving the folder structure
(everything shared lives under `assets/`, matching the real site's layout).

## Why this looks the way it does

This was rebuilt against your **actual** thomssen.dev `index.html` and
`assets/styles.css` (pasted into this session directly, since neither the
live site nor your local disk are reachable from here). Every structural
piece — the pill-shaped floating nav with its mobile burger menu, the
dark/light theme toggle, the button system, the footer, and the cookie
consent banner + settings modal — is copied from your real classes and
CSS almost verbatim. The Thomsen Shield content (hero, features, pricing,
FAQ, protection panel) is new, per the earlier instruction not to reuse
the old homepage's actual copy.

## Files

- **`index.html`** — the homepage: nav, hero, "how it works", features,
  pricing, FAQ, and the footer.
- **`refund.html`**, **`terms.html`**, **`privacy.html`** — legal pages,
  same nav/footer/cookie system, content matching the figures already
  live at `thomsen-shield-license-api/public/{refund,terms,privacy}.html`
  (€50.00 one-time, 7-day refund, Stripe, Denmark law).
- **`thomsen-shield/index.html`** — the post-purchase download page. It's
  a folder with an `index.html` inside, not a `thomsen-shield.html` file,
  so the existing "Download Thomsen Shield" link on the Stripe success
  page (`https://thomssen.dev/thomsen-shield`, no extension - see
  `thomsen-shield-license-api/api/purchase/success.ts`) resolves without
  needing a clean-URL rewrite. Its nav/footer links use root-relative
  paths (`/index.html`, `/assets/main2.css`, ...) rather than the relative
  ones the top-level pages use, since it's one folder deeper. The
  "Download for Windows" button is a placeholder (`href="#"`, clearly
  marked in the file) since no installer is published yet - clicking it
  shows a message pointing to `support@thomssen.dev` instead of doing
  nothing. Swap in the real installer URL there once one exists.
- **`assets/main2.css`** — one stylesheet for the whole site. Sections
  1–13 are your real site's CSS, unmodified, so this file is a drop-in
  superset of your actual `assets/styles.css` (nothing here would break
  your existing `apps.html` or other pages if you swap it in — it adds
  Shield's components in a clearly marked section 14, on top of your file
  rather than instead of it). Both dark and light themes are supported,
  same as the real site. Named `main2.css` (not `styles.css`, not even
  `main.css` anymore) - see "A caching gotcha" below for why.
- **`assets/site.js`** — theme toggle, mobile nav burger, footer year,
  and the FAQ accordion.
- **`assets/consent.js`** — builds the cookie banner and the "Cookie
  Settings" modal (Essential/Analytics toggle) and stores the choice in
  `localStorage` under `thomsen-consent`. No third-party consent library.
- **`assets/thomsen-shield-dashboard.png`** — a real screenshot of the
  app's Dashboard. Not currently referenced by any page (removed from an
  earlier hero/gallery draft to declutter), kept here in case you want to
  use it somewhere later.

## To install

Copy everything in this folder into the root of your site, keeping
`assets/` as a subfolder — `index.html` (and the legal pages) expect
`assets/main2.css`, `assets/site.js`, and `assets/consent.js` alongside
it, exactly like your real site's own pages.

If you're merging this into your existing site rather than replacing it
outright: your real `assets/styles.css` sections 1–13 are reproduced
unchanged here, so you can most likely just replace your file with this
one and add these new HTML pages — the classes `.product`, `.apps-hero`,
etc. from your real Apps page are preserved too.

## Buy

Every "Purchase" button (nav, hero, pricing, final CTA) points at
`https://api.thomssen.dev/buy` — the real Stripe Checkout redirect built
in `thomsen-shield-license-api/`. It 404s until that endpoint is
deployed; nothing to change here once it is.

## Editable values

Only `LAST_UPDATED` on the three legal pages has no other source of
truth — update the date whenever a page's wording changes materially.
Everything else (pricing, refund window, features, requirements) is
pulled from `thomsen-shield/README.md`, its real Terms/Privacy/Refund
pages, and the real Dashboard screenshot — not invented.

## A caching gotcha (Hostinger)

This stylesheet has already been renamed once for a real reason, not a
style choice: on this Hostinger (hPanel) account, updates to
`assets/styles.css` stopped taking effect at some point - the file on
disk was confirmed correct (right size, right content, no hidden
characters), but the live URL kept serving an old version missing entire
CSS rules, even after cache-clearing, incognito testing, and ruling out
a CDN. Renaming it to `assets/main.css` fixed it immediately.

It then happened a **second time** to `main.css` itself: after several
further edits, the live site kept showing an old layout (wrong card
order, wrong sizing) even though the HTML on the same pages was clearly
up to date, and the CSS on disk in this repo was correct. Same fix -
renamed again, this time to `assets/main2.css` (the name live now, and
the one every page's `<link rel="stylesheet">` points to).

This points to some cache keyed to the exact file path, on a layer that
has never shown up in hPanel's visible settings (not the cache manager,
not LiteSpeed Cache, nothing found), and it can apparently recur on a
name that's already been "fixed" once. If a future edit to
`assets/main2.css` doesn't show up live after a hard refresh **and**
after confirming the uploaded file's content is actually correct, this
is almost certainly the same thing again - rename the file once more
(update the five `<link>` tags that reference it: `index.html`,
`terms.html`, `privacy.html`, `refund.html`, `thomsen-shield/index.html`)
rather than debugging further. Given it's now recurred twice, it's
probably worth raising with Hostinger support directly - it's
server-side behavior no one has been able to pin down from outside
their infrastructure.

## Fonts

Like the real site, these pages load Inter from Google Fonts. If you'd
rather self-host it or drop the external request, remove the two
`<link rel="preconnect">` tags and the `fonts.googleapis.com` stylesheet
link from each page's `<head>` — the `--font-sans` stack already falls
back to system fonts if Inter fails to load.
