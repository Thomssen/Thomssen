import type { VercelRequest, VercelResponse } from "@vercel/node";
import { supabaseAdmin } from "../../lib/supabase.js";
import { pageShell, escapeHtml } from "../../lib/page-shell.js";

/**
 * GET /api/purchase/success (aliased to /success - see vercel.json), the
 * Checkout success_url target: `?session_id={CHECKOUT_SESSION_ID}`.
 *
 * `session_id` is Stripe's own long, unguessable token, so looking a
 * license up *only* by that - never by email or any listing - is what
 * keeps this from ever exposing another customer's license key. A
 * session_id a stranger doesn't have is simply a 404 here.
 */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  const sessionId = typeof req.query.session_id === "string" ? req.query.session_id : null;
  res.setHeader("Content-Type", "text/html; charset=utf-8");

  if (!sessionId) {
    res.status(400).send(pageShell("Thomsen Shield", errorBody("Missing purchase reference.")));
    return;
  }

  const { data, error } = await supabaseAdmin()
    .from("licenses")
    .select("license_key, created_at")
    .eq("stripe_checkout_session_id", sessionId)
    .maybeSingle();

  if (error) {
    console.error("purchase/success lookup failed", error);
    res.status(500).send(pageShell("Thomsen Shield", errorBody("Something went wrong loading your license.")));
    return;
  }

  if (!data) {
    // The webhook usually lands within a second or two of the redirect
    // here, but never assume it beat the browser here - poll gently
    // rather than show a hard failure for what's almost always just a
    // few seconds of lag.
    const attempt = Number(req.query.attempt ?? "0");
    if (attempt < 10) {
      res.status(200).send(pageShell("Finishing up... - Thomsen Shield", processingBody(sessionId, attempt + 1)));
    } else {
      res.status(202).send(pageShell("Thomsen Shield", notReadyBody()));
    }
    return;
  }

  res.status(200).send(pageShell("Purchase Successful - Thomsen Shield", successBody(data.license_key)));
}

function successBody(licenseKey: string): string {
  const safeKey = escapeHtml(licenseKey);
  return `
<div class="panel">
  <span class="logo" aria-hidden="true">S</span>
  <span class="badge">Purchase Successful</span>
  <h1>Thomsen Shield</h1>
  <p class="lead">Your Lifetime License</p>

  <div class="card">
    <span class="key-label">License Key</span>
    <code class="key" id="license-key">${safeKey}</code>
    <hr class="rule" />
    <div class="meta-row"><span>Price</span><span>€30.00 Lifetime</span></div>
    <div class="meta-row"><span>Compatibility</span><span>Windows 10 / 11</span></div>
  </div>

  <div class="actions">
    <button class="btn btn-primary" id="copy-btn" onclick="copyKey()">Copy License</button>
    <a class="btn btn-secondary" href="https://thomssen.dev/thomsen-shield">Download Thomsen Shield</a>
  </div>

  <p class="hint">Keep this key somewhere safe - open Thomsen Shield, go to Activate License, and paste it in. A copy of this page's details is not emailed automatically yet; save it now.</p>
  <p class="hint"><a href="/refund.html">Refund policy</a> &middot; <a href="/terms.html">Terms</a> &middot; <a href="/privacy.html">Privacy</a> &middot; <a href="mailto:support@thomssen.dev">support@thomssen.dev</a></p>
</div>
<script>
  function copyKey() {
    var key = document.getElementById('license-key').textContent;
    navigator.clipboard.writeText(key).then(function () {
      var btn = document.getElementById('copy-btn');
      var original = btn.textContent;
      btn.textContent = 'Copied';
      setTimeout(function () { btn.textContent = original; }, 1600);
    });
  }
</script>`;
}

function processingBody(sessionId: string, nextAttempt: number): string {
  return `
<div class="panel">
  <span class="logo" aria-hidden="true">S</span>
  <div class="spinner" aria-hidden="true"></div>
  <h1>Finishing up</h1>
  <p class="lead">Confirming your payment - this only takes a few seconds.</p>
</div>
<meta http-equiv="refresh" content="1.5;url=/api/purchase/success?session_id=${encodeURIComponent(sessionId)}&attempt=${nextAttempt}" />`;
}

function notReadyBody(): string {
  return `
<div class="panel">
  <span class="logo" aria-hidden="true">S</span>
  <h1>Almost there</h1>
  <p class="lead">Your payment is still being confirmed. Refresh this page in a minute, or check the email from Stripe for your receipt - if this persists, contact support and mention your payment confirmation email.</p>
</div>`;
}

function errorBody(message: string): string {
  return `
<div class="panel">
  <span class="logo" aria-hidden="true">S</span>
  <h1>Something went wrong</h1>
  <p class="lead">${escapeHtml(message)}</p>
</div>`;
}
