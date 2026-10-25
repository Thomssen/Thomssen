import { useEffect, useRef, useState, type ReactNode } from "react";
import { DotsIcon } from "@/components/icons";
import "./Menu.css";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
}

interface MenuProps {
  items: MenuItem[];
  trigger?: ReactNode;
  align?: "start" | "end";
  label?: string;
}

export function Menu({ items, trigger, align = "end", label = "More actions" }: MenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const toggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    setOpen((v) => !v);
  };

  return (
    <div className="menu" ref={rootRef}>
      {trigger ? (
        // Custom triggers (e.g. a <Button>) are already a full interactive
        // element - wrap in a plain span, not another <button>, so we never
        // nest interactive elements inside each other.
        <span className="menu__trigger menu__trigger--custom" onClick={toggle}>
          {trigger}
        </span>
      ) : (
        <button type="button" className="menu__trigger" aria-haspopup="menu" aria-expanded={open} aria-label={label} onClick={toggle}>
          <DotsIcon size={16} />
        </button>
      )}
      {open && (
        <div className={`menu__panel menu__panel--${align}`} role="menu">
          {items.map((item, i) => (
            <button
              key={i}
              type="button"
              role="menuitem"
              className={["menu__item", item.danger ? "menu__item--danger" : ""].filter(Boolean).join(" ")}
              disabled={item.disabled}
              onClick={(e) => {
                e.stopPropagation();
                setOpen(false);
                item.onSelect();
              }}
            >
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
