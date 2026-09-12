/**
 * THE PHYSICAL FACTS THIS WORLD HAS, and how wide each one runs.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 1 Law 2 Law 4 Seed B1.a
 *
 * Each row is one real measurable thing about a region over a period. It is stated as HOW THIS
 * PERIOD STANDS TO WHAT THIS REGION NORMALLY HAS — "growing conditions were 0.82 of normal" — and
 * not in millimetres, because every consumer of it already declares its own normal: a recipe has a
 * yield, a route has a capacity, a policy has a sum insured. The absolute scale would be a second
 * copy of a number that already exists somewhere (Law 4), and nobody in this world measures rain.
 *
 * WHAT IS DECLARED HERE IS A WIDTH, NOT A PATH (Law 2, Appendix B: no written price path, and no
 * shock schedule either). Two technologies per fact per region — how much of last period's
 * departure still stands, and how far a period departs at all — and the period's condition is drawn
 * from them. A region's climate is a fact about that region, so both are drawn per region from the
 * widths below (Seed B1.a: nothing in the world is typed where it could be drawn).
 */
import { moduleKey, type ModuleKey, type RegionId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { between, type Spread } from '../../rng/spread.js';

/** This module's own identifier, branded here rather than in the kernel (ARCHITECTURE 4.9b). */
export type FactId = ModuleKey<'EnvironmentFact'>;
export const factId = (s: string): FactId => moduleKey(s, 'EnvironmentFact');

export const GROWING = factId('growing');
export const WIND = factId('wind');
export const WARMTH = factId('warmth');

export interface FactDecl {
  readonly id: FactId;
  readonly name: string;
  /** How much of last period's departure still stands this period. Weather has memory; a storm does not. */
  readonly persistence: Spread;
  /**
   * How far a period departs, as the half-width of the departure IN LOG SPACE. Log space because a
   * physical condition is a product of influences and never negative — half the normal rain and
   * twice the normal rain are the same size of departure in opposite directions, which is true of
   * weather and false of any additive index. Nothing is clamped: `exp` of a real number is positive
   * by arithmetic (Law 6).
   */
  readonly swing: Spread;
  readonly why: string;
}

/**
 * THREE FACTS, AND EACH IS READ BY SEVERAL SYSTEMS. That is the whole reason this module exists:
 * one storm is a producer's lost crop, a blocked passage and every policy in the region at once,
 * and a world where each of those drew its own hazard would be three unrelated draws for one event.
 */
export const FACTS: readonly FactDecl[] = [
  {
    id: GROWING,
    name: 'growing conditions',
    persistence: { low: 0.45, high: 0.75, why: 'a season carries; soil moisture and snowpack are months deep' },
    swing: { low: 0.18, high: 0.4, why: 'how far a growing season departs from normal where this region is' },
    why: 'Goods B4, Commodities Spot B3: what is started is not all finished, and a bad season is the reason.',
  },
  {
    id: WIND,
    name: 'the strongest wind standing over the region',
    persistence: { low: 0.05, high: 0.2, why: 'a storm is this period and gone; it does not carry' },
    swing: { low: 0.25, high: 0.6, why: 'how violent this region gets, which is a fact about where it is' },
    why: 'Freight B4, Commodities Spot B3, Insurers B4: it blocks a passage, flattens a crop and hits every policy in the region at once.',
  },
  {
    id: WARMTH,
    name: 'warmth over the period',
    persistence: { low: 0.3, high: 0.6, why: 'weather runs in spells' },
    swing: { low: 0.1, high: 0.25, why: 'how far a spell departs where this region is' },
    why: 'Commodities Spot E2, Goods B4: what a household burns to stay warm, and what a crop needs.',
  },
];

/** One region's own climate: the two technologies, drawn under the region's and the fact's names. */
export interface ClimateDecl {
  readonly fact: FactId;
  readonly region: RegionId;
  readonly persistence: number;
  readonly swing: number;
}

/**
 * Seed A5, Audit D3: the module's own labelled stream, so that adding a fact never reshuffles
 * another module's draws. One draw per declared number, in the order the table declares them.
 */
export function drawClimate(regions: readonly RegionId[], seed: string): readonly ClimateDecl[] {
  const rng = prng(seed, 'environment');
  const out: ClimateDecl[] = [];
  for (const fact of FACTS) {
    for (const region of regions) {
      out.push({
        fact: fact.id,
        region,
        persistence: between(rng, fact.persistence),
        swing: between(rng, fact.swing),
      });
    }
  }
  return out;
}
