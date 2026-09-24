import type { VercelRequest, VercelResponse } from "@vercel/node";
import { requireAdmin } from "../../lib/admin-auth.js";
import { supabaseAdmin } from "../../lib/supabase.js";

interface Body {
  id?: unknown;
}

/** Marks a license refunded - distinct from disable: a manual toggle you
 * might reverse, where a refund reflects a real Stripe refund and is the
 * status statusErrorMessage() (lib/license-repo.ts) already gives its own
 * "was refunded" message for. One-directional by design; use "Re-enable"
 * (admin/disable.ts) if a refund was marked by mistake. */
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

  const { error } = await supabaseAdmin().from("licenses").update({ status: "refunded" }).eq("id", body.id);
  if (error) {
    console.error("admin/refund failed", error);
    res.status(500).json({ error: "Update failed." });
    return;
  }

  res.status(200).json({ ok: true, status: "refunded" });
}
