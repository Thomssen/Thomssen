import type { VercelRequest, VercelResponse } from "@vercel/node";
import { stripeClient } from "../../lib/stripe.js";
import { optionalEnv } from "../../lib/env.js";

/**
 * GET /api/stripe/checkout (aliased to /buy - see vercel.json).
 *
 * The website's "Purchase" button (€30.00 Lifetime) can be a plain
 * `<a href>` to this URL - no client-side JavaScript needed on a static
 * site. This creates a Checkout Session and immediately redirects to
 * Stripe's own hosted page.
 *
 * The price is defined inline (price_data) rather than via a stored
 * Price/STRIPE_PRICE_ID: Stripe's Managed Payments, on by default for new
 * accounts, hard-requires a product tax code on every line item regardless
 * of automatic_tax - and that code lives on the Product object, which is
 * entirely separate between test and live mode. A stored Price makes that
 * one more dashboard step to get right in the correct mode; defining the
 * tax code (and the €30.00 itself, already fixed everywhere else in this
 * product) here means it's correct in both modes with nothing to configure
 * in Stripe at all. tax_code txcd_10000000 = "General - Electronically
 * Supplied Services", tax_behavior "inclusive" so the customer always pays
 * exactly €30.00 regardless of jurisdiction and Stripe (merchant of record
 * under Managed Payments) carves the applicable tax out of that.
 */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "GET") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  try {
    const session = await stripeClient().checkout.sessions.create({
      mode: "payment",
      line_items: [
        {
          price_data: {
            currency: "eur",
            unit_amount: 3000,
            tax_behavior: "inclusive",
            product_data: {
              name: "Thomsen Shield - Lifetime License",
              tax_code: "txcd_10000000",
            },
          },
          quantity: 1,
        },
      ],
      success_url: optionalEnv("CHECKOUT_SUCCESS_URL") ?? "https://api.thomssen.dev/success?session_id={CHECKOUT_SESSION_ID}",
      cancel_url: optionalEnv("CHECKOUT_CANCEL_URL") ?? "https://thomssen.dev/",
      // Stripe emails a receipt automatically; the license key itself is
      // handed over on the success page (session_id is an unguessable,
      // single-use-in-practice token - see api/purchase/success.ts) and
      // is deliberately not duplicated into Checkout's own metadata here.
      customer_creation: "always",
      // Shows a promo-code field on Checkout - lets a 100%-off code (made
      // in the Stripe Dashboard, Product catalog -> Coupons) exercise this
      // exact real flow for $0, e.g. to verify production end to end
      // without a real charge. No effect for a customer who doesn't have
      // a code; nothing here grants a discount by itself.
      allow_promotion_codes: true,
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
