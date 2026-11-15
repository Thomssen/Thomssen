export const API = process.env.NEXT_PUBLIC_API_URL ?? "https://api.psychosec.org";
export interface Release { version: string; download_url: string; checksum_sha256: string; published_at: string; notes: string | null }
/** Latest release metadata from the API; null when none is published or the API is unreachable. */
export async function latestRelease(): Promise<Release | null> {
  try {
    const r = await fetch(`${API}/api/client/version`, { next: { revalidate: 60 } });
    if (!r.ok) return null;
    return ((await r.json()) as { release: Release | null }).release;
  } catch { return null; }
}
