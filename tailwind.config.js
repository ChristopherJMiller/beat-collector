/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/templates/**/*.rs",
    "./src/handlers/html.rs",
  ],
  theme: {
    extend: {
      colors: {
        'primary': '#10b981', // green-500 for music theme
      },
    },
  },
  plugins: [],
}
