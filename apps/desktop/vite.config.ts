import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Tauri devUrl is http://localhost:1420; binding all interfaces keeps the
// sandbox/browser preview working too. strictPort prevents silent port hops.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    // Dev-only: allow sandbox/proxied preview hosts (no effect on prod builds).
    allowedHosts: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  preview: { port: 1420, strictPort: true, host: true, allowedHosts: true },
  build: { target: 'es2022', sourcemap: false, chunkSizeWarningLimit: 1024 },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test-setup.ts'],
    include: ['src/**/*.{test,spec}.?(c|m)[jt]s?(x)'],
    testTimeout: 10_000,
  },
});
