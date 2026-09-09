import { defineConfig } from 'vite';

// GitHub Pages serves the site under /<repo>/; the Android WebView serves it from root.
const base = process.env['PHOENIX_BASE'] ?? '/Project-Phoenix/';

export default defineConfig({
  base,
  build: {
    target: 'es2022',
    sourcemap: true,
  },
  worker: {
    format: 'es',
  },
});
