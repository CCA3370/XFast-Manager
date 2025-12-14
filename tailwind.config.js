/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        aviation: {
          dark: '#0a0e1a',
          blue: '#1e3a8a',
          gray: '#1f2937',
        }
      }
    },
  },
  plugins: [],
}
