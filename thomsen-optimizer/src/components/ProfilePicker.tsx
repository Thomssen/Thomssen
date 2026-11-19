import { Select } from "@/components/ui/Field";
import type { ProfileConfig } from "@/types";

/** Dropdown of gaming profiles, with an optional "use default" entry. */
export function ProfilePicker({ profiles, value, onChange, allowDefault, defaultName, disabled }: { profiles: ProfileConfig[]; value: string | null; onChange: (id: string | null) => void; allowDefault?: boolean; defaultName?: string; disabled?: boolean }) {
  const opts = [
    ...(allowDefault ? [{ value: "", label: `Default (${defaultName ?? "Balanced"})` }] : []),
    ...profiles.map((p) => ({ value: p.id, label: p.name })),
  ];
  return <Select value={value ?? ""} onChange={(v) => onChange(v === "" ? null : v)} options={opts} disabled={disabled} aria-label="Gaming profile" />;
}
