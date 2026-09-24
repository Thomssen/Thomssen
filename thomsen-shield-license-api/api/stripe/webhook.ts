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

  if (event.type !== "checkout.session.completed") {
    // Acknowledge anything we don't act on so Stripe stops retrying it.
    res.status(200).json({ received: true, ignored: event.type });
    return;
  }

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
