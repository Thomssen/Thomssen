import { useState } from "react";
import { ChevronDown, ChevronRight, ScanSearch, Trash2 } from "lucide-react";
import { Page, Section } from "@/components/layout/Page";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { EmptyState } from "@/components/ui/EmptyState";
import { ConfirmDialog } from "@/components/ui/Modal";
import { StatTile } from "@/components/ui/StatTile";
import { useToast } from "@/components/ui/Toast";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { formatBytes1024 } from "@/lib/units";
import type { CleanReport, ScanSummary } from "@/types";
import "./CleanerPage.css";

/** Scan -> review -> confirm -> delete only what is ticked. */
export function CleanerPage() {
  const toast = useToast();
  const [scan, setScan] = useState<ScanSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<CleanReport | null>(null);

  const run = async () => {
    setScanning(true);
    setReport(null);
    try {
      const s = await api.cleanerScan();
      setScan(s);
      setSel(new Set(s.categories.filter((c) => c.defaultSelected && c.bytes > 0).map((c) => c.id)));
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setScanning(false);
    }
  };

  const chosen = scan?.categories.filter((c) => sel.has(c.id)) ?? [];
  const chosenBytes = chosen.reduce((a, c) => a + c.bytes, 0);
  const chosenFiles = chosen.reduce((a, c) => a + c.files, 0);

  const clean = async () => {
    if (!scan) return;
    setBusy(true);
    try {
      const r = await api.cleanerClean(scan.scanId, [...sel]);
      setReport(r);
      setScan(null);
      toast.success(`${formatBytes1024(r.totalFreed)} freed.`);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
      setConfirm(false);
    }
  };

  const toggle = (id: string) => {
    const n = new Set(sel);
    n.has(id) ? n.delete(id) : n.add(id);
    setSel(n);
  };

  return (
    <Page
      title="System Cleaner"
      eyebrow="Storage"
      description="Finds temporary files and caches, shows you what it found, and removes only the categories you tick. Documents, saved games and settings are never touched. Files that a running program is using are skipped."
      actions={
        <Button variant="primary" icon={<ScanSearch size={15} />} onClick={run} loading={scanning}>
          {scan ? "Scan again" : "Scan"}
        </Button>
      }
    >
      {report && (
        <Section>
          <Card>
            <h3 className="clean__result-title">Cleanup finished</h3>
            <div className="clean__tiles">
              <StatTile label="Space freed" value={formatBytes1024(report.totalFreed)} tone="good" />
              <StatTile label="Files deleted" value={report.totalDeleted.toLocaleString()} />
              <StatTile label="Skipped" value={report.totalSkipped.toLocaleString()} note="in use, changed or no permission" />
            </div>
            <ul className="clean__report">
              {report.categories.map((c) => (
                <li key={c.id}>
                  <strong>{c.name}</strong> - {formatBytes1024(c.bytesFreed)} freed{c.deletedFiles ? `, ${c.deletedFiles.toLocaleString()} files` : ""}
                  {c.skippedFiles > 0 && `, ${c.skippedFiles.toLocaleString()} skipped`}
                  {c.error && <span className="clean__err"> - {c.error}</span>}
                </li>
              ))}
            </ul>
            <p className="set__note">Deleted files were removed permanently - Thomsen Optimizer cannot restore them. Figures count only files that were actually removed.</p>
          </Card>
        </Section>
      )}

      {!scan && !report && !scanning && <EmptyState title="Nothing scanned yet" description="Press Scan. Nothing is deleted during a scan." />}

      {scan && (
        <>
          <Section title={`Found ${formatBytes1024(scan.totalBytes)} in ${scan.categories.filter((c) => c.bytes > 0).length} categories`}>
            <div className="stack stack--sm">
              {scan.categories.map((c) => {
                const isOpen = open === c.id;
                const empty = c.bytes === 0;
                return (
                  <Card key={c.id} className={`clean__cat${empty ? " is-empty" : ""}`}>
                    <div className="clean__row">
                      <input type="checkbox" className="rec__check" checked={sel.has(c.id)} disabled={empty} onChange={() => toggle(c.id)} aria-label={`Clean ${c.name}`} />
                      <button type="button" className="clean__main" onClick={() => setOpen(isOpen ? null : c.id)}>
                        <span className="clean__name">{c.name}</span>
                        <span className="clean__desc">{c.description}</span>
                      </button>
                      {c.risk === "medium" && <Badge tone="warn">Not recommended</Badge>}
                      <span className="clean__size">{empty ? "Nothing found" : `${formatBytes1024(c.bytes)} · ${c.files.toLocaleString()} files`}</span>
                      {isOpen ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    </div>
                    {c.warning && <p className="clean__warn">{c.warning}</p>}
                    {isOpen && (
                      <div className="clean__details">
                        <p className="clean__label">Locations</p>
                        <ul className="clean__paths">{c.locations.map((l) => <li key={l} className="mono selectable">{l}</li>)}{c.locations.length === 0 && <li>Windows Recycle Bin</li>}</ul>
                        {c.largest.length > 0 && (
                          <>
                            <p className="clean__label">Largest files{c.files > c.largest.length ? ` (showing ${c.largest.length} of ${c.files.toLocaleString()})` : ""}</p>
                            <ul className="clean__paths">{c.largest.map((f) => <li key={f.path} className="mono selectable"><span>{f.path}</span><span>{formatBytes1024(f.size)}</span></li>)}</ul>
                          </>
                        )}
                        {c.truncated && <p className="clean__warn">Very large - only the first files were scanned; run the cleaner again afterwards.</p>}
                      </div>
                    )}
                  </Card>
                );
              })}
            </div>
          </Section>
          <div className="opt__bar">
            <span>{chosen.length === 0 ? "Select categories to clean." : `${chosen.length} categor${chosen.length === 1 ? "y" : "ies"} · ${formatBytes1024(chosenBytes)} · ${chosenFiles.toLocaleString()} files`}</span>
            <Button variant="danger" icon={<Trash2 size={15} />} disabled={chosen.length === 0} onClick={() => setConfirm(true)}>
              Delete selected…
            </Button>
          </div>
        </>
      )}

      <ConfirmDialog
        open={confirm}
        onClose={() => setConfirm(false)}
        onConfirm={clean}
        busy={busy}
        danger
        title="Permanently delete these files?"
        confirmLabel="Delete"
        message={
          <>
            About {formatBytes1024(chosenBytes)} in {chosen.map((c) => c.name).join(", ")} will be deleted permanently. They do not go to the Recycle Bin and cannot be restored by this app. {chosen.some((c) => c.risk === "medium") && "Shader caches are included - games may stutter until they are rebuilt. "}Files in use are skipped.
          </>
        }
      />
    </Page>
  );
}
