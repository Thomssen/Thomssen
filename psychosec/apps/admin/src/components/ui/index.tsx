import clsx from "clsx";
import type { ButtonHTMLAttributes, HTMLAttributes, InputHTMLAttributes, ReactNode } from "react";

export const cn = clsx;

export function Card({ className, ...p }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("rounded-lg border border-line bg-panel", className)} {...p} />;
}
export function CardHeader({ title, right }: { title: string; right?: ReactNode }) {
  return <div className="flex items-center justify-between border-b border-line px-4 py-3"><h2 className="text-sm font-medium tracking-wide text-fg">{title}</h2>{right}</div>;
}
type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "default" | "danger" | "quiet" };
export function Button({ variant = "default", className, ...p }: BtnProps) {
  return <button className={cn("inline-flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm transition disabled:cursor-not-allowed disabled:opacity-40",
    variant === "default" && "border-line bg-bg text-fg hover:border-accent hover:text-accent",
    variant === "danger" && "border-bad/40 bg-bad/10 text-bad hover:bg-bad/20",
    variant === "quiet" && "border-transparent text-dim hover:text-fg", className)} {...p} />;
}
export function Input(p: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...p} className={cn("w-full rounded-md border border-line bg-bg px-3 py-1.5 text-sm text-fg placeholder:text-dim focus:border-accent focus:outline-none", p.className)} />;
}
export function Badge({ tone = "dim", children }: { tone?: "ok" | "bad" | "warn" | "accent" | "dim"; children: ReactNode }) {
  const t = { ok: "text-ok border-ok/30 bg-ok/10", bad: "text-bad border-bad/30 bg-bad/10", warn: "text-warn border-warn/30 bg-warn/10", accent: "text-accent border-accent/30 bg-accent/10", dim: "text-dim border-line" }[tone];
  return <span className={cn("inline-block rounded border px-1.5 py-0.5 text-xs", t)}>{children}</span>;
}
export function Table({ head, children, empty }: { head: string[]; children: ReactNode; empty?: string }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-left text-sm">
        <thead><tr className="border-b border-line text-xs uppercase tracking-wider text-dim">{head.map((h) => <th key={h} className="px-4 py-2 font-normal">{h}</th>)}</tr></thead>
        <tbody className="divide-y divide-line [&>tr:hover]:bg-white/[0.02]">{children}</tbody>
      </table>
      {empty && <p className="px-4 py-8 text-center text-sm text-dim">{empty}</p>}
    </div>
  );
}
export const Td = ({ className, ...p }: HTMLAttributes<HTMLTableCellElement>) => <td className={cn("px-4 py-2.5 align-middle", className)} {...p} />;
export function Avatar({ url, name }: { url: string | null; name: string }) {
  // eslint-disable-next-line @next/next/no-img-element
  return url ? <img src={url} alt="" className="h-7 w-7 rounded-full border border-line" /> : <span className="grid h-7 w-7 place-items-center rounded-full border border-line bg-bg text-xs text-dim">{name.slice(0, 1).toUpperCase()}</span>;
}
export function Stat({ label, value, tone }: { label: string; value: number | string; tone?: string }) {
  return <Card className="p-4"><div className="text-xs uppercase tracking-wider text-dim">{label}</div><div className={cn("mt-2 text-2xl font-semibold tabular-nums", tone)}>{value}</div></Card>;
}
export function ErrorNote({ message }: { message: string | null }) {
  return message ? <div className="rounded-md border border-bad/30 bg-bad/10 px-3 py-2 text-sm text-bad">{message}</div> : null;
}
export function Loading() { return <div className="py-10 text-center text-sm text-dim">Loading…</div>; }
