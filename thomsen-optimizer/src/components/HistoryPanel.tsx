import { useState } from "react";
import { RotateCcw } from "lucide-react";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { useAsync } from "@/hooks/useAsync";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardHeader } from "@/components/ui/Card";
import { ConfirmDialog } from "@/components/ui/Modal";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { useToast } from "@/components/ui/Toast";
import { formatDateTime } from "@/lib/format";
import type { ChangeRecord } from "@/types";

const STATUS: Record<ChangeRecord["status"], { label: string; tone: "good" | "neutral" | "warn" | "bad" }> = {
  pending: { label: "Pending", tone: "warn" },
  applied: { label: "Active", tone: "good" },
  restored: { label: "Restored", tone: "neutral" },
  restoreFailed: { label: "Restore failed", tone: "bad" },
  irreversible: { label: "Cannot be undone", tone: "neutral" },
};

function scopeLabel(scope: string): string {
  if (scope.startsWith("session:")) return "Gaming session";
  return { manual: "Manual", startup: "Startup Manager", cleanup: "Cleanup", process: "Closed app" }[scope] ?? scope;
}

/** The modification history: every change the app made, with per-change restore and Restore All. */
export function HistoryPanel({ onChanged }: { onChanged?: () => void }) {
  const { data, loading, error, reload } = useAsync(() => api.history(), []);
  const toast = useToast();
  const [confirmAll, setConfirmAll] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  const records = data ?? [];
  const active = records.filter((r) => r.kind.type !== "irreversible" && (r.status === "applied" || r.status === "pending" || r.status === "restoreFailed"));

  const restoreOne = async (r: ChangeRecord) => {
    setBusy(r.id);
    try {
      await api.restoreChange(r.id);
      toast.success(`Restored: ${r.label}`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      await reload();
      onChanged?.();
    }
  };

  const restoreAll = async () => {
    setBusy("all");
    try {
      const rep = await api.restoreAll();
      if (rep.nothingToRestore) toast.success("Nothing to restore - Windows is as the app found it.");
      else if (rep.failed.length) toast.error(`Restored ${rep.restored}; ${rep.failed.length} could not be restored: ${rep.failed[0]}`);
      else toast.success(`Restored ${rep.restored} change${rep.restored === 1 ? "" : "s"}.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
      setConfirmAll(false);
      await reload();
      onChanged?.();
    }
  };

  return (
    <Card flush>
      <CardHeader
        title="Modification history"
        description="Every Windows setting this app has changed, with the original value kept so it can be put back. Closed apps and deleted files are listed for your records but cannot be restored."
        actions={
          <Button size="sm" variant="secondary" icon={<RotateCcw size={14} />} disabled={active.length === 0} onClick={() => setConfirmAll(true)}>
            Restore all ({active.length})
          </Button>
        }
      />
      {loading && !data ? (
        <LoadingRow label="Loading history" />
      ) : error ? (
        <EmptyState inset title="Could not load the history" description={error} />
      ) : records.length === 0 ? (
        <EmptyState inset title="No changes yet" description="When the app changes a Windows setting, it appears here and can be restored." />
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>Change</th>
                <th>Source</th>
                <th>When</th>
                <th>Status</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {records.slice(0, 60).map((r) => {
                const s = STATUS[r.status];
                const canRestore = r.kind.type !== "irreversible" && (r.status === "applied" || r.status === "pending" || r.status === "restoreFailed");
                return (
                  <tr key={r.id}>
                    <td>
                      <div className="table__primary">{r.label}</div>
                      {(r.error || r.detail) && <div className="table__muted" style={{ fontSize: 11.5 }}>{r.error ?? r.detail}</div>}
                    </td>
                    <td>{scopeLabel(r.scope)}</td>
                    <td>{formatDateTime(r.createdAt)}</td>
                    <td>
                      <Badge tone={s.tone}>{s.label}</Badge>
                    </td>
                    <td className="table__actions">
                      {canRestore && (
                        <Button size="sm" variant="ghost" loading={busy === r.id} onClick={() => restoreOne(r)}>
                          Restore
                        </Button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      <ConfirmDialog
        open={confirmAll}
        onClose={() => setConfirmAll(false)}
        onConfirm={restoreAll}
        busy={busy === "all"}
        title="Restore all changes?"
        confirmLabel="Restore all"
        message="This puts every setting changed by Thomsen Optimizer back to the value it had before, and ends any active gaming session. Closed apps and deleted files are not affected."
      />
    </Card>
  );
}
