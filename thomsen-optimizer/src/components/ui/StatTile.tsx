import type { ReactNode } from "react";
import "./StatTile.css";

interface StatTileProps {
  label: string;
  value: ReactNode;
  /** Small note under the value, e.g. "last 24 hours". */
  note?: ReactNode;
  tone?: "neutral" | "good" | "bad" | "warn" | "accent";
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
      {/* `key` forces a fresh mount whenever the value itself changes, which
       * is what replays the CSS entrance animation below - simpler than a
       * JS-driven count-up, and costs nothing when the value is unchanged
       * (no key change, no remount, no animation). */}
      <span className="stat-tile__value" key={String(value)}>
        {value}
      </span>
      {note && <span className="stat-tile__note">{note}</span>}
    </Tag>
  );
}
