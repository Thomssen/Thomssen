import type { VercelRequest, VercelResponse } from "@vercel/node";
import { stripeClient } from "../../lib/stripe.js";
import { requireEnv, optionalEnv } from "../../lib/env.js";

/**
 * GET /api/stripe/checkout (aliased to /buy - see vercel.json).
 *
 * The website's "Buy - $19.99 Lifetime" button can be a plain `<a href>`
 * to this URL - no client-side JavaScript needed on a static site. This
 * creates a Checkout Session and immediately redirects to Stripe's own
 * hosted page.
 */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "GET") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  try {
    const session = await stripeClient().checkout.sessions.create({
      mode: "payment",
      line_items: [{ price: requireEnv("STRIPE_PRICE_ID"), quantity: 1 }],
      success_url: optionalEnv("CHECKOUT_SUCCESS_URL") ?? "https://api.thomssen.dev/success?session_id={CHECKOUT_SESSION_ID}",
      cancel_url: optionalEnv("CHECKOUT_CANCEL_URL") ?? "https://thomssen.dev/",
      // Stripe emails a receipt automatically; the license key itself is
      // handed over on the success page (session_id is an unguessable,
      // single-use-in-practice token - see api/purchase/success.ts) and
      // is deliberately not duplicated into Checkout's own metadata here.
      customer_creation: "always",
    });

    if (!session.url) {
      res.status(502).json({ error: "Stripe didn't return a checkout URL." });
      return;
    }
    res.redirect(303, session.url);
  } catch (err) {
    console.error("stripe/checkout failed", err);
    res.status(502).json({ error: "Couldn't start checkout. Please try again." });
  }
}
