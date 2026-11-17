import { useState, type ReactNode } from "react";
import { Eye, EyeOff, type LucideIcon } from "lucide-react";

export function Card({ title, subtitle, right, children, className = "" }: { title?: string; subtitle?: string; right?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={`rounded-xl2 border border-ink-700/70 bg-ink-850 p-5 ${className}`}>
      {(title || right) && (
        <header className="mb-4 flex items-start justify-between gap-4">
          <div>
            {title && <h3 className="text-[15px] font-semibold tracking-tight text-zinc-50">{title}</h3>}
            {subtitle && <p className="mt-0.5 max-w-xl text-[12.5px] leading-relaxed text-mute-400">{subtitle}</p>}
          </div>
          {right}
        </header>
      )}
      {children}
    </section>
  );
}

export function PageHeader({ title, subtitle, right }: { title: string; subtitle: string; right?: ReactNode }) {
  return (
    <div className="mb-6 flex items-end justify-between gap-4">
      <div>
        <h1 className="text-[26px] font-semibold tracking-tight">{title}</h1>
        <p className="mt-1 text-mute-400">{subtitle}</p>
      </div>
      {right}
    </div>
  );
}

export function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`focus-ring relative h-[22px] w-[40px] shrink-0 rounded-full transition-colors duration-200 disabled:opacity-40 ${checked ? "bg-zinc-100" : "bg-ink-600"}`}
    >
      <span className={`absolute top-[3px] h-4 w-4 rounded-full transition-all duration-200 ${checked ? "left-[21px] bg-ink-950" : "left-[3px] bg-zinc-400"}`} />
    </button>
  );
}

export function Row({ icon: Icon, title, desc, children }: { icon?: LucideIcon; title: string; desc?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 border-b border-ink-700/50 py-3 last:border-0 last:pb-0 first:pt-0">
      <div className="flex min-w-0 items-start gap-3">
        {Icon && <Icon size={17} className="mt-0.5 shrink-0 text-mute-400" />}
        <div className="min-w-0">
          <div className="font-medium text-zinc-100">{title}</div>
          {desc && <div className="mt-0.5 text-[12.5px] leading-relaxed text-mute-400">{desc}</div>}
        </div>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function Slider({ label, value, min, max, step = 1, unit = "", onChange, hint }: { label: string; value: number; min: number; max: number; step?: number; unit?: string; onChange: (v: number) => void; hint?: string }) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className="py-2.5">
      <div className="mb-2 flex items-baseline justify-between">
        <label className="font-medium">{label}</label>
        <span className="font-mono text-[12.5px] text-mute-400">{value}{unit}</span>
      </div>
      <input
        type="range" min={min} max={max} step={step} value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full focus-ring"
        style={{ background: `linear-gradient(to right,#f4f4f5 ${pct}%,#2c2c33 ${pct}%)` }}
      />
      {hint && <p className="mt-1.5 text-[12px] text-mute-500">{hint}</p>}
    </div>
  );
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="inline-flex rounded-lg bg-ink-900 p-0.5 ring-1 ring-ink-700">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => onChange(o.value)}
          className={`focus-ring rounded-md px-3 py-1.5 text-[12.5px] font-medium transition-colors duration-150 ${value === o.value ? "bg-zinc-100 text-ink-950" : "text-mute-400 hover:text-zinc-100"}`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

type BtnKind = "primary" | "secondary" | "danger" | "ghost";
export function Button({ kind = "secondary", icon: Icon, children, className = "", ...rest }: { kind?: BtnKind; icon?: LucideIcon; children?: ReactNode } & React.ButtonHTMLAttributes<HTMLButtonElement>) {
  const styles: Record<BtnKind, string> = {
    primary: "bg-zinc-100 text-ink-950 hover:bg-white",
    secondary: "bg-ink-700 text-zinc-100 hover:bg-ink-600",
    danger: "bg-red-500/10 text-red-300 ring-1 ring-red-500/30 hover:bg-red-500/20",
    ghost: "text-mute-400 hover:bg-ink-800 hover:text-zinc-100",
  };
  return (
    <button {...rest} className={`focus-ring inline-flex items-center justify-center gap-2 rounded-lg px-3.5 py-2 text-[13px] font-medium transition-all duration-150 active:scale-[.98] disabled:pointer-events-none disabled:opacity-40 ${styles[kind]} ${className}`}>
      {Icon && <Icon size={15} />}
      {children}
    </button>
  );
}

export function Input({ className = "", ...rest }: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input {...rest} className={`focus-ring w-full rounded-lg border border-ink-700 bg-ink-900 px-3 py-2 text-zinc-100 placeholder:text-mute-600 focus:border-ink-500 ${className}`} />;
}

export function Select<T extends string>({ value, options, onChange }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <select value={value} onChange={(e) => onChange(e.target.value as T)} className="focus-ring rounded-lg border border-ink-700 bg-ink-900 px-3 py-2 text-zinc-100">
      {options.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );
}

export function Badge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "good" | "warn" | "bad" }) {
  const t = { neutral: "bg-ink-700 text-zinc-300", good: "bg-emerald-500/10 text-emerald-300 ring-1 ring-emerald-500/25", warn: "bg-amber-500/10 text-amber-300 ring-1 ring-amber-500/25", bad: "bg-red-500/10 text-red-300 ring-1 ring-red-500/25" }[tone];
  return <span className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11.5px] font-medium ${t}`}>{children}</span>;
}

export function Notice({ tone = "warn", children }: { tone?: "warn" | "info" | "bad"; children: ReactNode }) {
  const t = { warn: "border-amber-500/25 bg-amber-500/5 text-amber-200", info: "border-ink-600 bg-ink-800 text-zinc-300", bad: "border-red-500/30 bg-red-500/5 text-red-200" }[tone];
  return <div className={`rounded-lg border px-3.5 py-2.5 text-[12.5px] leading-relaxed ${t}`}>{children}</div>;
}

/** Text that stays masked until the user reveals it. */
export function Secret({ value }: { value: string }) {
  const [show, setShow] = useState(false);
  return (
    <span className="inline-flex items-center gap-2">
      <span className="font-mono">{show ? value : "•".repeat(Math.min(Math.max(value.length, 6), 18))}</span>
      <button onClick={() => setShow(!show)} className="focus-ring rounded p-1 text-mute-500 hover:text-zinc-100" aria-label={show ? "Hide" : "Reveal"}>
        {show ? <EyeOff size={14} /> : <Eye size={14} />}
      </button>
    </span>
  );
}
