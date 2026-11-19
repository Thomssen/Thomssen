import { useEffect, useState } from "react";
import { Cpu, HardDrive, MemoryStick, MonitorPlay, Network, Activity, Undo2, Zap, Search } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card, CardHeader } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { MetricCard } from "@/components/ui/MetricCard";
import { loadTone } from "@/components/ui/Meter";
import { ConfirmDialog } from "@/components/ui/Modal";
import { OptimizeNowButton } from "@/components/OptimizeNow";
import { useToast } from "@/components/ui/Toast";
import { useMonitor } from "@/state/MonitorProvider";
import { useNavigation } from "@/state/NavigationProvider";
import { useSettings } from "@/state/SettingsProvider";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { formatBytes1024, formatDuration, formatMhz, formatPercent, formatRate, formatTemp } from "@/lib/units";
import "./DashboardPage.css";

export function DashboardPage() {
  const { sample, history, booster, refreshBooster } = useMonitor();
  const { settings } = useSettings();
  const { navigate } = useNavigation();
  const toast = useToast();
  const dash = useAsync(() => api.dashboard(), []);
  const info = useAsync(() => api.systemInfo(), []);
  const [busy, setBusy] = useState<string | null>(null);
  const [confirmRestore, setConfirmRestore] = useState(false);
  const [analysisCount, setAnalysisCount] = useState<number | null>(null);

  // Hardware details are gathered in the background right after launch.
  useEffect(() => {
    if (info.data) return;
    const t = window.setInterval(() => void info.reload(), 2500);
    return () => window.clearInterval(t);
  }, [info.data]); // eslint-disable-line react-hooks/exhaustive-deps

  // Light refresh of non-sample facts (running game, FPS, plan).
  useEffect(() => {
    const t = window.setInterval(() => void dash.reload(), 5000);
    return () => window.clearInterval(t);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const profile = settings?.profiles.find((p) => p.id === settings.defaultProfileId);
  const hw = info.data;
  const mem = sample ? (sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100 : null;
  const gpu = sample?.gpu;

  const analyze = async () => {
    setBusy("analyze");
    try {
      const r = await api.analyze();
      setAnalysisCount(r.recommendations.filter((x) => x.severity !== "info").length);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const toggleBooster = async () => {
    if (!settings) return;
    setBusy("boost");
    try {
      if (booster?.active) {
        await api.deactivateProfile();
        toast.success("Gaming changes restored.");
      } else {
        await api.activateProfile(settings.defaultProfileId, false);
        toast.success(`${profile?.name ?? "Profile"} profile activated.`);
      }
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      refreshBooster();
      void dash.reload();
    }
  };

  const restore = async () => {
    setBusy("restore");
    try {
      const rep = await api.restoreAll();
      toast.success(rep.nothingToRestore ? "Nothing to restore." : rep.failed.length ? `Restored ${rep.restored}, ${rep.failed.length} failed.` : `Restored ${rep.restored} change${rep.restored === 1 ? "" : "s"}.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      setConfirmRestore(false);
      refreshBooster();
      void dash.reload();
    }
  };

  const d = dash.data;
  return (
    <Page
      title="Dashboard"
      eyebrow="Overview"
      description="Live readings from your PC. Every value comes from Windows or your hardware - metrics that are not supported are shown as unavailable instead of being estimated."
      actions={
        <>
          <Button variant="secondary" icon={<Search size={15} />} onClick={analyze} loading={busy === "analyze"}>
            Analyze System
          </Button>
          <OptimizeNowButton onDone={() => void dash.reload()} />
        </>
      }
    >
      {analysisCount !== null && (
        <Section>
          <Card className="dash__notice">
            <p>
              <strong>{analysisCount === 0 ? "No performance issues found." : `${analysisCount} thing${analysisCount === 1 ? "" : "s"} worth a look.`}</strong>{" "}
              <span className="dash__muted">Analysis uses current measurements; nothing was changed.</span>
            </p>
            <Button size="sm" variant="secondary" onClick={() => navigate("optimize")}>
              View details
            </Button>
          </Card>
        </Section>
      )}

      <Section title="Live monitoring">
        <div className="dash__metrics">
          <MetricCard label="CPU" icon={<Cpu size={15} />} value={formatPercent(sample?.cpuUsage)} percent={sample?.cpuUsage ?? null} tone={loadTone(sample?.cpuUsage)} sub={sample?.cpuTempC != null ? formatTemp(sample.cpuTempC) : "Temperature unavailable"} trend={history.map((h) => h.cpu)} />
          <MetricCard
            label="GPU"
            icon={<MonitorPlay size={15} />}
            unavailable={!gpu || gpu.usage == null}
            value={gpu?.usage != null ? formatPercent(gpu.usage) : "No GPU counter available"}
            percent={gpu?.usage ?? null}
            tone={loadTone(gpu?.usage)}
            sub={gpu ? `${gpu.temperatureC != null ? formatTemp(gpu.temperatureC) : "Temperature unavailable"}${gpu.vramUsedMb != null ? ` · VRAM ${formatBytes1024(gpu.vramUsedMb * 1048576)}` : ""}` : undefined}
            trend={history.filter((h) => h.gpu != null).map((h) => h.gpu as number)}
          />
          <MetricCard label="Memory" icon={<MemoryStick size={15} />} value={formatPercent(mem)} percent={mem} tone={loadTone(mem)} sub={sample ? `${formatBytes1024(sample.memoryAvailableBytes)} available of ${formatBytes1024(sample.memoryTotalBytes)}` : undefined} trend={history.map((h) => h.mem)} />
          <MetricCard label="Disk activity" icon={<HardDrive size={15} />} value={sample ? formatRate(sample.diskReadBps + sample.diskWriteBps) : "-"} sub={sample ? `Read ${formatRate(sample.diskReadBps)} · Write ${formatRate(sample.diskWriteBps)}` : undefined} trend={history.map((h) => h.diskMBps)} />
          <MetricCard label="Network" icon={<Network size={15} />} value={sample ? formatRate(sample.netDownBps) : "-"} sub={sample ? `Down · Up ${formatRate(sample.netUpBps)}` : undefined} trend={history.map((h) => h.netDownMBps)} />
          <MetricCard label="Background processes" icon={<Activity size={15} />} value={sample ? String(sample.processCount) : "-"} sub={sample ? `Uptime ${formatDuration(sample.uptimeSecs)}` : undefined} />
        </div>
      </Section>

      <Section title="Gaming status">
        <div className="dash__two">
          <Card>
            <CardHeader title="Game Booster" description="Activates your chosen profile and restores everything afterwards." actions={<Badge tone={booster?.active ? "good" : "neutral"}>{booster?.active ? "Active" : "Inactive"}</Badge>} />
            <dl className="kv">
              <div>
                <dt>Active game</dt>
                <dd>{d?.runningGames.length ? d.runningGames.join(", ") : "None detected"}</dd>
              </div>
              <div>
                <dt>Gaming mode</dt>
                <dd>{booster?.active ? "On" : "Off"}</dd>
              </div>
              <div>
                <dt>Optimization profile</dt>
                <dd>{booster?.active ? booster.profileName : `${profile?.name ?? "Balanced"} (default, not active)`}</dd>
              </div>
              <div>
                <dt>Current FPS</dt>
                <dd>{d?.fps != null ? d.fps.toFixed(0) : "Unavailable (start an FPS capture on the Performance page)"}</dd>
              </div>
              <div>
                <dt>Changes that can be restored</dt>
                <dd>{d?.restorableChanges ?? 0}</dd>
              </div>
            </dl>
            <div className="dash__buttons">
              <Button variant={booster?.active ? "secondary" : "primary"} icon={<Zap size={15} />} onClick={toggleBooster} loading={busy === "boost"}>
                {booster?.active ? "Disable Game Booster" : "Enable Game Booster"}
              </Button>
              <Button variant="secondary" icon={<Undo2 size={15} />} onClick={() => setConfirmRestore(true)} disabled={(d?.restorableChanges ?? 0) === 0 && !booster?.active}>
                Restore Changes
              </Button>
            </div>
          </Card>

          <Card>
            <CardHeader title="This PC" description="From Windows and your hardware." />
            <dl className="kv">
              <div>
                <dt>Processor</dt>
                <dd>{hw ? `${hw.cpu.name}` : "Reading…"}</dd>
              </div>
              <div>
                <dt>Graphics</dt>
                <dd>{hw ? (hw.gpus.length ? hw.gpus.map((g) => g.name).join(" + ") : "Not detected") : "Reading…"}</dd>
              </div>
              <div>
                <dt>Memory</dt>
                <dd>{hw ? `${formatBytes1024(hw.memory.totalBytes, 0)}${hw.memory.speedMhz ? ` · ${hw.memory.speedMhz} MT/s` : ""}` : "Reading…"}</dd>
              </div>
              <div>
                <dt>Windows</dt>
                <dd>{hw ? `${hw.os.name} ${hw.os.version} (build ${hw.os.build})` : "Reading…"}</dd>
              </div>
              <div>
                <dt>Storage</dt>
                <dd>{hw ? hw.drives.filter((x) => !x.removable && x.totalBytes > 0).map((x) => `${x.mount.replace(/\\$/, "")} ${formatBytes1024(x.freeBytes, 0)} free / ${formatBytes1024(x.totalBytes, 0)}`).join(" · ") || "-" : "Reading…"}</dd>
              </div>
              <div>
                <dt>Power plan</dt>
                <dd>{d?.powerPlan?.name ?? "-"}</dd>
              </div>
            </dl>
          </Card>
        </div>
      </Section>

      <p className="dash__footnote">
        Thomsen Optimizer itself: {sample ? `${sample.selfCpu.toFixed(1)}% CPU · ${formatBytes1024(sample.selfMemoryBytes)} RAM` : "measuring…"} · refresh {(settings?.refreshMs ?? 1000) / 1000}s while this page is open{info.data && hw?.cpu.currentMhz ? ` · CPU clock ${formatMhz(hw.cpu.currentMhz)}` : ""}
      </p>

      <ConfirmDialog
        open={confirmRestore}
        onClose={() => setConfirmRestore(false)}
        onConfirm={restore}
        busy={busy === "restore"}
        title="Restore all changes?"
        confirmLabel="Restore"
        message="Every Windows setting changed by Thomsen Optimizer returns to its previous value and any active gaming session ends. Closed apps and deleted files cannot be brought back."
      />
    </Page>
  );
}
