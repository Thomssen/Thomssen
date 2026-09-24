import Stripe from "stripe";
import { requireEnv } from "./env.js";

let cached: Stripe | null = null;

export function stripeClient(): Stripe {
  if (!cached) {
    cached = new Stripe(requireEnv("STRIPE_SECRET_KEY"));
  }
  return cached;
}
