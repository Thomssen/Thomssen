import { useEffect, useRef, useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { StatTile } from "@/components/ui/StatTile";
import { Sparkline } from "@/components/ui/Sparkline";
import { useNavigation } from "@/state/NavigationProvider";
import { getDashboardSummary } from "@/services/system";
import type { DashboardSummary } from "@/types";
import "./DashboardPage.css";

const POLL_MS = 3000;

/** Deliberately minimal, per spec: status, a handful of counts, one small
 * graph. No dense stats grid, no neon "cybersecurity dashboard" look. */
export function DashboardPage() {
  const { navigate } = useNavigation();
  const [summary, setSummary] = useState<DashboardSummary | null>(null);

  const pollRef = useRef<number | null>(null);
  useEffect(() => {
    let cancelled = false;
    const tick = () => void getDashboardSummary().then((s) => !cancelled && setSummary(s)).catch(() => {});
    tick();
    pollRef.current = window.setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
  }, []);

  return (
    <Page title="Dashboard" eyebrow="Thomsen Shield">
      <div className="stack">
        <div className="dash__status">
          <Card className="dash__status-card">
            <span className="dash__status-label">Protection</span>
            <Badge tone={summary?.protectionActive ? "good" : "neutral"}>{summary ? (summary.protectionActive ? "Active" : "Disabled") : "—"}</Badge>
          </Card>
          <Card className="dash__status-card">
            <span className="dash__status-label">Windows Firewall</span>
            <Badge tone={summary?.firewallActive ? "good" : "bad"}>{summary ? (summary.firewallActive ? "Active" : "Disabled") : "—"}</Badge>
          </Card>
          {summary?.lockdownActive && (
            <Card className="dash__status-card">
              <span className="dash__status-label">Lockdown Mode</span>
              <Badge tone="bad">Active</Badge>
            </Card>
          )}
        </div>

        <div className="dash__stats">
          <StatTile label="Active Connections" value={summary?.activeConnections ?? "—"} onClick={() => navigate("connections")} />
          <StatTile label="Blocked IPs" value={summary?.blockedIps ?? "—"} tone={summary && summary.blockedIps > 0 ? "warn" : "neutral"} onClick={() => navigate("blocklist")} />
          <StatTile
            label="Suspicious"
            value={summary?.suspiciousConnections ?? "—"}
            tone={summary && summary.suspiciousConnections > 0 ? "bad" : "good"}
            onClick={() => navigate("connections")}
          />
        </div>

        <Card>
          <div className="dash__graph-head">
            <h3>Live Network Activity</h3>
            <span className="dash__graph-hint">Connections per sample</span>
          </div>
          <Sparkline points={summary?.activityPoints ?? []} height={56} />
        </Card>
      </div>
    </Page>
  );
}
