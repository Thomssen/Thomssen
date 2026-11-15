import Link from "next/link";
import { DownloadCard } from "@/components/download-card";

export default function Home() {
  return (
    <main className="py-16">
      <p className="text-xs tracking-[0.3em] text-dim">PRIVATE OSINT &amp; INVESTIGATION TERMINAL</p>
      <h1 className="mt-4 text-4xl font-semibold tracking-tight sm:text-5xl">A quiet workspace for serious investigations.</h1>
      <p className="mt-5 max-w-2xl text-dim">PSYCHOSEC is a native Windows terminal for public-source research: usernames, domains, IP addresses, email infrastructure, websites and files, organised into encrypted local cases. Investigation data stays on your computer.</p>
      <ul className="mt-8 grid gap-2 text-sm text-dim sm:grid-cols-2">
        {["Real lookups against public sources: no simulated results", "Encrypted, local-only case storage", "HTML, JSON and CSV report export", "Discord-verified private access", "Signed, verified automatic updates"].map((t) => <li key={t} className="flex gap-2"><span className="text-accent">▸</span>{t}</li>)}
      </ul>
      <div className="mt-10 max-w-xl"><DownloadCard /></div>
      <p className="mt-6 text-sm text-dim">Access requires membership of the PSYCHOSEC Discord server. <Link href="/members" className="text-accent underline">Member login</Link></p>
    </main>
  );
}
