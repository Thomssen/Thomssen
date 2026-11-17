/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: { 950: "#08080a", 900: "#0c0c0e", 850: "#111114", 800: "#17171b", 700: "#212126", 600: "#2c2c33", 500: "#42424b" },
        mute: { 400: "#9a9aa4", 500: "#7a7a85", 600: "#5e5e68" },
      },
      fontFamily: { sans: ['"Segoe UI Variable Text"', '"Segoe UI"', "system-ui", "sans-serif"], mono: ['"Cascadia Code"', "Consolas", "monospace"] },
      borderRadius: { xl2: "18px" },
      keyframes: {
        rise: { "0%": { opacity: 0, transform: "translateY(6px)" }, "100%": { opacity: 1, transform: "none" } },
        pop: { "0%": { opacity: 0, transform: "scale(.97)" }, "100%": { opacity: 1, transform: "none" } },
      },
      animation: { rise: "rise .28s cubic-bezier(.2,.8,.2,1) both", pop: "pop .18s ease-out both" },
    },
  },
  plugins: [],
};
