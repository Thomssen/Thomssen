import { supabaseAdmin, type LicenseRow } from "./supabase.js";

export async function findLicenseByKey(licenseKey: string): Promise<LicenseRow | null> {
  const { data, error } = await supabaseAdmin().from("licenses").select("*").eq("license_key", licenseKey).maybeSingle();
  if (error) throw error;
  return data;
}

/** `null` means the license's status is fine to proceed with; otherwise
 * the message to return to the client. */
export function statusErrorMessage(license: LicenseRow): string | null {
  if (license.status === "disabled") return "This license has been disabled. Contact support if you believe this is a mistake.";
  if (license.status === "refunded") return "This license was refunded and is no longer valid.";
  return null;
}
