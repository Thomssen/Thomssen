import { Sidebar, TitleBar } from "./components/Shell";
import { useApp } from "./lib/app";
import Dashboard from "./pages/Dashboard";
import Filters from "./pages/Filters";
import Apps from "./pages/Apps";
import Appearance from "./pages/Appearance";
import SettingsPage from "./pages/SettingsPage";

export default function App() {
  const { page, settings, toasts } = useApp();
  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="min-w-0 flex-1 overflow-y-auto">
          {settings ? (
            <div key={page} className="mx-auto max-w-[880px] animate-rise px-8 py-7">
              {page === "dashboard" && <Dashboard />}
              {page === "filters" && <Filters />}
              {page === "apps" && <Apps />}
              {page === "appearance" && <Appearance />}
              {page === "settings" && <SettingsPage />}
            </div>
          ) : (
            <div className="grid h-full place-items-center text-mute-500">Loading…</div>
          )}
        </main>
      </div>
      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2">
        {toasts.map((t) => (
          <div key={t.id} className={`animate-pop max-w-sm rounded-xl border px-4 py-3 text-[12.5px] shadow-2xl ${t.kind === "error" ? "border-red-500/30 bg-ink-850 text-red-200" : "border-ink-600 bg-ink-800 text-zinc-100"}`}>
            {t.text}
          </div>
        ))}
      </div>
    </div>
  );
}
