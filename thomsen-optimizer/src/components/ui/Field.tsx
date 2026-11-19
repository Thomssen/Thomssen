import {
  forwardRef,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from "react";
import { ChevronDown, CheckIcon } from "@/components/icons";
import "./Field.css";

interface FieldProps {
  label?: ReactNode;
  hint?: ReactNode;
  error?: ReactNode;
  /** Right-aligned adornment in the label row (e.g. a small action). */
  aside?: ReactNode;
  htmlFor?: string;
  children: ReactNode;
  className?: string;
}

export function Field({ label, hint, error, aside, htmlFor, children, className }: FieldProps) {
  return (
    <div className={["field", error ? "field--error" : "", className ?? ""].filter(Boolean).join(" ")}>
      {(label || aside) && (
        <div className="field__labelrow">
          {label && (
            <label className="field__label" htmlFor={htmlFor}>
              {label}
            </label>
          )}
          {aside && <div className="field__aside">{aside}</div>}
        </div>
      )}
      {children}
      {error ? <p className="field__error">{error}</p> : hint ? <p className="field__hint">{hint}</p> : null}
    </div>
  );
}

type InputProps = InputHTMLAttributes<HTMLInputElement> & { mono?: boolean };

export const TextInput = forwardRef<HTMLInputElement, InputProps>(function TextInput({ className, mono, ...rest }, ref) {
  return (
    <input
      ref={ref}
      className={["input", mono ? "input--mono" : "", className ?? ""].filter(Boolean).join(" ")}
      spellCheck={false}
      autoComplete="off"
      {...rest}
    />
  );
});

type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & { mono?: boolean };

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(function Textarea({ className, mono, ...rest }, ref) {
  return (
    <textarea
      ref={ref}
      className={["input", "textarea", mono ? "input--mono" : "", className ?? ""].filter(Boolean).join(" ")}
      spellCheck={false}
      {...rest}
    />
  );
});

export interface SelectOption {
  value: string;
  label: ReactNode;
  disabled?: boolean;
}

interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
  "aria-label"?: string;
}

/** A custom-drawn dropdown, not a native <select>: a native select's closed
 * button can be themed, but the open popup list is always rendered by the
 * OS, not the browser - on Windows that's an unstyled light listbox no CSS
 * here can reach, clashing hard with the rest of this (deliberately dark
 * only, see tokens.css) app. This renders both states itself, so the open
 * panel matches everywhere else a floating panel appears (see Menu.tsx). */
export function Select({ value, onChange, options, disabled, className, style, "aria-label": ariaLabel }: SelectProps) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(() => Math.max(0, options.findIndex((o) => o.value === value)));
  const rootRef = useRef<HTMLDivElement>(null);
  const selected = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    setActiveIndex(Math.max(0, options.findIndex((o) => o.value === value)));
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const commit = (index: number) => {
    const opt = options[index];
    if (!opt || opt.disabled) return;
    onChange(opt.value);
    setOpen(false);
  };

  const onTriggerKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;
    if (!open && (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      setOpen(true);
      return;
    }
    if (!open) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActiveIndex((i) => Math.min(options.length - 1, i + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActiveIndex((i) => Math.max(0, i - 1));
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      commit(activeIndex);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
    } else if (e.key === "Tab") {
      setOpen(false);
    }
  };

  return (
    <span className={["select", className ?? ""].filter(Boolean).join(" ")} style={style} ref={rootRef}>
      <button
        type="button"
        className="input select__el"
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={ariaLabel}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={onTriggerKeyDown}
      >
        <span className="select__value">{selected?.label ?? ""}</span>
      </button>
      <ChevronDown className="select__chevron" size={15} />
      {open && (
        <div className="select__panel" role="listbox">
          {options.map((opt, i) => (
            <button
              key={opt.value}
              type="button"
              role="option"
              aria-selected={opt.value === value}
              disabled={opt.disabled}
              className={["select__option", i === activeIndex ? "select__option--active" : "", opt.value === value ? "select__option--selected" : ""]
                .filter(Boolean)
                .join(" ")}
              onMouseEnter={() => setActiveIndex(i)}
              onClick={(e) => {
                e.stopPropagation();
                commit(i);
              }}
            >
              <span>{opt.label}</span>
              {opt.value === value && <CheckIcon size={14} />}
            </button>
          ))}
        </div>
      )}
    </span>
  );
}
