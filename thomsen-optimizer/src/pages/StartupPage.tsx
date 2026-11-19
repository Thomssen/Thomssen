import { useMemo, useState } from "react";
import { Lock, RefreshCw } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Toggle } from "@/components/ui/Toggle";
import { TextInput } from "@/components/ui/Field";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { useToast } from "@/components/ui/Toast";
import { useAsync } from "@/hooks/useAsync";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import type { StartupEntry } from "@/types";

export function StartupPage() {
  const toast = useToast();
  const { data, loading, error, reload, setData } = useAsync(() => api.startupList(), []);
  const [q, setQ] = useState("");
  const [busy, setBusy] = useState<string | null>(null);

  const rows = useMemo(() => (data ?? []).filter((e) => `${e.name} ${e.publisher ?? ""} ${e.location}`.toLowerCase().includes(q.toLowerCase())), [data, q]);
  const enabled = (data ?? []).filter((e) => e.enabled).length;

  const toggle = async (e: StartupEntry, value: boolean) => {
    setBusy(e.id);
    try {
      await api.startupSet(e.id, value);
      setData((data ?? []).map((x) => (x.id === e.id ? { ...x, enabled: value } : x)));
      toast.success(`${e.name} ${value ? "will start with Windows" : "will no longer start with Windows"}.`);
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Page
      title="Startup Manager"
      eyebrow="Windows"
      description="Programs that start when you sign in. Disabling uses the same switch as Task Manager: the entry is kept exactly where it is and can be switched back on at any time. Every change is recorded in the restore history."
      actions={
        <Button variant="secondary" icon={<RefreshCw size={15} />} onClick={() => void reload()} loading={loading}>
          Refresh
        </Button>
      }
    >
      <Section>
        <div className="table-toolbar">
          <div className="table-toolbar__search">
            <TextInput placeholder="Search startup items" value={q} onChange={(e) => setQ(e.target.value)} />
          </div>
          <span className="table-toolbar__spacer" />
          <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{enabled} enabled of {data?.length ?? 0}</span>
        </div>
        {loading && !data ? (
          <LoadingRow label="Reading startup entries" />
        ) : error ? (
          <EmptyState title="Could not read startup entries" description={error} />
        ) : rows.length === 0 ? (
          <EmptyState title={q ? "No matches" : "No startup entries"} />
        ) : (
          <Card flush>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Application</th>
                    <th>Publisher</th>
                    <th>Location</th>
                    <th>Impact</th>
                    <th>Status</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {rows.map((e) => (
                    <tr key={e.id}>
                      <td>
                        <div className="table__primary">{e.name}</div>
                        <div className="table__mono selectable" style={{ maxWidth: 380, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }} title={e.command}>{e.command}</div>
                      </td>
                      <td>{e.publisher ?? <span className="table__muted">Unknown</span>}</td>
                      <td>{e.location}</td>
                      <td title="Windows does not expose a reliable per-program startup cost to normal applications, so none is invented here.">
                        <span className="table__muted">Not measured</span>
                      </td>
                      <td>
                        <Badge tone={e.enabled ? "good" : "neutral"}>{e.enabled ? "Enabled" : "Disabled"}</Badge>
                      </td>
                      <td className="table__actions">
                        {e.protected ? (
                          <span className="table__muted" title={e.protectedReason ?? "Protected"} style={{ display: "inline-flex", gap: 6, alignItems: "center", fontSize: 12 }}>
                            <Lock size={13} /> Protected
                          </span>
                        ) : (
                          <Toggle checked={e.enabled} disabled={busy === e.id} onChange={(v) => toggle(e, v)} label={`${e.enabled ? "Disable" : "Enable"} ${e.name}`} />
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
        <p className="set__note">Entries for all users can need administrator rights to change in some Windows versions; if Windows refuses, the app says so and changes nothing. Windows security components are protected and cannot be disabled here.</p>
      </Section>
    </Page>
  );
}
