import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  build: {
    outDir: 'dist',
    target: 'esnext',
    // Nym SDK uses top-level await and large Wasm
    rollupOptions: {
      output: {
        manualChunks: {
          'nym-sdk': ['@nymproject/sdk-full-fat'],
        },
      },
    },
  },
  server: {
    port: 8080,
    headers: {
      // Required for SharedArrayBuffer (Nym Wasm uses it)
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  optimizeDeps: {
    exclude: ['@nymproject/sdk-full-fat'],
  },
});
