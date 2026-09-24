import { useEffect, useState } from "react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { Field, TextInput } from "@/components/ui/Field";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import { addAllowlistEntry, listAllowlist, removeAllowlistEntry } from "@/services/allowlist";
import type { AllowlistEntry } from "@/types";
import "@/components/ui/Table.css";

export function AllowlistPage() {
  const toast = useToast();
  const [entries, setEntries] = useState<AllowlistEntry[] | null>(null);
  const [addOpen, setAddOpen] = useState(false);
  const [value, setValue] = useState("");
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState(false);

  const load = () => void listAllowlist().then(setEntries).catch((e) => toast.error(errorMessage(e)));
  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  const builtIn = entries?.filter((e) => e.isBuiltIn) ?? [];
  const custom = entries?.filter((e) => !e.isBuiltIn) ?? [];

  const submitAdd = async () => {
    if (!value.trim()) return;
    setBusy(true);
    try {
      await addAllowlistEntry(value.trim(), label.trim() || undefined);
      toast.success(`${value.trim()} added to the allowlist`);
      setAddOpen(false);
      setValue("");
      setLabel("");
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async (entry: AllowlistEntry) => {
    try {
      await removeAllowlistEntry(entry.id);
      toast.success(`${entry.value} removed`);
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  return (
    <Page
      title="Allowlist"
      eyebrow="Trusted addresses"
      description="Allowlisted IPs and ranges are never automatically blocked, no matter what Protection detects."
      actions={
        <Button variant="primary" icon={<PlusIcon size={15} />} onClick={() => setAddOpen(true)}>
          Add IP
        </Button>
      }
    >
      {entries === null ? (
        <LoadingRow label="Loading allowlist" />
      ) : (
        <div className="stack">
          <Section title="Local network ranges" description="Protected automatically so Thomsen Shield can never block your own network.">
            <Card flush>
              <div className="table-wrap">
                <table className="table">
                  <tbody>
                    {builtIn.map((e) => (
                      <tr key={e.id}>
                        <td className="table__mono selectable">{e.value}</td>
                        <td className="table__muted">{e.label}</td>
                        <td style={{ textAlign: "right" }}>
                          <Badge tone="neutral" dot={false}>
                            Built-in
                          </Badge>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          </Section>

          <Section title="Custom entries">
            <Card flush>
              {custom.length === 0 ? (
                <EmptyState inset title="No custom entries" description="Add a trusted IP or range so Protection never blocks it." />
              ) : (
                <div className="table-wrap">
                  <table className="table">
                    <thead>
                      <tr>
                        <th>IP / Range</th>
                        <th>Label</th>
                        <th aria-label="Actions" />
                      </tr>
                    </thead>
                    <tbody>
                      {custom.map((e) => (
                        <tr key={e.id}>
                          <td className="table__mono selectable">{e.value}</td>
                          <td className="table__muted">{e.label ?? "—"}</td>
                          <td>
                            <div className="table__actions">
                              <button type="button" className="fw__remove allow__remove" onClick={() => void remove(e)} aria-label={`Remove ${e.value}`} title="Remove">
                                <TrashIcon size={15} />
                              </button>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </Card>
          </Section>
        </div>
      )}

      <Modal
        open={addOpen}
        onClose={() => setAddOpen(false)}
        title="Add to allowlist"
        description="Accepts a single IP (185.34.22.10) or a CIDR range (185.34.22.0/24)."
        footer={
          <>
            <Button variant="ghost" onClick={() => setAddOpen(false)} disabled={busy}>
              Cancel
            </Button>
            <Button variant="primary" onClick={() => void submitAdd()} loading={busy}>
              Add
            </Button>
          </>
        }
      >
        <div className="stack stack--sm">
          <Field label="IP or range">
            <TextInput mono autoFocus value={value} onChange={(e) => setValue(e.target.value)} placeholder="185.34.22.10" />
          </Field>
          <Field label="Label" hint="Optional.">
            <TextInput value={label} onChange={(e) => setLabel(e.target.value)} placeholder="e.g. Home router" />
          </Field>
        </div>
      </Modal>
    </Page>
  );
}
