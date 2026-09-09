import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'

export default defineConfig({
  base: '/actualised-ai/',
  plugins: [solid()],
})
