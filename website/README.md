# website/

Staged files for the main **thomssen.dev** site, which isn't checked into this
repo (only `thomsen-shield` and `thomsen-shield-license-api` live here). This
folder exists so the new pages have somewhere to be committed; copy its
contents into your actual site folder (`C:\Users\krist\Desktop\Thomsen Site`)
and delete this folder from the repo once merged, if you don't want the main
site tracked here.

## Files

- **`refund.html`** - Refund Policy + a "Request a Refund" form.
- **`terms.html`** - Terms of Service.
- **`privacy.html`** - Privacy Policy.
- **`legal.css`** - shared stylesheet for all three pages above (design tokens,
  layout, cards, buttons, form fields, the custom dropdown, footer links).
  Only `refund.html` needs the small inline `<script>` at the bottom of that
  file (the reason dropdown + the form's mailto submit handler); the other
  two pages have no JS.

All three match the dark near-black/white/grey design already used across
your other Thomsen pages (`thomsen-shield-license-api/public/{privacy,terms,refund}.html`)
and the shared tokens in `thomsen-shield/src/styles/tokens.css`.

## To install

1. Copy `refund.html`, `terms.html`, `privacy.html`, and `legal.css` into the
   root of your site folder, next to `index.html`.
2. Add links to them near your existing Privacy/Terms/Cookies links in the
   footer of `index.html` (and any other page with that footer), e.g.:

   ```html
   <a href="terms.html">Terms</a>
   <a href="privacy.html">Privacy</a>
   <a href="refund.html">Refund Policy</a>
   ```

   Match them to whatever markup/classes your existing footer links already
   use so they inherit the same styling - the snippet above is a plain
   fallback. The three pages already cross-link each other the same way in
   their own footers.
3. Open each page in a browser and click through. The back link points to
   `/` (root) rather than `index.html` so it doesn't add `/index.html` to
   the address bar - that only works if your host serves `index.html` as
   the default document at `/`, which is standard on virtually every static
   host. The three legal pages still cross-link each other with the `.html`
   extension (`terms.html`, `privacy.html`, `refund.html`); dropping those
   too needs a clean-URL rewrite rule at the host level (Vercel's
   `cleanUrls`, Netlify's `_redirects`, an `.htaccess` rule, etc.) that
   depends on where thomssen.dev is actually hosted, so it isn't done here.

## Editable policy values

No Terms, Privacy, or Refund policy currently exists anywhere in the
connected repo, so the copy on all three pages is placeholder wording sized
to be safe and easy to edit, not a finished legal document. Every important
value is wrapped in an HTML comment marker so Ctrl/Cmd+F finds every
occurrence within a given file:

| File            | Marker              | What it is                                | Occurrences |
| ---------------- | ------------------- | ------------------------------------------ | ----------- |
| refund.html      | `REFUND_WINDOW`      | Days customers can request a refund (`14`) | 3           |
| refund.html      | `REVIEW_TIME`        | How long you take to review a request      | 1           |
| refund.html      | `PROCESSING_TIME`    | How long an approved refund takes to land  | 1           |
| refund.html      | `SUPPORT_EMAIL`      | Contact address (`support@thomssen.dev`)   | 3           |
| terms.html       | `BUSINESS_OWNER`     | Legal name behind thomssen.dev             | 1           |
| terms.html       | `PAYMENT_PROCESSOR`  | Who processes payments (`Stripe`)          | 1           |
| terms.html       | `GOVERNING_LAW`      | Jurisdiction (`Denmark`)                   | 1           |
| terms.html       | `SUPPORT_EMAIL`      | Contact address                            | 1           |
| privacy.html     | `BUSINESS_OWNER`     | Legal name behind thomssen.dev             | 1           |
| privacy.html     | `SUPPORT_EMAIL`      | Contact address                            | 2           |
| all three        | `LAST_UPDATED`       | Date shown under the title                 | 1 each      |

`support@thomssen.dev`, `Kristian Thomsen`, `Stripe`, and `Denmark` were
reused as-is because they're the real values already live on your other
legal pages (`thomsen-shield-license-api/public/terms.html`). Everything
else - the refund window, review/processing time, and every bullet point -
is a placeholder. Read through and edit before publishing, especially
`privacy.html`'s "Who we share it with" section, which is deliberately
vague about hosting/analytics providers since those weren't confirmed for
the main site.

## The refund form

"Submit Refund Request" builds a pre-filled `mailto:` to `support@thomssen.dev`
with the form fields in the body - no backend required. If you'd rather have
submissions go to a real endpoint (e.g. `thomsen-shield-license-api`), replace
the handler in `refund.html`'s `<script>` block with a `fetch()` POST. The
"Reason for refund" field is a custom-styled dropdown (not a native
`<select>`) so its open state stays dark/on-brand on every OS instead of
falling back to a native popup that can render light regardless of theme.
