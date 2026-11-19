/** Color math for the selectable accent (Settings > Appearance). Given one
 * hex color from the 10-color palette, derives a hover/pressed shade and a
 * low-opacity tint - the same two derived values every other accent color
 * in the app (buttons, toggles, focus rings, ...) already needs. Tuned so
 * the default #5865F2 reproduces the hand-picked Discord Blurple hover
 * (#4752C4) almost exactly - this isn't a coincidence, it's what the
 * darken-and-desaturate amounts below were chosen to do - so switching from
 * the old hardcoded shades to this formula doesn't visibly change the
 * default theme, only the other 9 colors. */

/** The 6 selectable accent colors - shared between Settings > Appearance
 * and the first-run setup wizard's Appearance step, so there is exactly
 * one place that defines "what colors are there to pick from". */
export const ACCENT_COLORS: { name: string; hex: string }[] = [
  { name: "Original", hex: "#ffffff" },
  { name: "Blurple", hex: "#5865f2" },
  { name: "Cyan", hex: "#5ec8e0" },
  { name: "Rose", hex: "#e0609e" },
  { name: "Orange", hex: "#e08a3c" },
  { name: "Emerald", hex: "#4fb876" },
];

function hexToRgb(hex: string): [number, number, number] {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex.trim());
  if (!m) return [88, 101, 242]; // malformed input - fall back to Discord Blurple rather than throw
  return [parseInt(m[1]!, 16), parseInt(m[2]!, 16), parseInt(m[3]!, 16)];
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  const d = max - min;
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    switch (max) {
      case r:
        h = ((g - b) / d) % 6;
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      default:
        h = (r - g) / d + 4;
    }
    h *= 60;
    if (h < 0) h += 360;
  }
  return [h, s * 100, l * 100];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  s /= 100;
  l /= 100;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let [r, g, b] = [0, 0, 0];
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

function toHex(n: number): string {
  return Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
}

// WCAG 2.1 relative luminance / contrast ratio - used only to figure out how
// far to darken a chromatic accent for the light theme (see below). Human
// vision (and this formula) weights green far more than blue, so a cyan or
// emerald at the same HSL lightness as Blurple reads much brighter - a flat
// lightness ceiling can't account for that, a measured contrast ratio can.
function relativeLuminance([r, g, b]: [number, number, number]): number {
  const [rs, gs, bs] = [r, g, b].map((c) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs! + 0.7152 * gs! + 0.0722 * bs!;
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  return la > lb ? (la + 0.05) / (lb + 0.05) : (lb + 0.05) / (la + 0.05);
}

const WHITE: [number, number, number] = [255, 255, 255];

export interface AccentShades {
  /** The picked hue, clamped to stay visible - used for the base accent. */
  base: string;
  /** Darker, less saturated - hover/pressed state on solid accent fills. */
  hover: string;
  /** The same color at low opacity - focus rings, subtle backgrounds. */
  soft: string;
  /** Text/icon color for content sitting directly on `base` - the primary
   * button, toggle-on thumb, selected segmented-control item. White for
   * every chromatic color (all of them are darkened enough, see below, to
   * keep it AA-legible) - except "Original", whose base is a light grey in
   * dark theme and a dark grey in light theme, so it needs the opposite
   * text color in each, exactly like the pre-Blurple design it's restoring
   * (dark text on its light button, light text on its dark one). */
  text: string;
}

export function deriveAccentShades(hex: string, theme: "dark" | "light" = "dark"): AccentShades {
  const [r, g, b] = hexToRgb(hex);
  let [h, s, l] = rgbToHsl(r, g, b);

  if (s === 0) {
    // "Original" (true grey/white/black - no hue to preserve) is the one
    // entry that predates this accent system entirely: before v1.0.2 the
    // app had no brand colour at all, just a neutral that was already
    // theme-adaptive on its own - near-white on the dark theme, near-black
    // on the light one (see tokens.css's pre-redesign history). A single
    // fixed grey can't read clearly against both a near-black and a
    // near-white surface, so this is the one pick that still depends on
    // which theme is active, exactly as it always did.
    l = theme === "light" ? 18 : 92;
  } else {
    // Every chromatic palette entry already sits well inside both ranges
    // (the default #5865F2 is 65% lightness, 86% saturation), so this
    // clamp is normally a no-op - it only exists to guarantee a visible,
    // legible accent even for a genuinely extreme pick.
    s = Math.max(s, 35);
    l = Math.min(Math.max(l, 42), 70);

    // Dark theme stops here, unchanged from every version before this one -
    // these colors were already tuned and shipped against near-black
    // surfaces. Light theme is a different story: white button text (used
    // on every accent-colored fill - buttons, toggle-on, the selected
    // segmented-control item) needs 4.5:1 contrast to stay AA-legible, and
    // because green/cyan/orange hues read far brighter than blue/purple at
    // the very same lightness, the clamp above leaves some hues (Cyan,
    // Orange, Emerald measured well under 3:1) washed out against both
    // white text and the light theme's white/near-white surfaces - the
    // same weakness Blurple never had, since 65% lightness blue already
    // happens to clear the bar. Darkening per-hue only as far as actually
    // needed fixes both at once (this is also what makes the color
    // readable as a thin element - a border, the sidebar's active
    // indicator, a stat number - against a white card, since that's the
    // exact same contrast calculation) while leaving Blurple untouched and
    // every color's own saturation/hue exactly as picked.
    if (theme === "light") {
      let rgb = hslToRgb(h, s, l);
      while (l > 20 && contrastRatio(rgb, WHITE) < 4.5) {
        l -= 1;
        rgb = hslToRgb(h, s, l);
      }
    }
  }
  const [r2, g2, b2] = hslToRgb(h, s, l);

  // Hover moves toward the middle of the lightness range rather than
  // always darkening - for every chromatic colour that's the same thing
  // (they all start light, at l >= 42), but for achromatic "Original"'s
  // near-black light-theme base, darkening further would crowd the 15%
  // floor into barely-there; lightening it a touch stays legible and still
  // reads as a distinct hover state.
  const hoverL = s === 0 && theme === "light" ? Math.min(85, l + 12) : Math.max(15, l - 12);
  const [hr, hg, hb] = hslToRgb(h, s * 0.6, hoverL);

  // Same pairing the pre-Blurple design used: #0a0b0c on its light button
  // (dark theme), #f7f7f8 on its dark one (light theme) - reusing those
  // exact values rather than inventing new ones.
  const text = s === 0 ? (theme === "light" ? "#f7f7f8" : "#0a0b0c") : "#ffffff";

  return {
    base: `#${toHex(r2)}${toHex(g2)}${toHex(b2)}`,
    hover: `#${toHex(hr)}${toHex(hg)}${toHex(hb)}`,
    soft: `rgba(${r2}, ${g2}, ${b2}, 0.16)`,
    text,
  };
}
