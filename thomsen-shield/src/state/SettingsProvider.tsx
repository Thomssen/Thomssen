import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import type { Settings } from "@/types";
import { getSettings, saveSettings } from "@/services/settings";

const FALLBACK: Settings = {
  onboardingCompleted: false,
  protectionMode: "standard",
  startWithWindows: false,
  startMinimized: false,
  minimizeToTray: true,
  protection: {
    enabled: true,
    detectConnectionSpikes: true,
    detectRepeatedConnections: true,
    autoBlockSuspiciousIps: true,
    blockDuration: "30m",
    sensitivity: "normal",
  },
  lockdownMode: false,
  notifyOnBlockedIp: true,
  notifyOnSuspiciousActivity: true,
};

interface SettingsContextValue {
  settings: Settings;
  loading: boolean;
  /** Persist a partial change and update local state with the server's copy. */
  update: (patch: Partial<Settings>) => Promise<Settings>;
  reload: () => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings>(FALLBACK);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    try {
      const s = await getSettings();
      setSettings(s);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const update = useCallback(async (patch: Partial<Settings>) => {
    const next = await saveSettings(patch);
    setSettings(next);
    return next;
  }, []);

  const value = useMemo<SettingsContextValue>(
    () => ({ settings, loading, update, reload }),
    [settings, loading, update, reload],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx) throw new Error("useSettings must be used within <SettingsProvider>");
  return ctx;
}
