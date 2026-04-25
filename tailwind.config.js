/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        background: '#171717',
        surface: '#0f0f0f',
        border: {
          DEFAULT: '#242424',
          subtle: '#242424',
          card: '#2e2e2e',
          hover: '#363636',
        },
        brand: {
          DEFAULT: '#3ecf8e',
          hover: '#00c573',
          translucent: 'rgba(62, 207, 142, 0.3)',
        },
        text: {
          primary: '#fafafa',
          secondary: '#b4b4b4',
          muted: '#898989',
        },
        danger: '#e5484d',
      },
      fontFamily: {
        sans: ['Circular', 'system-ui', 'sans-serif'],
        mono: ['Source Code Pro', 'monospace'],
      },
    },
  },
  plugins: [],
}
