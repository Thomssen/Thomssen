/** A tiny inline line graph for the Dashboard's "Live Network Activity" -
 * deliberately minimal: one muted stroke, no axes, no gridlines, no labels,
 * no chart library. Keeps the dashboard calm rather than "cybersecurity
 * dashboard with neon graphs". */
interface SparklineProps {
  points: number[];
  width?: number;
  height?: number;
  className?: string;
}

export function Sparkline({ points, width = 560, height = 64, className }: SparklineProps) {
  if (points.length < 2) {
    return (
      <div className={className} style={{ width: "100%", height, display: "flex", alignItems: "center" }}>
        <span style={{ fontSize: 12, color: "var(--text-faint)" }}>Collecting activity…</span>
      </div>
    );
  }

  const max = Math.max(1, ...points);
  const min = 0;
  const range = max - min || 1;
  const stepX = width / (points.length - 1);
  const pad = 4;

  const toXY = (v: number, i: number): [number, number] => {
    const x = i * stepX;
    const y = pad + (1 - (v - min) / range) * (height - pad * 2);
    return [x, y];
  };

  const linePath = points.map((v, i) => toXY(v, i)).map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
  const [lastX, lastY] = toXY(points[points.length - 1]!, points.length - 1);
  const areaPath = `${linePath} L${width},${height} L0,${height} Z`;

  return (
    <svg
      className={className}
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height={height}
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      {/* `points.length` is fixed (a rolling window - see ACTIVITY_HISTORY_LEN
       * on the Rust side), so the path always has the same number of
       * commands between updates and a plain CSS transition on `d` can
       * smoothly morph it tick to tick, no charting library required. */}
      <path d={areaPath} fill="var(--accent-brand-soft)" stroke="none" style={{ transition: "d var(--dur) var(--ease)" }} />
      <path
        d={linePath}
        fill="none"
        stroke="var(--accent-brand)"
        strokeWidth={1.5}
        strokeLinejoin="round"
        strokeLinecap="round"
        style={{ transition: "d var(--dur) var(--ease)" }}
      />
      <circle cx={lastX} cy={lastY} r={2.5} fill="var(--accent-brand)" style={{ transition: "cx var(--dur) var(--ease), cy var(--dur) var(--ease)" }} />
    </svg>
  );
}
