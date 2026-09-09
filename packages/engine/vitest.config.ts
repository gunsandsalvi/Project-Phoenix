import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    name: 'engine',
    include: ['test/**/*.test.ts'],
    // A year of this world is 52 periods of every market, every participant and the whole audit,
    // and it grows every time a system is added. The default five seconds is the harness's own,
    // not a statement about the model: a test that runs a year takes as long as a year takes.
    testTimeout: 30_000,
  },
});
