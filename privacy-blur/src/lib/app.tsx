import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Entry, type Settings, type Status } from "./api";

export type Page = "dashboard" | "filters" | "apps" | "appearance" | "settings";

interface Toast { id: number; text: string; kind: "info" | "error" }

interface Ctx {
  settings: Settings | null;
  status: Status | null;
  entries: Entry[];
  page: Page;
  setPage: (p: Page) => void;
  patch: (fn: (s: Settings) => void) => void;
  reloadEntries: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  toast: (text: string, kind?: "info" | "error") => void;
  toasts: Toast[];
}

const C = createContext<Ctx>(null as unknown as Ctx);
export const useApp = () => useContext(C);

export function AppProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [page, setPage] = useState<Page>("dashboard");
  const [toasts, setToasts] = useState<Toast[]>([]);
  const latest = useRef<Settings | null>(null);
  const timer = useRef<number | null>(null);
  const seq = useRef(0);

  const toast = useCallback((text: string, kind: "info" | "error" = "info") => {
    const id = ++seq.current;
    setToasts((t) => [...t, { id, text, kind }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "error" ? 6500 : 3200);
  }, []);

  const refreshStatus = useCallback(async () => {
    try { setStatus(await api.status()); } catch { /* backend not ready yet */ }
  }, []);
  const reloadEntries = useCallback(async () => setEntries(await api.entries()), []);

  useEffect(() => {
    api.settings().then((s) => { latest.current = s; setSettings(s); });
    reloadEntries();
    refreshStatus();
    const iv = window.setInterval(() => { if (document.visibilityState === "visible") refreshStatus(); }, 1000);
    const u1 = listen("state-changed", () => refreshStatus());
    const u2 = listen<string>("navigate", (e) => setPage(e.payload as Page));
    return () => { clearInterval(iv); u1.then((f) => f()); u2.then((f) => f()); };
  }, [refreshStatus, reloadEntries]);

  const flush = useCallback(async () => {
    timer.current = null;
    if (!latest.current) return;
    try {
      const r = await api.saveSettings(latest.current);
      r.warnings.forEach((w) => toast(w, "error"));
      refreshStatus();
    } catch (e) {
      toast(`Could not save settings: ${e}`, "error");
    }
  }, [toast, refreshStatus]);

  const patch = useCallback((fn: (s: Settings) => void) => {
    if (!latest.current) return;
    const next = structuredClone(latest.current);
    fn(next);
    latest.current = next;
    setSettings(next);
    if (timer.current) clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, 250);
  }, [flush]);

  return (
    <C.Provider value={{ settings, status, entries, page, setPage, patch, reloadEntries, refreshStatus, toast, toasts }}>
      {children}
    </C.Provider>
  );
}
