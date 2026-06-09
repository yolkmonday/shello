/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        otter: {
          dark: 'rgb(var(--otter-dark) / <alpha-value>)',
          surface: 'rgb(var(--otter-surface) / <alpha-value>)',
          card: 'rgb(var(--otter-card) / <alpha-value>)',
          border: 'rgb(var(--otter-border) / <alpha-value>)',
          teal: 'rgb(var(--otter-teal) / <alpha-value>)',
          'teal-dim': 'rgb(var(--otter-teal-dim) / <alpha-value>)',
          'teal-glow': 'rgb(var(--otter-teal) / 0.12)',
          amber: 'rgb(var(--otter-amber) / <alpha-value>)',
          coral: 'rgb(var(--otter-coral) / <alpha-value>)',
          blue: 'rgb(var(--otter-blue) / <alpha-value>)',
          purple: 'rgb(var(--otter-purple) / <alpha-value>)',
          text: 'rgb(var(--otter-text) / <alpha-value>)',
          muted: 'rgb(var(--otter-muted) / <alpha-value>)',
          subtle: 'rgb(var(--otter-subtle) / <alpha-value>)',
        },
      },
      fontFamily: {
        body: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
}
