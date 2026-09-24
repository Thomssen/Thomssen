import { useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Toggle } from "@/components/ui/Toggle";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Select } from "@/components/ui/Field";
import { ConfirmDialog } from "@/components/ui/Modal";
import { useSettings } from "@/state/SettingsProvider";
import { useLicense } from "@/state/LicenseProvider";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { clearBlockHistory } from "@/services/blocklist";
import { clearLogs } from "@/services/logs";
import { resetThomsenShieldRules } from "@/services/firewall";
import { APP } from "@/config/app";
import { BLOCK_DURATION_LABELS, type BlockDuration, type SensitivityLevel } from "@/types";
import "./SettingsPage.css";

type Tab = "general" | "protection" | "notifications" | "appearance" | "data" | "about";

const TABS: { value: Tab; label: string }[] = [
  { value: "general", label: "General" },
  { value: "protection", label: "Protection" },
  { value: "notifications", label: "Notifications" },
  { value: "appearance", label: "Appearance" },
  { value: "data", label: "Data" },
  { value: "about", label: "About" },
];

export function SettingsPage() {
  const [tab, setTab] = useState<Tab>("general");

  return (
    <Page title="Settings" eyebrow="Thomsen Shield">
      <div className="stack">
        <SegmentedControl options={TABS} value={tab} onChange={setTab} aria-label="Settings section" />
        {tab === "general" && <GeneralTab />}
        {tab === "protection" && <ProtectionTab />}
        {tab === "notifications" && <NotificationsTab />}
        {tab === "appearance" && <AppearanceTab />}
        {tab === "data" && <DataTab />}
        {tab === "about" && <AboutTab />}
      </div>
    </Page>
  );
}

function SettingsRow({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="set__row">
      <div>
        <p className="set__row-title">{title}</p>
        {hint && <p className="set__row-hint">{hint}</p>}
      </div>
      {children}
    </div>
  );
}

function GeneralTab() {
  const { settings, update } = useSettings();
  return (
    <Card>
      <SettingsRow title="Start with Windows" hint="Launches Thomsen Shield automatically when you sign in.">
        <Toggle checked={settings.startWithWindows} onChange={(v) => void update({ startWithWindows: v })} label="Start with Windows" />
      </SettingsRow>
      <div className="rule set__spaced" />
      <SettingsRow title="Start minimized" hint="Opens directly to the tray instead of showing the window.">
        <Toggle checked={settings.startMinimized} onChange={(v) => void update({ startMinimized: v })} label="Start minimized" />
      </SettingsRow>
      <div className="rule set__spaced" />
      <SettingsRow title="Minimize to tray" hint="Closing the window keeps Thomsen Shield running in the background.">
        <Toggle checked={settings.minimizeToTray} onChange={(v) => void update({ minimizeToTray: v })} label="Minimize to tray" />
      </SettingsRow>
    </Card>
  );
}

const DURATIONS: BlockDuration[] = ["5m", "30m", "1h", "24h", "permanent"];
const SENSITIVITIES: { value: SensitivityLevel; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "normal", label: "Normal" },
  { value: "high", label: "High" },
];

function ProtectionTab() {
  const { settings, update } = useSettings();
  const protection = settings.protection;

  const patchProtection = (partial: Partial<typeof protection>) => void update({ protection: { ...protection, ...partial } });

  return (
    <Card>
      <SettingsRow title="Automatic protection" hint="Monitor connections and respond to abuse automatically.">
        <Toggle checked={protection.enabled} onChange={(v) => patchProtection({ enabled: v })} label="Automatic protection" />
      </SettingsRow>
      <div className="rule set__spaced" />
      <SettingsRow title="Sensitivity" hint="How readily suspicious activity is flagged.">
        <SegmentedControl size="sm" options={SENSITIVITIES} value={protection.sensitivity} onChange={(v) => patchProtection({ sensitivity: v })} aria-label="Sensitivity" />
      </SettingsRow>
      <div className="rule set__spaced" />
      <SettingsRow title="Default block duration" hint="Used for new automatic blocks.">
        <Select
          value={protection.blockDuration}
          onChange={(v) => patchProtection({ blockDuration: v as BlockDuration })}
          style={{ width: 160 }}
          options={DURATIONS.map((d) => ({ value: d, label: BLOCK_DURATION_LABELS[d] }))}
        />
      </SettingsRow>
      <p className="set__note">For the full detection configuration, see the Protection page.</p>
    </Card>
  );
}

function NotificationsTab() {
  const { settings, update } = useSettings();
  return (
    <Card>
      <SettingsRow title="Blocked IP notification" hint="A system notification whenever Thomsen Shield blocks a connection.">
        <Toggle checked={settings.notifyOnBlockedIp} onChange={(v) => void update({ notifyOnBlockedIp: v })} label="Blocked IP notification" />
      </SettingsRow>
      <div className="rule set__spaced" />
      <SettingsRow title="Suspicious activity notification" hint="A system notification when unusual activity is detected.">
        <Toggle checked={settings.notifyOnSuspiciousActivity} onChange={(v) => void update({ notifyOnSuspiciousActivity: v })} label="Suspicious activity notification" />
      </SettingsRow>
    </Card>
  );
}

function AppearanceTab() {
  return (
    <Card>
      <SettingsRow title="Theme" hint="Dark theme only for now - a light theme may be added in a future update.">
        <Badge tone="neutral" dot={false}>
          Dark
        </Badge>
      </SettingsRow>
    </Card>
  );
}

function DataTab() {
  const toast = useToast();
  const [confirm, setConfirm] = useState<"logs" | "blocks" | "rules" | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    try {
      if (confirm === "logs") {
        await clearLogs();
        toast.success("Logs cleared");
      } else if (confirm === "blocks") {
        await clearBlockHistory();
        toast.success("Block history cleared");
      } else if (confirm === "rules") {
        const { removed } = await resetThomsenShieldRules();
        toast.success(`Removed ${removed} Thomsen Shield rule${removed === 1 ? "" : "s"}`);
      }
      setConfirm(null);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Card>
        <SettingsRow title="Clear logs" hint="Permanently deletes the Events & Logs history.">
          <Button variant="secondary" size="sm" onClick={() => setConfirm("logs")}>
            Clear Logs
          </Button>
        </SettingsRow>
        <div className="rule set__spaced" />
        <SettingsRow title="Clear block history" hint="Removes expired and removed entries from the Blocklist. Active blocks are kept.">
          <Button variant="secondary" size="sm" onClick={() => setConfirm("blocks")}>
            Clear Block History
          </Button>
        </SettingsRow>
        <div className="rule set__spaced" />
        <SettingsRow title="Reset Thomsen Shield firewall rules" hint="Removes every firewall rule Thomsen Shield created and restores a clean state.">
          <Button variant="danger" size="sm" onClick={() => setConfirm("rules")}>
            Reset Rules
          </Button>
        </SettingsRow>
      </Card>

      <ConfirmDialog
        open={confirm !== null}
        onClose={() => setConfirm(null)}
        onConfirm={() => void run()}
        title={confirm === "logs" ? "Clear logs?" : confirm === "blocks" ? "Clear block history?" : "Reset Thomsen Shield Rules"}
        message={
          confirm === "rules"
            ? "This removes every firewall rule Thomsen Shield created. Rules you created yourself outside Thomsen Shield are never touched."
            : "This cannot be undone."
        }
        confirmLabel={confirm === "logs" ? "Clear Logs" : confirm === "blocks" ? "Clear History" : "Reset Rules"}
        danger
        busy={busy}
      />
    </>
  );
}

function AboutTab() {
  const { license, deactivate } = useLicense();
  const toast = useToast();
  const [busy, setBusy] = useState(false);

  const doDeactivate = async () => {
    setBusy(true);
    try {
      await deactivate();
      toast.success("License deactivated on this PC");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card>
      <div className="about__block">
        <span className="about__logo" aria-hidden="true">
          S
        </span>
        <div>
          <p className="about__name">{APP.name}</p>
          <p className="about__version mono">Version {APP.version}</p>
        </div>
      </div>
      <div className="rule set__spaced" />
      <SettingsRow title="License" hint={license.key ? `Activated ${license.activatedAt ? new Date(license.activatedAt).toLocaleDateString() : ""}` : "No active license"}>
        <div className="about__license">
          <Badge tone={license.status === "active" ? "good" : "neutral"} dot={false}>
            {license.status === "active" ? "Active" : "Not activated"}
          </Badge>
          {license.status === "active" && (
            <Button variant="ghost" size="sm" onClick={() => void doDeactivate()} loading={busy}>
              Deactivate
            </Button>
          )}
        </div>
      </SettingsRow>
      <div className="rule set__spaced" />
      <p className="about__made-by">Made by Thomsen</p>
    </Card>
  );
}
