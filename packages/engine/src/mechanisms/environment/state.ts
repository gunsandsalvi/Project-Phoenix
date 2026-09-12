/**
 * THE STANDING PHYSICAL STATE: what the world is doing to a region this period, and how it moves.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 2 Law 4 Law 6
 *
 * One number per (fact, region), carried period to period as STATE. It is not a schedule and not a
 * path: what is declared is a memory and a width (`data.ts`), and the period's departure is drawn.
 * A scenario at item 16 is a stated OPENING of this state, never a second mechanism beside it.
 *
 * THE DEPARTURE LIVES IN LOG SPACE and the condition is its exponential, so the condition is
 * positive by arithmetic and never by a bound (Law 6). Nothing here clamps, and nothing here knows
 * what a bad season DOES — each consumer reads the condition and applies it to its own normal.
 */
import type { Period } from '../../calendar/calendar.js';
import type { RegionId } from '../../core/ids.js';
import { add, finite, mul } from '../../core/num.js';
import type { Prng } from '../../rng/prng.js';
import type { ClimateDecl, FactId } from './data.js';

/** What this period is, for one fact in one region. */
export interface Condition {
  readonly fact: FactId;
  readonly region: RegionId;
  /** How far this period stands from normal, in log space: 0 is an ordinary period. */
  readonly departure: number;
  /** What that comes to as a multiple of what this region normally has. Positive by arithmetic. */
  readonly ofNormal: number;
}

/** The module's own store. One writer (this module), and the period it was last written for. */
export interface Weather {
  /** Keyed `fact|region`; the value is the departure, which is the thing that carries. */
  readonly departures: Map<string, number>;
  written: Period | undefined;
}

export const keyOf = (fact: FactId, region: RegionId): string => `${fact}|${region}`;

/**
 * B3, B4: the period's draw. `persistence` of what stood last period still stands, and the rest is
 * a fresh departure across this region's own width — which is an AR(1) in log space and is the
 * shortest honest statement of weather that has memory and no trend.
 *
 * The FIRST period has no last period, so what stands is the fresh draw alone. That is not a
 * special case: `persistence × nothing` is nothing, and an opening world whose weather was normal
 * by construction would be an opening outcome somebody stated (Law 2).
 */
export function moveOn(climate: ClimateDecl, before: number | undefined, rng: Prng): Condition {
  const carried = before === undefined ? 0 : mul(climate.persistence, before, 'what still stands');
  const fresh = mul(
    add(-climate.swing, mul(2 * climate.swing, rng.next(), 'across the width'), 'this period own'),
    before === undefined ? 1 : 1 - climate.persistence,
    'the part of it that is new',
  );
  const departure = finite(add(carried, fresh, 'how far this period stands from normal'), 'departure');
  return {
    fact: climate.fact,
    region: climate.region,
    departure,
    ofNormal: finite(Math.exp(departure), 'what this period comes to as a multiple of normal'),
  };
}
