import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { LicenseInfo } from "@/types";
import { activateLicense, deactivateLicense, getLicenseInfo } from "@/services/license";

const FALLBACK: LicenseInfo = { status: "unlicensed", key: null, activatedAt: null, provider: "dev" };

interface LicenseContextValue {
  license: LicenseInfo;
  loading: boolean;
  activate: (key: string) => Promise<void>;
  deactivate: () => Promise<void>;
}

const LicenseContext = createContext<LicenseContextValue | null>(null);

export function LicenseProvider({ children }: { children: ReactNode }) {
  const [license, setLicense] = useState<LicenseInfo>(FALLBACK);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    void getLicenseInfo()
      .then(setLicense)
      .finally(() => setLoading(false));
  }, []);

  const activate = useCallback(async (key: string) => {
    const info = await activateLicense(key);
    setLicense(info);
  }, []);

  const deactivate = useCallback(async () => {
    await deactivateLicense();
    setLicense(FALLBACK);
  }, []);

  const value = useMemo<LicenseContextValue>(
    () => ({ license, loading, activate, deactivate }),
    [license, loading, activate, deactivate],
  );

  return <LicenseContext.Provider value={value}>{children}</LicenseContext.Provider>;
}

export function useLicense(): LicenseContextValue {
  const ctx = useContext(LicenseContext);
  if (!ctx) throw new Error("useLicense must be used within <LicenseProvider>");
  return ctx;
}
