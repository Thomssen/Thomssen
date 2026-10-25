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
      <path d={areaPath} fill="var(--border-faint)" stroke="none" />
      <path d={linePath} fill="none" stroke="var(--text-secondary)" strokeWidth={1.5} strokeLinejoin="round" strokeLinecap="round" />
      <circle cx={lastX} cy={lastY} r={2.5} fill="var(--text)" />
    </svg>
  );
}
