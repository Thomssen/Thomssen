import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { CheckIcon, XIcon } from "@/components/icons";
import "./Toast.css";

type ToastTone = "info" | "success" | "error";

interface ToastMsg {
  id: number;
  tone: ToastTone;
  message: string;
}

interface ToastContextValue {
  toast: (message: string, tone?: ToastTone) => void;
  success: (message: string) => void;
  error: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

// Matches Toast.css's `--dur-fast`-based exit animation - the toast stays
// mounted (and keeps its height, so later toasts don't jump) for exactly
// this long after dismissal, playing the fade/slide-out, instead of
// vanishing instantly the moment `toasts` drops it.
const EXIT_MS = 150;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastMsg[]>([]);
  const [leavingIds, setLeavingIds] = useState<ReadonlySet<number>>(new Set());

  const remove = useCallback((id: number) => {
    setLeavingIds((prev) => new Set(prev).add(id));
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
      setLeavingIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }, EXIT_MS);
  }, []);

  const toast = useCallback(
    (message: string, tone: ToastTone = "info") => {
      const id = Date.now() + Math.random();
      setToasts((prev) => [...prev, { id, tone, message }]);
      window.setTimeout(() => remove(id), tone === "error" ? 6000 : 3600);
    },
    [remove],
  );

  const value = useMemo<ToastContextValue>(
    () => ({ toast, success: (m) => toast(m, "success"), error: (m) => toast(m, "error") }),
    [toast],
  );

  return (
    <ToastContext.Provider value={value}>
      {children}
      {createPortal(
        <div className="toast-stack">
          {toasts.map((t) => (
            <div key={t.id} className={`toast toast--${t.tone}${leavingIds.has(t.id) ? " toast--leaving" : ""}`}>
              {t.tone === "success" && <CheckIcon size={15} className="toast__icon" />}
              {t.tone === "error" && <XIcon size={15} className="toast__icon" />}
              <span className="toast__message">{t.message}</span>
              <button type="button" className="toast__close" onClick={() => remove(t.id)} aria-label="Dismiss">
                <XIcon size={13} />
              </button>
            </div>
          ))}
        </div>,
        document.body,
      )}
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within <ToastProvider>");
  return ctx;
}
