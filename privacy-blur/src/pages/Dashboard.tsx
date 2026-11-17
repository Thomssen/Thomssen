import { Play, Pause, Eye, Layers, Cpu, AlertTriangle, Radio } from "lucide-react";
import { Badge, Button, Card, Notice, PageHeader } from "../components/ui";
import { api, CATEGORY_LABEL } from "../lib/api";
import { useApp } from "../lib/app";

const STYLE_LABEL = { gaussian: "Gaussian blur", pixelate: "Pixelation", solid: "Solid black" } as const;

function Stat({ icon: Icon, label, value, sub }: { icon: typeof Eye; label: string; value: string | number; sub?: string }) {
  return (
    <div className="rounded-xl2 border border-ink-700/70 bg-ink-850 p-4">
      <div className="flex items-center gap-2 text-mute-400"><Icon size={15} /><span className="text-[12px]">{label}</span></div>
      <div className="mt-2 text-[26px] font-semibold leading-none tracking-tight">{value}</div>
      {sub && <div className="mt-1.5 truncate text-[12px] text-mute-500">{sub}</div>}
    </div>
  );
}

export default function Dashboard() {
  const { status, settings, toast } = useApp();
  if (!status || !settings) return null;
  const e = status.engine;
  const cats = Object.entries(e.by_category).sort((a, b) => b[1] - a[1]);
  const maxCat = Math.max(1, ...cats.map(([, n]) => n));
  const toggle = async () => { try { await api.setProtection(!status.running); } catch (x) { toast(String(x), "error"); } };

  return (
    <>
      <PageHeader title="Dashboard" subtitle="Live view of what Privacy Blur is protecting right now." />

      {status.warning && <div className="mb-4"><Notice tone="bad">{status.warning}</Notice></div>}
      {status.panic && <div className="mb-4"><Notice tone="bad"><b>Panic cover is on.</b> Every monitor is covered in black, including in screen captures. Press your panic shortcut or use the sidebar button to remove it.</Notice></div>}

      <Card className="mb-4">
        <div className="flex items-center justify-between gap-6">
          <div className="flex items-center gap-4">
            <div className={`grid h-14 w-14 place-items-center rounded-2xl transition-colors duration-300 ${status.running ? "bg-emerald-400/10 text-emerald-300 ring-1 ring-emerald-400/30" : "bg-ink-700 text-mute-400"}`}>
              {status.running ? <Radio size={26} className="animate-pulse" /> : <Pause size={26} />}
            </div>
            <div>
              <div className="text-[20px] font-semibold tracking-tight">{status.running ? "Protection is active" : "Protection is paused"}</div>
              <div className="mt-0.5 text-mute-400">
                {status.running
                  ? `Scanning ${status.monitors.filter((m) => m.enabled).length} monitor(s) · ${STYLE_LABEL[settings.appearance.style]}`
                  : "Nothing is being censored. Start protection to blur sensitive information."}
              </div>
            </div>
          </div>
          <Button kind={status.running ? "secondary" : "primary"} icon={status.running ? Pause : Play} onClick={toggle} className="!px-5 !py-2.5">
            {status.running ? "Stop protection" : "Start protection"}
          </Button>
        </div>
      </Card>

      <div className="mb-4 grid grid-cols-4 gap-3">
        <Stat icon={Eye} label="Hidden right now" value={status.running ? e.active_regions : 0} sub="sensitive elements" />
        <Stat icon={Layers} label="Detected this session" value={e.total_detected} sub="distinct elements" />
        <Stat icon={Cpu} label="Censoring mode" value={STYLE_LABEL[settings.appearance.style].split(" ")[0]} sub={`${settings.appearance.safe_minimum ? "Safe minimum on" : "Safe minimum off"}`} />
        <Stat icon={Radio} label="Scan cycle" value={status.running ? `${e.last_cycle_ms + e.last_detect_ms} ms` : "—"} sub={`every ${settings.performance.interval_ms} ms`} />
      </div>

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="Active applications" subtitle="Applications currently visible and covered by your rules.">
          {e.visible_apps.length === 0 ? (
            <p className="text-mute-500">{status.running ? "No protected application is visible." : "Start protection to see active applications."}</p>
          ) : (
            <ul className="space-y-1.5">
              {e.visible_apps.slice(0, 8).map((exe) => {
                const n = e.apps_with_detections.find((a) => a.exe === exe)?.count ?? 0;
                return (
                  <li key={exe} className="flex items-center justify-between rounded-lg bg-ink-900 px-3 py-2">
                    <span className="truncate font-mono text-[12.5px]">{exe}</span>
                    {n > 0 ? <Badge tone="good">{n} hidden</Badge> : <span className="text-[12px] text-mute-600">clean</span>}
                  </li>
                );
              })}
            </ul>
          )}
        </Card>
        <Card title="What is hidden" subtitle="Categories currently covered (counts only, never content).">
          {cats.length === 0 ? (
            <p className="text-mute-500">Nothing sensitive is visible.</p>
          ) : (
            <ul className="space-y-2.5">
              {cats.map(([k, n]) => (
                <li key={k}>
                  <div className="mb-1 flex justify-between text-[12.5px]"><span>{CATEGORY_LABEL[k] ?? k}</span><span className="font-mono text-mute-400">{n}</span></div>
                  <div className="h-1.5 rounded-full bg-ink-700"><div className="h-full rounded-full bg-zinc-200 transition-all duration-500" style={{ width: `${(n / maxCat) * 100}%` }} /></div>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      <Card title="Engine health">
        <div className="grid grid-cols-3 gap-3 text-[12.5px]">
          <div className="rounded-lg bg-ink-900 p-3"><div className="text-mute-500">Local OCR</div><div className="mt-1 flex items-center gap-2">{e.ocr_ok ? <Badge tone="good">Ready</Badge> : <Badge tone={status.running ? "warn" : "neutral"}>{status.running ? "Unavailable" : "Idle"}</Badge>}</div><div className="mt-1.5 text-mute-500">{e.ocr || "Starts with protection"}</div></div>
          <div className="rounded-lg bg-ink-900 p-3"><div className="text-mute-500">UI Automation</div><div className="mt-1">{e.uia_ok ? <Badge tone="good">Running</Badge> : <Badge>Idle</Badge>}</div><div className="mt-1.5 text-mute-500">Exact text positions where apps expose them</div></div>
          <div className="rounded-lg bg-ink-900 p-3"><div className="text-mute-500">Monitors</div><div className="mt-1.5">{status.monitors.map((m) => <div key={m.name} className="truncate">{m.width}×{m.height} · {m.scale_percent}%{m.enabled ? "" : " · off"}</div>)}</div></div>
        </div>
        {e.last_error && <div className="mt-3"><Notice tone="warn"><AlertTriangle size={13} className="mr-1.5 inline -translate-y-px" />{e.last_error}</Notice></div>}
      </Card>

      {status.capture_apps.length > 0 && (
        <div className="mt-4">
          <Notice tone="warn">
            <b>Screen-sharing software detected</b> ({status.capture_apps.join(", ")}). Share your <b>entire screen</b>, not a single window: window capture does not include Privacy Blur's overlay.
            See Settings → Screen sharing &amp; recording to test what your capture path records.
          </Notice>
        </div>
      )}
    </>
  );
}
