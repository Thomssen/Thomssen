import { useEffect, useState } from "react";
import { Card, CardHeader } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Meter } from "@/components/ui/Meter";
import { ConfirmDialog } from "@/components/ui/Modal";
import { SettingRow, Divider } from "@/components/ui/SettingRow";
import { BrandMark } from "@/components/BrandMark";
import { useToast } from "@/components/ui/Toast";
import { useAsync } from "@/hooks/useAsync";
import { useMonitor } from "@/state/MonitorProvider";
import { api } from "@/services/api";
import { errorMessage, isTauri } from "@/services/ipc";
import { APP, COMPANY } from "@/config/app";
import { formatBytes1024 } from "@/lib/units";
import type { UpdateInfo } from "@/types";

export function AboutTab() {
  const toast = useToast();
  const { sample } = useMonitor();
  const info = useAsync(() => api.appInfo(), []);
  const [upd, setUpd] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [progress, setProgress] = useState<{ downloaded: number; total: number | null; stage: string } | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    let un: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      un = await listen<{ downloaded: number; total: number | null; stage: string }>("update://progress", (e) => setProgress(e.payload));
    });
    return () => un?.();
  }, []);

  const check = async () => {
    setChecking(true);
    try {
      setUpd(await api.updateCheck());
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setChecking(false);
    }
  };

  const install = async () => {
    setConfirm(false);
    setInstalling(true);
    try {
      await api.updateInstall();
    } catch (e) {
      toast.error(errorMessage(e));
      setInstalling(false);
      setProgress(null);
    }
  };

  const pct = progress?.total ? (progress.downloaded / progress.total) * 100 : null;

  return (
    <>
      <Card>
        <div className="about__block">
          <span className="about__logo" aria-hidden="true"><BrandMark /></span>
          <div>
            <p className="about__name">{APP.name}</p>
            <p className="about__version mono">Version {info.data?.version ?? APP.version} · Developer: {COMPANY}</p>
          </div>
        </div>
        <Divider />
        <SettingRow title="Updates" hint={upd ? (!upd.configured ? "This build has no update signing key, so automatic updates are disabled. Download new versions from GitHub Releases." : upd.available ? `Version ${upd.version} is available.` : `You are up to date (${upd.currentVersion}).`) : "Updates are downloaded from GitHub Releases and installed only if their signature verifies."}>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {upd?.available && <Badge tone="good">{upd.version}</Badge>}
            {upd?.available ? (
              <Button variant="primary" onClick={() => setConfirm(true)} loading={installing}>Install update</Button>
            ) : (
              <Button variant="secondary" onClick={check} loading={checking}>Check for updates</Button>
            )}
          </div>
        </SettingRow>
        {upd?.available && upd.notes && <p className="set__note" style={{ whiteSpace: "pre-wrap" }}>{upd.notes}</p>}
        {installing && (
          <div className="upd__bar">
            <Meter value={pct} />
            <p className="set__note">{progress?.stage === "installing" ? "Installing…" : pct != null ? `Downloading… ${pct.toFixed(0)}%` : "Starting download…"}</p>
          </div>
        )}
        <Divider />
        <SettingRow title="Resource use of this app" hint="Measured by the app itself, so you can check it stays light.">
          <span className="mono">{sample ? `${sample.selfCpu.toFixed(1)}% CPU · ${formatBytes1024(sample.selfMemoryBytes)} RAM` : "measuring…"}</span>
        </SettingRow>
        <Divider />
        <SettingRow title="Run level" hint="Runs with normal user rights. Administrator approval is requested only for the few actions that need it (restore points, some all-users startup entries).">
          <Badge tone={info.data?.elevated ? "warn" : "good"}>{info.data?.elevated ? "Administrator" : "Standard user"}</Badge>
        </SettingRow>
        <Divider />
        <div className="about__links">
          <Button size="sm" variant="secondary" onClick={() => api.openExternal("website")}>Website</Button>
          <Button size="sm" variant="secondary" onClick={() => api.openExternal("github")}>GitHub</Button>
          <Button size="sm" variant="secondary" onClick={() => api.openExternal("releases")}>Releases</Button>
        </div>
      </Card>

      <Card>
        <CardHeader title="License & privacy" />
        <p className="about__license">
          Thomsen Optimizer is free software from {COMPANY}: no payments, subscriptions, license keys, accounts or advertising. Everything - settings, change history, game library and FPS measurements - stays on your PC. The only network request the app makes is the optional update check to GitHub. FPS capture uses Intel PresentMon (MIT licence), which you download separately. The software is provided "as is" without warranty; results depend on your hardware and are never guaranteed.
        </p>
      </Card>

      <ConfirmDialog open={confirm} onClose={() => setConfirm(false)} onConfirm={install} title={`Install version ${upd?.version}?`} confirmLabel="Download and install" message="The update is downloaded from GitHub and installed only if its signature is valid. Any active gaming changes are restored first, then the app restarts." />
    </>
  );
}
