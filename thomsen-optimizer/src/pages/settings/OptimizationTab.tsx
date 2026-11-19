import { useState } from "react";
import { Card, CardHeader } from "@/components/ui/Card";
import { Toggle } from "@/components/ui/Toggle";
import { Button } from "@/components/ui/Button";
import { Select } from "@/components/ui/Field";
import { SettingRow, Divider } from "@/components/ui/SettingRow";
import { HistoryPanel } from "@/components/HistoryPanel";
import { useToast } from "@/components/ui/Toast";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { useSave } from "./BasicTabs";

const AGES = [0, 1, 24, 72, 168].map((h) => ({ value: String(h), label: h === 0 ? "No minimum" : h < 24 ? `${h} hour` : `${h / 24} day${h === 24 ? "" : "s"}` }));

export function OptimizationTab() {
  const { settings: s, save } = useSave();
  const toast = useToast();
  const [busy, setBusy] = useState(false);

  const restorePoint = async () => {
    setBusy(true);
    try {
      await api.createRestorePoint();
      toast.success("Windows restore point created.");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Card>
        <CardHeader title="Optimization" />
        <SettingRow title="Confirm before changing settings" hint="Shows exactly what will change and asks first. Recommended. Disabling still records everything for restore.">
          <Toggle checked={s.confirmChanges} onChange={(v) => save({ confirmChanges: v })} label="Confirm before changing settings" />
        </SettingRow>
        <Divider />
        <SettingRow title="Create a Windows restore point before optimizing" hint="Needs administrator approval (a Windows prompt). Windows allows one automatic restore point every 24 hours by default.">
          <Toggle checked={s.autoRestorePoint} onChange={(v) => save({ autoRestorePoint: v })} label="Create restore point automatically" />
        </SettingRow>
        <Divider />
        <SettingRow title="Create a restore point now" hint="A system-wide Windows snapshot, in addition to this app's own change history.">
          <Button variant="secondary" loading={busy} onClick={restorePoint}>
            Create restore point
          </Button>
        </SettingRow>
        <Divider />
        <SettingRow title="Keep recent files when cleaning" hint="The System Cleaner skips files modified more recently than this, so files in active use are left alone.">
          <div style={{ width: 150 }}>
            <Select value={String(s.cleanerMinAgeHours)} onChange={(v) => save({ cleanerMinAgeHours: Number(v) })} options={AGES} aria-label="Minimum file age" />
          </div>
        </SettingRow>
        <Divider />
        <SettingRow title="Backup & data" hint="Settings, history and measurements are stored locally in your Windows profile.">
          <Button variant="secondary" onClick={() => api.openExternal("dataFolder")}>
            Open data folder
          </Button>
        </SettingRow>
      </Card>
      <HistoryPanel />
    </>
  );
}
