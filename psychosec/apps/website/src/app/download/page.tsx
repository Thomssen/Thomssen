import { DownloadCard } from "@/components/download-card";

export const metadata = { title: "Download · PSYCHOSEC" };
export default function Download() {
  return (
    <main className="max-w-xl py-12">
      <h1 className="text-2xl font-semibold">Download PSYCHOSEC</h1>
      <p className="mb-6 mt-2 text-sm text-dim">Free for authorized members. The installer is per-user and does not need administrator rights.</p>
      <DownloadCard />
      <div className="mt-8 space-y-2 text-sm text-dim">
        <p>After installing, launch <span className="font-mono text-fg">Psychosec</span> from the Start Menu. A browser window opens for Discord sign-in.</p>
        <p>Windows SmartScreen may warn about new or unsigned releases. Verify the SHA-256 shown above before running the installer.</p>
      </div>
    </main>
  );
}
