import type { VercelRequest, VercelResponse } from "@vercel/node";
import { requireAdmin } from "../../lib/admin-auth.js";
import { supabaseAdmin } from "../../lib/supabase.js";

interface Body {
  id?: unknown;
  /** Toggle: pass `reenable: true` to set the license back to active -
   * one endpoint for both directions rather than two near-identical ones. */
  reenable?: unknown;
}

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

  const status = body.reenable === true ? "active" : "disabled";
  const { error } = await supabaseAdmin().from("licenses").update({ status }).eq("id", body.id);
  if (error) {
    console.error("admin/disable failed", error);
    res.status(500).json({ error: "Update failed." });
    return;
  }

  res.status(200).json({ ok: true, status });
}
