import type { ReactNode } from "react";
import "./StatTile.css";

interface StatTileProps {
  label: string;
  value: ReactNode;
  /** Small note under the value, e.g. "last 24 hours". */
  note?: ReactNode;
  tone?: "neutral" | "good" | "bad" | "warn";
  onClick?: () => void;
}

export function StatTile({ label, value, note, tone = "neutral", onClick }: StatTileProps) {
  const Tag = onClick ? "button" : "div";
  return (
    <Tag
      className={["stat-tile", `stat-tile--${tone}`, onClick ? "stat-tile--button" : ""].filter(Boolean).join(" ")}
      onClick={onClick}
      type={onClick ? "button" : undefined}
    >
      <span className="stat-tile__label">{label}</span>
      <span className="stat-tile__value">{value}</span>
      {note && <span className="stat-tile__note">{note}</span>}
    </Tag>
  );
}
