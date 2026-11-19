import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { deriveAccentShades } from "@/lib/color";
import { useSettings } from "./SettingsProvider";

export type Theme = "dark" | "light";
export type WindowControlsStyle = "windows" | "mac";

export const THEME_KEY = "thomsen-optimizer-theme";
const WINDOW_STYLE_KEY = "thomsen-optimizer-window-style";

interface AppearanceContextValue {
  theme: Theme;
  setTheme: (t: Theme) => void;
  windowControlsStyle: WindowControlsStyle;
  setWindowControlsStyle: (s: WindowControlsStyle) => void;
}

const Ctx = createContext<AppearanceContextValue | null>(null);

function stored<T extends string>(key: string, valid: readonly T[], fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return (valid as readonly string[]).includes(v ?? "") ? (v as T) : fallback;
  } catch {
    return fallback;
  }
}

function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Best-effort only.
  }
}

/** Theme + window-control style are client-side rendering preferences (read
 * before first paint, see main.tsx). Accent colour, animations and scaling
 * come from the Rust-backed settings so they survive a reinstall. */
export function AppearanceProvider({ children }: { children: ReactNode }) {
  const { settings } = useSettings();
  const [theme, setThemeState] = useState<Theme>(() => stored(THEME_KEY, ["dark", "light"], "dark"));
  const [windowControlsStyle, setStyle] = useState<WindowControlsStyle>(() => stored(WINDOW_STYLE_KEY, ["windows", "mac"], "windows"));

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  const accent = settings?.accentColor ?? "#ffffff";
  useEffect(() => {
    const { base, hover, soft, text } = deriveAccentShades(accent, theme);
    const root = document.documentElement.style;
    root.setProperty("--accent-brand", base);
    root.setProperty("--accent-brand-hover", hover);
    root.setProperty("--accent-brand-soft", soft);
    root.setProperty("--btn-primary-text", text);
  }, [accent, theme]);

  const animations = settings?.animations ?? true;
  useEffect(() => {
    document.documentElement.toggleAttribute("data-reduce-motion", !animations);
  }, [animations]);

  const scale = settings?.uiScale ?? 1;
  useEffect(() => {
    (document.documentElement.style as CSSStyleDeclaration & { zoom: string }).zoom = scale === 1 ? "" : String(scale);
  }, [scale]);

  const value: AppearanceContextValue = {
    theme,
    setTheme: (t) => {
      setThemeState(t);
      persist(THEME_KEY, t);
    },
    windowControlsStyle,
    setWindowControlsStyle: (s) => {
      setStyle(s);
      persist(WINDOW_STYLE_KEY, s);
    },
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAppearance(): AppearanceContextValue {
  const c = useContext(Ctx);
  if (!c) throw new Error("useAppearance must be used within <AppearanceProvider>");
  return c;
}
