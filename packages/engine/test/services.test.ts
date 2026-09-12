/**
 * The rest of the economy (13c.2): the things that cannot be put in a box, the distance between the
 * gate and the shelf, and a basket whose preference is a quantity.
 *
 * @spec Goods A1 Goods A2.a Goods C1 Goods C3 Households A2.a Households A2.b Households C3 Households C4 Commodities Spot A3 Freight A3 Law 2 Law 6
 */
import { describe, expect, it } from 'vitest';
import { downTick, rungsUpTo, sum } from '../src/index.js';
import { CONSUMPTION } from '../src/mechanisms/households/data.js';
import { GOODS } from '../src/mechanisms/goods/data.js';

describe('a thing that cannot be put in a box (Freight A3, Commodities Spot A3)', () => {
  it('says of every line whether a unit of it can be somewhere other than where it was made', () => {
    for (const g of GOODS) expect(typeof g.portable).toBe('boolean');
  });

  it('never declares a store for something that cannot be moved', () => {
    // A store is somewhere a thing waits that is not where it will be used, and getting it there is
    // the move the line has just said is impossible. The seed throws on the pair; this is the same
    // statement made where a reader can see it.
    for (const g of GOODS) {
      if (!g.portable) expect(g.storagePerUnit).toBeNull();
    }
  });
});

describe('a preference is a QUANTITY, not a share of spending (Goods A2.a, Households C3)', () => {
  it('declares what a member takes in its own physical unit and never a share', () => {
    const units = new Map(GOODS.map((g) => [g.subUnit, g.unit]));
    for (const row of CONSUMPTION) {
      // 13c.2: the row that used to say `share: 1` said nothing about bread and everything about
      // money. These two say how much bread, which is a fact about people (Law 2).
      expect(row).not.toHaveProperty('share');
      expect(units.has(row.subUnit)).toBe(true);
      expect(row.neededPerMember).toBeGreaterThanOrEqual(0);
      expect(row.wantedPerMember).toBeGreaterThanOrEqual(0);
      expect(row.neededPerMember + row.wantedPerMember).toBeGreaterThan(0);
    }
  });

  it('lets two cohorts take different amounts of the same thing, which is what A2.a needs', () => {
    const byGood = new Map<string, number[]>();
    for (const row of CONSUMPTION) {
      const list = byGood.get(row.subUnit) ?? [];
      list.push(row.neededPerMember + row.wantedPerMember);
      byGood.set(row.subUnit, list);
    }
    const shared = [...byGood.values()].filter((l) => l.length > 1);
    expect(shared.length).toBeGreaterThan(0);
    // A2.a: the same income in different hands is different demand, and it can only be that if the
    // hands want different things. Two cohorts wanting identical baskets would be one cohort.
    expect(shared.some((l) => new Set(l).size > 1)).toBe(true);
  });
});

describe('the curve under a want (Goods C1, Clearing A2, Law 6)', () => {
  const LEVELS = [10, 8, 6, 4, 2];

  it('is the money divided by the price while the money binds', () => {
    // A want it cannot reach at any of these levels: the curve is the budget curve exactly.
    const rungs = rungsUpTo(LEVELS, 100, 1000);
    expect(sum(rungs.map((r) => r.qty)).value).toBe(downTick(100 / 2));
    expect(rungs[0]?.qty).toBe(downTick(100 / 10));
  });

  it('is flat at what it wanted once the price has fallen far enough', () => {
    const rungs = rungsUpTo(LEVELS, 100, 12);
    // It never takes more than it wanted, however cheap the thing got: a household does not buy
    // grain by the lorry-load because it is cheap, and nothing caps it — the want is a quantity it
    // named itself and the arithmetic takes whichever ran out first.
    expect(sum(rungs.map((r) => r.qty)).value).toBe(downTick(12));
    let running = 0;
    for (const r of rungs) {
      running += r.qty;
      expect(running).toBeLessThanOrEqual(downTick(12));
      expect(r.qty).toBeGreaterThan(0);
    }
  });

  it('refines without moving: every level of a coarse grid says the same on a fine one (Law 2)', () => {
    const coarse = rungsUpTo([10, 6, 2], 100, 12);
    const fine = rungsUpTo([10, 8, 6, 4, 2], 100, 12);
    for (const level of [10, 6, 2]) {
      const upToCoarse = sum(coarse.filter((r) => r.price >= level).map((r) => r.qty)).value;
      const upToFine = sum(fine.filter((r) => r.price >= level).map((r) => r.qty)).value;
      expect(upToFine).toBe(upToCoarse);
    }
  });

  it('asks for nothing when it has no money or wants none of it', () => {
    expect(rungsUpTo(LEVELS, 0, 10)).toEqual([]);
    expect(rungsUpTo(LEVELS, 100, 0)).toEqual([]);
    expect(rungsUpTo([0, -1], 100, 10)).toEqual([]);
  });
});
