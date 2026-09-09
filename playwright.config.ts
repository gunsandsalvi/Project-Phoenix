import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'packages/app/e2e',
  timeout: 60_000,
  use: {
    baseURL: 'http://127.0.0.1:4173/Project-Phoenix/',
    viewport: { width: 448, height: 998 }, // Pixel-class portrait CSS viewport
    // A preinstalled Chromium can be named instead of downloading one (PHOENIX_CHROMIUM=/path/to/chrome).
    launchOptions:
      process.env['PHOENIX_CHROMIUM'] === undefined
        ? {}
        : { executablePath: process.env['PHOENIX_CHROMIUM'] },
  },
  webServer: {
    command: 'npm run preview --workspace packages/app -- --host 127.0.0.1 --port 4173',
    url: 'http://127.0.0.1:4173/Project-Phoenix/',
    reuseExistingServer: !process.env['CI'],
    timeout: 60_000,
  },
});
