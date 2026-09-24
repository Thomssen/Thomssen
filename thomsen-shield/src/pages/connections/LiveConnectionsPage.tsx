import { useEffect, useMemo, useRef, useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Menu } from "@/components/ui/Menu";
import { Modal } from "@/components/ui/Modal";
import { EmptyState } from "@/components/ui/EmptyState";
import { TextInput } from "@/components/ui/Field";
import { SearchIcon } from "@/components/icons";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { blockConnectionApp, blockConnectionIp, getConnections } from "@/services/connections";
import { addAllowlistEntry } from "@/services/allowlist";
import type { LiveConnection } from "@/types";
import "@/components/ui/Table.css";
import "./LiveConnectionsPage.css";

const POLL_MS = 4000;

export function LiveConnectionsPage() {
  const toast = useToast();
  const [connections, setConnections] = useState<LiveConnection[] | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<LiveConnection | null>(null);
  const pollRef = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = () => void getConnections().then((c) => !cancelled && setConnections(c)).catch(() => {});
    tick();
    pollRef.current = window.setInterval(tick, POLL_MS);
    return () => {
      cancelled = true;
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
  }, []);

  const filtered = useMemo(() => {
    if (!connections) return [];
    const q = query.trim().toLowerCase();
    if (!q) return connections;
    return connections.filter((c) => c.appName.toLowerCase().includes(q) || c.remoteAddress.includes(q) || String(c.remotePort).includes(q));
  }, [connections, query]);

  const copyIp = async (ip: string) => {
    try {
      await navigator.clipboard.writeText(ip);
      toast.success(`Copied ${ip}`);
    } catch {
      toast.error("Couldn't copy to clipboard.");
    }
  };

  const doBlockIp = async (c: LiveConnection) => {
    try {
      await blockConnectionIp(c.remoteAddress);
      toast.success(`Blocked ${c.remoteAddress}`);
      setSelected(null);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const doBlockApp = async (c: LiveConnection) => {
    if (!c.processPath) {
      toast.error("No executable path known for this process.");
      return;
    }
    try {
      await blockConnectionApp(c.processPath);
      toast.success(`Blocked ${c.appName}`);
      setSelected(null);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const doAllowlist = async (c: LiveConnection) => {
    try {
      await addAllowlistEntry(c.remoteAddress);
      toast.success(`${c.remoteAddress} added to the allowlist`);
      setSelected(null);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <Page title="Live Connections" eyebrow="Network" description="Every active connection this PC currently has open, refreshed automatically.">
      <Card flush>
        <div className="table-toolbar" style={{ padding: "18px 20px 0" }}>
          <div className="table-toolbar__search">
            <SearchIcon />
            <TextInput placeholder="Filter by app, IP, or port" value={query} onChange={(e) => setQuery(e.target.value)} />
          </div>
          <div className="table-toolbar__spacer" />
          <span className="lc__count">{filtered.length} connection{filtered.length === 1 ? "" : "s"}</span>
        </div>

        {connections === null ? (
          <div className="loading-row" style={{ padding: "20px 20px 24px" }}>
            Loading connections…
          </div>
        ) : filtered.length === 0 ? (
          <EmptyState inset title="No matching connections" description="Try a different search, or wait for new activity." />
        ) : (
          <div className="table-wrap" style={{ marginTop: 14 }}>
            <table className="table">
              <thead>
                <tr>
                  <th>Application</th>
                  <th>Process</th>
                  <th>Remote IP</th>
                  <th>Remote Port</th>
                  <th>Protocol</th>
                  <th>Status</th>
                  <th aria-label="Actions" />
                </tr>
              </thead>
              <tbody>
                {filtered.map((c) => (
                  <tr key={c.id} className="is-clickable" onClick={() => setSelected(c)}>
                    <td className="table__primary">
                      {c.appName}
                      {c.isSuspicious && (
                        <Badge tone="bad" className="lc__inline-badge">
                          Suspicious
                        </Badge>
                      )}
                      {c.isAllowlisted && (
                        <Badge tone="good" dot={false} className="lc__inline-badge">
                          Allowlisted
                        </Badge>
                      )}
                    </td>
                    <td className="table__mono">{c.processId}</td>
                    <td className="table__mono selectable">{c.remoteAddress}</td>
                    <td className="table__num">{c.remotePort}</td>
                    <td>{c.protocol}</td>
                    <td>
                      <Badge tone="neutral" dot={false}>
                        {c.status}
                      </Badge>
                    </td>
                    <td onClick={(e) => e.stopPropagation()}>
                      <div className="table__actions">
                        <Menu
                          items={[
                            { label: "Block IP", onSelect: () => void doBlockIp(c), danger: true },
                            { label: "Copy IP", onSelect: () => void copyIp(c.remoteAddress) },
                            { label: "Block Application", onSelect: () => void doBlockApp(c), danger: true, disabled: !c.processPath },
                            { label: "Add to Allowlist", onSelect: () => void doAllowlist(c), disabled: c.isAllowlisted },
                          ]}
                        />
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <Modal open={!!selected} onClose={() => setSelected(null)} title={selected?.appName ?? ""} description="Connection details" width={480}>
        {selected && (
          <div className="lc__detail">
            <DetailRow label="Process ID" value={String(selected.processId)} mono />
            <DetailRow label="Executable" value={selected.processPath ?? "Unknown"} mono />
            <DetailRow label="Local address" value={`${selected.localAddress}:${selected.localPort}`} mono />
            <DetailRow label="Remote address" value={`${selected.remoteAddress}:${selected.remotePort}`} mono />
            <DetailRow label="Protocol" value={selected.protocol} />
            <DetailRow label="Status" value={selected.status} />
          </div>
        )}
        <div className="lc__detail-actions">
          <Button variant="secondary" size="sm" onClick={() => selected && void copyIp(selected.remoteAddress)}>
            Copy IP
          </Button>
          <Button variant="secondary" size="sm" onClick={() => selected && void doAllowlist(selected)} disabled={selected?.isAllowlisted}>
            Add to Allowlist
          </Button>
          <Button variant="danger" size="sm" onClick={() => selected && void doBlockApp(selected)} disabled={!selected?.processPath}>
            Block Application
          </Button>
          <Button variant="danger" size="sm" onClick={() => selected && void doBlockIp(selected)}>
            Block IP
          </Button>
        </div>
      </Modal>
    </Page>
  );
}

function DetailRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="lc__detail-row">
      <span className="lc__detail-label">{label}</span>
      <span className={["lc__detail-value selectable", mono ? "mono" : ""].filter(Boolean).join(" ")}>{value}</span>
    </div>
  );
}
