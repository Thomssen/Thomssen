import { useEffect } from "react";
import { Page, Section } from "@/components/layout/Page";
import { Card, CardHeader } from "@/components/ui/Card";
import { Meter, loadTone } from "@/components/ui/Meter";
import { LoadingRow } from "@/components/ui/Spinner";
import { Badge } from "@/components/ui/Badge";
import { useMonitor } from "@/state/MonitorProvider";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { formatBytes1024, formatDuration, formatMhz, formatPercent, formatTemp } from "@/lib/units";
import "./DashboardPage.css";

const Row = ({ k, v }: { k: string; v: React.ReactNode }) => (
  <div>
    <dt>{k}</dt>
    <dd>{v ?? "-"}</dd>
  </div>
);

export function SystemInfoPage() {
  const { sample } = useMonitor();
  const { data: hw, reload } = useAsync(() => api.systemInfo(), []);
  useEffect(() => {
    if (hw) return;
    const t = window.setInterval(() => void reload(), 2500);
    return () => window.clearInterval(t);
  }, [hw]); // eslint-disable-line react-hooks/exhaustive-deps

  const gpu = sample?.gpu;
  const memPct = sample ? (sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100 : null;

  return (
    <Page title="System Information" eyebrow="Hardware" description="Read from Windows (registry, WMI/CIM and system APIs). Fields a driver does not report are shown as not available.">
      {!hw ? (
        <LoadingRow label="Reading hardware details" />
      ) : (
        <div className="stack">
          <Card>
            <CardHeader title="Processor" />
            <dl className="kv">
              <Row k="Name" v={hw.cpu.name} />
              <Row k="Physical cores" v={hw.cpu.physicalCores} />
              <Row k="Logical processors" v={hw.cpu.logicalProcessors} />
              <Row k="Base clock" v={formatMhz(hw.cpu.baseMhz)} />
              <Row k="Current clock" v={formatMhz(hw.cpu.currentMhz)} />
              <Row k="Architecture" v={hw.cpu.architecture} />
              <Row k="Utilisation" v={<span style={{ display: "inline-flex", gap: 10, alignItems: "center" }}><span style={{ width: 120 }}><Meter value={sample?.cpuUsage ?? null} tone={loadTone(sample?.cpuUsage)} /></span>{formatPercent(sample?.cpuUsage)}</span>} />
              <Row k="Temperature" v={formatTemp(sample?.cpuTempC)} />
            </dl>
          </Card>

          <Card>
            <CardHeader title="Graphics" description={hw.gpus.length > 1 ? "Multiple adapters found." : undefined} />
            {hw.gpus.length === 0 && <p className="dash__muted">No dedicated display adapter was reported by Windows.</p>}
            {hw.gpus.map((g, i) => (
              <dl className="kv" key={i} style={{ marginBottom: i < hw.gpus.length - 1 ? 14 : 0 }}>
                <Row k="Model" v={g.name} />
                <Row k="Manufacturer" v={g.manufacturer ?? "Not available"} />
                <Row k="Video memory" v={g.vramBytes ? formatBytes1024(g.vramBytes, 0) : "Not available"} />
                <Row k="Driver version" v={g.driverVersion ?? "Not available"} />
              </dl>
            ))}
            <dl className="kv" style={{ marginTop: 6 }}>
              <Row k="Current utilisation" v={gpu?.usage != null ? formatPercent(gpu.usage) : "Not available"} />
              <Row k="Temperature" v={formatTemp(gpu?.temperatureC)} />
              <Row k="Live data source" v={gpu?.source ?? "None available on this system"} />
            </dl>
          </Card>

          <Card>
            <CardHeader title="Memory" />
            <dl className="kv">
              <Row k="Installed" v={formatBytes1024(hw.memory.totalBytes, 0)} />
              <Row k="Available now" v={sample ? formatBytes1024(sample.memoryAvailableBytes) : "-"} />
              <Row k="In use" v={sample ? `${formatBytes1024(sample.memoryUsedBytes)} (${formatPercent(memPct)})` : "-"} />
              <Row k="Speed" v={hw.memory.speedMhz ? `${hw.memory.speedMhz} MT/s` : "Not available"} />
              <Row k="Modules" v={hw.memory.modules ?? "Not available"} />
            </dl>
          </Card>

          <Card>
            <CardHeader title="Storage" />
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr><th>Drive</th><th>Type</th><th>File system</th><th className="table__num">Free</th><th className="table__num">Total</th></tr>
                </thead>
                <tbody>
                  {hw.drives.filter((d) => d.totalBytes > 0).map((d) => (
                    <tr key={d.mount}>
                      <td className="table__primary">{d.mount.replace(/\\$/, "")} <span className="table__muted">{d.name}</span></td>
                      <td>{d.removable ? "Removable" : d.kind}</td>
                      <td>{d.fileSystem}</td>
                      <td className="table__num">{formatBytes1024(d.freeBytes)}</td>
                      <td className="table__num">{formatBytes1024(d.totalBytes)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {hw.physicalDisks.length > 0 && (
              <>
                <p className="clean__label" style={{ margin: "16px 0 6px", fontSize: 11, fontWeight: 600, letterSpacing: "0.04em", textTransform: "uppercase", color: "var(--text-muted)" }}>Physical disks & health</p>
                <div className="table-wrap">
                  <table className="table">
                    <tbody>
                      {hw.physicalDisks.map((d, i) => (
                        <tr key={i}>
                          <td className="table__primary">{d.name}</td>
                          <td>{[d.mediaType, d.busType].filter(Boolean).join(" · ") || "Type not available"}</td>
                          <td className="table__num">{d.sizeBytes ? formatBytes1024(d.sizeBytes, 0) : "-"}</td>
                          <td>{d.health ? <Badge tone={d.health === "Healthy" ? "good" : "warn"}>{d.health}</Badge> : <span className="table__muted">Health not reported</span>}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}
          </Card>

          <Card>
            <CardHeader title="Operating system" />
            <dl className="kv">
              <Row k="Windows" v={hw.os.name} />
              <Row k="Version" v={hw.os.version || "-"} />
              <Row k="Build" v={hw.os.build} />
              <Row k="System architecture" v={hw.os.architecture} />
              <Row k="Uptime" v={formatDuration(sample?.uptimeSecs ?? hw.os.uptimeSecs)} />
            </dl>
          </Card>
        </div>
      )}
      <Section>{null}</Section>
    </Page>
  );
}
