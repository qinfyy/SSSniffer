import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    vue()
  ],

  build: {
    sourcemap: true,
    chunkSizeWarningLimit: 999999 // vite别叫！
  }
})
