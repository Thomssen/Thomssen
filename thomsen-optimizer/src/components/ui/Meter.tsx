import "./Meter.css";

interface MeterProps {
  /** 0-100. `null` renders an empty track (metric unavailable). */
  value: number | null;
  tone?: "accent" | "good" | "warn" | "bad";
  height?: number;
  label?: string;
}

/** Thin progress bar. Never animates beyond a short width transition. */
export function Meter({ value, tone = "accent", height = 4, label }: MeterProps) {
  const v = value == null ? 0 : Math.max(0, Math.min(100, value));
  return (
    <div className="meter" style={{ height }} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={value == null ? undefined : Math.round(v)}>
      <div className={`meter__fill meter__fill--${tone}`} style={{ width: `${v}%` }} />
    </div>
  );
}

/** Tone for a load percentage: calm until genuinely high. */
export function loadTone(v: number | null | undefined): MeterProps["tone"] {
  if (v == null) return "accent";
  return v >= 90 ? "bad" : v >= 75 ? "warn" : "accent";
}
