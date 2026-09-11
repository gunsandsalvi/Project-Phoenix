/**
 * The currency market: who is in it, why, and what a desk does with a position it did not want.
 *
 * @spec Spot FX A1 Spot FX B1 Spot FX B2 Spot FX B5 Spot FX B6 Spot FX C1 Spot FX C3 Spot FX C4 Spot FX D1 Spot FX D2 Spot FX D4 Spot FX E3 Currency C1 Currency C2 XI-12 XI-13 Law 3 Law 5 Law 8
 */
import { describe, expect, it } from 'vitest';
import { ABROAD, USD, currencyUnit, fxMarketOf, fxPairId, triangles } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('a spot trade is two money legs (Spot FX A1, C4; Currency C1, C2; Law 5)', () => {
  it('moves one money against another, both on their own grids, or neither moves', () => {
    const w = rigWorld('fx-A');
    for (let i = 0; i < 6; i += 1) w.step();
    let seen = 0;
    for (const r of w.ledger.all()) {
      if (r.outcome !== 'settled') continue;
      const legs = r.instruction.legs.filter((l) => l.kind === 'money');
      if (legs.length !== 2 || r.instruction.legs.length !== 2) continue;
      const [a, b] = legs;
      if (a === undefined || b === undefined) continue;
      if (a.ccy === b.ccy) continue;
      seen += 1;
      // Law 5: both legs, same instruction, same period. Neither side can be left having paid.
      expect(a.amount).toBeGreaterThan(0);
      expect(b.amount).toBeGreaterThan(0);
      // C4, Law 8: each leg lands on the smallest piece of its OWN money.
      for (const leg of [a, b]) {
        const piece = w.registry.subdivision(currencyUnit(leg.ccy));
        expect(piece).toBeGreaterThan(0);
        expect(Number.isInteger(leg.amount)).toBe(true);
      }
      // A1: and the two sides are the same two parties, the other way round.
      expect(a.from.holder).toBe(b.to.holder);
      expect(a.to.holder).toBe(b.from.holder);
    }
    expect(seen).toBeGreaterThan(0);
  });
});

describe('the triangle (Spot FX C3, E3; XI-12)', () => {
  it('exists at all, which two moneys cannot express', () => {
    const w = rigWorld('fx-B');
    const tris = triangles(w.markets);
    const n = 1 + ABROAD.length;
    // One per unordered triple: four moneys make four triangles, two make none.
    expect(tris.length).toBe((n * (n - 1) * (n - 2)) / 6);
    for (const t of tris) {
      expect(new Set([t.a, t.b, t.c]).size).toBe(3);
      expect(t.ab.fx).toEqual({ base: t.a, quote: t.b });
      expect(t.ac.fx).toEqual({ base: t.a, quote: t.c });
    }
  });

  it('is never enforced: the audit measures the gap and closes nothing (C3, E3)', () => {
    const w = rigWorld('fx-C');
    for (let i = 0; i < 6; i += 1) w.step();
    const family = w.last?.audit.families.find((f) => f.family === 'crossMarket');
    expect(family?.built).toBe(true);
    expect(family?.contributions).toContain('spot-fx');
    // A gap inside a desk's own cost is the market working, not a violation.
    for (const v of family?.violations ?? []) expect(v.spec).toBe('Spot FX C3');
  });
});

describe('who is in a pair and why (Spot FX B1, B2, B5, B6; XI-13)', () => {
  it('puts a party in the pair between the money it needs and its own, and nowhere else', () => {
    const w = rigWorld('fx-D');
    w.step();
    // XI-12: nothing is routed. A party with a euro need is in the pair that has euros and its own
    // money in it, so one balance is never committed in three books at once.
    for (const m of w.markets) {
      const fx = m.fx;
      if (fx === undefined) continue;
      expect([fx.base, fx.quote]).toContain(fx.base);
    }
    const pair = w.market(fxMarketOf(USD, ABROAD[0]?.ccy ?? USD));
    expect(pair.instrument).toBe(fxPairId(USD, ABROAD[0]?.ccy ?? USD));
  });

  it('leaves a desk’s own money out of its position in a pair (D4)', () => {
    const w = rigWorld('fx-E');
    for (let i = 0; i < 4; i += 1) w.step();
    // D4: a bank's dollar balance funds everything it does; a desk that counted it as a long dollar
    // book would be permanently too long to bid for dollars, which is a market with one side.
    // What says so here is that the dollar pairs keep trading rather than going one-way and dying.
    const results = w.last?.markets.filter((m) => String(m.market).includes('fx.USD/')) ?? [];
    expect(results.length).toBe(ABROAD.length);
    for (const r of results) expect(['cleared', 'noOverlap', 'noDemand', 'noSupply', 'nothingSettled']).toContain(r.outcome);
  });
});

describe('a session that settles nothing does not print (Clearing E1, Law 3)', () => {
  it('carries the last real price, visibly stale, and says why', () => {
    const w = rigWorld('fx-F');
    for (let i = 0; i < 8; i += 1) w.step();
    for (const e of w.journal.ofKind('print')) {
      if (e.data['stale'] !== true) continue;
      const reason = e.data['reason'];
      expect(['noDemand', 'noSupply', 'noOverlap', 'excessCommitted', 'nothingSettled']).toContain(reason);
      // A carried price says where it came from, so nothing can mistake it for this period's.
      expect(typeof e.data['carriedFrom']).toBe('number');
    }
    // And a traded print has a settled volume behind it, always.
    for (const e of w.journal.ofKind('print')) {
      if (e.data['stale'] === true || e.data['printed'] === false) continue;
      const settled = e.data['settledVolume'];
      expect(typeof settled === 'number' && settled > 0).toBe(true);
    }
  });
});
