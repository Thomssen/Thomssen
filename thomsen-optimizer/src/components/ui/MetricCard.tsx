import type { ReactNode } from "react";
import { Meter } from "./Meter";
import { Sparkline } from "./Sparkline";
import "./MetricCard.css";

interface MetricCardProps {
  label: string;
  /** Headline value, e.g. "37%". Use "Unavailable" for unsupported sensors. */
  value: ReactNode;
  sub?: ReactNode;
  /** 0-100 renders a meter; omit for none. */
  percent?: number | null;
  tone?: "accent" | "good" | "warn" | "bad";
  trend?: number[];
  icon?: ReactNode;
  unavailable?: boolean;
}

export function MetricCard({ label, value, sub, percent, tone, trend, icon, unavailable }: MetricCardProps) {
  return (
    <div className={["metric", unavailable ? "metric--off" : ""].filter(Boolean).join(" ")}>
      <div className="metric__head">
        <span className="metric__label">{label}</span>
        {icon && <span className="metric__icon">{icon}</span>}
      </div>
      <div className="metric__value">{value}</div>
      {sub && <div className="metric__sub">{sub}</div>}
      {percent !== undefined && <Meter value={unavailable ? null : percent} tone={tone} label={label} />}
      {trend && trend.length > 1 && !unavailable && (
        <div className="metric__trend">
          <Sparkline points={trend} height={34} width={240} />
        </div>
      )}
    </div>
  );
}
