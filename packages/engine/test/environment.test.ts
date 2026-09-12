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

describe('one event, several consequences (Law 4)', () => {
  it('takes a real crop out of a bad season, at the point the tonnes would have been made', () => {
    // Goods B4, Commodities Spot B3. The declared yield is what an ORDINARY period leaves; what a
    // period actually leaves is that times the conditions the line stood in. So a shortfall is
    // tonnes that were never made, published with the season that took them — never a write-down
    // and never a multiplier on a price (Law 3: a shortage reaches a price by there being less).
    const w = ranWorld('environment-yield', 8);
    const produced = w.journal.ofKind('firms.produced');
    expect(produced.length).toBeGreaterThan(0);
    const exposed = produced.filter((e) => num(e, 'season') !== 1);
    // At least one line in this world stands in the weather, and it did not stand at normal in
    // every one of eight periods — which is the whole of "the world produces its own shocks".
    expect(exposed.length).toBeGreaterThan(0);
    for (const e of produced) {
      const season = num(e, 'season');
      const started = num(e, 'started');
      const finished = num(e, 'finished');
      const scrapped = num(e, 'scrapped');
      const survived = num(e, 'survived');
      expect(season).toBeGreaterThan(0);
      // Law 5, Goods B4: what was started is what came off plus what did not. Exactly, in units.
      expect(finished + scrapped).toBe(started);
      // B4 is ONE-DIRECTIONAL: not everything started is finished, and no season makes more tonnes
      // than went onto the line. The survival rate is a fraction raised to a positive power, so it
      // stays inside its own range by arithmetic — there is no clamp anywhere to find (Law 6).
      expect(survived).toBeGreaterThan(0);
      expect(survived).toBeLessThanOrEqual(1);
      expect(scrapped).toBeGreaterThanOrEqual(0);
      expect(finished).toBeLessThanOrEqual(started);
    }
    // The same season reaches every line of that good in that region in that period: one fact.
    const byPeriod = new Map<string, Set<number>>();
    for (const e of produced) {
      const key = `${e.period}|${String(e.data['good'])}`;
      byPeriod.set(key, (byPeriod.get(key) ?? new Set()).add(num(e, 'season')));
    }
    for (const seasons of byPeriod.values()) expect(seasons.size).toBe(1);
  });

  it('is a multiple of one for a line made indoors, and that is an answer rather than a default', () => {
    const w = ranWorld('environment-indoors', 5);
    const indoors = w.journal
      .ofKind('firms.produced')
      .filter((e) => String(e.data['good']) !== 'grain');
    expect(indoors.length).toBeGreaterThan(0);
    // A mill and an oven do not stand in the weather, and their declaration says so with an empty
    // list — which is different from saying their yield is high (Law 6, Law 16).
    for (const e of indoors) expect(num(e, 'season')).toBe(1);
  });
});

function num(e: { readonly data: Readonly<Record<string, unknown>> }, key: string): number {
  const v = e.data[key];
  if (typeof v !== 'number') throw new Error(`${key} is not a number`);
  return v;
}
