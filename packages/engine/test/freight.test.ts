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
  drawCarriers,
  inTransit,
  paramId,
  partyId,
  plantKindId,
  regionId,
} from '../src/index.js';
import { ranWorld, rigDraw, rigWorld } from './rig.js';

describe('a route is a real leg with a real limit (A4, B2)', () => {
  it('declares its four technologies, and every one of them is one', () => {
    const w = rigWorld('freight-a');
    const legs = [...w.registry.regions.keys()].filter((r) => r !== REGION);
    expect(legs.length).toBeGreaterThan(0);
    for (const to of legs) {
      for (const [what, dimension] of [
        ['transit', 'periods'],
        ['perVessel', 'ratio'],
        ['sailsIn', 'ratio'],
        ['sailsHardness', 'ratio'],
      ] as const) {
        const d = w.params.decl(paramId(`freight.${REGION}.${to}.${what}`));
        // Law 2: a fact about the world — how long the crossing takes, what a hull carries, what
        // the passage is sailable in. None of them is a claim about an answer, so none has a death.
        expect(d.kind).toBe('technology');
        expect(d.dimension).toBe(dimension);
        expect(d.value).toBeGreaterThan(0);
      }
    }
    // Law 6: and there is no freight RATE anywhere. What it costs to move a tonne is cleared.
    const rates = [...w.params.all()].filter((d) => String(d.id).startsWith('freight.'));
    expect(rates.length).toBeGreaterThan(0);
    for (const d of rates) expect(String(d.id)).not.toContain('rate');
  });

  it('gives the hulls to named carriers, and a carrier is a firm (B1)', () => {
    const w = rigWorld('freight-b');
    const carriers = drawCarriers(6, [REGION], rigDraw('freight-b').banks.map((b) => b.bank), 'freight-b');
    expect(carriers.length).toBeGreaterThan(0);
    let withHulls = 0;
    for (const c of carriers) {
      const id = partyId(c.carrier);
      expect(w.parties.has(id)).toBe(true);
      // Small-Business Pools A6.b, Law 15: it is a FIRM. A party kind of its own would be a second
      // kind behaving identically, so every rule written for one would be written again.
      expect(w.parties.get(id).kind).toBe(FIRM);
      const view = w.participantView(id);
      const hulls = view
        .holdings()
        .filter((h) => w.instruments.get(h.instrument).kind === plantKindId(VESSEL))
        .reduce((a, h) => a + view.quantity(h.instrument), 0);
      if (hulls > 0) withHulls += 1;
    }
    // B2: a carrier without a ship is not a carrier, and how many it has is its own drawn size.
    expect(withHulls).toBeGreaterThan(0);
  });
});

describe('the session says what it did, including that it did nothing (Clearing C4.b)', () => {
  it('runs every leg every period and reports the outcome of each', () => {
    const w = ranWorld('freight-c', 4);
    const sessions = w.journal.ofKind(FREIGHT_SESSION);
    expect(sessions.length).toBe(4);
    for (const e of sessions) {
      expect(e.public).toBe(true);
      const byRoute = e.data['byRoute'] as Record<string, { outcome?: string; carriers?: number }>;
      expect(Object.keys(byRoute).length).toBeGreaterThan(0);
      for (const leg of Object.values(byRoute)) {
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
