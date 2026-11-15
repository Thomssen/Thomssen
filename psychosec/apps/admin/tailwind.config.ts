import type { Config } from "tailwindcss";
export default {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: { bg: "#08090B", panel: "#0D0F12", line: "#292B2F", fg: "#F2F2F2", dim: "#858585", accent: "#87BDBB", ok: "#91C5A5", bad: "#D78585", warn: "#D6BF8A" },
      fontFamily: { sans: ["Inter", "ui-sans-serif", "system-ui", "sans-serif"], mono: ["ui-monospace", "SFMono-Regular", "Consolas", "monospace"] },
    },
  },
  plugins: [],
} satisfies Config;
