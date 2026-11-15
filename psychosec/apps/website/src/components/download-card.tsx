import { latestRelease } from "@/lib";

export async function DownloadCard() {
  const r = await latestRelease();
  return (
    <div className="rounded-lg border border-line bg-panel p-6">
      <div className="text-xs uppercase tracking-wider text-dim">Windows 10 / 11 · 64-bit</div>
      {r ? (
        <>
          <div className="mt-2 text-2xl font-semibold">Version {r.version}</div>
          <div className="mt-1 text-sm text-dim">Released {new Date(r.published_at).toLocaleDateString(undefined, { dateStyle: "long" })}</div>
          <a href={r.download_url} className="mt-5 inline-block rounded-md border border-accent/40 bg-accent/10 px-5 py-2.5 text-sm font-medium text-accent hover:bg-accent/20">Download Psychosec-Setup-v{r.version}.exe</a>
          <p className="mt-4 break-all font-mono text-xs text-dim">SHA-256 {r.checksum_sha256}</p>
          {r.notes && <p className="mt-3 whitespace-pre-line text-sm text-dim">{r.notes}</p>}
        </>
      ) : (
        <p className="mt-3 text-sm text-dim">No release is published yet. Check back soon.</p>
      )}
    </div>
  );
}
