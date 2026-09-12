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
  inTransit,
  paramId,
  plantKindId,
  regionId,
} from '../src/index.js';
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
