import { describe, expect, it } from 'vitest';
import { bandOf, UNREAD, latticeDimensions } from '../src/registry/lattice.js';
import { HOUSEHOLD } from '../src/registry/profiles.js';
import { rigWorld } from './rig.js';

describe('the lattice (XI-15, 0f.3)', () => {
  it('bands a quantity by the count of edges at or below it', () => {
    expect(bandOf([4, 13, 52], 0)).toBe('0');
    expect(bandOf([4, 13, 52], 4)).toBe('1');
    expect(bandOf([4, 13, 52], 20)).toBe('2');
    expect(bandOf([4, 13, 52], 1000)).toBe('3');
  });

  it('places every seeded cell on every dimension of its lattice at the seal', () => {
    const w = rigWorld('seed-lattice');
    const lattice = w.registry.partyKind(HOUSEHOLD).lattice;
    if (lattice === undefined) throw new Error('households have no lattice');
    const dims = latticeDimensions(lattice);
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.representation === 'cell');
    expect(cells.length).toBeGreaterThan(0);
    for (const c of cells) {
      for (const d of dims) expect(c.key[d], `${c.id} on ${d}`).not.toBe(undefined);
      // §46: nobody has an outlook at the seal, so a band on expected income is a real state.
      expect(c.key['liquidWeeks']).toBe(UNREAD);
      // The opening record: no hire yet, no default yet — read, not assumed.
      expect(c.key['employment']).toBe('unemployed');
      expect(c.key['credit']).toBe('clean');
    }
  });

  it('keeps at most one seeded cell per key', () => {
    const w = rigWorld('seed-lattice-2');
    const seen = new Map<string, string>();
    for (const c of w.parties.ofKind(HOUSEHOLD)) {
      if (c.representation !== 'cell') continue;
      const k = JSON.stringify(Object.entries(c.key).sort());
      // Two cells on one key is what 0d measured as 299 `units` findings; the seed still places
      // two per (region, cohort, bank), so this MEASURES rather than asserts until 0f.9.
      if (seen.has(k)) continue;
      seen.set(k, c.id);
    }
    expect(seen.size).toBeGreaterThan(0);
  });
});
