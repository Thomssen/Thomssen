import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { XIcon } from "@/components/icons";
import { Button } from "./Button";
import "./Modal.css";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  width?: number;
}

export function Modal({ open, onClose, title, description, children, footer, width = 460 }: ModalProps) {
  const cardRef = useRef<HTMLDivElement>(null);
  // Every caller passes onClose as an inline arrow function, so it's a new
  // reference on every render of the *parent* - including every keystroke in
  // any field the modal contains, since that updates the parent's state.
  // Keeping it out of the effect below (via this ref) instead of in its
  // dependency array is what actually matters here: with onClose in the
  // deps, typing in a second field re-ran the open/close effect on every
  // keystroke - its cleanup returned focus to whatever had focus before the
  // modal opened, then the freshly re-run effect's timeout below "found no
  // focus in the card" and grabbed the *first* focusable field again. One
  // keystroke in "Note" was enough to visibly bounce focus back to the field
  // above it.
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCloseRef.current();
    };
    document.addEventListener("keydown", onKey);
    const prevFocus = document.activeElement as HTMLElement | null;
    const t = window.setTimeout(() => {
      const card = cardRef.current;
      if (!card) return;
      // An autoFocus field (e.g. the IP address input) already has focus by
      // now - don't steal it. Without this check, this timeout unconditionally
      // grabbed the *first* input/textarea/select/button in the card, and
      // since the close (X) button is the first focusable element in the DOM
      // (it's in modal__head, above modal__body), it always won this over
      // whatever the modal's own content wanted focused - a keystroke right
      // after opening could land while focus was still mid-jump to it.
      if (card.contains(document.activeElement)) return;
      card.querySelector<HTMLElement>(".modal__body input, .modal__body textarea, .modal__body select, .modal__footer button")?.focus();
    }, 30);
    return () => {
      document.removeEventListener("keydown", onKey);
      window.clearTimeout(t);
      prevFocus?.focus?.();
    };
  }, [open]);

  if (!open) return null;

  return createPortal(
    <div className="modal is-in" role="presentation" onMouseDown={onClose}>
      <div className="modal__backdrop" />
      <div
        ref={cardRef}
        className="modal__card"
        style={{ width: `min(${width}px, calc(100vw - 40px))` }}
        role="dialog"
        aria-modal="true"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="modal__head">
          <div>
            <h2 className="modal__title">{title}</h2>
            {description && <p className="modal__description">{description}</p>}
          </div>
          <button type="button" className="modal__x" onClick={onClose} aria-label="Close">
            <XIcon size={16} />
          </button>
        </div>
        {children && <div className="modal__body">{children}</div>}
        {footer && <div className="modal__footer">{footer}</div>}
      </div>
    </div>,
    document.body,
  );
}

interface ConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: ReactNode;
  confirmLabel?: string;
  danger?: boolean;
  busy?: boolean;
}

export function ConfirmDialog({ open, onClose, onConfirm, title, message, confirmLabel = "Confirm", danger, busy }: ConfirmDialogProps) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      width={420}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button variant={danger ? "danger" : "primary"} onClick={onConfirm} loading={busy}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p className="modal__message">{message}</p>
    </Modal>
  );
}
