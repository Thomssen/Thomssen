import { useEffect, useState } from "react";
import { TitleBar } from "@/components/TitleBar/TitleBar";
import { Sidebar } from "@/components/Sidebar/Sidebar";
import { RouteView } from "@/app/RouteView";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { useToast } from "@/components/ui/Toast";
import { useSettings } from "@/state/SettingsProvider";
import { useMonitor } from "@/state/MonitorProvider";
import { isTauri } from "@/services/ipc";
import { APP } from "@/config/app";
import "@/styles/app.css";

/** Frameless window shell: title bar on top, sidebar + page below. */
export default function App() {
  const { settings, loading, update, reload } = useSettings();
  const { lastEvent } = useMonitor();
  const toast = useToast();
  const [showWelcome, setShowWelcome] = useState(false);

  useEffect(() => {
    if (settings && !settings.onboardingCompleted) setShowWelcome(true);
  }, [settings?.onboardingCompleted]); // eslint-disable-line react-hooks/exhaustive-deps

  // Surface Game Booster activity as toasts (events arrive while the window is open).
  useEffect(() => {
    if (!lastEvent) return;
    const e = lastEvent.event;
    if (e.type === "activated") toast.success(`${e.game}: ${e.profileName} profile activated.`);
    else if (e.type === "failed") toast.error(`Could not boost ${e.game}: ${e.message}`);
    else if (e.type === "restored" && (e.restored > 0 || e.failed.length > 0)) e.failed.length ? toast.error(`Restored ${e.restored}, ${e.failed.length} failed.`) : toast.success(`Game closed - ${e.restored} setting${e.restored === 1 ? "" : "s"} restored.`);
  }, [lastEvent]); // eslint-disable-line react-hooks/exhaustive-deps

  if (loading || !settings) {
    return (
      <div className="app">
        <TitleBar showRoute={false} />
        <div className="app__body">
          {!loading && (
            <EmptyState
              title="Could not load settings"
              description="The app could not read its settings file. Your data was not changed."
              action={<Button variant="primary" onClick={() => void reload()}>Try again</Button>}
            />
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      <TitleBar />
      <div className="app__body">
        <Sidebar />
        <main className="app__main">
          {!isTauri() && (
            <div className="app__notice" role="status">
              <span>Design preview: no Windows backend is connected, so the numbers below are placeholders for layout only. Run the installed app for real data.</span>
            </div>
          )}
          <RouteView />
        </main>
      </div>
      <Modal
        open={showWelcome}
        onClose={() => void 0}
        title={`Welcome to ${APP.name}`}
        description="A few things to know before you start:"
        width={500}
        footer={
          <Button
            variant="primary"
            onClick={() => {
              setShowWelcome(false);
              void update({ onboardingCompleted: true, lastSeenVersion: APP.version });
            }}
          >
            Got it
          </Button>
        }
      >
        <ul className="welcome__list">
          <li><strong>Nothing changes without you.</strong> The app analyses first and explains every recommendation.</li>
          <li><strong>Everything is reversible.</strong> Windows settings are recorded and can be restored from Settings &gt; Optimization, or with Restore Changes on the Dashboard.</li>
          <li><strong>No fake numbers.</strong> Results show what was actually done; FPS is only shown when it can be measured.</li>
          <li><strong>Private.</strong> No account, no ads, no telemetry. Data stays on this PC.</li>
        </ul>
      </Modal>
    </div>
  );
}
