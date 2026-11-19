import { Card, CardHeader } from "@/components/ui/Card";
import { Toggle } from "@/components/ui/Toggle";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { SettingRow, Divider } from "@/components/ui/SettingRow";
import { useAppearance } from "@/state/AppearanceProvider";
import { ACCENT_COLORS } from "@/lib/color";
import { useSave } from "./BasicTabs";

export function AppearanceTab() {
  const { settings: s, save } = useSave();
  const { theme, setTheme, windowControlsStyle, setWindowControlsStyle } = useAppearance();
  return (
    <Card>
      <CardHeader title="Appearance" description="Thomsen Optimizer shares its look with Thomsen Shield, VM and VPN: the same tokens, type and components." />
      <SettingRow title="Theme" hint="Dark is the default and matches the other Thomsen apps.">
        <SegmentedControl options={[{ value: "dark", label: "Dark" }, { value: "light", label: "Light" }]} value={theme} onChange={setTheme} size="sm" aria-label="Theme" />
      </SettingRow>
      <Divider />
      <SettingRow title="Accent colour" hint="Used sparingly: primary buttons, switches and the active-page marker.">
        <div className="set__swatches" role="radiogroup" aria-label="Accent colour">
          {ACCENT_COLORS.map((c) => (
            <button key={c.hex} type="button" role="radio" aria-checked={s.accentColor.toLowerCase() === c.hex} aria-label={c.name} title={c.name} className={`set__swatch${s.accentColor.toLowerCase() === c.hex ? " is-on" : ""}`} style={{ background: c.hex }} onClick={() => save({ accentColor: c.hex })} />
          ))}
        </div>
      </SettingRow>
      <Divider />
      <SettingRow title="Animations" hint="Turn off to remove interface transitions.">
        <Toggle checked={s.animations} onChange={(v) => save({ animations: v })} label="Animations" />
      </SettingRow>
      <Divider />
      <SettingRow title="Interface scale" hint="Zooms the whole interface.">
        <div className="set__range">
          <input type="range" min={0.8} max={1.3} step={0.05} value={s.uiScale} onChange={(e) => save({ uiScale: Number(e.target.value) })} aria-label="Interface scale" />
          <output>{Math.round(s.uiScale * 100)}%</output>
        </div>
      </SettingRow>
      <Divider />
      <SettingRow title="Window buttons">
        <SegmentedControl options={[{ value: "windows", label: "Windows" }, { value: "mac", label: "Round" }]} value={windowControlsStyle} onChange={setWindowControlsStyle} size="sm" aria-label="Window buttons" />
      </SettingRow>
    </Card>
  );
}
