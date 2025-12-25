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
  base: '/', // Serve from root
  build: {
    outDir: '../static',
    emptyOutDir: true
  },
  server: {
    host: '0.0.0.0', // Listen on all addresses
    port: 3000,
    hmr: {
      // Force HMR to use the correct port if behind a proxy/tunnel
      clientPort: 3000
    },
    watch: {
      // Use polling if file system events aren't propagating (common in some VMs/Docker)
      usePolling: true
    },
    proxy: {
      '/api': {
        target: 'http://localhost:11391',
        changeOrigin: true
      },
      '/ws': {
        target: 'ws://localhost:11391',
        ws: true
      }
    }
  }
})
// Config updated to trigger restart
