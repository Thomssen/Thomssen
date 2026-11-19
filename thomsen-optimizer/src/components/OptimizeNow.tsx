import { useState } from "react";
import { Sparkles } from "lucide-react";
import { api } from "@/services/api";
import { errorMessage } from "@/services/ipc";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { Spinner } from "@/components/ui/Spinner";
import { useToast } from "@/components/ui/Toast";
import { useNavigation } from "@/state/NavigationProvider";
import { useSettings } from "@/state/SettingsProvider";
import type { Recommendation } from "@/types";

/**
 * "Optimize Now": applies only the recommended, safe *Windows settings*
 * (never closes apps, never deletes files). The modal first shows exactly
 * what will change, as required by the safety rules.
 */
export function OptimizeNowButton({ size = "md", variant = "primary", onDone }: { size?: "sm" | "md" | "lg"; variant?: "primary" | "secondary"; onDone?: () => void }) {
  const { settings } = useSettings();
  const { navigate } = useNavigation();
  const toast = useToast();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [recs, setRecs] = useState<Recommendation[]>([]);

  const run = async () => {
    setBusy(true);
    try {
      await api.optimizeNow();
      setOpen(false);
      onDone?.();
      navigate("results");
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const start = async () => {
    setOpen(true);
    setLoading(true);
    try {
      const report = await api.analyze();
      const safe = report.recommendations.filter((r) => r.safeAuto);
      setRecs(safe);
      if (safe.length === 0) setLoading(false);
      else if (settings && !settings.confirmChanges) {
        setLoading(false);
        await run();
      }
    } catch (e) {
      toast.error(errorMessage(e));
      setOpen(false);
    } finally {
      setLoading(false);
    }
  };

  return (
    <>
      <Button variant={variant} size={size} icon={<Sparkles size={15} />} onClick={start}>
        Optimize Now
      </Button>
      <Modal
        open={open}
        onClose={() => !busy && setOpen(false)}
        title="Optimize now"
        description="Only recommended, reversible Windows settings are changed. Nothing is closed or deleted."
        width={520}
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)} disabled={busy}>
              {recs.length === 0 && !loading ? "Close" : "Cancel"}
            </Button>
            {recs.length > 0 && (
              <Button variant="primary" onClick={run} loading={busy}>
                Apply {recs.length} change{recs.length === 1 ? "" : "s"}
              </Button>
            )}
          </>
        }
      >
        {loading ? (
          <div style={{ display: "flex", gap: 10, alignItems: "center", padding: "8px 0" }}>
            <Spinner /> <span>Analysing your PC…</span>
          </div>
        ) : recs.length === 0 ? (
          <p className="modal__message">Your PC already has the recommended safe settings, so there is nothing to change. Open the Optimize page to review optional suggestions.</p>
        ) : (
          <ul style={{ display: "flex", flexDirection: "column", gap: 12, listStyle: "none", padding: 0 }}>
            {recs.map((r) => (
              <li key={r.id}>
                <p style={{ color: "var(--text)", fontWeight: 600, fontSize: 13 }}>{r.title}</p>
                <p style={{ fontSize: 12, color: "var(--text-muted)" }}>{r.changes}</p>
                <p style={{ fontSize: 12, color: "var(--text-faint)" }}>Downside: {r.downsides} Undo: {r.howToReverse}</p>
              </li>
            ))}
          </ul>
        )}
      </Modal>
    </>
  );
}
