import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1431,
    strictPort: true,
    watch: {
      // Cargo writes to the workspace root's target directory, outside src-tauri.
      // Watching build artifacts can fail on Windows while the linker holds an executable.
      ignored: ['**/src-tauri/**', '**/target/**', '**/target-*/**'],
    },
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    // Production optimizations
    minify: 'esbuild',
    esbuild: {
      drop: ['console', 'debugger'], // Remove console.log and debugger in production
    },
    rollupOptions: {
      output: {
        manualChunks: {
          'vue-vendor': ['vue', 'vue-router', 'pinia'],
          'i18n-vendor': ['vue-i18n'],
        },
      },
    },
  },
})
