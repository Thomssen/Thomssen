import Stripe from "stripe";
import { requireEnv } from "./env.js";

let cached: Stripe | null = null;

export function stripeClient(): Stripe {
  if (!cached) {
    cached = new Stripe(requireEnv("STRIPE_SECRET_KEY"), {
      // Pinned explicitly rather than left to whatever the installed SDK
      // defaults to: Managed Payments (see api/stripe/checkout.ts) needs
      // 2025-03-31.basil or later, and a future `npm update` bumping this
      // package shouldn't be able to silently change which API version
      // every request uses. Matches the account's own webhook destination
      // version already, so nothing else needs to change in step.
      apiVersion: "2026-08-26.dahlia",
    });
  }
  return cached;
}
