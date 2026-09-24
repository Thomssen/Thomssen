import { useState } from "react";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Toggle } from "@/components/ui/Toggle";
import { Badge } from "@/components/ui/Badge";
import { useSettings } from "@/state/SettingsProvider";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { completeOnboarding } from "@/services/settings";
import { APP } from "@/config/app";
import type { ProtectionMode } from "@/types";
import "./OnboardingWizard.css";

const STEP_COUNT = 4;

const MODES: { value: ProtectionMode; title: string; description: string; recommended?: boolean }[] = [
  {
    value: "standard",
    title: "Standard",
    description: "Balanced protection. Detects and automatically blocks clearly abusive connections while leaving normal apps alone.",
    recommended: true,
  },
  {
    value: "strict",
    title: "Strict",
    description: "Reacts faster and blocks more readily. Best if this PC is regularly exposed to untrusted networks.",
  },
  {
    value: "monitoring_only",
    title: "Monitoring Only",
    description: "Watches connections and the firewall but never blocks anything automatically. You stay in full control.",
  },
];

/** Renders itself in place of the sidebar/content while
 * `settings.onboardingCompleted` is false - see `App.tsx`. Finishing calls
 * `completeOnboarding()`, which flips that flag and lets `App.tsx`'s own
 * reactive render switch back to the normal app (or the license screen). */
export function OnboardingWizard() {
  const { reload } = useSettings();
  const toast = useToast();

  const [step, setStep] = useState(0);
  const [finishing, setFinishing] = useState(false);

  const [mode, setMode] = useState<ProtectionMode>("standard");
  const [notifyBlocked, setNotifyBlocked] = useState(true);
  const [notifySuspicious, setNotifySuspicious] = useState(true);

  const goBack = () => setStep((s) => Math.max(s - 1, 0));

  const finish = () =>
    (async () => {
      setFinishing(true);
      try {
        await completeOnboarding({
          protectionMode: mode,
          notifyOnBlockedIp: notifyBlocked,
          notifyOnSuspiciousActivity: notifySuspicious,
          protection: {
            enabled: mode !== "monitoring_only",
            detectConnectionSpikes: true,
            detectRepeatedConnections: true,
            autoBlockSuspiciousIps: mode !== "monitoring_only",
            blockDuration: mode === "strict" ? "1h" : "30m",
            sensitivity: mode === "strict" ? "high" : "normal",
          },
        });
        await reload();
      } catch (e) {
        toast.error(errorMessage(e));
      } finally {
        setFinishing(false);
      }
    })();

  return (
    <div className="onboarding">
      <div className="onboarding__panel">
        {step > 0 && (
          <div className="onboarding__progress">
            <span className="onboarding__step-label">
              Step {step + 1} of {STEP_COUNT}
            </span>
            <div className="onboarding__bar">
              <div className="onboarding__bar-fill" style={{ width: `${((step + 1) / STEP_COUNT) * 100}%` }} />
            </div>
          </div>
        )}

        <div className="onboarding__content" key={step}>
          {step === 0 && <WelcomeStep onNext={() => setStep(1)} />}

          {step === 1 && <ProtectionModeStep mode={mode} onChange={setMode} />}

          {step === 2 && (
            <NotificationsStep
              notifyBlocked={notifyBlocked}
              onNotifyBlockedChange={setNotifyBlocked}
              notifySuspicious={notifySuspicious}
              onNotifySuspiciousChange={setNotifySuspicious}
            />
          )}

          {step === 3 && <ReadyStep mode={mode} notifyBlocked={notifyBlocked} notifySuspicious={notifySuspicious} />}
        </div>

        {step > 0 && (
          <div className="onboarding__nav">
            <Button variant="ghost" onClick={goBack} disabled={finishing}>
              Back
            </Button>
            <div className="onboarding__nav-right">
              {step < STEP_COUNT - 1 && (
                <Button variant="primary" onClick={() => setStep((s) => s + 1)}>
                  Continue
                </Button>
              )}
              {step === STEP_COUNT - 1 && (
                <Button variant="primary" onClick={finish} loading={finishing}>
                  Finish Setup
                </Button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// -- Steps --------------------------------------------------------------------

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="onboarding__welcome">
      <span className="onboarding__logo" aria-hidden="true">
        S
      </span>
      <h1 className="onboarding__title">Welcome to {APP.name}</h1>
      <p className="onboarding__lead">Lightweight network protection for Windows. Monitor connections, control your firewall, and automatically block abusive traffic.</p>
      <div className="onboarding__welcome-actions">
        <Button variant="primary" onClick={onNext}>
          Get Started
        </Button>
      </div>
    </div>
  );
}

function ProtectionModeStep({ mode, onChange }: { mode: ProtectionMode; onChange: (m: ProtectionMode) => void }) {
  return (
    <div className="onboarding__step">
      <h2 className="onboarding__heading">Choose a protection mode</h2>
      <p className="onboarding__description">This sets your starting defaults - every option here can be changed later in Protection settings.</p>

      <div className="onboarding__modes">
        {MODES.map((m) => (
          <button key={m.value} type="button" className={["onboarding__mode", mode === m.value ? "is-selected" : ""].filter(Boolean).join(" ")} onClick={() => onChange(m.value)}>
            <span className="onboarding__mode-head">
              <span className="onboarding__mode-title">{m.title}</span>
              {m.recommended && (
                <Badge tone="good" dot={false}>
                  Recommended
                </Badge>
              )}
            </span>
            <span className="onboarding__mode-desc">{m.description}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function NotificationsStep({
  notifyBlocked,
  onNotifyBlockedChange,
  notifySuspicious,
  onNotifySuspiciousChange,
}: {
  notifyBlocked: boolean;
  onNotifyBlockedChange: (v: boolean) => void;
  notifySuspicious: boolean;
  onNotifySuspiciousChange: (v: boolean) => void;
}) {
  return (
    <div className="onboarding__step">
      <h2 className="onboarding__heading">Notifications</h2>
      <p className="onboarding__description">Get a quiet system notification when Thomsen Shield acts. Change this anytime in Settings.</p>
      <Card>
        <div className="onboarding__row">
          <div>
            <p className="onboarding__row-title">Notify when an IP is blocked</p>
            <p className="onboarding__hint">A short notification each time Thomsen Shield blocks a connection.</p>
          </div>
          <Toggle checked={notifyBlocked} onChange={onNotifyBlockedChange} label="Notify when an IP is blocked" />
        </div>
        <div className="rule onboarding__field-spaced" />
        <div className="onboarding__row">
          <div>
            <p className="onboarding__row-title">Notify on suspicious activity</p>
            <p className="onboarding__hint">Lets you know when unusual connection patterns are detected, even before anything is blocked.</p>
          </div>
          <Toggle checked={notifySuspicious} onChange={onNotifySuspiciousChange} label="Notify on suspicious activity" />
        </div>
      </Card>
    </div>
  );
}

function ReadyStep({ mode, notifyBlocked, notifySuspicious }: { mode: ProtectionMode; notifyBlocked: boolean; notifySuspicious: boolean }) {
  const modeInfo = MODES.find((m) => m.value === mode)!;
  return (
    <div className="onboarding__step">
      <h2 className="onboarding__heading">You're ready</h2>
      <p className="onboarding__description">Everything below can be changed later from Settings.</p>
      <Card>
        <div className="onboarding__rows">
          <div className="onboarding__row">
            <span>Protection mode</span>
            <span className="mono">{modeInfo.title}</span>
          </div>
          <div className="onboarding__row">
            <span>Automatic blocking</span>
            <Badge tone={mode === "monitoring_only" ? "neutral" : "good"}>{mode === "monitoring_only" ? "Off" : "On"}</Badge>
          </div>
          <div className="onboarding__row">
            <span>Blocked-IP notifications</span>
            <Badge tone={notifyBlocked ? "good" : "neutral"}>{notifyBlocked ? "On" : "Off"}</Badge>
          </div>
          <div className="onboarding__row">
            <span>Suspicious-activity notifications</span>
            <Badge tone={notifySuspicious ? "good" : "neutral"}>{notifySuspicious ? "On" : "Off"}</Badge>
          </div>
        </div>
      </Card>
      <p className="onboarding__hint">Some actions - like creating firewall rules - will briefly ask for administrator permission the first time they're needed. Thomsen Shield does not require administrator rights just to run.</p>
    </div>
  );
}
