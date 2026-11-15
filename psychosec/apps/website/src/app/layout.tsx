import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import "./globals.css";

export const metadata: Metadata = {
  title: "PSYCHOSEC",
  description: "PSYCHOSEC is a private OSINT & investigation terminal for authorized members.",
  icons: { icon: "/icon.png" },
};
export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body className="flex min-h-screen flex-col">
        <header className="mx-auto flex w-full max-w-4xl items-center justify-between px-5 py-6">
          <Link href="/" className="text-sm font-semibold tracking-[0.35em] text-accent">PSYCHOSEC</Link>
          <nav className="flex gap-5 text-sm text-dim">
            <Link href="/download" className="hover:text-fg">Download</Link>
            <Link href="/privacy" className="hover:text-fg">Privacy</Link>
            <Link href="/members" className="hover:text-fg">Member login</Link>
          </nav>
        </header>
        <div className="mx-auto w-full max-w-4xl flex-1 px-5">{children}</div>
        <footer className="mx-auto w-full max-w-4xl border-t border-line px-5 py-6 text-xs text-dim">
          © PSYCHOSEC · Access is limited to authorized members · <Link href="/privacy" className="underline">Privacy policy</Link>
        </footer>
      </body>
    </html>
  );
}
