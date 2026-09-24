import type { VercelRequest, VercelResponse } from "@vercel/node";
import { requireAdmin } from "../../lib/admin-auth.js";
import { supabaseAdmin } from "../../lib/supabase.js";

interface Body {
  id?: unknown;
}

/** Clears a license's device binding so the customer can activate it on a
 * new PC - the manual reset path the spec asks for, for a customer who's
 * upgraded or reinstalled and can't deactivate it themselves. */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "POST") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }
  if (!requireAdmin(req, res)) return;

  const body = req.body as Body;
  if (typeof body.id !== "string") {
    res.status(400).json({ error: "Missing license id." });
    return;
  }

  const { data: current, error: fetchError } = await supabaseAdmin().from("licenses").select("device_reset_count").eq("id", body.id).maybeSingle();
  if (fetchError || !current) {
    res.status(404).json({ error: "License not found." });
    return;
  }

  const { error } = await supabaseAdmin()
    .from("licenses")
    .update({ device_id: null, device_activated_at: null, device_reset_count: current.device_reset_count + 1 })
    .eq("id", body.id);

  if (error) {
    console.error("admin/reset-device failed", error);
    res.status(500).json({ error: "Update failed." });
    return;
  }

  res.status(200).json({ ok: true });
}
