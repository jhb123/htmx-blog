/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./templates/**/*.{html.tera,js}'],
  theme: {
    extend: {
      fontFamily: {
        'serif': [ 'Alice','ui-serif'],
        'sans': ['Work Sans', 'ui-sans']
      },
    },
  },
  plugins: [],
}

