import { Card, CardHeader } from "@/components/ui/Card";
import { Toggle } from "@/components/ui/Toggle";
import { Select } from "@/components/ui/Field";
import { SettingRow, Divider } from "@/components/ui/SettingRow";
import { ProfilePicker } from "@/components/ProfilePicker";
import { useToast } from "@/components/ui/Toast";
import { useLoadedSettings } from "@/state/SettingsProvider";
import type { Settings } from "@/types";

/** Shared save-with-toast helper for the simple tabs. */
export function useSave() {
  const { settings, update } = useLoadedSettings();
  const toast = useToast();
  const save = async (patch: Partial<Settings>) => {
    const err = await update(patch);
    if (err) toast.error(err);
  };
  return { settings, save };
}

export function GeneralTab() {
  const { settings: s, save } = useSave();
  return (
    <Card>
      <CardHeader title="General" />
      <SettingRow title="Launch on Windows startup" hint="Starts minimised in the tray so the Game Booster can react to games straight away. You can turn it off any time.">
        <Toggle checked={s.launchOnStartup} onChange={(v) => save({ launchOnStartup: v })} label="Launch on Windows startup" />
      </SettingRow>
      <Divider />
      <SettingRow title="Minimise to system tray" hint="Closing the window keeps the app running in the tray. Quit from the tray menu.">
        <Toggle checked={s.minimizeToTray} onChange={(v) => save({ minimizeToTray: v })} label="Minimise to system tray" />
      </SettingRow>
      <Divider />
      <SettingRow title="Start minimised" hint="When launched with Windows, stay in the tray instead of opening the window.">
        <Toggle checked={s.startMinimized} onChange={(v) => save({ startMinimized: v })} label="Start minimised" />
      </SettingRow>
      <Divider />
      <SettingRow title="Check for updates automatically" hint="Contacts GitHub Releases to look for a newer signed version. No other network traffic leaves the app.">
        <Toggle checked={s.checkUpdatesAutomatically} onChange={(v) => save({ checkUpdatesAutomatically: v })} label="Check for updates automatically" />
      </SettingRow>
    </Card>
  );
}

export function GamingTab() {
  const { settings: s, save } = useSave();
  return (
    <Card>
      <CardHeader title="Gaming" />
      <SettingRow title="Detect games automatically">
        <Toggle checked={s.autoDetectGames} onChange={(v) => save({ autoDetectGames: v })} label="Detect games automatically" />
      </SettingRow>
      <Divider />
      <SettingRow title="Enable Game Booster automatically" hint="Activates a game's profile when it starts (per-game auto boost must also be on).">
        <Toggle checked={s.autoBoost} onChange={(v) => save({ autoBoost: v })} label="Enable Game Booster automatically" />
      </SettingRow>
      <Divider />
      <SettingRow title="Default gaming profile" hint="Used for games that have no profile of their own.">
        <div style={{ width: 210 }}>
          <ProfilePicker profiles={s.profiles} value={s.defaultProfileId} onChange={(id) => id && save({ defaultProfileId: id })} />
        </div>
      </SettingRow>
      <Divider />
      <SettingRow title="Restore settings when games close" hint="Recommended. Turn off only if you want changes to stay until you restore them yourself.">
        <Toggle checked={s.restoreWhenGameCloses} onChange={(v) => save({ restoreWhenGameCloses: v })} label="Restore settings when games close" />
      </SettingRow>
    </Card>
  );
}

const REFRESH = [500, 1000, 2000, 5000].map((v) => ({ value: String(v), label: v < 1000 ? "0.5 seconds" : `${v / 1000} second${v === 1000 ? "" : "s"}` }));
const RETENTION = [7, 30, 90, 365].map((v) => ({ value: String(v), label: `${v} days` }));

export function MonitoringTab() {
  const { settings: s, save } = useSave();
  return (
    <Card>
      <CardHeader title="Monitoring" description="The app reads sensors only as often as needed: live rate while a monitoring page is open, every few seconds elsewhere, and rarely while minimised or gaming." />
      <SettingRow title="Live refresh rate" hint="How often graphs update while the Dashboard or Performance page is open. During a game the app never samples faster than every 2 seconds.">
        <div style={{ width: 150 }}>
          <Select value={String(s.refreshMs)} onChange={(v) => save({ refreshMs: Number(v) })} options={REFRESH} aria-label="Refresh rate" />
        </div>
      </SettingRow>
      <Divider />
      <SettingRow title="Enable FPS monitoring" hint="Allows FPS capture through PresentMon on the Performance page. Nothing runs until you press Start.">
        <Toggle checked={s.fpsMonitoring} onChange={(v) => save({ fpsMonitoring: v })} label="Enable FPS monitoring" />
      </SettingRow>
      <Divider />
      <SettingRow title="Performance history retention" hint="Saved FPS sessions older than this are deleted automatically. Data never leaves your PC.">
        <div style={{ width: 150 }}>
          <Select value={String(s.historyRetentionDays)} onChange={(v) => save({ historyRetentionDays: Number(v) })} options={RETENTION} aria-label="History retention" />
        </div>
      </SettingRow>
      <Divider />
      <SettingRow title="Temperature monitoring" hint="Reads CPU/GPU temperatures where Windows or the GPU driver exposes them. Many PCs do not expose CPU temperature without a vendor driver - the app will say 'Unavailable' rather than guess, and installs no drivers.">
        <Toggle checked={s.temperatureMonitoring} onChange={(v) => save({ temperatureMonitoring: v })} label="Temperature monitoring" />
      </SettingRow>
    </Card>
  );
}
