import { useEffect, useMemo, useState } from "react";
import { Area, AreaChart, CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { Camera, Square, Trash2 } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card, CardHeader } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { MetricCard } from "@/components/ui/MetricCard";
import { loadTone } from "@/components/ui/Meter";
import { Select, TextInput, Field } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { EmptyState } from "@/components/ui/EmptyState";
import { Toggle } from "@/components/ui/Toggle";
import { useToast } from "@/components/ui/Toast";
import { SettingRow } from "@/components/ui/SettingRow";
import { useMonitor } from "@/state/MonitorProvider";
import { useLoadedSettings } from "@/state/SettingsProvider";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { formatDateTime } from "@/lib/format";
import { formatBytes1024, formatChange, formatDuration, formatPercent, formatRate, formatTemp, fpsText } from "@/lib/units";
import type { Comparison, FpsSession, LiveFps } from "@/types";
import "./PerformancePage.css";

const GRID = "var(--border)";
const AXIS = { fontSize: 10.5, fill: "var(--text-faint)" } as const;
const TIP = { background: "var(--surface-2)", border: "1px solid var(--border-strong)", borderRadius: 8, fontSize: 12, color: "var(--text)" } as const;

export function PerformancePage() {
  const { sample, history } = useMonitor();
  const { settings, update } = useLoadedSettings();
  const toast = useToast();
  const status = useAsync(() => api.fpsStatus(), []);
  const games = useAsync(() => api.games(false), []);
  const sessions = useAsync(() => api.fpsSessions(), []);
  const [live, setLive] = useState<LiveFps | null>(null);
  const [gameId, setGameId] = useState("");
  const [starting, setStarting] = useState(false);
  const [saveOpen, setSaveOpen] = useState(false);
  const [label, setLabel] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [cmp, setCmp] = useState<Comparison | null>(null);

  const running = (games.data ?? []).filter((g) => g.running);
  useEffect(() => {
    if (!gameId && running[0]) setGameId(running[0].id);
  }, [running.length]); // eslint-disable-line react-hooks/exhaustive-deps

  // Refresh which games run while this page is open.
  useEffect(() => {
    const t = window.setInterval(() => void games.reload(), 5000);
    return () => window.clearInterval(t);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Poll the capture only while one is running.
  const capturing = status.data?.capturing ?? false;
  useEffect(() => {
    if (!capturing) {
      setLive(null);
      return;
    }
    const t = window.setInterval(() => void api.fpsLive().then(setLive).catch(() => {}), 1000);
    void api.fpsLive().then(setLive).catch(() => {});
    return () => window.clearInterval(t);
  }, [capturing]);

  const chart = useMemo(() => history.map((h, i) => ({ i, cpu: Math.round(h.cpu), gpu: h.gpu == null ? null : Math.round(h.gpu), mem: Math.round(h.mem), disk: +h.diskMBps.toFixed(2), down: +h.netDownMBps.toFixed(2), up: +h.netUpMBps.toFixed(2) })), [history]);
  const ft = useMemo(() => (live?.recent ?? []).map((v, i) => ({ i, ms: +v.toFixed(2) })), [live]);

  const start = async () => {
    setStarting(true);
    try {
      await api.fpsStart(gameId);
      await status.reload();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setStarting(false);
    }
  };
  const stop = async (save: boolean) => {
    try {
      const s = await api.fpsStop(save, label || "Untitled");
      setSaveOpen(false);
      setLabel("");
      await status.reload();
      if (s) {
        toast.success(`Saved: ${fpsText(s.stats.avgFps)} FPS average over ${s.stats.frames.toLocaleString()} frames.`);
        await sessions.reload();
      }
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };
  const choosePresentMon = async () => {
    const p = await api.pickExecutable("Choose the PresentMon console executable");
    if (!p) return;
    const err = await update({ presentmonPath: p });
    if (err) toast.error(err);
    else await status.reload();
  };
  const togglePick = (id: string) => setPicked((p) => (p.includes(id) ? p.filter((x) => x !== id) : [...p, id].slice(-2)));
  const compare = async () => {
    if (picked.length !== 2) return;
    try {
      setCmp(await api.fpsCompare(picked[0]!, picked[1]!));
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const st = status.data;
  const gpu = sample?.gpu;

  return (
    <Page title="Performance" eyebrow="Monitoring" description="Real-time system load and measured frame rates. FPS comes from PresentMon; if a game's frames cannot be measured the numbers say 'Unavailable' - they are never simulated.">
      <Section title="System load">
        <div className="perf__metrics">
          <MetricCard label="CPU" value={formatPercent(sample?.cpuUsage)} percent={sample?.cpuUsage ?? null} tone={loadTone(sample?.cpuUsage)} sub={`Temperature: ${formatTemp(sample?.cpuTempC)}`} />
          <MetricCard label="GPU" unavailable={gpu?.usage == null} value={gpu?.usage != null ? formatPercent(gpu.usage) : "Unavailable"} percent={gpu?.usage ?? null} tone={loadTone(gpu?.usage)} sub={`Temperature: ${formatTemp(gpu?.temperatureC)}`} />
          <MetricCard label="RAM" value={sample ? formatPercent((sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100) : "-"} percent={sample ? (sample.memoryUsedBytes / Math.max(1, sample.memoryTotalBytes)) * 100 : null} sub={sample ? `${formatBytes1024(sample.memoryUsedBytes)} of ${formatBytes1024(sample.memoryTotalBytes)}` : undefined} />
          <MetricCard label="VRAM" unavailable={gpu?.vramUsedMb == null} value={gpu?.vramUsedMb != null ? formatBytes1024(gpu.vramUsedMb * 1048576) : "Unavailable"} percent={gpu?.vramUsedMb != null && gpu.vramTotalMb ? (gpu.vramUsedMb / gpu.vramTotalMb) * 100 : null} sub={gpu?.vramTotalMb ? `of ${formatBytes1024(gpu.vramTotalMb * 1048576)}` : undefined} />
          <MetricCard label="Disk" value={sample ? formatRate(sample.diskReadBps + sample.diskWriteBps) : "-"} sub={sample ? `R ${formatRate(sample.diskReadBps)} · W ${formatRate(sample.diskWriteBps)}` : undefined} />
          <MetricCard label="Network" value={sample ? formatRate(sample.netDownBps) : "-"} sub={sample ? `Up ${formatRate(sample.netUpBps)} · Uptime ${formatDuration(sample.uptimeSecs)}` : undefined} />
        </div>
        <Card className="perf__chart">
          <CardHeader title="Utilisation" description={`Last ${chart.length} readings · ${gpu?.source ? `GPU source: ${gpu.source}` : "no GPU counters available on this system"}`} />
          <div className="perf__plot">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chart} margin={{ top: 4, right: 8, left: -18, bottom: 0 }}>
                <CartesianGrid stroke={GRID} vertical={false} />
                <XAxis dataKey="i" hide />
                <YAxis domain={[0, 100]} tick={AXIS} tickLine={false} axisLine={false} unit="%" />
                <Tooltip contentStyle={TIP} labelFormatter={() => ""} />
                <Line type="monotone" dataKey="cpu" name="CPU" stroke="var(--accent-brand)" dot={false} strokeWidth={1.6} isAnimationActive={false} />
                <Line type="monotone" dataKey="gpu" name="GPU" stroke="var(--good)" dot={false} strokeWidth={1.4} isAnimationActive={false} connectNulls />
                <Line type="monotone" dataKey="mem" name="RAM" stroke="var(--warn)" dot={false} strokeWidth={1.2} strokeDasharray="4 3" isAnimationActive={false} />
              </LineChart>
            </ResponsiveContainer>
          </div>
          <div className="perf__legend">
            <span><i style={{ background: "var(--accent-brand)" }} />CPU</span>
            <span><i style={{ background: "var(--good)" }} />GPU</span>
            <span><i style={{ background: "var(--warn)" }} />RAM</span>
          </div>
        </Card>
      </Section>

      <Section title="Frame rate (FPS)" description="Measured with Intel PresentMon by reading Windows' frame-presentation events. Nothing is injected into the game.">
        <Card>
          <SettingRow title="Enable FPS monitoring" hint="Off by default so the app does nothing extra while you play. Capturing only runs when you press Start.">
            <Toggle checked={settings.fpsMonitoring} onChange={async (v) => { const e = await update({ fpsMonitoring: v }); if (e) toast.error(e); }} label="Enable FPS monitoring" />
          </SettingRow>

          {settings.fpsMonitoring && (
            <>
              <div className="rule set__spaced" />
              {!st?.presentmonOk ? (
                <div className="perf__setup">
                  <p className="perf__setup-title">PresentMon is not set up</p>
                  <p className="perf__setup-text">Download the PresentMon console application from Intel's GitHub releases (MIT licensed, ~2 MB) and point Thomsen Optimizer at the .exe. It is not bundled so you always get the official, current build.</p>
                  <div className="perf__setup-actions">
                    <Button variant="secondary" onClick={() => api.openExternal("presentmon")}>Open PresentMon releases</Button>
                    <Button variant="primary" onClick={choosePresentMon}>Select PresentMon…</Button>
                  </div>
                </div>
              ) : (
                <div className="perf__capture">
                  <div className="perf__capture-row">
                    <Select value={gameId} onChange={setGameId} disabled={capturing} aria-label="Game to measure" options={running.length ? running.map((g) => ({ value: g.id, label: g.name })) : [{ value: "", label: "No running game detected" }]} />
                    {!capturing ? (
                      <Button variant="primary" icon={<Camera size={15} />} onClick={start} disabled={!gameId || running.length === 0} loading={starting}>
                        Start measuring
                      </Button>
                    ) : (
                      <Button variant="danger" icon={<Square size={14} />} onClick={() => setSaveOpen(true)}>
                        Stop
                      </Button>
                    )}
                    <Button variant="ghost" onClick={choosePresentMon}>Change PresentMon…</Button>
                  </div>
                  <p className="perf__path mono" title={st.presentmonPath ?? ""}>{st.presentmonPath}</p>
                  {!st.elevated && <p className="perf__hint">If a capture reports "access denied", relaunch as administrator <button className="gb__link" onClick={() => api.relaunchElevated().catch((e) => toast.error(errorMessage(e)))}>Relaunch elevated</button> or add your user to the "Performance Log Users" group.</p>}

                  {capturing && (
                    <div className="perf__live">
                      {live?.error ? (
                        <p className="perf__error">{live.error}</p>
                      ) : live?.waitingForFrames ? (
                        <p className="perf__hint">PresentMon is running - waiting for the game to present frames…</p>
                      ) : (
                        <>
                          <div className="perf__fps">
                            <span className="perf__fps-value">{live?.currentFps != null ? live.currentFps.toFixed(0) : "-"}</span>
                            <span className="perf__fps-unit">FPS · {live?.frames.toLocaleString() ?? 0} frames · {formatDuration(live?.elapsedSecs ?? 0)}</span>
                          </div>
                          <div className="perf__plot perf__plot--sm">
                            <ResponsiveContainer width="100%" height="100%">
                              <AreaChart data={ft} margin={{ top: 4, right: 8, left: -18, bottom: 0 }}>
                                <CartesianGrid stroke={GRID} vertical={false} />
                                <XAxis dataKey="i" hide />
                                <YAxis tick={AXIS} tickLine={false} axisLine={false} unit="ms" />
                                <Tooltip contentStyle={TIP} labelFormatter={() => "Frame time"} />
                                <Area type="monotone" dataKey="ms" stroke="var(--accent-brand)" fill="var(--accent-brand-soft)" strokeWidth={1.3} isAnimationActive={false} />
                              </AreaChart>
                            </ResponsiveContainer>
                          </div>
                        </>
                      )}
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </Card>
      </Section>

      <Section title="Sessions & comparison" description="Pick two saved sessions to compare. Label them 'Before optimization' and 'After optimization'. Results are measurements of separate runs, not guaranteed improvements.">
        {sessions.data && sessions.data.length > 0 ? (
          <Card flush>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th />
                    <th>Session</th>
                    <th className="table__num">Avg FPS</th>
                    <th className="table__num">1% low</th>
                    <th className="table__num">Stability</th>
                    <th className="table__num">Frames</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {sessions.data.map((s: FpsSession) => (
                    <tr key={s.id}>
                      <td style={{ width: 34 }}><input type="checkbox" className="rec__check" checked={picked.includes(s.id)} onChange={() => togglePick(s.id)} aria-label={`Compare ${s.label}`} /></td>
                      <td>
                        <div className="table__primary">{s.label} <span className="table__muted">· {s.gameName}</span></div>
                        <div className="table__muted" style={{ fontSize: 11.5 }}>{formatDateTime(s.startedAt)}{s.profileId ? ` · profile: ${s.profileId}` : " · no profile active"}</div>
                      </td>
                      <td className="table__num">{fpsText(s.stats.avgFps)}</td>
                      <td className="table__num">{fpsText(s.stats.onePercentLowFps)}</td>
                      <td className="table__num">{s.stats.stabilityPct.toFixed(0)}%</td>
                      <td className="table__num">{s.stats.frames.toLocaleString()}</td>
                      <td className="table__actions"><Button size="sm" variant="ghost" icon={<Trash2 size={14} />} aria-label="Delete session" onClick={async () => { await api.fpsDeleteSession(s.id); setPicked((p) => p.filter((x) => x !== s.id)); await sessions.reload(); }} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="perf__compare-bar">
              <span>{picked.length === 2 ? "Ready to compare" : `Select two sessions (${picked.length}/2)`}</span>
              <Button variant="primary" size="sm" disabled={picked.length !== 2} onClick={compare}>Compare</Button>
            </div>
          </Card>
        ) : (
          <EmptyState title="No saved sessions" description="Start measuring a running game, then stop and save it. Repeat after changing something to compare." />
        )}
      </Section>

      <Modal open={saveOpen} onClose={() => setSaveOpen(false)} title="Stop and save this capture?" width={440}
        footer={<>
          <Button variant="ghost" onClick={() => stop(false)}>Discard</Button>
          <Button variant="primary" onClick={() => stop(true)}>Save session</Button>
        </>}>
        <Field label="Label" hint="For example: Before optimization, After optimization, or Competitive profile.">
          <TextInput value={label} maxLength={60} onChange={(e) => setLabel(e.target.value)} placeholder="Before optimization" />
        </Field>
        <div className="perf__chips">
          {["Before optimization", "After optimization"].map((l) => <button key={l} type="button" className="gb__chip" onClick={() => setLabel(l)}>{l}</button>)}
        </div>
      </Modal>

      <Modal open={!!cmp} onClose={() => setCmp(null)} title="Session comparison" width={620} footer={<Button variant="primary" onClick={() => setCmp(null)}>Close</Button>}>
        {cmp && <CompareView c={cmp} />}
      </Modal>
    </Page>
  );
}

function CompareView({ c }: { c: Comparison }) {
  const rows: [string, string, string, string][] = [
    ["Average FPS", fpsText(c.a.stats.avgFps), fpsText(c.b.stats.avgFps), formatChange(c.avgFpsChangePct)],
    ["1% low FPS", fpsText(c.a.stats.onePercentLowFps), fpsText(c.b.stats.onePercentLowFps), formatChange(c.lowFpsChangePct)],
    ["Frame-time stability", `${c.a.stats.stabilityPct.toFixed(0)}%`, `${c.b.stats.stabilityPct.toFixed(0)}%`, `${c.stabilityChangePts > 0 ? "+" : ""}${c.stabilityChangePts.toFixed(1)} pts`],
    ["Worst frame", `${c.a.stats.maxFrametimeMs.toFixed(1)} ms`, `${c.b.stats.maxFrametimeMs.toFixed(1)} ms`, ""],
    ["Frames captured", c.a.stats.frames.toLocaleString(), c.b.stats.frames.toLocaleString(), ""],
  ];
  const n = Math.min(c.a.frameTimes.length, c.b.frameTimes.length);
  const data = Array.from({ length: n }, (_, i) => ({ i, a: c.a.frameTimes[i], b: c.b.frameTimes[i] }));
  return (
    <div className="stack">
      {!c.sameGame && <p className="perf__error">These sessions are from different games, so the comparison is not meaningful.</p>}
      <table className="table">
        <thead>
          <tr><th /><th className="table__num">{c.a.label}</th><th className="table__num">{c.b.label}</th><th className="table__num">Change</th></tr>
        </thead>
        <tbody>
          {rows.map((r) => <tr key={r[0]}><td>{r[0]}</td><td className="table__num">{r[1]}</td><td className="table__num">{r[2]}</td><td className="table__num table__primary">{r[3]}</td></tr>)}
        </tbody>
      </table>
      <div className="perf__plot perf__plot--sm">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} margin={{ top: 4, right: 8, left: -18, bottom: 0 }}>
            <CartesianGrid stroke={GRID} vertical={false} />
            <XAxis dataKey="i" hide />
            <YAxis tick={AXIS} tickLine={false} axisLine={false} unit="ms" />
            <Tooltip contentStyle={TIP} labelFormatter={() => ""} />
            <Line dataKey="a" name={c.a.label} stroke="var(--text-muted)" dot={false} strokeWidth={1.2} isAnimationActive={false} />
            <Line dataKey="b" name={c.b.label} stroke="var(--accent-brand)" dot={false} strokeWidth={1.4} isAnimationActive={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <p className="perf__hint">{c.disclaimer}</p>
      <Badge dot={false}>Measured values, not a guarantee</Badge>
    </div>
  );
}
