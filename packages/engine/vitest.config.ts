import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    name: 'engine',
    include: ['test/**/*.test.ts'],
    // A year of this world is 52 periods of every market, every participant and the whole audit,
    // and it grows every time a system is added — and the world is now a country: thirty million
    // people, three thousand named firms, thirty banks (docs/BUGS.md 12-12). The tests build SCALE
    // MODELS of it (test/rig.ts), but a rig with enough firms in it to have listed two of them is
    // still fifty firms and half a million people, and a year of that is a minute.
    //
    // The default five seconds is the harness's own, not a statement about the model: a test that
    // runs a year takes as long as a year takes.
    testTimeout: 180_000,
  },
});
