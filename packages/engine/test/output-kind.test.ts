/**
 * Stock or capacity: whether a line's output can be HELD, and what a lease buys.
 *
 * @spec Goods A1 Goods A3 Goods E4 Commodities Spot A3 Commodities Spot D3 Capital Programme A2 Law 2 Law 4 Law 19
 */
import { describe, expect, it } from 'vitest';
import { GOODS, spoilageOf, type GoodDecl } from '../src/mechanisms/goods/data.js';
import { STORAGE, rentedRoom } from '../src/registry/physical.js';
import { none, some } from '../src/core/option.js';
import { ranWorld } from './rig.js';

describe('what a line makes: a thing, or an hour of somebody time', () => {
  it('every good says which, and the two are not the same question as portability', () => {
    const capacity = GOODS.filter((d) => d.output === 'capacity');
    const stock = GOODS.filter((d) => d.output === 'stock');
    expect(capacity.length).toBeGreaterThan(0);
    expect(stock.length).toBeGreaterThan(0);
    /**
     * POWER IS THE CASE THAT PROVES THEY ARE TWO FACTS. It is the most movable thing in the file
     * and nobody stores a megawatt-hour: `portable: true`, `output: 'capacity'`. `portable` used
     * to be called "the one fact that divides a manufacture from a service", and reading it for
     * this question is what put a school's unsold teaching hours on its balance sheet as inventory.
     */
    const power = GOODS.find((d) => d.subUnit === 'power');
    expect(power?.portable).toBe(true);
    expect(power?.output).toBe('capacity');
  });

  it('a capacity line does not declare a spoilage, because nobody chose it (Law 2)', () => {
    for (const d of GOODS) {
      if (d.output === 'capacity') {
        // The count of declared numbers falls by one per service line: `spoilagePerPeriod: 1`
        // eighteen times was a shape standing in for what `output` now says.
        expect(d.spoilagePerPeriod).toBeNull();
        expect(spoilageOf(d)).toBe(1);
      } else {
        expect(d.spoilagePerPeriod).not.toBeNull();
        expect(spoilageOf(d)).toBe(d.spoilagePerPeriod);
      }
    }
  });

  it('refuses a line that declares both — a contradiction, not a hint', () => {
    const service = GOODS.find((d) => d.output === 'capacity');
    expect(service).toBeDefined();
    if (service === undefined) return;
    const both: GoodDecl = { ...service, spoilagePerPeriod: 0.5 };
    expect(() => spoilageOf(both)).toThrow(/makes nothing that waits and declares a spoilage/);
    const neither: GoodDecl = { ...GOODS[0], output: 'stock', spoilagePerPeriod: null } as GoodDecl;
    expect(() => spoilageOf(neither)).toThrow(/does not say how fast it goes off/);
  });

  it('nothing that cannot be held asks for somewhere to put it', () => {
    // Commodities Spot A3: the FORBID assembly runs, asserted here on the data it runs over.
    for (const d of GOODS) {
      if (d.output === 'capacity') expect(d.storagePerUnit).toBeNull();
    }
  });
});

describe('what a lease buys (A-64)', () => {
  it('is room, and the capacity read counts it', () => {
    /**
     * A firm short of room bid for it, won, and paid the letter — and its capacity did not move,
     * because the read counted the storage VINTAGES it owned and a lease is not a holding. So next
     * period it was short of exactly the same room and rented it again. The money was conserved,
     * so no audit family could see it: the buyer of a service that did not exist (Law 1).
     */
    expect(rentedRoom(none())).toEqual(new Map());
    expect(rentedRoom(some({ data: { space: 40 } }))).toEqual(new Map([[STORAGE, 40]]));
    // A lease of nothing is not a lease, and a malformed event buys nothing rather than NaN of it.
    expect(rentedRoom(some({ data: { space: 0 } }))).toEqual(new Map());
    expect(rentedRoom(some({ data: {} }))).toEqual(new Map());
  });

  it('is one event per taker per period, carrying what it rented in TOTAL (Law 4)', () => {
    /**
     * A taker that matched three letters rented one amount of room, and three events would have
     * made its own capacity read see the last of them. The per-match events are still there under
     * their own kind — `commodities.let` — because who let it to whom is a real fact and the
     * letter's income has a payer.
     */
    const w = ranWorld('storage', 10);
    const totals = w.journal.ofKind('commodities.leased');
    for (const e of totals) {
      expect(e.subjects.length).toBe(1);
      expect(Number(e.data['space'])).toBeGreaterThan(0);
    }
    // One per taker per period at most: the same party never appears twice in one period.
    for (let p = 0; p <= w.period; p += 1) {
      const inPeriod = totals.filter((e) => e.period === p).map((e) => e.subjects[0]);
      expect(new Set(inPeriod).size).toBe(inPeriod.length);
    }
    // And every total is the sum of that taker's own matches, read from the other kind (Law 19).
    for (const e of totals) {
      const matches = w.journal
        .ofKind('commodities.let')
        .filter((x) => x.period === e.period && x.data['taker'] === e.data['taker']);
      const summed = matches.reduce((t, x) => t + Number(x.data['space']), 0);
      expect(Number(e.data['space'])).toBe(summed);
    }
  });

  it('a failed payment takes no room (Money E1)', () => {
    // The decrements used to sit outside the settled branch, so a taker whose rent did not settle
    // absorbed the letter's space anyway — one insolvent taker could shut a region's storage
    // market for the period. Every `commodities.let` names a payment that actually settled.
    const w = ranWorld('storage', 10);
    for (const e of w.journal.ofKind('commodities.let')) {
      expect(Number(e.data['paid'])).toBeGreaterThan(0);
      expect(Number(e.data['space'])).toBeGreaterThan(0);
    }
  });
});
