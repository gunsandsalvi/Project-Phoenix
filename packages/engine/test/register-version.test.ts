/**
 * The register's write count, which is what a kept answer is allowed to trust (0g.5).
 *
 * @spec Law 18 Law 19 Law 4
 *
 * `view.memo` hands back an answer while every version it was computed at still stands, so a
 * writer that did not bump the register's version would hand a reader something the world has left
 * behind — the one way a cache can lie. That is a rule that can be a check, so it is one: every
 * public writer of the register is exercised here and the count must move.
 */
import { describe, expect, it } from 'vitest';
import { rigWorld } from './rig.js';

describe('every write to the register moves its version (Law 18, 0g.5)', () => {
  it('moves while a world runs, and never moves backwards', () => {
    const w = rigWorld('register.version');
    let last = w.register.version;
    expect(last).toBeGreaterThan(0);
    for (let i = 0; i < 6; i += 1) {
      w.step();
      const now = w.register.version;
      // A period of this world settles instructions, so the count rises — and it is monotone,
      // because it counts writes rather than describing a state.
      expect(now).toBeGreaterThan(last);
      last = now;
    }
  });

  it('does not move when nothing is written', () => {
    const w = rigWorld('register.version.still');
    const at = w.register.version;
    // Reads are reads: asking the register anything, however much of it, writes nothing.
    for (const p of w.parties.all()) w.register.holdingsOf(p.id);
    for (const i of w.instruments.all()) w.register.holdersOf(i.id);
    expect(w.register.version).toBe(at);
  });
});
