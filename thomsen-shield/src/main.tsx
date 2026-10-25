import React from "react";
import ReactDOM from "react-dom/client";

import "@fontsource-variable/inter";
import "@/styles/tokens.css";
import "@/styles/base.css";

import App from "@/App";
import { SettingsProvider } from "@/state/SettingsProvider";
import { LicenseProvider } from "@/state/LicenseProvider";
import { NavigationProvider } from "@/state/NavigationProvider";
import { ToastProvider } from "@/components/ui/Toast";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { installGlobalErrorLogging } from "@/services/errorLogging";

// A release build has no console and no crash reporting attached, so an
// uncaught exception anywhere in the frontend - not just a render-time one
// a page's own ErrorBoundary would catch - can otherwise leave the WebView
// showing nothing, with zero evidence of why. This mirrors the same
// "write a plain trail to disk" approach used on the Rust side (see
// `debug_log` in src-tauri/src/lib.rs) for the one class of failure that
// approach can't see: this process's own frontend.
installGlobalErrorLogging();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <SettingsProvider>
        <LicenseProvider>
          <ToastProvider>
            <NavigationProvider>
              <App />
            </NavigationProvider>
          </ToastProvider>
        </LicenseProvider>
      </SettingsProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
