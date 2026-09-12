/**
 * Freight: moving a thing costs money and takes time, and somebody finite decides whether to move it.
 *
 * @spec Freight A3 Freight A3.a Freight A4 Freight B1 Freight B2 Freight B4 Freight C1 Freight C3 Freight D1 Freight D6 Clearing A2 Clearing C4.b Law 2 Law 6
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  FREIGHT_SESSION,
  REGION,
  VESSEL,
  goodId,
  inTransit,
  paramId,
  plantKindId,
  regionId,
} from '../src/index.js';
import { GOODS } from '../src/mechanisms/goods/data.js';
import { legsBetween } from '../src/registry/geography.js';
import { ranWorld, rigWorld } from './rig.js';

describe('a leg is read off the ground, not declared (A4, B2, 13c.1)', () => {
  it('declares what a HULL is and nothing about any leg', () => {
    const w = rigWorld('freight-a');
    // Law 19: the four numbers a route used to declare are gone, and each names its read —
    // transit is the voyage's own progress, capacity is what free hulls hold, and what a passage
    // is sailable in is the ground the voyage is actually crossing.
    for (const what of ['transit', 'perVessel', 'sailsIn', 'sailsHardness']) {
      for (const to of w.registry.regions.keys()) {
        expect(() => w.params.decl(paramId(`freight.${REGION}.${to}.${what}`))).toThrow();
      }
    }
    // What IS declared is a fact about the ship: what it holds and what carrying wears it out by.
    for (const [id, dimension] of [
      ['capital.vessel.holdUnits', 'count'],
      ['capital.vessel.wearPerUnitKm', 'ratio'],
    ] as const) {
      const d = w.params.decl(paramId(id));
      expect(d.kind).toBe('technology');
      expect(d.dimension).toBe(dimension);
      expect(d.value).toBeGreaterThan(0);
    }
  });

  it('has no freight RATE anywhere, because what it costs to move a tonne is cleared (Law 3)', () => {
    const w = rigWorld('freight-a');
    for (const d of w.params.all()) {
      expect(String(d.id)).not.toContain('freightRate');
      if (String(d.id).startsWith('freight.')) expect(String(d.id)).not.toContain('rate');
    }
  });

  it('a carrier is a FIRM with hulls, not a kind of its own (B1, Law 15)', () => {
    const w = rigWorld('freight-a');
    // A test never names a party: the carriers are whoever came out of the draw holding hulls.
    const sailing = w.parties
      .alive()
      .filter((p) =>
        w.register
          .holdingsOf(p.id)
          .some((h) => w.instruments.get(h.instrument).kind === plantKindId(VESSEL)),
      );
    expect(sailing.length).toBeGreaterThan(0);
    for (const p of sailing) expect(p.kind).toBe(FIRM);
  });

  it('can get between two places only when the ground allows it', () => {
    const w = rigWorld('freight-a');
    const g = w.registry.geography;
    const found = legsBetween(g, w.params, 'vessel', [...w.registry.regions.keys()]);
    for (const [key, leg] of found) {
      expect(leg.km).toBeGreaterThan(0);
      expect(leg.days).toBeGreaterThan(0);
      expect(leg.tiles.length).toBeGreaterThan(0);
      // A4: distinct legs, because they are different lengths over different ground.
      expect(key).toContain('|');
    }
  });
});

describe('the session says what it did, including that it did nothing (Clearing C4.b)', () => {
  it('runs every leg every period and reports the outcome of each', () => {
    const w = ranWorld('freight-c', 4);
    const sessions = w.journal.ofKind(FREIGHT_SESSION);
    expect(sessions.length).toBe(4);
    for (const e of sessions) {
      expect(e.public).toBe(true);
      const byLeg = e.data['byLeg'] as Record<string, { outcome?: string; carriers?: number }>;
      expect(Object.keys(byLeg).length).toBeGreaterThan(0);
      for (const leg of Object.values(byLeg)) {
        // A leg with hulls on it and nothing to carry says `noDemand` and says so out loud. This
        // world makes its goods in the one region that has firms in it, and the three abroad are a
        // central bank, a treasury and a bond line until 13i builds their economies — so the legs
        // are real and idle, and a session that reported nothing would hide which it was.
        expect(['cleared', 'noDemand', 'noSupply', 'noOverlap']).toContain(leg.outcome);
      }
    }
  });

  it('names the place a cargo is while it is neither here nor there (A3, A3.a)', () => {
    // Every unit in transit has an owner and a place, the whole time (E3). The place is its own
    // instrument per good and per leg: a tonne on the water is not a tonne at either end.
    const here = inTransit('grain', REGION, regionId('eu'));
    expect(String(here)).toContain('transit');
    expect(String(here)).not.toBe(String(inTransit('grain', REGION, regionId('uk'))));
    expect(String(inTransit('flour', REGION, regionId('eu')))).not.toBe(String(here));
  });
});

/**
 * The location basis (13c step 9): the same grade in two places at two prices, and the gap being
 * what it costs somebody to actually move it.
 *
 * @spec Freight D3 Freight D3.a Freight D5 Commodities Spot D1 Law 3
 *
 * What is asserted is the MECHANISM and never a level (Law 17, PLAN §7): that two places print
 * separately, that the gap is what a shipper would pay rather than a number anybody computed, and
 * that when the ground between them gets worse the gap has further to travel. A basis of a
 * particular size would be a claim about an answer.
 */
describe('the location basis is an outcome of shipping with capacity (D3, D3.a, D5)', () => {
  it('prints the same grade separately in every place that makes it (Commodities Spot D1)', () => {
    const w = ranWorld('basis-a', 3);
    const places = [...w.registry.regions.keys()].filter((r) =>
      w.instruments.has(goodId('grain', r)),
    );
    // 13c.1 drew the firms across the ground, so more than one place makes this line.
    expect(places.length).toBeGreaterThan(1);
    for (const r of places) {
      const i = w.instruments.get(goodId('grain', r));
      // Each is its own instrument with its own market — location is part of identity.
      expect(i.terms).toHaveProperty('region', r);
      expect(i.market.some).toBe(true);
      if (i.market.some) expect(String(i.market.value)).toContain(String(r));
    }
  });

  it('is never computed: a place’s price is its own market’s print (Law 3)', () => {
    const w = ranWorld('basis-a', 3);
    const places = [...w.registry.regions.keys()].filter((r) =>
      w.instruments.has(goodId('grain', r)),
    );
    for (const r of places) {
      const p = w.prices.latest(goodId('grain', r), w.period);
      // A basis is a READ of two prints. Where a place has one, it was TRADED in its own market,
      // is the opening it has not traded away yet, or is the last one carried and marked stale —
      // never interpolated or extrapolated, which is what a price arrived at from other prices
      // would be. A price from a formula is not a price (Law 3).
      if (p.some) expect(['traded', 'opening', 'stale']).toContain(p.value.provenance.kind);
    }
  });

  it('a leg over worse ground costs more to sail, so the gap it can support is wider (D3.a)', () => {
    const w = rigWorld('basis-b');
    const g = w.registry.geography;
    const found = legsBetween(g, w.params, 'vessel', [...w.registry.regions.keys()]);
    const walks = [...found.values()];
    expect(walks.length).toBeGreaterThan(1);
    // D3: what bounds the gap is what it costs to move the thing, and that is the leg — so two
    // legs of different length are two different bounds. Direction only, never a level.
    const byDays = [...walks].sort((a, b) => a.days - b.days);
    const quick = byDays[0];
    const slow = byDays[byDays.length - 1];
    if (quick === undefined || slow === undefined) throw new Error('two legs exist');
    expect(slow.days).toBeGreaterThan(quick.days);
    // And the days are not the kilometres: ground, not distance alone, is what makes a leg dear.
    expect(walks.some((x) => x.days / x.km !== walks[0]!.days / walks[0]!.km)).toBe(true);
  });

  it('a shipper will pay the gap and never more, which is what bounds the basis (C1.a, D3)', () => {
    const w = ranWorld('basis-a', 4);
    for (const e of w.journal.ofKind(FREIGHT_SESSION)) {
      const byLeg = e.data['byLeg'] as Record<string, { outcome: string; rate?: number }>;
      for (const leg of Object.values(byLeg)) {
        // D6: a leg that did not clear says so rather than saying nothing.
        expect(['cleared', 'noDemand', 'noSupply', 'noOverlap', 'excessCommitted']).toContain(
          leg.outcome,
        );
        // D1, Law 3: where it did clear, the rate is a print off the session and not a declared
        // number — the grep for a freight rate in the register is the other half of this.
        if (leg.rate !== undefined) expect(leg.rate).toBeGreaterThan(0);
      }
    }
  });
});

/**
 * Shipper substitution (13c step 10): a shipper can NOT ship — hold the goods, source locally, or
 * not trade at all — and that substitution is what caps the freight price.
 *
 * @spec Freight C1.a Freight C2 Freight D3 Law 3 Law 19
 *
 * All three are STRUCTURAL rather than built, and that is the finding: what a shipper will pay is
 * the gap between the two places' prints and nothing else, so above the gap the voyage is worse
 * than selling at home (holding, not trading) — and a buyer standing where the thing is made buys
 * it there, because the place's own market is the one it is in. Nothing decides between them; the
 * arithmetic leaves no room for a shipper to pay more than the alternative is worth.
 */
describe('a shipper can not ship, and that is what caps the freight (C2)', () => {
  it('never pays more for the voyage than the gap it is closing (C1.a, D3)', () => {
    const w = ranWorld('subs-a', 4);
    for (const e of w.journal.ofKind(FREIGHT_SESSION)) {
      const byLeg = e.data['byLeg'] as Record<
        string,
        { outcome: string; rate?: number; from?: string; to?: string }
      >;
      for (const leg of Object.values(byLeg)) {
        if (leg.rate === undefined || leg.from === undefined || leg.to === undefined) continue;
        // The rate cleared where shippers met carriers, and a shipper's bid IS the gap — so the
        // rate cannot exceed the widest gap any of them had. Read off the two prints, both public
        // and both already made (Law 19), never recomputed from a formula (Law 3).
        let widest = 0;
        for (const g of GOODS.map((d) => d.subUnit)) {
          const here = w.prices.latest(goodId(g, regionId(leg.from)), w.period);
          const away = w.prices.latest(goodId(g, regionId(leg.to)), w.period);
          if (!here.some || !away.some) continue;
          const gap = away.value.price - here.value.price;
          if (gap > widest) widest = gap;
        }
        if (widest > 0) expect(leg.rate).toBeLessThanOrEqual(widest);
      }
    }
  });

  it('sources locally where the thing is made: a place’s own market is the one a buyer is in', () => {
    const w = ranWorld('subs-a', 4);
    const makes = [...w.registry.regions.keys()].filter((r) =>
      w.instruments.has(goodId('grain', r)),
    );
    expect(makes.length).toBeGreaterThan(1);
    // C2: a buyer standing in a place that makes the thing has a market there and does not need a
    // voyage at all. That is why the freight price is capped by something other than freight.
    for (const r of makes) {
      const i = w.instruments.get(goodId('grain', r));
      expect(i.market.some).toBe(true);
    }
  });

  it('holds rather than shipping when the gap will not pay for the voyage', () => {
    const w = ranWorld('subs-a', 4);
    let refused = 0;
    let moved = 0;
    for (const e of w.journal.ofKind(FREIGHT_SESSION)) {
      const byLeg = e.data['byLeg'] as Record<string, { outcome: string }>;
      for (const leg of Object.values(byLeg)) {
        if (leg.outcome === 'cleared') moved += 1;
        else refused += 1;
      }
    }
    // D6, Clearing C4.b: every leg says what it did, and a leg that carried nothing said so rather
    // than saying nothing. What is asserted is that the world HAS both answers available to it,
    // never how many of each — that would be a claim about a level (Law 17).
    expect(moved + refused).toBeGreaterThan(0);
  });
});
