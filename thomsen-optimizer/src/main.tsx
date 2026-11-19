import React from "react";
import ReactDOM from "react-dom/client";

import "@fontsource-variable/inter";
import "@/styles/tokens.css";
import "@/styles/base.css";
import "@/components/ui/Table.css";

import App from "@/App";
import { SettingsProvider } from "@/state/SettingsProvider";
import { NavigationProvider } from "@/state/NavigationProvider";
import { MonitorProvider } from "@/state/MonitorProvider";
import { AppearanceProvider, THEME_KEY } from "@/state/AppearanceProvider";
import { ToastProvider } from "@/components/ui/Toast";
import { ErrorBoundary } from "@/components/ErrorBoundary";

// Set before React renders so a saved "light" theme never flashes dark.
try {
  const saved = localStorage.getItem(THEME_KEY);
  document.documentElement.setAttribute("data-theme", saved === "light" ? "light" : "dark");
} catch {
  // Falls back to the default dark tokens.
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <SettingsProvider>
        <AppearanceProvider>
          <ToastProvider>
            <NavigationProvider>
              <MonitorProvider>
                <App />
              </MonitorProvider>
            </NavigationProvider>
          </ToastProvider>
        </AppearanceProvider>
      </SettingsProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
