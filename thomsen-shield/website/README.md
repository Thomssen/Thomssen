# Thomsen Shield - website section

A drop-in product section for thomssen.dev, in the same structure as your
Thomsen Web Scanner section (image left, info right, CTA, compatibility
line, divider, feature pills) but built as a paid product.

## Why this exists as a standalone file

This session runs in a cloud sandbox, not on your Windows machine, so it
has no way to open `C:\Users\krist\Desktop\Thomsen Site` and edit your
actual site files directly. `thomsen-shield-section.html` is reconstructed
from the same design tokens your other Thomsen apps share (see
`thomsen-vm/src/styles/tokens.css`), not copied from your site's real CSS -
it should already look like the same family, but if you paste me your
site's actual HTML/CSS for the Web Scanner section, I can match it exactly
instead of reconstructing it.

## Using it

1. Open `thomsen-shield-section.html` directly in a browser to preview it.
2. Copy everything between the `COPY FROM HERE` / `COPY TO HERE` comments
   into your site, where the Web Scanner section lives.
3. Copy `assets/thomsen-shield-dashboard.png` alongside it (a real
   screenshot of the app's Dashboard, not a mockup).
4. The Buy button already points at `https://api.thomssen.dev/buy` - the
   real Stripe Checkout redirect built in `thomsen-shield-license-api/`.
   It 404s until that's deployed; nothing to change here once it is.

## If you want an exact match instead

Every rule in the section is scoped under `.ts-shield` using CSS custom
properties (`--ts-*`) so it can't collide with your site's existing
styles. If your site already defines the same near-black/white/grey
tokens at `:root`, you can delete the `:root`-equivalent block inside
`.ts-shield` and let the section inherit your site's own values instead.
