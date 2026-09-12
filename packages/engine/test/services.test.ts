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
import { OCCUPATION_OF } from '../src/mechanisms/firms/data.js';
import { OCCUPATIONS } from '../src/mechanisms/labour/data.js';

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

describe('the sixteen lines that cannot be put in a box (13c.2)', () => {
  const services = GOODS.filter((g) => !g.portable);

  it('has a service economy at all, and it is most of the lines a person works in', () => {
    expect(services.length).toBeGreaterThanOrEqual(16);
  });

  it('keeps nothing, waits for nothing and stands on no ground of its own', () => {
    for (const g of services) {
      // An hour nobody bought is not an hour waiting: it is gone, and the wages were paid anyway.
      expect(g.spoilagePerPeriod).toBe(1);
      // Made to order, which is what having no stock to make it from means.
      expect(g.leadTimePeriods).toBe(0);
      expect(g.storagePerUnit).toBeNull();
      // The ground is in the premises it is made in; counting it again would be counting it twice.
      expect(g.standsOn).toBeNull();
    }
  });

  it('is made of labour first, and of real things after (A2.a, A2.c)', () => {
    for (const g of services) {
      expect(g.labourHoursPerUnit).toBeGreaterThan(0);
      expect(g.plant.length).toBeGreaterThan(0);
      // Every input is a physical good this world makes, in its own physical unit.
      const known = new Set(GOODS.map((x) => x.subUnit));
      for (const i of g.inputs) expect(known.has(i.subUnit)).toBe(true);
    }
  });

  it('never consumes a good that consumes a service, so the chain still has a bottom', () => {
    // `openingLevels` walks the recipes from the bottom up and a cycle has no bottom. A world where
    // a machine works buys an engineer's week and the engineer's week buys a machine would have no
    // cost to work out, and the seed would be right to refuse it.
    const byName = new Map(GOODS.map((g) => [g.subUnit, g]));
    const serviceNames = new Set(services.map((g) => g.subUnit));
    const buysAService = new Set(
      GOODS.filter((g) => g.inputs.some((i) => serviceNames.has(i.subUnit))).map((g) => g.subUnit),
    );
    for (const s of services) {
      const seen = new Set<string>();
      const walk = (name: string): void => {
        if (seen.has(name)) return;
        seen.add(name);
        for (const i of byName.get(name)?.inputs ?? []) walk(i.subUnit);
      };
      for (const i of s.inputs) walk(i.subUnit);
      for (const reached of seen) expect(buysAService.has(reached)).toBe(false);
    }
  });

  it('is bought by firms as well as by people: the made goods name the services they use', () => {
    const serviceNames = new Set(services.map((g) => g.subUnit));
    const buyers = GOODS.filter((g) => g.inputs.some((i) => serviceNames.has(i.subUnit)));
    // A world where services were only ever final demand would have half of what services are.
    expect(buyers.length).toBeGreaterThan(5);
  });

  it('employs its own trade, and no two lines share one (Labour A3.a)', () => {
    const trades = services.map((g) => OCCUPATION_OF[g.subUnit]);
    for (const t of trades) expect(OCCUPATIONS.some((o) => o.id === t)).toBe(true);
    // A nurse out of work is not a bricklayer's vacancy filled, and a world where every service was
    // one trade would answer a shortage of clinicians by sending it a security guard.
    expect(new Set(trades).size).toBe(trades.length);
  });
});
