/**
 * The physical world as standing state (13c): one fact, read by everybody, written by one module.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 2 Law 4 Law 6 Observer A3
 */
import { describe, expect, it } from 'vitest';
import { ENVIRONMENT_STATE, FACTS, REGION, conditionsIn, paramId, regionId } from '../src/index.js';
import { drawClimate } from '../src/mechanisms/environment/data.js';
import { moveOn } from '../src/mechanisms/environment/state.js';
import { prng } from '../src/rng/prng.js';
import { ranWorld, rigWorld } from './rig.js';

const PERIODS = 6;

describe('the physical world is state, not a schedule (Law 2, Appendix B)', () => {
  it('publishes one public fact per region every period, and never a level anybody stated', () => {
    const w = ranWorld('environment-a', PERIODS);
    const events = w.journal.ofKind(ENVIRONMENT_STATE);
    expect(events.length).toBeGreaterThan(0);
    const regions = new Set<string>();
    const periods = new Set<number>();
    for (const e of events) {
      // Observer A3: the weather is not private state. A producer, a carrier and a household are
      // all standing in it, and each acts on what it can see.
      expect(e.public).toBe(true);
      const region = e.subjects[0];
      expect(typeof region).toBe('string');
      regions.add(String(region));
      periods.add(e.period);
      const facts = e.data['facts'];
      expect(typeof facts).toBe('object');
      for (const value of Object.values(facts as Record<string, number>)) {
        // Law 6: a condition is `exp` of a real number, so it is positive BY ARITHMETIC. Nothing
        // clamps it, and there is no floor anywhere for a reader to find and lean on.
        expect(value).toBeGreaterThan(0);
        expect(Number.isFinite(value)).toBe(true);
      }
      expect(Object.keys(facts as object).sort()).toEqual(FACTS.map((f) => String(f.id)).sort());
    }
    // Every region this world has, every period it ran: the weather does not skip a place or a day.
    expect(regions.size).toBeGreaterThan(1);
    expect(periods.size).toBe(PERIODS);
    expect(events.length).toBe(regions.size * PERIODS);
  });

  it('is one writer: the same period is not moved twice', () => {
    // Law 4. A phase that ran again inside one period would move the weather a second time, and
    // everybody who had already read it would have read a fact that no longer stood.
    const w = rigWorld('environment-b');
    w.step();
    const at = w.period;
    const first = w.journal.ofKind(ENVIRONMENT_STATE).filter((e) => e.period === at);
    expect(first.length).toBeGreaterThan(0);
    // One event per region, and exactly one per region: the same region is not written twice.
    expect(new Set(first.map((e) => String(e.subjects[0]))).size).toBe(first.length);
    const here = conditionsIn({ period: at, journal: w.journal }, REGION);
    expect(here?.size).toBe(FACTS.length);
  });

  it('carries: this period departs from where last period left it, by no more than its own width', () => {
    // The departure is an AR(1) in log space, so the arithmetic bound on the next one is exact and
    // is not a tolerance: `persistence × before` plus at most `(1 - persistence) × swing` either
    // way. A draw that ignored last period would break this on the first period that mattered.
    const climate = drawClimate([regionId('north')], 'environment-c');
    const first = climate[0];
    expect(first).toBeDefined();
    if (first === undefined) return;
    const rng = prng('environment-c', 'test');
    let before: number | undefined;
    for (let i = 0; i < 40; i += 1) {
      const now = moveOn(first, before, rng);
      if (before !== undefined) {
        const centre = first.persistence * before;
        const reach = (1 - first.persistence) * first.swing;
        expect(now.departure).toBeGreaterThanOrEqual(centre - reach);
        expect(now.departure).toBeLessThanOrEqual(centre + reach);
      }
      expect(now.ofNormal).toBeGreaterThan(0);
      before = now.departure;
    }
  });

  it('declares two technologies per fact per region and no hazard anywhere', () => {
    // Law 2: what is declared is a MEMORY and a WIDTH. A probability of a catastrophe would be a
    // claim about the answer living inside the module that consumes it — which is the refusal this
    // module exists to make possible, seen from the other side.
    const w = rigWorld('environment-d');
    for (const fact of FACTS) {
      for (const suffix of ['persistence', 'swing']) {
        const d = w.params.decl(paramId(`environment.${fact.id}.${REGION}.${suffix}`));
        expect(d.kind).toBe('technology');
        expect(d.dimension).toBe('ratio');
        expect(d.owner).toBe('model');
      }
    }
  });

  it('is the same weather from the same seed, and different from another', () => {
    // Seed A5, Audit D3. And the draws are the module's own labelled stream, so a world with one
    // more region does not reshuffle anybody else's numbers.
    const facts = (seed: string): string =>
      JSON.stringify(
        ranWorld(seed, 3)
          .journal.ofKind(ENVIRONMENT_STATE)
          .map((e) => [e.period, e.subjects[0], e.data['facts']]),
      );
    expect(facts('environment-e')).toBe(facts('environment-e'));
    expect(facts('environment-e')).not.toBe(facts('environment-f'));
  });
});
