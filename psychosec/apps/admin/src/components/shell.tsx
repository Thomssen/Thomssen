"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { Activity, FileText, Gauge, Settings, ShieldAlert, ShieldBan, Users, MonitorSmartphone, LogOut } from "lucide-react";
import type { AdminMe } from "@psychosec/shared";
import { api, ApiError, loginUrl } from "@/lib/api";
import { Avatar, Button, cn } from "./ui";

const Ctx = createContext<AdminMe | null>(null);
export const useAdmin = () => useContext(Ctx)!;

const NAV = [
  { href: "/", label: "Overview", icon: Gauge },
  { href: "/users", label: "Users", icon: Users },
  { href: "/sessions", label: "Sessions", icon: MonitorSmartphone },
  { href: "/activity", label: "Activity", icon: Activity },
  { href: "/security", label: "Security", icon: ShieldAlert },
  { href: "/bans", label: "Bans", icon: ShieldBan },
  { href: "/settings", label: "Settings", icon: Settings },
  { href: "/audit-logs", label: "Audit Logs", icon: FileText },
];

function Splash({ children }: { children: ReactNode }) {
  return (
    <main className="grid min-h-screen place-items-center p-6">
      <div className="w-full max-w-sm rounded-lg border border-line bg-panel p-8 text-center">
        <div className="text-sm font-semibold tracking-[0.35em] text-accent">PSYCHOSEC</div>
        <div className="mt-1 text-xs tracking-widest text-dim">ADMINISTRATION</div>
        <div className="mt-6">{children}</div>
      </div>
    </main>
  );
}

export function Shell({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<AdminMe | null>(null);
  const [state, setState] = useState<"loading" | "anon" | "denied" | "error" | "ok">("loading");
  const [msg, setMsg] = useState("");
  const path = usePathname();
  useEffect(() => {
    api<AdminMe>("/api/auth/me?kind=admin").then((m) => { setMe(m); setState("ok"); }).catch((e) => {
      if (e instanceof ApiError) { setMsg(e.message); setState(e.status === 401 ? "anon" : "denied"); } else { setMsg("Cannot reach the API."); setState("error"); }
    });
  }, []);
  if (state === "loading") return <Splash><p className="text-sm text-dim">Checking session…</p></Splash>;
  if (state === "anon") return <Splash><p className="mb-5 text-sm text-dim">Restricted area. Sign in with an authorized Discord administrator account.</p><a href={loginUrl}><Button className="w-full justify-center">Sign in with Discord</Button></a></Splash>;
  if (state !== "ok" || !me) return <Splash><p className="mb-5 text-sm text-bad">{msg}</p><a href={loginUrl}><Button className="w-full justify-center">Try another account</Button></a></Splash>;
  return (
    <Ctx.Provider value={me}>
      <div className="flex min-h-screen">
        <aside className="sticky top-0 hidden h-screen w-56 shrink-0 flex-col border-r border-line bg-panel md:flex">
          <div className="px-5 py-5 text-sm font-semibold tracking-[0.35em] text-accent">PSYCHOSEC</div>
          <nav className="flex-1 space-y-0.5 px-2">
            {NAV.map((n) => {
              const active = n.href === "/" ? path === "/" : path.startsWith(n.href);
              return <Link key={n.href} href={n.href} className={cn("flex items-center gap-3 rounded-md px-3 py-2 text-sm transition", active ? "bg-accent/10 text-accent" : "text-dim hover:text-fg")}><n.icon size={16} />{n.label}</Link>;
            })}
          </nav>
          <div className="border-t border-line p-3">
            <div className="flex items-center gap-2 px-1 pb-2"><Avatar url={me.avatar_url} name={me.username} /><div className="min-w-0"><div className="truncate text-sm">{me.display_name ?? me.username}</div><div className="text-xs text-dim">{me.role.replace("_", " ")}</div></div></div>
            <Button variant="quiet" className="w-full justify-start" onClick={async () => { await api("/api/auth/logout", { method: "POST" }).catch(() => {}); location.href = "/"; }}><LogOut size={14} />Sign out</Button>
          </div>
        </aside>
        <div className="min-w-0 flex-1">
          <nav className="flex gap-1 overflow-x-auto border-b border-line bg-panel px-2 py-2 md:hidden">
            {NAV.map((n) => <Link key={n.href} href={n.href} className={cn("shrink-0 rounded px-2.5 py-1 text-xs", path === n.href ? "bg-accent/10 text-accent" : "text-dim")}>{n.label}</Link>)}
          </nav>
          <main className="mx-auto max-w-6xl p-4 md:p-8">{children}</main>
        </div>
      </div>
    </Ctx.Provider>
  );
}

export const PageTitle = ({ title, sub }: { title: string; sub?: string }) => (
  <header className="mb-6"><h1 className="text-xl font-semibold">{title}</h1>{sub && <p className="mt-1 text-sm text-dim">{sub}</p>}</header>
);
