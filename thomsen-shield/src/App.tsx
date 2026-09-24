import { useEffect, useState } from "react";
import { TitleBar } from "@/components/TitleBar/TitleBar";
import { Sidebar } from "@/components/Sidebar/Sidebar";
import { RouteView } from "@/app/RouteView";
import { OnboardingWizard } from "@/pages/onboarding/OnboardingWizard";
import { LicenseActivationPage } from "@/pages/license/LicenseActivationPage";
import { useSettings } from "@/state/SettingsProvider";
import { useLicense } from "@/state/LicenseProvider";
import { getAppInfo } from "@/services/system";
import { setLockdownMode } from "@/services/protection";
import "@/styles/app.css";

/**
 * The frameless window shell: title bar on top, sidebar + content below - or,
 * before first-run setup / license activation has completed, that screen in
 * its place - all still under the same title bar so window controls keep
 * working everywhere.
 */
export default function App() {
  const { settings, loading: settingsLoading, reload: reloadSettings } = useSettings();
  const { license, loading: licenseLoading } = useLicense();
  const [warnings, setWarnings] = useState<string[]>([]);
  const [dismissed, setDismissed] = useState(false);
  const [lockdownBusy, setLockdownBusy] = useState(false);

  useEffect(() => {
    void getAppInfo()
      .then((info) => setWarnings(info.loadWarnings))
      .catch(() => {});
  }, []);

  const disableLockdown = async () => {
    setLockdownBusy(true);
    try {
      await setLockdownMode(false);
      await reloadSettings();
    } finally {
      setLockdownBusy(false);
    }
  };

  if (settingsLoading || licenseLoading) {
    return (
      <div className="app">
        <TitleBar showRoute={false} />
        <div className="app__body" />
      </div>
    );
  }

  if (!settings.onboardingCompleted) {
    return (
      <div className="app">
        <TitleBar showRoute={false} />
        <OnboardingWizard />
      </div>
    );
  }

  // Checked after onboarding: the wizard itself must not be gated behind a
  // license key before the user has even seen what they're activating.
  if (license.status !== "active") {
    return (
      <div className="app">
        <TitleBar showRoute={false} />
        <LicenseActivationPage />
      </div>
    );
  }

  return (
    <div className="app">
      <TitleBar />
      <div className="app__body">
        <Sidebar protectionActive={settings.protection.enabled} />
        <main className="app__main">
          {settings.lockdownMode && (
            <div className="app__lockdown" role="status">
              <span className="app__lockdown-dot" aria-hidden="true" />
              <span className="app__lockdown-label">Lockdown Mode is active - stricter rules are in effect. Allowlisted traffic keeps working.</span>
              <button type="button" onClick={disableLockdown} disabled={lockdownBusy}>
                {lockdownBusy ? "Disabling…" : "Disable Lockdown"}
              </button>
            </div>
          )}
          {warnings.length > 0 && !dismissed && (
            <div className="app__notice" role="status">
              <span>Some stored data could not be read: {warnings.join("; ")}. The rest loaded normally.</span>
              <button type="button" onClick={() => setDismissed(true)}>
                Dismiss
              </button>
            </div>
          )}
          <RouteView />
        </main>
      </div>
    </div>
  );
}
