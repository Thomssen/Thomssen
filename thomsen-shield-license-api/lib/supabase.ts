import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import { requireEnv } from "./env.js";

let cached: SupabaseClient | null = null;

/**
 * The service-role client - full read/write access, bypassing row-level
 * security. This is exactly why it only ever runs here, server-side: the
 * key this authenticates with must never reach the desktop app or any
 * browser-side JavaScript. See the project README.
 */
export function supabaseAdmin(): SupabaseClient {
  if (!cached) {
    cached = createClient(requireEnv("SUPABASE_URL"), requireEnv("SUPABASE_SERVICE_ROLE_KEY"), {
      auth: { persistSession: false },
    });
  }
  return cached;
}

export const PRODUCT = "thomsen-shield";

export type LicenseStatus = "active" | "disabled" | "refunded";

export interface LicenseRow {
  id: string;
  license_key: string;
  email: string;
  product: string;
  status: LicenseStatus;
  device_id: string | null;
  device_activated_at: string | null;
  device_reset_count: number;
  stripe_checkout_session_id: string | null;
  stripe_payment_intent_id: string | null;
  created_at: string;
  activated_at: string | null;
  last_validated_at: string | null;
}
