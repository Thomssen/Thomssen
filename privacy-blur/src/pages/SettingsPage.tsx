import { useEffect, useState } from "react";
import { Power, EyeOff, Rocket, Bell, Keyboard, Gauge, Video, FlaskConical, Folder, LogOut, ShieldCheck, Check, X, Minus, ScanText } from "lucide-react";
import { Badge, Button, Card, Notice, PageHeader, Row, Segmented, Select, Slider, Toggle } from "../components/ui";
import { api, type CaptureResult, type Method, type Scope, type Settings } from "../lib/api";
import { useApp } from "../lib/app";

function keyName(e: KeyboardEvent): string | null {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit\d$/.test(c)) return c.slice(5);
  if (/^F\d{1,2}$/.test(c)) return c;
  if (["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"].includes(c)) return null;
  return c; // Space, ArrowUp, Backquote …
}

function ShortcutField({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [rec, setRec] = useState(false);
  useEffect(() => {
    if (!rec) return;
    const h = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") return setRec(false);
      const k = keyName(e);
      if (!k) return;
      const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean) as string[];
      if (mods.length === 0) return; // global shortcuts need a modifier
      onChange([...mods, k].join("+"));
      setRec(false);
    };
    window.addEventListener("keydown", h, true);
    return () => window.removeEventListener("keydown", h, true);
  }, [rec, onChange]);
  return (
    <div className="flex items-center gap-2">
      <button onClick={() => setRec(!rec)} className={`focus-ring min-w-[170px] rounded-lg border px-3 py-2 font-mono text-[12.5px] transition-colors ${rec ? "border-zinc-200 bg-ink-800 text-zinc-100" : "border-ink-700 bg-ink-900 text-zinc-200 hover:border-ink-500"}`}>
        {rec ? "Press keys… (Esc cancels)" : value || "Not set"}
      </button>
      {value && !rec && <Button kind="ghost" icon={X} aria-label="Clear" onClick={() => onChange("")} />}
    </div>
  );
}

function CaptureCard() {
  const { settings, patch, toast } = useApp();
  const [res, setRes] = useState<CaptureResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [hold, setHold] = useState(0);
  if (!settings) return null;
  const inCaps = settings.capture.overlay_in_captures;
  const run = async () => {
    setBusy(true);
    try { setRes(await api.captureTest(hold)); } catch (e) { toast(String(e), "error"); }
    setBusy(false);
  };
  const verdict = (r: CaptureResult) => {
    if (!r.verified) return <Badge tone="warn"><Minus size={11} />Not verified</Badge>;
    return r.overlay_visible ? <Badge tone="good"><Check size={11} />Overlay recorded</Badge> : <Badge tone="bad"><X size={11} />Overlay not recorded</Badge>;
  };
  return (
    <Card title="Screen sharing & recording" subtitle="Which capture methods show the blur and which show the original screen.">
      <Row icon={Video} title="Show the blur in screen captures" desc={inCaps ? "On: the overlay is an ordinary window, so tools that capture the whole screen record the blurred version. Recommended for streaming." : "Off: the overlay is excluded from all captures. Only you see the blur; recordings and streams show the original, unblurred screen."}>
        <Toggle checked={inCaps} onChange={(v) => patch((s) => { s.capture.overlay_in_captures = v; })} />
      </Row>
      <div className="mt-3 space-y-2">
        <Notice tone="warn"><b>Window sharing is not protected.</b> Capturing a single window (Discord/Teams “Window”, OBS “Window Capture”) renders that window alone, without Privacy Blur's overlay. Share your <b>entire screen</b>.</Notice>
        <Notice tone="info">Nothing is claimed for OBS, Discord, Teams or Zoom until you check it: run the test below, then use “Keep test patch visible” and look at the patch in that app's own preview. If the magenta patch is visible there, the blur will be too.</Notice>
      </div>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Button kind="primary" icon={FlaskConical} disabled={busy} onClick={run}>{busy ? "Testing…" : "Run capture test"}</Button>
        <label className="flex items-center gap-2 text-[12.5px] text-mute-400">
          Keep test patch visible
          <Select value={String(hold)} options={[{ value: "0", label: "No" }, { value: "15", label: "15 s" }, { value: "30", label: "30 s" }, { value: "60", label: "60 s" }]} onChange={(v) => setHold(Number(v))} />
        </label>
      </div>
      {res && (
        <ul className="mt-4 space-y-2">
          {res.map((r) => (
            <li key={r.id} className="rounded-lg bg-ink-900 p-3">
              <div className="flex items-start justify-between gap-3"><span className="font-medium">{r.name}</span>{verdict(r)}</div>
              <p className="mt-1 text-[12px] text-mute-400">{r.detail}</p>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

export default function SettingsPage() {
  const { settings, patch } = useApp();
  const [folder, setFolder] = useState("");
  useEffect(() => { api.dataFolder().then(setFolder); }, []);
  if (!settings) return null;
  const g = settings.general, n = settings.notifications, p = settings.performance, sc = settings.shortcuts;
  const sets = (fn: (s: Settings) => void) => patch(fn);
  return (
    <>
      <PageHeader title="Settings" subtitle="Startup, shortcuts, notifications and performance." />

      <Card title="General" className="mb-4">
        <Row icon={Rocket} title="Start with Windows" desc="Launches Privacy Blur minimised to the tray when you sign in."><Toggle checked={g.autostart} onChange={(v) => sets((s) => { s.general.autostart = v; })} /></Row>
        <Row icon={Power} title="Start protection on launch" desc="Begin censoring as soon as the app starts."><Toggle checked={g.protect_on_launch} onChange={(v) => sets((s) => { s.general.protect_on_launch = v; })} /></Row>
        <Row icon={EyeOff} title="Start minimised" desc="Do not show this window at startup."><Toggle checked={g.start_minimized} onChange={(v) => sets((s) => { s.general.start_minimized = v; })} /></Row>
        <Row icon={ShieldCheck} title="Keep running in the system tray" desc="Closing the window hides it; protection continues. Turn off to exit when the window closes."><Toggle checked={g.minimize_to_tray} onChange={(v) => sets((s) => { s.general.minimize_to_tray = v; })} /></Row>
      </Card>

      <Card title="Keyboard shortcuts" subtitle="Global: they work from any application. A shortcut needs at least one modifier key." className="mb-4">
        <Row icon={Keyboard} title="Enable / disable protection"><ShortcutField value={sc.toggle} onChange={(v) => sets((s) => { s.shortcuts.toggle = v; })} /></Row>
        <Row icon={Keyboard} title="Open settings"><ShortcutField value={sc.open_settings} onChange={(v) => sets((s) => { s.shortcuts.open_settings = v; })} /></Row>
        <Row icon={Keyboard} title="Panic: cover every screen" desc="Instantly covers all monitors in black. Press again to remove."><ShortcutField value={sc.panic} onChange={(v) => sets((s) => { s.shortcuts.panic = v; })} /></Row>
      </Card>

      <Card title="Notifications" className="mb-4">
        <Row icon={Bell} title="Protection started / paused"><Toggle checked={n.on_toggle} onChange={(v) => sets((s) => { s.notifications.on_toggle = v; })} /></Row>
        <Row icon={Bell} title="Panic cover"><Toggle checked={n.on_panic} onChange={(v) => sets((s) => { s.notifications.on_panic = v; })} /></Row>
        <Row icon={Bell} title="Problems (for example OCR unavailable)"><Toggle checked={n.on_error} onChange={(v) => sets((s) => { s.notifications.on_error = v; })} /></Row>
      </Card>

      <Card title="Performance" subtitle="Faster updates follow scrolling more closely but use more CPU." className="mb-4">
        <Slider label="Detection update interval" value={p.interval_ms} min={100} max={2000} step={50} unit=" ms" onChange={(v) => sets((s) => { s.performance.interval_ms = v; })} hint="Time between screen checks. 150–300 ms is smooth; 500+ ms saves power." />
        <Slider label="Keep a blur after text disappears" value={p.hold_ms} min={0} max={3000} step={100} unit=" ms" onChange={(v) => sets((s) => { s.performance.hold_ms = v; })} hint="Prevents flicker when OCR misses a word for a single frame." />
        <Slider label="OCR image scale" value={p.ocr_scale} min={40} max={100} step={5} unit="%" onChange={(v) => sets((s) => { s.performance.ocr_scale = v; })} hint="Lower is faster but small text may be missed. 100 % is most accurate." />
        <Row icon={ScanText} title="Detection method" desc="Hybrid combines UI Automation (exact positions) with OCR (works everywhere, including games, remote desktops and images).">
          <Select<Method> value={p.method} options={[{ value: "hybrid", label: "Hybrid (recommended)" }, { value: "uia_only", label: "UI Automation only" }, { value: "ocr_only", label: "OCR only" }]} onChange={(v) => sets((s) => { s.performance.method = v; })} />
        </Row>
        <Row icon={Gauge} title="Scan area" desc="Foreground window only is the lightest option.">
          <Segmented<Scope> value={p.scope} options={[{ value: "all_windows", label: "All windows" }, { value: "foreground_only", label: "Foreground only" }]} onChange={(v) => sets((s) => { s.performance.scope = v; })} />
        </Row>
      </Card>

      <div className="mb-4"><CaptureCard /></div>

      <Card title="Privacy & data" className="mb-4">
        <ul className="list-disc space-y-1.5 pl-5 text-[12.5px] leading-relaxed text-mute-400">
          <li>Screenshots exist only in memory, are analysed on this PC with the Windows OCR engine, and are discarded.</li>
          <li>No network code, accounts, telemetry or updates. The app makes no internet connections.</li>
          <li>Detected text is never stored or logged; only counts and rectangles are kept.</li>
          <li>Your personal entries are encrypted with Windows DPAPI (current Windows user only).</li>
        </ul>
        <div className="mt-4 flex items-center justify-between gap-3 rounded-lg bg-ink-900 px-3 py-2.5">
          <span className="flex min-w-0 items-center gap-2 text-[12.5px] text-mute-400"><Folder size={14} /><span className="truncate font-mono">{folder}</span></span>
          <Button kind="danger" icon={LogOut} onClick={api.quit}>Exit Privacy Blur</Button>
        </div>
      </Card>
    </>
  );
}
