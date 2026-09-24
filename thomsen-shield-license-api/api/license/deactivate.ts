import type { VercelRequest, VercelResponse } from "@vercel/node";
import { supabaseAdmin } from "../../lib/supabase.js";
import { findLicenseByKey } from "../../lib/license-repo.js";
import { isWellFormedLicenseKey } from "../../lib/license-key.js";

interface DeactivateBody {
  licenseKey?: unknown;
  deviceId?: unknown;
}

/** Customer-initiated "free up this license" (e.g. before selling or
 * reinstalling this PC) - clears the device binding so the same license
 * can be activated elsewhere. Idempotent: deactivating a license that
 * isn't bound to this device is a no-op, not an error. */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "POST") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }

  const body = req.body as DeactivateBody;
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

  if (license.device_id === deviceId) {
    const { error } = await supabaseAdmin().from("licenses").update({ device_id: null, device_activated_at: null }).eq("id", license.id);
    if (error) {
      console.error("license/deactivate: failed to clear device binding", error);
      res.status(500).json({ error: "Couldn't deactivate. Please try again." });
      return;
    }
  }

  res.status(200).json({ ok: true });
}
