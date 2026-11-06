import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Rutas relativas: WebView2 sirve dist/ desde un host virtual o desde disco.
export default defineConfig({
  base: './',
  plugins: [vue()],
  build: { target: 'chrome110' },
})
