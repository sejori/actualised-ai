import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath } from 'node:url'

export default defineConfig(({ mode }) => ({
  base: mode === 'pages' ? '/actualised-ai/' : '/',
  define: {
    'import.meta.env.VITE_ORCHESTRATOR_MODE': JSON.stringify(mode === 'cloud' ? 'remote' : 'local'),
  },
  resolve: {
    alias: mode === 'cloud' ? {
      './company-runtime': fileURLToPath(new URL('./src/company-runtime.cloud.ts', import.meta.url)),
    } : {},
  },
  plugins: [tailwindcss(), solid()],
  build: {
    minify: false
  }
}))
