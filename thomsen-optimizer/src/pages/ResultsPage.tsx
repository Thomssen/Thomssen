import { CheckCircle2, CircleAlert, Info } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { StatTile } from "@/components/ui/StatTile";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { useAsync } from "@/hooks/useAsync";
import { useNavigation } from "@/state/NavigationProvider";
import { api } from "@/services/api";
import { formatDateTime } from "@/lib/format";
import { formatBytes1024 } from "@/lib/units";
import "./ResultsPage.css";

/** Shows exactly what the last optimization did. Every figure is a count of
 * an operation that actually completed - there are no estimated gains. */
export function ResultsPage() {
  const { navigate } = useNavigation();
  const { data, loading } = useAsync(() => api.results(), []);
  const latest = data?.[0];
  const older = data?.slice(1, 6) ?? [];

  return (
    <Page title="Optimization Results" eyebrow="Summary" description="A record of what was actually done. Thomsen Optimizer does not estimate speed-ups - to see the effect on a game, measure FPS before and after.">
      {loading && !data ? (
        <LoadingRow label="Loading results" />
      ) : !latest ? (
        <EmptyState title="No optimizations yet" description="Run Optimize Now from the Dashboard or apply recommendations on the Optimize page." action={<Button onClick={() => navigate("optimize")}>Go to Optimize</Button>} />
      ) : (
        <>
          <Section>
            <Card>
              <div className="res__head">
                <div>
                  <h2 className="res__title">{latest.failed > 0 && latest.applied === 0 ? "Nothing could be applied" : latest.title}</h2>
                  <p className="res__time">{formatDateTime(latest.timestamp)}</p>
                </div>
              </div>
              <div className="res__tiles">
                <StatTile label="Completed" value={latest.applied} tone={latest.applied ? "good" : "neutral"} note="operations" />
                <StatTile label="Already set" value={latest.alreadySet} note="no change needed" />
                <StatTile label="Failed" value={latest.failed} tone={latest.failed ? "bad" : "neutral"} note="rolled back / skipped" />
                <StatTile label="Space freed" value={latest.bytesFreed ? formatBytes1024(latest.bytesFreed) : "-"} note="from cleanup" />
              </div>
              {latest.items.length === 0 ? (
                <p className="res__empty">Nothing needed to change - everything already matched the recommendation.</p>
              ) : (
                <ul className="res__list">
                  {latest.items.map((it, i) => (
                    <li key={i} className={it.ok ? "is-ok" : it.kind === "info" ? "is-info" : "is-fail"}>
                      <span className="res__icon">{it.ok ? <CheckCircle2 size={16} /> : it.kind === "info" ? <Info size={16} /> : <CircleAlert size={16} />}</span>
                      <span>
                        <span className="res__label">{it.label}</span>
                        {it.detail && <span className="res__detail">{it.detail}</span>}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
              <div className="res__actions">
                <Button variant="primary" onClick={() => navigate("performance")}>
                  Measure FPS before / after
                </Button>
                <Button variant="secondary" onClick={() => navigate("optimize")}>
                  Review history & restore
                </Button>
              </div>
            </Card>
          </Section>

          {older.length > 0 && (
            <Section title="Earlier runs">
              <Card flush>
                <div className="table-wrap">
                  <table className="table">
                    <tbody>
                      {older.map((r) => (
                        <tr key={r.id}>
                          <td className="table__primary">{r.title}</td>
                          <td>{formatDateTime(r.timestamp)}</td>
                          <td className="table__num">{r.applied} completed</td>
                          <td className="table__num">{r.failed} failed</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Card>
            </Section>
          )}
        </>
      )}
    </Page>
  );
}
