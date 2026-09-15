/**
 * The small-business tier: firms with a weight, and a distribution rather than an average.
 *
 * @spec Small-Business Pools A1 Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A5 Small-Business Pools A6 Small-Business Pools A6.a Small-Business Pools A6.b Small-Business Pools E5 Seed B1.a XI-15 Law 2 Law 15
 *
 * ITEM 11, STEPS 1–4. §42 was 2 of 28 MET and both were the generic cell kernel: nothing in this
 * engine was a small firm. What is asserted here is that the SECTOR EXISTS and that it is a
 * distribution — A2.a is the clause the whole item turns on, and a draw that came out flat would
 * satisfy every other clause and remove the credit content of §42 without failing anything.
 */
import { describe, expect, it } from 'vitest';
import { CELLS_PER_KEY, SMALL_FIRM, SMALL_PER_NAMED, drawSmallBusiness, smallBusiness } from '../src/index.js';
import { rigDraw, rigWorld } from './rig.js';

describe('the sector exists, and it is cells with weights (A1, A6, XI-15)', () => {
  it('opens with small firms in it, each a cell standing for a count of them', () => {
    const w = rigWorld('sb');
    const cells = w.parties.ofKind(SMALL_FIRM);
    expect(cells.length).toBeGreaterThan(0);
    for (const c of cells) {
      // XI-15, E5: a weight is a COUNT of real firms, never a share and never a fraction.
      expect(c.representation).toBe('cell');
      if (c.representation !== 'cell') continue;
      expect(Number.isInteger(c.weight)).toBe(true);
      expect(c.weight).toBeGreaterThan(0);
    }
  });

  it('keys them on what their members must all share, and NEVER on a lender (A6.a)', () => {
    const w = rigWorld('sb');
    for (const c of w.parties.ofKind(SMALL_FIRM)) {
      if (c.representation !== 'cell') continue;
      // A6.a: region and bank are dimensions of the key; the line is a third, and it is forced —
      // a cell whose members were in different lines would have an averaged cost base.
      expect(Object.keys(c.key).sort()).toEqual(['bank', 'line', 'region']);
      // And the LENDER is not one of them: a lender is a loan row per (lender, cell), and lifting
      // it into the key is the relationship the model would then be unable to name.
      expect(Object.keys(c.key)).not.toContain('lender');
      // A cell lives where its bank books, and there is one statement of that (Law 4, Law 19).
      expect(c.key.region).toBe(String(w.parties.get(c.bank).region));
    }
  });

  it('is a DISTRIBUTION and not an average (A2.a — the clause the item turns on)', () => {
    const drew = rigDraw('sb');
    const sizes = drew.small.map((r) => r.size);
    expect(sizes.length).toBeGreaterThan(CELLS_PER_KEY);
    // A2.a: no representative small firm. A mean-preserving spread has to be able to move the count
    // of defaults, which it cannot if every member is the same size.
    expect(Math.max(...sizes)).toBeGreaterThan(Math.min(...sizes) * 2);
    // A3: and the dispersion is WITHIN a key, not only across keys — otherwise every firm banking
    // at one bank in one line is the same firm, which is A2.a one level down.
    const one = drew.small[0];
    if (one === undefined) throw new Error('this world drew no small firms');
    const first = drew.small.filter((r) => r.bank === one.bank && r.line === one.line);
    expect(new Set(first.map((r) => r.size)).size).toBeGreaterThan(1);
  });

  it('scales with the world and states no count of its own (Seed B1.a)', () => {
    const drew = rigDraw('sb');
    const total = drew.small.reduce((t, r) => t + r.weight, 0);
    // The sector is this world's named firms times a multiple, so a bigger world has more corner
    // shops. A count written in the module would have made it a fixed size the seed grew away from.
    expect(total).toBeGreaterThan(drew.firms.length);
    expect(SMALL_PER_NAMED).toBeGreaterThan(1);
  });

  it('declares what it does NOT do yet, rather than an empty phase that reads as done', () => {
    const m = smallBusiness([]);
    // 11.5–11.10: it sells, employs, borrows, defaults and is promoted, and each is a step. A
    // module with phases that ran and did nothing would be the "built on paper and dead in the
    // world" state this plan exists to find.
    expect(m.phases).toEqual([]);
    expect(m.participants).toEqual([]);
    expect(m.partyKinds.map((k) => String(k.id))).toEqual(['smallFirm']);
  });

  it('draws nothing where there is nothing to draw from (App A)', () => {
    expect(drawSmallBusiness([], [{ bank: 'bank.a', size: 1 }], 100, 's')).toEqual([]);
    expect(drawSmallBusiness(['bakery'], [], 100, 's')).toEqual([]);
    expect(drawSmallBusiness(['bakery'], [{ bank: 'bank.a', size: 1 }], 0, 's')).toEqual([]);
  });
});
