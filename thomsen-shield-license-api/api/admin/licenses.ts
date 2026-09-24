import type { VercelRequest, VercelResponse } from "@vercel/node";
import { requireAdmin } from "../../lib/admin-auth.js";
import { supabaseAdmin } from "../../lib/supabase.js";

/** GET /api/admin/licenses?q=<email or license key substring>
 * Protected by the admin bearer token - see lib/admin-auth.ts. Powers the
 * simple admin tool at /admin (public/admin.html). */
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (req.method !== "GET") {
    res.status(405).json({ error: "Method not allowed." });
    return;
  }
  if (!requireAdmin(req, res)) return;

  const q = typeof req.query.q === "string" ? req.query.q.trim() : "";

  let query = supabaseAdmin()
    .from("licenses")
    .select("id, license_key, email, status, device_id, device_reset_count, created_at, activated_at, last_validated_at")
    .order("created_at", { ascending: false })
    .limit(100);

  if (q) {
    query = query.or(`email.ilike.%${q}%,license_key.ilike.%${q}%`);
  }

  const { data, error } = await query;
  if (error) {
    console.error("admin/licenses query failed", error);
    res.status(500).json({ error: "Query failed." });
    return;
  }

  res.status(200).json({ licenses: data });
}
