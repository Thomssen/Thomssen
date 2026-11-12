/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        display: ['"Space Grotesk"', 'Inter', 'system-ui', 'sans-serif'],
        sans: ['Inter', 'system-ui', '-apple-system', 'Segoe UI', 'sans-serif'],
      },
      colors: { ink: '#050507' },
      keyframes: {
        rainbow: { '0%': { backgroundPosition: '0% 50%' }, '100%': { backgroundPosition: '200% 50%' } },
        hue: { '0%': { filter: 'hue-rotate(0deg)' }, '100%': { filter: 'hue-rotate(360deg)' } },
      },
      animation: {
        rainbow: 'rainbow 4s linear infinite',
        hue: 'hue 3s linear infinite',
      },
    },
  },
  plugins: [],
}
