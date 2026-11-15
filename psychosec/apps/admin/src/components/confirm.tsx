"use client";
import { useState } from "react";
import { Button, Input } from "./ui";

export interface ConfirmSpec { title: string; body: string; needReason?: boolean; danger?: boolean; confirmLabel: string; run: (reason: string) => Promise<void> }

export function ConfirmDialog({ spec, onClose }: { spec: ConfirmSpec; onClose: () => void }) {
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const bad = spec.needReason && reason.trim().length < 3;
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4" role="dialog" aria-modal>
      <div className="w-full max-w-md rounded-lg border border-line bg-panel p-5">
        <h3 className="text-base font-medium">{spec.title}</h3>
        <p className="mt-2 text-sm text-dim">{spec.body}</p>
        {spec.needReason && <div className="mt-4"><Input autoFocus placeholder="Reason (recorded in the audit log)" value={reason} onChange={(e) => setReason(e.target.value)} maxLength={500} /></div>}
        {err && <p className="mt-3 text-sm text-bad">{err}</p>}
        <div className="mt-5 flex justify-end gap-2">
          <Button variant="quiet" onClick={onClose} disabled={busy}>Cancel</Button>
          <Button variant={spec.danger ? "danger" : "default"} disabled={busy || !!bad} onClick={async () => {
            setBusy(true); setErr(null);
            try { await spec.run(reason.trim()); onClose(); } catch (e) { setErr((e as Error).message); setBusy(false); }
          }}>{busy ? "Working…" : spec.confirmLabel}</Button>
        </div>
      </div>
    </div>
  );
}
