import type { ReactNode } from "react";
import "./SettingRow.css";

/** A label + hint on the left, a control on the right - the settings layout used across Thomsen apps. */
export function SettingRow({ title, hint, children }: { title: ReactNode; hint?: ReactNode; children?: ReactNode }) {
  return (
    <div className="set__row">
      <div>
        <p className="set__row-title">{title}</p>
        {hint && <p className="set__row-hint">{hint}</p>}
      </div>
      {children && <div className="set__row-control">{children}</div>}
    </div>
  );
}

export function Divider() {
  return <div className="rule set__spaced" />;
}
