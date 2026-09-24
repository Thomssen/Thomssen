import { useEffect, useState } from "react";
import { Page } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Toggle } from "@/components/ui/Toggle";
import { Modal, ConfirmDialog } from "@/components/ui/Modal";
import { Field, TextInput } from "@/components/ui/Field";
import { EmptyState } from "@/components/ui/EmptyState";
import { LoadingRow } from "@/components/ui/Spinner";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { useToast } from "@/components/ui/Toast";
import { errorMessage } from "@/services/ipc";
import {
  createAppBlockRule,
  createIpBlockRule,
  getFirewallStatus,
  listFirewallRules,
  removeFirewallRule,
  resetThomsenShieldRules,
  setFirewallRuleEnabled,
} from "@/services/firewall";
import type { FirewallRule, FirewallStatus } from "@/types";
import "@/components/ui/Table.css";
import "./FirewallPage.css";

type AddKind = "ip" | "application" | null;

export function FirewallPage() {
  const toast = useToast();
  const [status, setStatus] = useState<FirewallStatus | null>(null);
  const [rules, setRules] = useState<FirewallRule[] | null>(null);
  const [addKind, setAddKind] = useState<AddKind>(null);
  const [addValue, setAddValue] = useState("");
  const [addBusy, setAddBusy] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const [resetBusy, setResetBusy] = useState(false);

  const load = () => {
    void getFirewallStatus().then(setStatus).catch(() => {});
    void listFirewallRules().then(setRules).catch((e) => toast.error(errorMessage(e)));
  };

  useEffect(load, []); // eslint-disable-line react-hooks/exhaustive-deps

  const submitAdd = async () => {
    if (!addValue.trim()) return;
    setAddBusy(true);
    try {
      if (addKind === "ip") await createIpBlockRule(addValue.trim());
      else if (addKind === "application") await createAppBlockRule(addValue.trim());
      toast.success("Rule created");
      setAddKind(null);
      setAddValue("");
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setAddBusy(false);
    }
  };

  const toggleRule = async (rule: FirewallRule, enabled: boolean) => {
    setRules((rs) => rs?.map((r) => (r.id === rule.id ? { ...r, enabled } : r)) ?? rs);
    try {
      await setFirewallRuleEnabled(rule.id, enabled);
    } catch (e) {
      toast.error(errorMessage(e));
      load();
    }
  };

  const removeRule = async (rule: FirewallRule) => {
    try {
      await removeFirewallRule(rule.id);
      toast.success("Rule removed");
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };

  const doReset = async () => {
    setResetBusy(true);
    try {
      const { removed } = await resetThomsenShieldRules();
      toast.success(`Removed ${removed} Thomsen Shield rule${removed === 1 ? "" : "s"}`);
      setResetOpen(false);
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setResetBusy(false);
    }
  };

  return (
    <Page
      title="Firewall"
      eyebrow="Windows Defender Firewall"
      description="Thomsen Shield only manages rules it created itself, prefixed ThomsenShield_, so cleanup is always safe and complete."
      actions={
        <>
          <Button variant="secondary" icon={<PlusIcon size={15} />} onClick={() => setAddKind("ip")}>
            Block IP
          </Button>
          <Button variant="secondary" icon={<PlusIcon size={15} />} onClick={() => setAddKind("application")}>
            Block Application
          </Button>
        </>
      }
    >
      <div className="stack">
        <Card className="fw__status">
          <div>
            <span className="fw__status-label">Firewall</span>
            <Badge tone={status?.active ? "good" : "bad"}>{status ? (status.active ? "Active" : "Disabled") : "—"}</Badge>
          </div>
          <div className="fw__profiles">
            <ProfileBadge label="Domain" on={status?.profilesEnabled.domain} />
            <ProfileBadge label="Private" on={status?.profilesEnabled.private} />
            <ProfileBadge label="Public" on={status?.profilesEnabled.public} />
          </div>
        </Card>

        <Card flush>
          <div className="card__header" style={{ padding: "18px 20px 0" }}>
            <div className="card__heading">
              <h3 className="card__title">Thomsen Shield rules</h3>
              <p className="card__description">Rules this app created in Windows Firewall.</p>
            </div>
            <Button variant="ghost" size="sm" onClick={() => setResetOpen(true)} disabled={!rules?.length}>
              Reset Thomsen Shield Rules
            </Button>
          </div>

          {rules === null ? (
            <LoadingRow label="Loading rules" />
          ) : rules.length === 0 ? (
            <EmptyState inset title="No rules yet" description="Rules you create here, or blocks Protection applies automatically, will appear in this list." />
          ) : (
            <div className="table-wrap" style={{ marginTop: 14 }}>
              <table className="table">
                <thead>
                  <tr>
                    <th>Rule</th>
                    <th>Type</th>
                    <th>Target</th>
                    <th>Direction</th>
                    <th>Created</th>
                    <th>Enabled</th>
                    <th aria-label="Actions" />
                  </tr>
                </thead>
                <tbody>
                  {rules.map((r) => (
                    <tr key={r.id}>
                      <td className="table__mono">{r.name}</td>
                      <td>{r.targetType === "ip" ? "IP address" : "Application"}</td>
                      <td className="table__mono selectable">{r.target}</td>
                      <td>{r.direction === "in" ? "Inbound" : "Outbound"}</td>
                      <td className="table__muted">{new Date(r.createdAt).toLocaleDateString()}</td>
                      <td>
                        <Toggle checked={r.enabled} onChange={(v) => void toggleRule(r, v)} label={`Enable ${r.name}`} />
                      </td>
                      <td>
                        <div className="table__actions">
                          <button type="button" className="fw__remove" onClick={() => void removeRule(r)} aria-label={`Remove ${r.name}`} title="Remove">
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
      </div>

      <Modal
        open={addKind !== null}
        onClose={() => setAddKind(null)}
        title={addKind === "ip" ? "Create IP block rule" : "Create application block rule"}
        description={addKind === "ip" ? "Blocks inbound traffic from this address." : "Blocks outbound traffic from this application."}
        footer={
          <>
            <Button variant="ghost" onClick={() => setAddKind(null)} disabled={addBusy}>
              Cancel
            </Button>
            <Button variant="primary" onClick={() => void submitAdd()} loading={addBusy}>
              Create Rule
            </Button>
          </>
        }
      >
        <Field label={addKind === "ip" ? "IP address" : "Executable path"} hint={addKind === "ip" ? "e.g. 185.34.22.10" : "e.g. C:\\Program Files\\App\\app.exe"}>
          <TextInput mono autoFocus value={addValue} onChange={(e) => setAddValue(e.target.value)} placeholder={addKind === "ip" ? "185.34.22.10" : "C:\\path\\to\\app.exe"} />
        </Field>
      </Modal>

      <ConfirmDialog
        open={resetOpen}
        onClose={() => setResetOpen(false)}
        onConfirm={() => void doReset()}
        title="Reset Thomsen Shield Rules"
        message="This removes every firewall rule Thomsen Shield created and restores Windows Firewall to its state before Thomsen Shield made any changes. Rules you created yourself outside Thomsen Shield are never touched."
        confirmLabel="Reset Rules"
        danger
        busy={resetBusy}
      />
    </Page>
  );
}

function ProfileBadge({ label, on }: { label: string; on?: boolean }) {
  return (
    <Badge tone={on ? "good" : "neutral"} dot={false}>
      {label}
    </Badge>
  );
}
