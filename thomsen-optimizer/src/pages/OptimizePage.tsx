import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, RefreshCw } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { ConfirmDialog } from "@/components/ui/Modal";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { useToast } from "@/components/ui/Toast";
import { HistoryPanel } from "@/components/HistoryPanel";
import { useAsync } from "@/hooks/useAsync";
import { useNavigation } from "@/state/NavigationProvider";
import { useSettings } from "@/state/SettingsProvider";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { formatBytes1024 } from "@/lib/units";
import type { ClosePlan, ProcGroup, Recommendation } from "@/types";
import type { Route } from "@/config/navigation";
import "./OptimizePage.css";

type Tab = "recommendations" | "apps" | "history";
const TABS = [
  { value: "recommendations", label: "Recommendations" },
  { value: "apps", label: "Background apps" },
  { value: "history", label: "History & restore" },
] as const;

const SEVERITY = { attention: { label: "Needs attention", tone: "warn" }, suggestion: { label: "Suggestion", tone: "neutral" }, info: { label: "Information", tone: "neutral" } } as const;
const CATEGORY: Record<string, string> = { cpu: "CPU", memory: "Memory", windows: "Windows", storage: "Storage", background: "Background", gpu: "Graphics" };

export function OptimizePage() {
  const [tab, setTab] = useState<Tab>("recommendations");
  const { navigate } = useNavigation();
  const { settings } = useSettings();
  const toast = useToast();
  const report = useAsync(() => api.analyze(), []);
  const procs = useAsync(() => api.processes(), []);
  const [openId, setOpenId] = useState<string | null>(null);
  const [selRecs, setSelRecs] = useState<Set<string>>(new Set());
  const [selApps, setSelApps] = useState<Set<string>>(new Set());
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);

  const recs = report.data?.recommendations ?? [];
  const applicable = recs.filter((r) => r.action.type === "changes");
  const groups = (procs.data ?? []).filter((g) => g.name.toLowerCase() !== "system" && g.memoryBytes > 20 * 1048576);

  const toggle = <T,>(set: Set<T>, v: T, setter: (s: Set<T>) => void) => {
    const n = new Set(set);
    n.has(v) ? n.delete(v) : n.add(v);
    setter(n);
  };

  const closePlans: ClosePlan[] = useMemo(() => groups.filter((g) => selApps.has(g.name) && !g.protected).flatMap((g) => g.members), [groups, selApps]);
  const total = selRecs.size + selApps.size;

  const apply = async () => {
    setBusy(true);
    try {
      await api.optimizeSelected({ recommendationIds: [...selRecs], close: closePlans, cleanScanId: null, cleanCategories: [] });
      navigate("results");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
      setConfirm(false);
    }
  };

  const applyOne = async (r: Recommendation) => {
    setBusy(true);
    try {
      await api.applyRecommendation(r.id);
      navigate("results");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const needsConfirm = settings?.confirmChanges ?? true;
  const startApply = () => (needsConfirm ? setConfirm(true) : void apply());

  return (
    <Page
      title="Optimize"
      eyebrow="Analysis"
      description="Recommendations are generated from measurements of this PC. Each one explains what it changes, why it may help, the downsides, and how to undo it. Nothing is applied until you choose it."
      actions={
        <Button variant="secondary" icon={<RefreshCw size={15} />} onClick={() => { void report.reload(); void procs.reload(); }} loading={report.loading || procs.loading}>
          Analyze again
        </Button>
      }
    >
      <div className="opt__tabs">
        <SegmentedControl options={TABS} value={tab} onChange={setTab} aria-label="Optimize sections" />
      </div>

      {tab === "recommendations" && (
        <Section>
          {report.loading && !report.data ? (
            <LoadingRow label="Analysing your PC" />
          ) : report.error ? (
            <EmptyState title="Analysis failed" description={report.error} />
          ) : recs.length === 0 ? (
            <EmptyState title="Nothing to recommend" description="Your settings and current resource use look fine. Check back after installing new software or games." />
          ) : (
            <div className="stack stack--sm">
              {recs.map((r) => {
                const canSelect = r.action.type === "changes";
                const open = openId === r.id;
                const sev = SEVERITY[r.severity];
                return (
                  <Card key={r.id} className="rec">
                    <div className="rec__row">
                      {canSelect ? (
                        <input type="checkbox" className="rec__check" checked={selRecs.has(r.id)} onChange={() => toggle(selRecs, r.id, setSelRecs)} aria-label={`Select ${r.title}`} />
                      ) : (
                        <span className="rec__check-spacer" />
                      )}
                      <button type="button" className="rec__main" onClick={() => setOpenId(open ? null : r.id)} aria-expanded={open}>
                        <span className="rec__title">{r.title}</span>
                        <span className="rec__evidence">{r.evidence}</span>
                      </button>
                      <Badge tone={sev.tone}>{sev.label}</Badge>
                      <Badge dot={false}>{CATEGORY[r.category]}</Badge>
                      {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </div>
                    {open && (
                      <div className="rec__details">
                        <dl className="rec__dl">
                          <dt>What it changes</dt>
                          <dd>{r.changes}</dd>
                          <dt>Why it might help</dt>
                          <dd>{r.why}</dd>
                          <dt>Possible downsides</dt>
                          <dd>{r.downsides}</dd>
                          <dt>Restart required</dt>
                          <dd>{r.restartRequired ? "Yes" : "No"}</dd>
                          <dt>How to reverse it</dt>
                          <dd>{r.howToReverse}</dd>
                        </dl>
                        <div className="rec__actions">
                          {r.action.type === "changes" && (
                            <Button size="sm" variant="primary" onClick={() => applyOne(r)} loading={busy}>
                              Apply this
                            </Button>
                          )}
                          {r.action.type === "navigate" && (
                            <Button size="sm" variant="secondary" onClick={() => (r.action.type === "navigate" ? (r.action.route === "optimize" ? setTab("apps") : navigate(toRoute(r.action.route))) : undefined)}>
                              Review
                            </Button>
                          )}
                        </div>
                      </div>
                    )}
                  </Card>
                );
              })}
              {applicable.length === 0 && <p className="opt__hint">None of these have an automatic fix - they are advice or lead to another page.</p>}
            </div>
          )}
        </Section>
      )}

      {tab === "apps" && (
        <Section title="Optional background applications" description="Largest user programs first. Windows components, security software, anti-cheat, GPU driver services and this app are protected and cannot be selected. Apps with open windows are asked to close, so they can offer to save your work; background-only programs are ended.">
          {procs.loading && !procs.data ? (
            <LoadingRow label="Reading running programs" />
          ) : groups.length === 0 ? (
            <EmptyState title="No programs found" />
          ) : (
            <Card flush>
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th />
                      <th>Program</th>
                      <th className="table__num">Memory</th>
                      <th className="table__num">CPU</th>
                      <th>Type</th>
                    </tr>
                  </thead>
                  <tbody>
                    {groups.slice(0, 80).map((g: ProcGroup) => (
                      <tr key={g.name}>
                        <td style={{ width: 36 }}>
                          <input type="checkbox" className="rec__check" disabled={g.protected} checked={selApps.has(g.name)} onChange={() => toggle(selApps, g.name, setSelApps)} aria-label={`Select ${g.name}`} />
                        </td>
                        <td>
                          <div className="table__primary">
                            {g.name}
                            {g.count > 1 ? ` ×${g.count}` : ""}
                          </div>
                          {g.protected && <div className="table__muted" style={{ fontSize: 11.5 }}>Protected - {g.protectedReason}</div>}
                        </td>
                        <td className="table__num">{formatBytes1024(g.memoryBytes)}</td>
                        <td className="table__num">{g.cpu.toFixed(1)}%</td>
                        <td>{g.hasWindow ? <Badge dot={false}>Has window</Badge> : <Badge dot={false}>Background</Badge>}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          )}
        </Section>
      )}

      {tab === "history" && (
        <Section>
          <HistoryPanel />
        </Section>
      )}

      {tab !== "history" && (
        <div className="opt__bar">
          <span>{total === 0 ? "Select recommendations or apps to apply." : `${selRecs.size} recommendation${selRecs.size === 1 ? "" : "s"}, ${selApps.size} app${selApps.size === 1 ? "" : "s"} selected`}</span>
          <Button variant="primary" disabled={total === 0} onClick={startApply} loading={busy}>
            Apply selected
          </Button>
        </div>
      )}

      <ConfirmDialog
        open={confirm}
        onClose={() => setConfirm(false)}
        onConfirm={apply}
        busy={busy}
        title="Apply the selected changes?"
        confirmLabel="Apply"
        message={
          <>
            {selRecs.size > 0 && <>{selRecs.size} Windows setting change{selRecs.size === 1 ? "" : "s"} will be applied and recorded so you can restore them. </>}
            {selApps.size > 0 && <>{selApps.size} application{selApps.size === 1 ? "" : "s"} ({[...selApps].slice(0, 4).join(", ")}{selApps.size > 4 ? "…" : ""}) will be asked to close - closing cannot be undone by this app; save your work first.</>}
          </>
        }
      />
    </Page>
  );
}

function toRoute(r: string): Route {
  return (["cleaner", "startup", "games", "optimize", "game-booster"].includes(r) ? r : "dashboard") as Route;
}
