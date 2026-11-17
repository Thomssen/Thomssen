import { useEffect, useRef, useState } from "react";
import { Card, Notice, PageHeader, Row, Slider, Toggle } from "../components/ui";
import { api, type Settings, type Style } from "../lib/api";
import { useApp } from "../lib/app";

function Preview({ appearance, style, selected, onPick, title, desc }: { appearance: Settings["appearance"]; style: Style; selected: boolean; onPick: () => void; title: string; desc: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const seq = useRef(0);
  const key = JSON.stringify({ ...appearance, style });
  useEffect(() => {
    const my = ++seq.current;
    const t = setTimeout(async () => {
      try {
        const { w, h, px } = await api.previewStyle({ ...appearance, style });
        if (my !== seq.current || !ref.current) return;
        ref.current.width = w; ref.current.height = h;
        ref.current.getContext("2d")!.putImageData(new ImageData(px, w, h), 0, 0);
      } catch { /* preview is cosmetic */ }
    }, 90);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return (
    <button onClick={onPick} className={`focus-ring overflow-hidden rounded-xl2 border text-left transition-all duration-200 ${selected ? "border-zinc-200 bg-ink-800" : "border-ink-700 bg-ink-850 hover:border-ink-500"}`}>
      <canvas ref={ref} className="block w-full" style={{ aspectRatio: "560/116" }} />
      <div className="flex items-center justify-between px-4 py-3">
        <div><div className="font-medium">{title}</div><div className="text-[12px] text-mute-400">{desc}</div></div>
        <span className={`h-4 w-4 rounded-full border-2 transition-all ${selected ? "border-zinc-100 bg-zinc-100 shadow-[inset_0_0_0_3px_#17171b]" : "border-ink-500"}`} />
      </div>
    </button>
  );
}

export default function Appearance() {
  const { settings, patch } = useApp();
  const [, force] = useState(0);
  useEffect(() => force(1), []);
  if (!settings) return null;
  const a = settings.appearance;
  const set = (fn: (x: Settings["appearance"]) => void) => patch((s) => fn(s.appearance));
  return (
    <>
      <PageHeader title="Appearance" subtitle="How censored information looks. The previews below are rendered by the same engine that draws on your screen." />
      <div className="mb-4 grid grid-cols-3 gap-3">
        <Preview appearance={a} style="gaussian" selected={a.style === "gaussian"} onPick={() => set((x) => { x.style = "gaussian"; })} title="Gaussian blur" desc="Soft, natural look" />
        <Preview appearance={a} style="pixelate" selected={a.style === "pixelate"} onPick={() => set((x) => { x.style = "pixelate"; })} title="Pixelation" desc="Mosaic blocks" />
        <Preview appearance={a} style="solid" selected={a.style === "solid"} onPick={() => set((x) => { x.style = "solid"; })} title="Solid black" desc="Maximum certainty" />
      </div>

      <Card title="Strength" className="mb-4">
        <Slider label="Blur intensity" value={a.intensity} min={0} max={100} unit="%" onChange={(v) => set((x) => { x.intensity = v; })} hint="Scales the Gaussian radius between 1 px and the radius below." />
        <Slider label="Blur radius" value={a.blur_radius} min={2} max={60} unit=" px" onChange={(v) => set((x) => { x.blur_radius = v; })} hint="Maximum Gaussian radius (3-pass box approximation of a true Gaussian)." />
        <Slider label="Pixelation strength" value={a.pixel_size} min={3} max={40} unit=" px" onChange={(v) => set((x) => { x.pixel_size = v; })} hint="Size of each mosaic block." />
        <Slider label="Overlay opacity" value={a.opacity} min={20} max={100} unit="%" onChange={(v) => set((x) => { x.opacity = v; })} />
        <Slider label="Padding" value={a.padding} min={0} max={20} unit=" px" onChange={(v) => set((x) => { x.padding = v; })} hint="Extra margin around detected text so partial letters never peek out." />
      </Card>

      <Card title="Safety">
        <Row title="Safe minimum strength" desc="Never allow a blur or mosaic weaker than needed to make text unreadable for its size, whatever the sliders say. Turn off only if you want exactly the slider values.">
          <Toggle checked={a.safe_minimum} onChange={(v) => set((x) => { x.safe_minimum = v; })} />
        </Row>
        <div className="mt-3 space-y-2">
          {!a.safe_minimum && <Notice>With safe minimum off, low intensity or small mosaic blocks can leave text <b>legible or recoverable</b>.</Notice>}
          {a.opacity < 100 && <Notice>Opacity below 100 % lets the original text show through the overlay. It is <b>not protective</b>; use 100 % for real privacy.</Notice>}
          {a.style === "gaussian" && <Notice tone="info">The blur is computed on the CPU from a snapshot of the screen and then drawn opaque over the text. It is a real Gaussian-type filter, not a see-through tint, so with the safe minimum on it cannot be reversed by the viewer.</Notice>}
        </div>
      </Card>
    </>
  );
}
