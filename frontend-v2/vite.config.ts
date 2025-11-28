import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  base: '/v2/', // Relative base path for static hosting
  build: {
    outDir: '../static/v2',
    emptyOutDir: true
  },
  server: {
    host: '0.0.0.0', // Listen on all addresses
    port: 1393,
    hmr: {
      // Force HMR to use the correct port if behind a proxy/tunnel
      clientPort: 1393
    },
    watch: {
      // Use polling if file system events aren't propagating (common in some VMs/Docker)
      usePolling: true
    },
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true
      },
      '/ws': {
        target: 'ws://localhost:3000',
        ws: true
      }
    }
  }
})
// Config updated to trigger restart
