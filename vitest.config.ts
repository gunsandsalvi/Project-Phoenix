import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    projects: ['packages/engine', 'tools'],
    coverage: {
      provider: 'v8',
      include: ['packages/engine/src/**'],
    },
  },
});
