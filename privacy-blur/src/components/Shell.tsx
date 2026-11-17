import { LayoutDashboard, ListChecks, AppWindow, Palette, Settings as Cog, Minus, Square, X, ShieldCheck, Siren } from "lucide-react";
import { api } from "../lib/api";
import { useApp, type Page } from "../lib/app";

const NAV: { id: Page; label: string; icon: typeof Cog }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "filters", label: "Privacy Filters", icon: ListChecks },
  { id: "apps", label: "Applications", icon: AppWindow },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "settings", label: "Settings", icon: Cog },
];

export function TitleBar() {
  const btn = "grid h-9 w-11 place-items-center text-mute-400 transition-colors hover:bg-ink-700 hover:text-zinc-100";
  return (
    <div className="flex h-9 shrink-0 items-center justify-between border-b border-ink-800 bg-ink-950">
      <div data-tauri-drag-region className="flex h-full flex-1 items-center gap-2 pl-4 text-[12px] text-mute-500">
        <ShieldCheck size={14} data-tauri-drag-region /> <span data-tauri-drag-region>Privacy Blur</span>
      </div>
      <button className={btn} onClick={api.minimize} aria-label="Minimize"><Minus size={15} /></button>
      <button className={btn} onClick={api.toggleMaximize} aria-label="Maximize"><Square size={12} /></button>
      <button className={`${btn} hover:!bg-red-600 hover:!text-white`} onClick={api.close} aria-label="Close"><X size={16} /></button>
    </div>
  );
}

export function Sidebar() {
  const { page, setPage, status } = useApp();
  const on = status?.running;
  return (
    <aside className="flex w-[220px] shrink-0 flex-col border-r border-ink-800 bg-ink-900 p-3">
      <div className="mb-5 mt-1 flex items-center gap-2.5 px-2">
        <div className="grid h-8 w-8 place-items-center rounded-[10px] bg-zinc-100 text-ink-950"><ShieldCheck size={18} /></div>
        <div className="leading-tight">
          <div className="text-[14px] font-semibold tracking-tight">Privacy Blur</div>
          <div className="text-[11px] text-mute-500">100% local</div>
        </div>
      </div>
      <nav className="flex flex-1 flex-col gap-1">
        {NAV.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setPage(id)}
            className={`focus-ring flex items-center gap-3 rounded-[10px] px-3 py-2.5 text-left text-[13.5px] font-medium transition-all duration-150 ${page === id ? "bg-ink-700 text-white" : "text-mute-400 hover:bg-ink-800 hover:text-zinc-100"}`}
          >
            <Icon size={17} /> {label}
          </button>
        ))}
      </nav>
      <button
        onClick={() => api.setPanic(!status?.panic)}
        className={`focus-ring mb-2 flex items-center justify-center gap-2 rounded-[10px] px-3 py-2.5 text-[13px] font-medium transition-colors ${status?.panic ? "bg-red-600 text-white" : "bg-ink-800 text-red-300 hover:bg-red-500/15"}`}
      >
        <Siren size={15} /> {status?.panic ? "Remove panic cover" : "Panic: cover screen"}
      </button>
      <div className="flex items-center gap-2 rounded-[10px] bg-ink-850 px-3 py-2.5 text-[12.5px] text-mute-400">
        <span className={`h-2 w-2 rounded-full ${on ? "bg-emerald-400 shadow-[0_0_8px_#34d399aa]" : "bg-zinc-600"}`} />
        {on ? "Protection active" : "Protection paused"}
      </div>
    </aside>
  );
}
