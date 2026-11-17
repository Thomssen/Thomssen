import { useCallback, useEffect, useState } from "react";
import { Globe, MonitorSmartphone, ShieldCheck, Ban, RefreshCw, Plus, X, Monitor } from "lucide-react";
import { Badge, Button, Card, Input, PageHeader, Row, Segmented, Toggle } from "../components/ui";
import { api } from "../lib/api";
import { useApp } from "../lib/app";

type AppRow = { exe: string; running: boolean; browser: boolean };

export default function Apps() {
  const { settings, status, patch } = useApp();
  const [apps, setApps] = useState<AppRow[]>([]);
  const [manual, setManual] = useState("");
  const refresh = useCallback(async () => setApps(await api.listApps()), []);
  useEffect(() => { refresh(); const t = setInterval(refresh, 4000); return () => clearInterval(t); }, [refresh]);
  if (!settings) return null;
  const a = settings.apps;

  const toList = (exe: string, list: "selected" | "excluded") =>
    patch((s) => {
      const other = list === "selected" ? "excluded" : "selected";
      s.apps[other] = s.apps[other].filter((x) => x !== exe);
      if (!s.apps[list].includes(exe)) s.apps[list].push(exe);
    });
  const remove = (exe: string, list: "selected" | "excluded") => patch((s) => { s.apps[list] = s.apps[list].filter((x) => x !== exe); });
  const addManual = () => {
    let n = manual.trim().toLowerCase();
    if (!n) return;
    if (!n.endsWith(".exe")) n += ".exe";
    toList(n, a.mode === "selected" ? "selected" : "excluded");
    setManual("");
  };

  const chips = (list: "selected" | "excluded") => (
    <div className="flex flex-wrap gap-1.5">
      {a[list].length === 0 && <span className="text-mute-500">None</span>}
      {a[list].map((exe) => (
        <span key={exe} className="inline-flex items-center gap-1.5 rounded-full bg-ink-900 py-1 pl-3 pr-1.5 font-mono text-[12px] ring-1 ring-ink-700">
          {exe}
          <button onClick={() => remove(exe, list)} className="focus-ring rounded-full p-0.5 text-mute-500 hover:bg-ink-700 hover:text-white" aria-label={`Remove ${exe}`}><X size={12} /></button>
        </span>
      ))}
    </div>
  );

  return (
    <>
      <PageHeader title="Applications" subtitle="Decide where protection applies." />
      <Card className="mb-4">
        <Row icon={ShieldCheck} title="Protection scope" desc={a.mode === "all" ? "Every application is protected, except the ones you exclude." : "Only the applications you select below are protected."}>
          <Segmented value={a.mode} options={[{ value: "all", label: "All applications" }, { value: "selected", label: "Selected only" }]} onChange={(v) => patch((s) => { s.apps.mode = v; })} />
        </Row>
        <Row icon={Globe} title="Browser protection" desc="Chrome, Edge, Firefox, Brave, Opera, Vivaldi and other browsers. Turn off to leave all browsers untouched.">
          <Toggle checked={a.protect_browsers} onChange={(v) => patch((s) => { s.apps.protect_browsers = v; })} />
        </Row>
        <Row icon={MonitorSmartphone} title="Protect Privacy Blur itself" desc="Off by default so you can read this window. Turn on before sharing your screen if saved entries could be visible.">
          <Toggle checked={a.protect_self} onChange={(v) => patch((s) => { s.apps.protect_self = v; })} />
        </Row>
      </Card>

      <Card title="Running applications" subtitle="Applications with a visible window right now." right={<Button kind="ghost" icon={RefreshCw} onClick={refresh}>Refresh</Button>} className="mb-4">
        <ul className="max-h-[300px] space-y-1.5 overflow-y-auto pr-1">
          {apps.filter((x) => x.running).map((x) => {
            const sel = a.selected.includes(x.exe), exc = a.excluded.includes(x.exe);
            return (
              <li key={x.exe} className="flex items-center gap-3 rounded-lg bg-ink-900 px-3 py-2">
                <span className="min-w-0 flex-1 truncate font-mono text-[12.5px]">{x.exe}</span>
                {x.browser && <Badge>browser</Badge>}
                {sel && <Badge tone="good">protected</Badge>}
                {exc && <Badge tone="bad">excluded</Badge>}
                <Button kind={sel ? "primary" : "secondary"} icon={ShieldCheck} onClick={() => (sel ? remove(x.exe, "selected") : toList(x.exe, "selected"))}>{sel ? "Selected" : "Protect"}</Button>
                <Button kind={exc ? "danger" : "secondary"} icon={Ban} onClick={() => (exc ? remove(x.exe, "excluded") : toList(x.exe, "excluded"))}>{exc ? "Excluded" : "Exclude"}</Button>
              </li>
            );
          })}
        </ul>
      </Card>

      <div className="mb-4 grid grid-cols-2 gap-4">
        <Card title="Protected applications" subtitle={a.mode === "selected" ? "Active: only these are protected." : "Used when scope is “Selected only”."}>{chips("selected")}</Card>
        <Card title="Exclusions" subtitle="Never censored, in any mode.">{chips("excluded")}</Card>
      </div>
      <Card title="Add by executable name" className="mb-4">
        <div className="flex gap-2">
          <Input value={manual} placeholder="e.g. notepad.exe" onChange={(e) => setManual(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addManual()} />
          <Button icon={Plus} onClick={addManual}>{a.mode === "selected" ? "Protect" : "Exclude"}</Button>
        </div>
      </Card>

      <Card title="Monitors" subtitle="Each monitor can be protected independently. Scaling (100–300 %) is handled automatically.">
        {status?.monitors.map((m) => (
          <Row key={m.name} icon={Monitor} title={`${m.name.replace("\\\\.\\", "")}${m.primary ? " · primary" : ""}`} desc={`${m.width}×${m.height} px · ${m.scale_percent}% scaling · position ${m.x},${m.y}`}>
            <Toggle checked={m.enabled} onChange={(v) => patch((s) => { s.monitors.disabled = v ? s.monitors.disabled.filter((x) => x !== m.name) : [...s.monitors.disabled, m.name]; })} />
          </Row>
        ))}
      </Card>
    </>
  );
}
