import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import type { Settings } from "@/types";

interface SettingsContextValue {
  settings: Settings | null;
  loading: boolean;
  /** Saves a partial update; resolves with an error message or null. */
  update: (patch: Partial<Settings>) => Promise<string | null>;
  reload: () => Promise<void>;
}

const Ctx = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);
  // Always the newest value, so rapid successive edits never overwrite each other with stale data.
  const latest = useRef<Settings | null>(null);

  const reload = useCallback(async () => {
    const s = await api.getSettings();
    latest.current = s;
    setSettings(s);
    setLoading(false);
  }, []);

  useEffect(() => {
    void reload().catch(() => setLoading(false));
  }, [reload]);

  const update = useCallback(async (patch: Partial<Settings>) => {
    if (!latest.current) return "Settings are not loaded yet.";
    const next = { ...latest.current, ...patch };
    const previous = latest.current;
    latest.current = next;
    setSettings(next); // optimistic
    try {
      const saved = await api.saveSettings(next);
      latest.current = saved;
      setSettings(saved);
      return null;
    } catch (e) {
      latest.current = previous;
      setSettings(previous);
      return errorMessage(e);
    }
  }, []);

  const value = useMemo(() => ({ settings, loading, update, reload }), [settings, loading, update, reload]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useSettings(): SettingsContextValue {
  const c = useContext(Ctx);
  if (!c) throw new Error("useSettings must be used within <SettingsProvider>");
  return c;
}

/** For pages that cannot render without settings. */
export function useLoadedSettings(): Omit<SettingsContextValue, "settings"> & { settings: Settings } {
  const c = useSettings();
  if (!c.settings) throw new Error("settings not loaded");
  return c as Omit<SettingsContextValue, "settings"> & { settings: Settings };
}
