import { useState } from "react";
import { Play, Pause, Plus, X, Minus, ShieldCheck, Siren, ChevronDown } from "lucide-react";
import { Button, Input, Notice, Secret, Segmented, Toggle } from "./components/ui";
import { api, type EntryKind, type Settings, type Style } from "./lib/api";
import { useApp } from "./lib/app";

type Key = keyof Settings["filters"];
const TYPES: { key: Key; label: string }[] = [
  { key: "emails", label: "Emails" },
  { key: "names", label: "Names" },
  { key: "addresses", label: "Addresses" },
  { key: "phones", label: "Phone numbers" },
  { key: "ips", label: "IP addresses" },
  { key: "financial", label: "Cards & bank" },
  { key: "passwords", label: "Passwords & tokens" },
  { key: "usernames", label: "Usernames" },
  { key: "dob", label: "Birth dates" },
  { key: "custom", label: "My words" },
];

/** One box for everything: the kind is guessed so there is nothing to choose. */
function guessKind(v: string): EntryKind {
  if (v.includes("@")) return "email";
  const digits = v.replace(/\D/g, "").length;
  if (digits >= 7 && /^[+\d\s().-]+$/.test(v)) return "phone";
  return "keyword";
}

export default function App() {
  const { settings, status, entries, patch, reloadEntries, toast } = useApp();
  const [value, setValue] = useState("");
  const [more, setMore] = useState(false);
  if (!settings || !status) return <div className="grid h-full place-items-center text-mute-500">Loading…</div>;
  const f = settings.filters;
  const on = status.running;

  const add = async () => {
    try { await api.addEntry(guessKind(value.trim()), value); setValue(""); await reloadEntries(); }
    catch (e) { toast(String(e), "error"); }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-9 shrink-0 items-center justify-between">
        <div data-tauri-drag-region className="flex h-full flex-1 items-center gap-2 pl-4 text-[12px] text-mute-500">
          <ShieldCheck size={14} data-tauri-drag-region /> <span data-tauri-drag-region>Privacy Blur</span>
        </div>
        <button className="grid h-9 w-11 place-items-center text-mute-400 hover:bg-ink-700" onClick={api.minimize} aria-label="Minimize"><Minus size={15} /></button>
        <button className="grid h-9 w-11 place-items-center text-mute-400 hover:bg-red-600 hover:text-white" onClick={api.close} aria-label="Close"><X size={16} /></button>
      </div>

      <main className="flex-1 overflow-y-auto px-6 pb-6">
        <div className="mx-auto max-w-[460px] space-y-7">
          {status.warning && <Notice tone="bad">{status.warning}</Notice>}
          {status.engine.last_error && <Notice>{status.engine.last_error}</Notice>}

          <div className="flex items-center justify-between pt-2">
            <div>
              <div className="text-[22px] font-semibold tracking-tight">{on ? "Protected" : "Not protected"}</div>
              <div className="mt-0.5 text-mute-400">{on ? `${status.engine.active_regions} hidden right now` : "Nothing is being hidden"}</div>
            </div>
            <button onClick={() => api.setProtection(!on)} className={`focus-ring flex items-center gap-2 rounded-full px-5 py-2.5 font-medium transition-all duration-200 active:scale-95 ${on ? "bg-ink-700 text-zinc-100 hover:bg-ink-600" : "bg-zinc-100 text-ink-950 hover:bg-white"}`}>
              {on ? <Pause size={16} /> : <Play size={16} />}{on ? "Stop" : "Start"}
            </button>
          </div>

          <section>
            <h2 className="mb-2.5 text-[12px] font-medium text-mute-500">Hide</h2>
            <div className="grid grid-cols-2 gap-x-8 rounded-2xl bg-ink-850 px-5 py-1">
              {TYPES.map(({ key, label }) => (
                <div key={key} className="flex items-center justify-between border-b border-ink-700/40 py-3 [&:nth-last-child(-n+2)]:border-0">
                  <span>{label}</span>
                  <Toggle checked={f[key]} onChange={(v) => patch((s) => { s.filters[key] = v; })} />
                </div>
              ))}
            </div>
          </section>

          <section>
            <div className="mb-2 flex items-baseline justify-between">
              <h2 className="text-[12px] font-medium text-mute-500">Blur strength</h2>
              <span className="font-mono text-[12px] text-mute-400">{settings.appearance.intensity}%</span>
            </div>
            <input type="range" min={0} max={100} value={settings.appearance.intensity} className="w-full"
              style={{ background: `linear-gradient(to right,#f4f4f5 ${settings.appearance.intensity}%,#2c2c33 ${settings.appearance.intensity}%)` }}
              onChange={(e) => patch((s) => { s.appearance.intensity = Number(e.target.value); })} />
            <div className="mt-3">
              <Segmented<Style> value={settings.appearance.style} onChange={(v) => patch((s) => { s.appearance.style = v; })}
                options={[{ value: "gaussian", label: "Blur" }, { value: "pixelate", label: "Pixels" }, { value: "solid", label: "Black box" }]} />
            </div>
          </section>

          <section>
            <h2 className="mb-2.5 text-[12px] font-medium text-mute-500">My words</h2>
            <div className="flex gap-2">
              <Input className="!border-transparent !bg-ink-850" value={value} placeholder="Name, address, email, phone, anything…" onChange={(e) => setValue(e.target.value)} onKeyDown={(e) => e.key === "Enter" && value.trim() && add()} />
              <Button kind="primary" icon={Plus} disabled={!value.trim()} onClick={add} aria-label="Add" />
            </div>
            <ul className="mt-2.5 space-y-1.5">
              {entries.map((en) => (
                <li key={en.id} className="flex items-center justify-between rounded-lg bg-ink-850 px-4 py-2.5">
                  <Secret value={en.value} />
                  <button className="rounded p-1 text-mute-500 hover:text-white" aria-label="Delete" onClick={async () => { await api.deleteEntry(en.id); reloadEntries(); }}><X size={14} /></button>
                </li>
              ))}
            </ul>
            <p className="mt-3 text-[12px] text-mute-600">Stored encrypted on this PC. Never sent anywhere.</p>
          </section>

          <section>
            <button onClick={() => setMore(!more)} className="flex items-center gap-1.5 text-[12.5px] text-mute-400 hover:text-zinc-100">
              Options <ChevronDown size={14} className={`transition-transform ${more ? "rotate-180" : ""}`} />
            </button>
            {more && (
              <div className="mt-3 animate-rise space-y-2.5">
                {([
                  ["Start with Windows", settings.general.autostart, (s: Settings, v: boolean) => { s.general.autostart = v; }],
                  ["Keep running in tray when closed", settings.general.minimize_to_tray, (s: Settings, v: boolean) => { s.general.minimize_to_tray = v; }],
                  ["Skip browsers", !settings.apps.protect_browsers, (s: Settings, v: boolean) => { s.apps.protect_browsers = !v; }],
                ] as const).map(([label, val, set]) => (
                  <div key={label} className="flex items-center justify-between"><span>{label}</span><Toggle checked={val} onChange={(v) => patch((s) => set(s, v))} /></div>
                ))}
                <div className="flex items-center justify-between pt-1">
                  <span className="text-mute-400">Shortcuts: <span className="font-mono">{settings.shortcuts.toggle}</span> start/stop · <span className="font-mono">{settings.shortcuts.panic}</span> panic</span>
                  <Button kind="ghost" icon={Siren} onClick={() => api.setPanic(!status.panic)}>{status.panic ? "Uncover" : "Panic"}</Button>
                </div>
                <Notice tone="info">
                  Sharing a <b>single window</b> in Discord/Teams/OBS shows the original text: share your <b>whole screen</b>. Administrator windows, exclusive-fullscreen games and protected video can't be covered.
                </Notice>
                <div className="pt-1"><Button kind="danger" onClick={api.quit}>Exit Privacy Blur</Button></div>
              </div>
            )}
          </section>
        </div>
      </main>
      <div className="pointer-events-none fixed bottom-4 right-4 z-50">
        {useApp().toasts.map((t) => <div key={t.id} className="animate-pop mb-2 rounded-xl border border-ink-600 bg-ink-800 px-4 py-3 text-[12.5px]">{t.text}</div>)}
      </div>
    </div>
  );
}
