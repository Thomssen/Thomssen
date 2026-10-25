import type { VercelRequest, VercelResponse } from "@vercel/node";
import { supabaseAdmin } from "../../lib/supabase.js";
import { findLicenseByKey, statusErrorMessage } from "../../lib/license-repo.js";
import { isWellFormedLicenseKey } from "../../lib/license-key.js";
import { signToken } from "../../lib/signing.js";

interface ValidateBody {
  licenseKey?: unknown;
  deviceId?: unknown;
  // Accepted but not itself re-verified here - the server is the token's
  // *signer*; the signature check that matters is the client's own (see
  // `license::token` in the Rust client). What this endpoint actually
  // authorizes on is the licenseKey + deviceId binding below.
  token?: unknown;
}

/**
 * The periodic background check-in the Rust client's
 * `license::spawn_revalidation_loop` calls every few hours. A fresh,
 * further-out `expiresAt` on the returned token is what extends the
 * offline grace period - see `lib/signing.ts`.
 */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "POST") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  const body = req.body as ValidateBody;
  const licenseKey = body.licenseKey;
  const deviceId = body.deviceId;

  if (!isWellFormedLicenseKey(licenseKey) || typeof deviceId !== "string" || !deviceId) {
    res.status(400).json({ error: "Invalid request." });
    return;
  }

  const license = await findLicenseByKey(licenseKey);
  if (!license) {
    res.status(404).json({ error: "License key not found." });
    return;
  }

  const statusError = statusErrorMessage(license);
  if (statusError) {
    res.status(403).json({ error: statusError });
    return;
  }

  if (license.device_id !== deviceId) {
    res.status(409).json({ error: "This license is no longer active on this PC." });
    return;
  }

  const { error: updateError } = await supabaseAdmin().from("licenses").update({ last_validated_at: new Date().toISOString() }).eq("id", license.id);
  if (updateError) {
    console.error("license/validate: failed to record validation", updateError);
    // Still return a fresh token - a failed bookkeeping write is not a
    // reason to fail the device's revalidation and shrink its grace
    // period.
  }

  const token = signToken(license.license_key, deviceId);
  res.status(200).json({ token });
}
