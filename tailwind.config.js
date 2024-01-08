/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./templates/**/*.{html.tera,js}'],
  theme: {
    extend: {
      fontFamily: {
        'serif': [ 'Alice','ui-serif'],
        'sans': ['Work Sans', 'ui-sans']
      },
      dropShadow: {
        glow: [
          "0 0px 10px rgba(16, 185, 129, 0.4)",
          "0 0px 10px rgba(16, 185, 129, 0.4)"
        ]
      }
    },
  },
  plugins: [],
}

