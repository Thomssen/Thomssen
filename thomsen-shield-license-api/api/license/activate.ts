import type { VercelRequest, VercelResponse } from "@vercel/node";
import { supabaseAdmin } from "../../lib/supabase.js";
import { findLicenseByKey, statusErrorMessage } from "../../lib/license-repo.js";
import { isWellFormedLicenseKey } from "../../lib/license-key.js";
import { signToken } from "../../lib/signing.js";

interface ActivateBody {
  licenseKey?: unknown;
  deviceId?: unknown;
}

export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "POST") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  const body = req.body as ActivateBody;
  const licenseKey = body.licenseKey;
  const deviceId = body.deviceId;

  if (!isWellFormedLicenseKey(licenseKey)) {
    res.status(400).json({ error: "That license key doesn't look right. Check it and try again." });
    return;
  }
  if (typeof deviceId !== "string" || deviceId.length < 8 || deviceId.length > 128) {
    res.status(400).json({ error: "Missing or invalid device identifier." });
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

  if (license.device_id && license.device_id !== deviceId) {
    res.status(409).json({
      error: "This license is already active on another PC. Deactivate it there first, or contact support to reset it.",
    });
    return;
  }

  const now = new Date().toISOString();
  const baseUpdate = supabaseAdmin()
    .from("licenses")
    .update({
      device_id: deviceId,
      device_activated_at: now,
      activated_at: license.activated_at ?? now,
      last_validated_at: now,
    })
    .eq("id", license.id);
  // Compare-and-swap against the device_id we just read (null, or already
  // this same device - the only two cases that reach here, see the 409
  // above) rather than an unconditional update: without this, two activate
  // calls racing on the same never-yet-bound license could both pass the
  // check above and both walk away with a validly-signed token for one
  // purchase. Whichever write loses the race now matches zero rows instead
  // of silently overwriting the winner's binding.
  const { data: updated, error: updateError } = await (license.device_id === null ? baseUpdate.is("device_id", null) : baseUpdate.eq("device_id", license.device_id)).select(
    "id",
  );

  if (updateError) {
    console.error("license/activate: failed to record activation", updateError);
    res.status(500).json({ error: "Couldn't save the activation. Please try again." });
    return;
  }

  if (!updated || updated.length === 0) {
    res.status(409).json({
      error: "This license is already active on another PC. Deactivate it there first, or contact support to reset it.",
    });
    return;
  }

  const token = signToken(license.license_key, deviceId);
  res.status(200).json({ token });
}
