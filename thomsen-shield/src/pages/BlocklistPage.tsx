import { useEffect, useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Menu } from "@/components/ui/Menu";
import { Modal } from "@/components/ui/Modal";
import { Field, TextInput } from "@/components/ui/Field";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { PlusIcon } from "@/components/icons";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { addManualBlock, listBlocklist, makeBlockPermanent, unblockEntry } from "@/services/blocklist";
import { formatDateTime } from "@/lib/format";
import { BLOCK_REASON_LABELS, type BlockedEntry } from "@/types";
import "@/components/ui/Table.css";

export function BlocklistPage() {
  const toast = useToast();
  const [entries, setEntries] = useState<BlockedEntry[] | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [ip, setIp] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  const load = () => void listBlocklist().then(setEntries).catch((e) => toast.error(errorMessage(e)));
  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  const copyIp = async (v: string) => {
    try {
      await navigator.clipboard.writeText(v);
      toast.success(`Copied ${v}`);
    } catch {
      toast.error("Couldn't copy to clipboard.");
    }
  };

  const doUnblock = async (e: BlockedEntry) => {
    try {
      await unblockEntry(e.id);
      toast.success(`${e.ip} unblocked`);
      load();
    } catch (err) {
      toast.error(errorMessage(err));
    }
  };

  const doPermanent = async (e: BlockedEntry) => {
    try {
      await makeBlockPermanent(e.id);
      toast.success(`${e.ip} is now permanently blocked`);
      load();
    } catch (err) {
      toast.error(errorMessage(err));
    }
  };

  const submitAdd = async () => {
    if (!ip.trim()) return;
    setBusy(true);
    try {
      await addManualBlock(ip.trim(), note.trim() || undefined);
      toast.success(`${ip.trim()} blocked`);
      setAddOpen(false);
      setIp("");
      setNote("");
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Page
      title="Blocklist"
      eyebrow="Blocked addresses"
      description="Every IP Thomsen Shield has blocked automatically or manually, with when it expires."
      actions={
        <Button variant="primary" icon={<PlusIcon size={15} />} onClick={() => setAddOpen(true)}>
          Block IP
        </Button>
      }
    >
      <Card flush>
        {entries === null ? (
          <LoadingRow label="Loading blocklist" />
        ) : entries.length === 0 ? (
          <EmptyState inset title="Nothing blocked" description="Blocked IPs - automatic or manual - will show up here." />
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Blocked IP</th>
                  <th>Reason</th>
                  <th>Date</th>
                  <th>Expiration</th>
                  <th>Status</th>
                  <th aria-label="Actions" />
                </tr>
              </thead>
              <tbody>
                {entries.map((e) => (
                  <tr key={e.id}>
                    <td className="table__mono selectable">{e.ip}</td>
                    <td>{BLOCK_REASON_LABELS[e.reason]}</td>
                    <td className="table__muted">{formatDateTime(e.createdAt)}</td>
                    <td className="table__muted">{e.expiresAt ? formatDateTime(e.expiresAt) : "Permanent"}</td>
                    <td>
                      <Badge tone={e.status === "active" ? "bad" : e.status === "expired" ? "neutral" : "neutral"} dot={false}>
                        {e.status === "active" ? "Active" : e.status === "expired" ? "Expired" : "Removed"}
                      </Badge>
                    </td>
                    <td>
                      <div className="table__actions">
                        <Menu
                          items={[
                            { label: "Copy IP", onSelect: () => void copyIp(e.ip) },
                            { label: "Make permanent", onSelect: () => void doPermanent(e), disabled: e.status !== "active" || e.expiresAt === null },
                            { label: "Unblock", onSelect: () => void doUnblock(e), disabled: e.status !== "active", danger: true },
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

      <Modal
        open={addOpen}
        onClose={() => setAddOpen(false)}
        title="Block an IP"
        description="Adds a permanent block, unless you change its expiration later."
        footer={
          <>
            <Button variant="ghost" onClick={() => setAddOpen(false)} disabled={busy}>
              Cancel
            </Button>
            <Button variant="primary" onClick={() => void submitAdd()} loading={busy}>
              Block IP
            </Button>
          </>
        }
      >
        <div className="stack stack--sm">
          <Field label="IP address">
            <TextInput mono autoFocus value={ip} onChange={(e) => setIp(e.target.value)} placeholder="185.34.22.10" />
          </Field>
          <Field label="Note" hint="Optional - shown as the reason on this entry.">
            <TextInput value={note} onChange={(e) => setNote(e.target.value)} placeholder="e.g. Known scanner" />
          </Field>
        </div>
      </Modal>
    </Page>
  );
}
