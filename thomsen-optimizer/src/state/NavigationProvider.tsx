/**
 * A tiny router. One active `Route` at a time. Dependency-free by design,
 * same as every other Thomsen app.
 */

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

import { DEFAULT_ROUTE, type Route } from "@/config/navigation";

interface NavigationContextValue {
  route: Route;
  navigate: (route: Route) => void;
  /** An optional third breadcrumb segment for wherever the user is inside
   * the current route (a Settings tab, an open Attack Inspector) - reset to
   * null on every top-level `navigate()` so it can never linger onto a
   * different page. A page sets this itself; it's the one thing here that
   * isn't a plain top-level route. */
  subLabel: string | null;
  setSubLabel: (label: string | null) => void;
}

const NavigationContext = createContext<NavigationContextValue | null>(null);

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [route, setRoute] = useState<Route>(DEFAULT_ROUTE);
  const [subLabel, setSubLabel] = useState<string | null>(null);

  const navigate = useCallback((next: Route) => {
    setRoute((current) => (next === current ? current : next));
    setSubLabel(null);
  }, []);

  const value = useMemo(() => ({ route, navigate, subLabel, setSubLabel }), [route, navigate, subLabel]);

  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
}

export function useNavigation(): NavigationContextValue {
  const ctx = useContext(NavigationContext);
  if (!ctx) throw new Error("useNavigation must be used within <NavigationProvider>");
  return ctx;
}
