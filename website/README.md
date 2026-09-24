# website/

Staged files for the main **thomssen.dev** site, which isn't checked into this
repo (only `thomsen-shield` and `thomsen-shield-license-api` live here). This
folder exists so the new page has somewhere to be committed; copy its
contents into your actual site folder (`C:\Users\krist\Desktop\Thomsen Site`)
and delete this folder from the repo once merged, if you don't want the main
site tracked here.

## refund.html

Dedicated Refund Policy + refund request page. Self-contained (inline
`<style>`/`<script>`, no external dependencies), built to match the dark
near-black/white/grey design already used across your other Thomsen pages
(`thomsen-shield-license-api/public/{privacy,terms,refund}.html`) and the
shared tokens in `thomsen-shield/src/styles/tokens.css`.

### To install it

1. Copy `refund.html` into the root of your site folder, next to `index.html`.
2. Add a link to it near your existing Privacy/Terms/Cookies links in the
   footer of `index.html` (and any other page with that footer), e.g.:

   ```html
   <a href="refund.html">Refund Policy</a>
   ```

   Match it to whatever markup/classes your existing footer links already use
   so it inherits the same styling - the snippet above is a plain fallback.
3. Open `refund.html` in a browser and click through: the back link, the
   in-page "Request a Refund" anchor, and the return button all point to
   relative paths (`index.html`, `terms.html`, `privacy.html`, `refund.html`).
   Rename the hrefs if your files use different names.

### Editable policy values

No refund policy currently exists anywhere in the connected repo, so the
copy is placeholder wording sized to be safe, not a finished legal document.
Every important number/address is wrapped in an HTML comment marker so
Ctrl/Cmd+F finds every occurrence in the file:

| Marker             | What it is                                   | Occurrences |
| ------------------ | --------------------------------------------- | ----------- |
| `REFUND_WINDOW`     | Days customers can request a refund (`14`)    | 3           |
| `REVIEW_TIME`       | How long you take to review a request         | 1           |
| `PROCESSING_TIME`   | How long an approved refund takes to land     | 1           |
| `SUPPORT_EMAIL`     | Contact address (`support@thomssen.dev`)      | 3           |
| `LAST_UPDATED`      | Date shown under the title                    | 1           |

`support@thomssen.dev` was reused as-is because it's the real address already
live on your other legal pages; everything else (the 14-day window, review
time, processing time, and the eligibility/non-refundable bullet points) is a
placeholder - edit it to match your actual policy before publishing.

### The form

"Submit Refund Request" builds a pre-filled `mailto:` to `support@thomssen.dev`
with the form fields in the body - no backend required. If you'd rather have
submissions go to a real endpoint (e.g. `thomsen-shield-license-api`), replace
the handler in the page's `<script>` block with a `fetch()` POST.
