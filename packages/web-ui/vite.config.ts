import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  base: '/actualised-ai/',
  plugins: [tailwindcss(), solid()],
  build: {
    minify: false
  }
})
