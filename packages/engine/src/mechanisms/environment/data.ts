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
import { moduleKey, type ModuleKey, type PlaceId } from '../../core/ids.js';
import { Impossible } from '../../core/errors.js';
import { prng, type Prng } from '../../rng/prng.js';
import type { GeographyDecl } from '../../registry/geography.js';
import type { TileIndex } from '../../core/ids.js';
import { between, type Spread } from '../../rng/spread.js';
import { GROWING, WARMTH, WIND } from '../../registry/environment.js';

/** This module's own identifier, branded here rather than in the kernel (ARCHITECTURE 4.9b). */
export type FactId = ModuleKey<'EnvironmentFact'>;
export const factId = (s: string): FactId => moduleKey(s, 'EnvironmentFact');

/**
 * The facts this world has, branded. The NAMES are the kernel's, because four modules have to spell
 * them and none may import this one (4.9b); what a fact IS and how wide it runs is here.
 */
export const GROWING_FACT = factId(GROWING);
export const WIND_FACT = factId(WIND);
export const WARMTH_FACT = factId(WARMTH);

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
    id: GROWING_FACT,
    name: 'growing conditions',
    persistence: { low: 0.45, high: 0.75, why: 'a season carries; soil moisture and snowpack are months deep' },
    swing: { low: 0.18, high: 0.4, why: 'how far a growing season departs from normal where this region is' },
    why: 'Goods B4, Commodities Spot B3: what is started is not all finished, and a bad season is the reason.',
  },
  {
    id: WIND_FACT,
    name: 'the strongest wind standing over the region',
    persistence: { low: 0.05, high: 0.2, why: 'a storm is this period and gone; it does not carry' },
    swing: { low: 0.25, high: 0.6, why: 'how violent this region gets, which is a fact about where it is' },
    why: 'Freight B4, Commodities Spot B3, Insurers B4: it blocks a passage, flattens a crop and hits every policy in the region at once.',
  },
  {
    id: WARMTH_FACT,
    name: 'warmth over the period',
    persistence: { low: 0.3, high: 0.6, why: 'weather runs in spells' },
    swing: { low: 0.1, high: 0.25, why: 'how far a spell departs where this region is' },
    why: 'Commodities Spot E2, Goods B4: what a household burns to stay warm, and what a crop needs.',
  },
];

/** One region's own climate: the two technologies, drawn under the region's and the fact's names. */
export interface ClimateDecl {
  readonly fact: FactId;
  readonly region: PlaceId;
  readonly persistence: number;
  readonly swing: number;
}

/**
 * Seed A5, Audit D3: the module's own labelled stream, so that adding a fact never reshuffles
 * another module's draws. One draw per declared number, in the order the table declares them.
 */
export function drawClimate(
  regions: readonly PlaceId[],
  seed: string,
  /**
   * 13c.1: THE CLIMATE IS DRAWN ON THE MAP. Without the ground, each place got its own memory and
   * its own width out of an independent draw — so two places sharing a border could be as unlike
   * each other as two on opposite sides of the world, which is false of weather and takes the
   * correlation out of everything downstream. With it, each fact is a smooth field over the world
   * and a place reads it AT ITS OWN HEART, so neighbours are alike because they are near.
   *
   * The period's DEPARTURE stays each place's own: what is shared is the climate, not the storm.
   * Correlated losses across neighbouring places — two claims that are one event — are 13h's,
   * where the insurer that cares about the difference lives.
   */
  ground?: { readonly g: GeographyDecl; readonly heart: (p: PlaceId) => TileIndex },
): readonly ClimateDecl[] {
  const rng = prng(seed, 'environment');
  const out: ClimateDecl[] = [];
  for (const fact of FACTS) {
    const memory = ground === undefined ? undefined : smooth(rng, ground.g);
    const width = ground === undefined ? undefined : smooth(rng, ground.g);
    for (const region of regions) {
      const at = ground === undefined ? undefined : ground.heart(region);
      out.push({
        fact: fact.id,
        region,
        persistence:
          memory === undefined || at === undefined
            ? between(rng, fact.persistence)
            : along(fact.persistence, sample(memory, at)),
        swing:
          width === undefined || at === undefined
            ? between(rng, fact.swing)
            : along(fact.swing, sample(width, at)),
      });
    }
  }
  return out;
}

/** Reading a field at a tile. A tile off the end is the draw being wrong about its own world. */
function sample(field: Float64Array, at: TileIndex): number {
  const v = field[at];
  if (v === undefined) throw new Impossible('Law 2', `no climate at tile ${at}`);
  return v;
}

/** Where in a width a share of the way along lands. The same spread, read rather than drawn again. */
const along = (s: Spread, share: number): number => s.low + share * (s.high - s.low);

/**
 * A smooth field over the whole world, in [0, 1]: a coarse lattice of drawn numbers read between
 * its corners, so nearby tiles get nearby numbers. It is the same idea the ground is drawn with and
 * it is the draw's own arithmetic rather than anything the world declares.
 */
function smooth(rng: Prng, g: GeographyDecl): Float64Array {
  const side = CLIMATE_CELLS;
  const lattice = new Float64Array(side * side);
  for (let i = 0; i < lattice.length; i += 1) lattice[i] = rng.next();
  const out = new Float64Array(g.cols * g.rows);
  const corner = (cx: number, cy: number): number => {
    const at = (((cy % side) + side) % side) * side + (((cx % side) + side) % side);
    const v = lattice[at];
    if (v === undefined) throw new Impossible('Law 2', `no climate at lattice ${at}`);
    return v;
  };
  for (let y = 0; y < g.rows; y += 1) {
    for (let x = 0; x < g.cols; x += 1) {
      const fx = (x / g.cols) * side;
      const fy = (y / g.rows) * side;
      const x0 = Math.floor(fx);
      const y0 = Math.floor(fy);
      const tx = fx - x0;
      const ty = fy - y0;
      const sx = tx * tx * (3 - 2 * tx);
      const sy = ty * ty * (3 - 2 * ty);
      const top = corner(x0, y0) * (1 - sx) + corner(x0 + 1, y0) * sx;
      const bottom = corner(x0, y0 + 1) * (1 - sx) + corner(x0 + 1, y0 + 1) * sx;
      out[y * g.cols + x] = top * (1 - sy) + bottom * sy;
    }
  }
  return out;
}

/** RESOLUTION: how large a climate zone is. Coarser is fewer, larger bands of weather. */
const CLIMATE_CELLS = 3;
