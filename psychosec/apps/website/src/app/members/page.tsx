"use client";
import { useEffect, useState } from "react";
import { API } from "@/lib";

interface Me { username: string; display_name: string | null; avatar_url: string | null }
export default function Members() {
  const [me, setMe] = useState<Me | null>(null);
  const [state, setState] = useState<"loading" | "anon" | "ok" | "error">("loading");
  useEffect(() => {
    fetch(`${API}/api/auth/me?kind=web`, { credentials: "include" })
      .then(async (r) => { if (r.ok) { setMe(await r.json()); setState("ok"); } else setState("anon"); })
      .catch(() => setState("error"));
  }, []);
  return (
    <main className="max-w-md py-12">
      <h1 className="text-2xl font-semibold">Members</h1>
      {state === "loading" && <p className="mt-4 text-sm text-dim">Checking session…</p>}
      {state === "error" && <p className="mt-4 text-sm text-bad">Cannot reach the PSYCHOSEC service right now.</p>}
      {state === "anon" && (
        <div className="mt-4">
          <p className="mb-5 text-sm text-dim">Sign in with Discord. Access is limited to members of the PSYCHOSEC server.</p>
          <a href={`${API}/api/auth/discord/start?client=web`} className="inline-block rounded-md border border-accent/40 bg-accent/10 px-5 py-2.5 text-sm text-accent hover:bg-accent/20">Sign in with Discord</a>
        </div>
      )}
      {state === "ok" && me && (
        <div className="mt-4 rounded-lg border border-line bg-panel p-6">
          <p className="text-sm text-dim">Signed in as</p>
          <p className="mt-1 text-lg">{me.display_name ?? me.username}</p>
          <div className="mt-5 flex gap-3 text-sm">
            <a href="/download" className="text-accent underline">Download the latest version</a>
            <button className="text-dim underline" onClick={async () => { await fetch(`${API}/api/auth/logout`, { method: "POST", credentials: "include" }); location.href = "/members"; }}>Sign out</button>
          </div>
        </div>
      )}
    </main>
  );
}
