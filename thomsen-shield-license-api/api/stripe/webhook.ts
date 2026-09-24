import type { VercelRequest, VercelResponse } from "@vercel/node";
import type Stripe from "stripe";
import { stripeClient } from "../../lib/stripe.js";
import { requireEnv } from "../../lib/env.js";
import { readRawBody } from "../../lib/raw-body.js";
import { supabaseAdmin, PRODUCT } from "../../lib/supabase.js";
import { generateLicenseKey } from "../../lib/license-key.js";

// Required so Vercel hands this handler the untouched request stream
// instead of a pre-parsed body - see lib/raw-body.ts.
export const config = { api: { bodyParser: false } };

export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "POST") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  const signature = req.headers["stripe-signature"];
  if (!signature || Array.isArray(signature)) {
    res.status(400).json({ error: "Missing Stripe-Signature header." });
    return;
  }

  const rawBody = await readRawBody(req);

  // The one non-negotiable line in this whole file: nothing below this
  // point runs on a payload we haven't cryptographically confirmed came
  // from Stripe, signed with our specific webhook secret.
  let event: Stripe.Event;
  try {
    event = stripeClient().webhooks.constructEvent(rawBody, signature, requireEnv("STRIPE_WEBHOOK_SECRET"));
  } catch (err) {
    console.error("stripe/webhook signature verification failed", err);
    res.status(400).json({ error: "Invalid signature." });
    return;
  }

  if (event.type === "checkout.session.completed") {
    const session = event.data.object as Stripe.Checkout.Session;

    // "Only create the license after a confirmed successful payment" - the
    // event type alone isn't that confirmation for every payment method,
    // `payment_status` is.
    if (session.payment_status !== "paid") {
      res.status(200).json({ received: true, ignored: "not paid yet" });
      return;
    }

    const email = session.customer_details?.email ?? session.customer_email;
    if (!email) {
      console.error("stripe/webhook: checkout session has no email", session.id);
      res.status(200).json({ received: true, error: "missing email" });
      return;
    }

    const paymentIntentId = typeof session.payment_intent === "string" ? session.payment_intent : (session.payment_intent?.id ?? null);

    try {
      await createLicenseForSession(session.id, email, paymentIntentId);
      res.status(200).json({ received: true });
    } catch (err) {
      console.error("stripe/webhook: failed to record license", err);
      // A 5xx tells Stripe to retry - correct here, since the row was never
      // written and the unique constraint means a retry is always safe.
      res.status(500).json({ error: "Failed to record the license." });
    }
    return;
  }

  if (event.type === "charge.refunded") {
    try {
      await handleChargeRefunded(event.data.object as Stripe.Charge);
      res.status(200).json({ received: true });
    } catch (err) {
      console.error("stripe/webhook: failed to process refund", err);
      // 5xx so Stripe retries - handleChargeRefunded is a plain status
      // update keyed on payment_intent, safe to run again from scratch.
      res.status(500).json({ error: "Failed to process the refund." });
    }
    return;
  }

  // Acknowledge anything we don't act on so Stripe stops retrying it.
  res.status(200).json({ received: true, ignored: event.type });
}

async function createLicenseForSession(sessionId: string, email: string, paymentIntentId: string | null): Promise<void> {
  const supabase = supabaseAdmin();

  // Idempotency against duplicate webhook delivery: `stripe_checkout_session_id`
  // is UNIQUE (see supabase/schema.sql), and `ignoreDuplicates` turns this
  // into `INSERT ... ON CONFLICT (stripe_checkout_session_id) DO NOTHING`.
  // A second delivery of the same event simply no-ops instead of minting
  // a second license key for one payment.
  const { error } = await supabase
    .from("licenses")
    .upsert(
      {
        license_key: generateLicenseKey(),
        email,
        product: PRODUCT,
        status: "active",
        stripe_checkout_session_id: sessionId,
        stripe_payment_intent_id: paymentIntentId,
      },
      { onConflict: "stripe_checkout_session_id", ignoreDuplicates: true },
    );

  if (error) throw error;
}

/**
 * `charge.refunded` fires on every refund against a charge, partial or
 * full - `charge.refunded` (the boolean field, same name as the event) is
 * only true once the charge's *total* refunded amount equals its original
 * amount. A partial refund (a goodwill gesture that isn't meant to pull
 * the license) fires this same event with that field false; nothing to do
 * for those here.
 *
 * Looked up by `stripe_payment_intent_id` - the payment reference already
 * stored on the license row from `createLicenseForSession` above, not a
 * new field. Every step here is naturally idempotent: setting a status
 * that's already set is a no-op, so a Stripe redelivery of the same event
 * (or the rare double-delivery of two related refund events for one
 * charge) can never double-apply anything or create a second row.
 */
async function handleChargeRefunded(charge: Stripe.Charge): Promise<void> {
  if (!charge.refunded) {
    console.log("stripe/webhook: partial refund, license unaffected", charge.id);
    return;
  }

  const paymentIntentId = typeof charge.payment_intent === "string" ? charge.payment_intent : (charge.payment_intent?.id ?? null);
  if (!paymentIntentId) {
    console.warn("stripe/webhook: charge.refunded had no payment_intent", charge.id);
    return;
  }

  const supabase = supabaseAdmin();
  const { data: license, error: fetchError } = await supabase.from("licenses").select("id, status").eq("stripe_payment_intent_id", paymentIntentId).maybeSingle();
  if (fetchError) throw fetchError;

  if (!license) {
    // Not necessarily wrong - a refund on a charge this table never had a
    // row for (e.g. a duplicate/rare payment method edge case) has nothing
    // here to revoke either way.
    console.warn("stripe/webhook: no license found for refunded payment_intent", paymentIntentId);
    return;
  }

  if (license.status === "refunded") {
    console.log("stripe/webhook: license already refunded, skipping", license.id);
    return;
  }

  const { error: updateError } = await supabase.from("licenses").update({ status: "refunded" }).eq("id", license.id);
  if (updateError) throw updateError;

  console.log("stripe/webhook: license marked refunded", license.id);
}
