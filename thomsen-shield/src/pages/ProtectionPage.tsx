import { useEffect, useState } from "react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Field, Select } from "@/components/ui/Field";
import { Toggle } from "@/components/ui/Toggle";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Badge } from "@/components/ui/Badge";
import { LoadingRow } from "@/components/ui/Spinner";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { getProtectionSettings, saveProtectionSettings, setLockdownMode } from "@/services/protection";
import { useSettings } from "@/state/SettingsProvider";
import { BLOCK_DURATION_LABELS, type BlockDuration, type ProtectionSettings, type SensitivityLevel } from "@/types";
import "./ProtectionPage.css";

const DURATIONS: BlockDuration[] = ["5m", "30m", "1h", "24h", "permanent"];
const SENSITIVITIES: { value: SensitivityLevel; label: string }[] = [
  { value: "low", label: "Low" },
  { value: "normal", label: "Normal" },
  { value: "high", label: "High" },
];

export function ProtectionPage() {
  const toast = useToast();
  const { settings, reload } = useSettings();
  const [protection, setProtection] = useState<ProtectionSettings | null>(null);
  const [lockdownBusy, setLockdownBusy] = useState(false);

  useEffect(() => {
    void getProtectionSettings().then(setProtection).catch((e) => toast.error(errorMessage(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const patch = async (partial: Partial<ProtectionSettings>) => {
    const prev = protection;
    setProtection((p) => (p ? { ...p, ...partial } : p));
    try {
      const next = await saveProtectionSettings(partial);
      setProtection(next);
    } catch (e) {
      setProtection(prev);
      toast.error(errorMessage(e));
    }
  };

  const toggleLockdown = async (enabled: boolean) => {
    setLockdownBusy(true);
    try {
      await setLockdownMode(enabled);
      await reload();
      toast.success(enabled ? "Lockdown Mode enabled" : "Lockdown Mode disabled");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setLockdownBusy(false);
    }
  };

  if (!protection) {
    return (
      <Page title="Protection" eyebrow="Automatic protection">
        <LoadingRow label="Loading protection settings" />
      </Page>
    );
  }

  return (
    <Page title="Protection" eyebrow="Automatic protection" description="Detects abnormal connection behavior and, when enabled, blocks it automatically. Legitimate applications can open many connections - these settings are tuned to avoid flagging them.">
      <div className="stack">
        <Card>
          <div className="prot__row">
            <div>
              <p className="prot__row-title">Enable Protection</p>
              <p className="prot__row-hint">Turns automatic monitoring and blocking on or off. Firewall rules you've created manually are never affected.</p>
            </div>
            <Toggle checked={protection.enabled} onChange={(v) => void patch({ enabled: v })} label="Enable Protection" />
          </div>
        </Card>

        <Section title="Detection">
          <Card>
            <div className="prot__row">
              <div>
                <p className="prot__row-title">Detect connection spikes</p>
                <p className="prot__row-hint">Flags a sudden, unusual burst of new connections.</p>
              </div>
              <Toggle checked={protection.detectConnectionSpikes} onChange={(v) => void patch({ detectConnectionSpikes: v })} disabled={!protection.enabled} label="Detect connection spikes" />
            </div>
            <div className="rule prot__field-spaced" />
            <div className="prot__row">
              <div>
                <p className="prot__row-title">Detect repeated connections from the same IP</p>
                <p className="prot__row-hint">Flags one remote address opening connections far more often than normal.</p>
              </div>
              <Toggle checked={protection.detectRepeatedConnections} onChange={(v) => void patch({ detectRepeatedConnections: v })} disabled={!protection.enabled} label="Detect repeated connections from the same IP" />
            </div>
          </Card>
        </Section>

        <Section title="Response">
          <Card>
            <div className="prot__row">
              <div>
                <p className="prot__row-title">Automatically block suspicious IPs</p>
                <p className="prot__row-hint">When off, Thomsen Shield flags suspicious activity in Events &amp; Logs but never blocks anything on its own.</p>
              </div>
              <Toggle checked={protection.autoBlockSuspiciousIps} onChange={(v) => void patch({ autoBlockSuspiciousIps: v })} disabled={!protection.enabled} label="Automatically block suspicious IPs" />
            </div>
            <div className="rule prot__field-spaced" />
            <div className="prot__field-spaced">
              <Field label="Temporary block duration" hint="How long an automatic block lasts before it's lifted, unless you make it permanent.">
                <Select
                  value={protection.blockDuration}
                  onChange={(v) => void patch({ blockDuration: v as BlockDuration })}
                  disabled={!protection.enabled || !protection.autoBlockSuspiciousIps}
                  options={DURATIONS.map((d) => ({ value: d, label: BLOCK_DURATION_LABELS[d] }))}
                />
              </Field>
            </div>
          </Card>
        </Section>

        <Section title="Sensitivity" description="How readily Thomsen Shield flags a connection as suspicious. Normal is right for most people.">
          <Card>
            <SegmentedControl options={SENSITIVITIES} value={protection.sensitivity} onChange={(v) => void patch({ sensitivity: v })} disabled={!protection.enabled} aria-label="Sensitivity" />
            <p className="prot__sensitivity-hint">
              {protection.sensitivity === "low" && "Only flags clearly abusive patterns. Fewer false positives, slower to react."}
              {protection.sensitivity === "normal" && "Balanced for everyday use - recommended for most PCs."}
              {protection.sensitivity === "high" && "Reacts faster and flags more. Best for PCs regularly exposed to untrusted networks."}
            </p>
          </Card>
        </Section>

        <Section title="Lockdown Mode" description="Applies stricter connection rules and blocks new suspicious remote sources faster. Allowlisted traffic keeps working, and it can always be turned off from here or from the banner at the top of the app.">
          <Card>
            <div className="prot__row">
              <div>
                <p className="prot__row-title">Lockdown Mode</p>
                <p className="prot__row-hint">Meant for short periods of heightened risk, not everyday use.</p>
              </div>
              <div className="prot__lockdown-control">
                {settings.lockdownMode && <Badge tone="bad">Active</Badge>}
                <Toggle checked={settings.lockdownMode} onChange={toggleLockdown} disabled={lockdownBusy} label="Lockdown Mode" />
              </div>
            </div>
          </Card>
        </Section>
      </div>
    </Page>
  );
}
