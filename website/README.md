# website/

The Thomsen Shield product site (`index.html` + the legal pages), staged
here because the connected repo has no other home for it - it isn't
checked in anywhere else, and this session can't reach your local
`C:\Users\krist\Desktop\Thomsen Site` or the live `thomssen.dev` to edit
either directly. Copy this folder's contents into wherever thomssen.dev is
actually served from.

This is a full rebuild of the homepage around Thomsen Shield as the main
product (not a personal-portfolio page with a product section) - built
fresh rather than adapted from any prior homepage content, per instruction.

## Files

- **`index.html`** - the homepage: nav, hero, "how it works", features,
  a protection/status showcase, a screenshot gallery, pricing, FAQ, and
  the footer. Also owns the cookie banner and its consent JS.
- **`refund.html`**, **`terms.html`**, **`privacy.html`** - legal pages,
  now written specifically for Thomsen Shield (not a generic multi-product
  site) using the same figures already live at
  `thomsen-shield-license-api/public/{refund,terms,privacy}.html`
  ($19.99 one-time, 7-day refund, Stripe, Denmark law). Keep both copies
  in sync if a figure changes.
- **`styles.css`** - shared tokens, reset, buttons, form fields, and the
  custom dropdown, used by all four pages.
- **`home.css`** - homepage-only components (nav, hero, feature grid,
  protection panel, gallery, pricing card, FAQ accordion, footer, cookie
  banner). Only `index.html` links this file.
- **`assets/thomsen-shield-dashboard.png`** - a real screenshot of the
  app's Dashboard (copied from `thomsen-shield/website/assets/`), used in
  the hero and the screenshot gallery.

All four pages match the dark near-black/white/grey design used across
the other Thomsen apps (see `thomsen-shield/src/styles/tokens.css`) - no
colors beyond the existing muted status green/amber/red, no external
fonts or JS libraries.

## To install

Copy everything in this folder (including `assets/`) into the root of
your site folder, preserving the relative paths - `index.html` expects
`styles.css`, `home.css`, and `assets/thomsen-shield-dashboard.png`
alongside it.

## Buy / download

Every "Buy" button (nav, hero, pricing, final CTA) points at
`https://api.thomssen.dev/buy` - the real Stripe Checkout redirect built
in `thomsen-shield-license-api/`, per `thomsen-shield/website/README.md`.
It 404s until that endpoint is deployed; nothing to change here once it
is. There's no separate free-download link - Shield's own README and the
existing product section only ever show a single Buy CTA, so this follows
that same pattern rather than inventing an unverified download URL.

## Screenshot gallery

The gallery section (`#gallery`) has one real screenshot and two
placeholder tiles (`.gallery__placeholder`) marked "coming soon" for
Live Connections and Firewall. To fill one in, copy a
`.gallery__tile` block (see the Dashboard tile in `index.html`) in place
of a placeholder, point its `<img src>` at a new file in `assets/`, and
write accurate `alt` text.

## Cookie banner

There was no existing cookie banner in the connected project to carry
over (this session never had access to your live site), so the one in
`index.html`/`home.css` is a fresh build in the same visual language as
the rest of the site: a translucent, blurred card, pill buttons, bottom
corner placement. It stores the visitor's choice in `localStorage`
(`thomsen-shield-cookie-consent`) - no cookie-consent library. If your
real site already had a banner with specific legal wording, swap this
one's copy in `index.html` for that instead.

## What's still a placeholder

Only `LAST_UPDATED` on the three legal pages has no other source of
truth - update the date whenever a page's wording changes materially.
Everything else (pricing, refund window, features, requirements) is
pulled from `thomsen-shield/README.md`, its Terms/Privacy/Refund pages,
and the real Dashboard screenshot, not invented.
