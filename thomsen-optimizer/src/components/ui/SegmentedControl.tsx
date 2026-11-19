import { useLayoutEffect, useRef, useState } from "react";
import "./SegmentedControl.css";

interface Option<T extends string> {
  value: T;
  label: string;
}

interface SegmentedControlProps<T extends string> {
  options: readonly Option<T>[];
  value: T;
  onChange: (value: T) => void;
  size?: "sm" | "md";
  disabled?: boolean;
  "aria-label"?: string;
}

export function SegmentedControl<T extends string>({ options, value, onChange, size = "md", disabled, ...rest }: SegmentedControlProps<T>) {
  const itemRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  // The sliding background pill behind the active item - measured in
  // pixels off the real DOM rather than guessed from option index, so it
  // stays exact regardless of label length or font metrics. `null` until
  // the first measurement lands, which skips the slide-in from nowhere on
  // first mount (see the indicator's own opacity below).
  const [indicator, setIndicator] = useState<{ x: number; w: number } | null>(null);

  useLayoutEffect(() => {
    const measure = () => {
      const el = itemRefs.current.get(value);
      if (el) setIndicator({ x: el.offsetLeft, w: el.offsetWidth });
    };
    measure();
    // Re-measure once web fonts have loaded, since label widths change.
    let alive = true;
    document.fonts?.ready.then(() => alive && measure());
    return () => {
      alive = false;
    };
  }, [value, options]);

  return (
    <div className={["segmented", `segmented--${size}`, disabled ? "is-disabled" : ""].filter(Boolean).join(" ")} role="tablist" aria-label={rest["aria-label"]}>
      {indicator && (
        <span
          className="segmented__indicator"
          style={{ transform: `translateX(${indicator.x}px)`, width: indicator.w }}
          aria-hidden="true"
        />
      )}
      {options.map((opt) => (
        <button
          key={opt.value}
          ref={(el) => {
            if (el) itemRefs.current.set(opt.value, el);
            else itemRefs.current.delete(opt.value);
          }}
          type="button"
          role="tab"
          aria-selected={opt.value === value}
          disabled={disabled}
          className={["segmented__item", opt.value === value ? "is-active" : ""].filter(Boolean).join(" ")}
          onClick={() => onChange(opt.value)}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
