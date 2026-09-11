/**
 * What this world's currency markets are, and what the parties in them are like.
 *
 * @spec Spot FX A3 Spot FX B5 Spot FX D1 Spot FX E3 Currency A3 Law 15 Seed B4
 *
 * Data only (Law 15). A PAIR IS NOT DECLARED: every pair among the currencies the registry has is a
 * market, because a currency somebody holds and a currency somebody wants is a market whether or not
 * anybody wrote it down (A3). What IS declared is the width a desk's own numbers are drawn from,
 * because two desks that quote the same rate are one desk and a pair with one quote has no market.
 */
import { paramId, type ParamId } from '../../core/ids.js';
import type { Spread } from '../../rng/spread.js';

/** A desk's own numbers, under its own name (XI-14: declared, never a literal). */
export const fxParam = (bank: string, what: string): ParamId => paramId(`fx.${what}.${bank}`);

/** Spot FX D1, E3: what a currency desk is like. Every number is a share; none is an amount. */
export interface FxDispersion {
  readonly inventoryLimit: Spread;
  readonly edge: Spread;
  readonly arbitrageEdge: Spread;
}

export const FX_SPREAD: FxDispersion = {
  inventoryLimit: {
    low: 0.05,
    high: 0.3,
    why: 'Spot FX D1, D2: the most of its own capital a desk will have standing behind a position in one currency. Every capacity is finite and enumerable (Clearing B3.a), and a currency desk without one is the buyer of last resort this world does not have — the thing that makes a peg hold until it does not. Its own, so no two desks stop at the same moment.',
  },
  edge: {
    low: 0.0005,
    high: 0.003,
    why: 'Spot FX D3: what a desk wants for standing between two currencies for a period — what the position costs it to carry and what it may be wrong about. It is not a stated SPREAD: the spread is twice this and falls out (C5.a), which is why a desk skewed by its own inventory quotes two sides that are not symmetric about anything.',
  },
  arbitrageEdge: {
    low: 0.001,
    high: 0.006,
    why: 'Spot FX C2.a, E3: what a three-legged trade must beat before this desk does it — its own cost of doing three trades at once and carrying all three overnight. It is what makes triangular consistency an OUTCOME with a width rather than an identity the kernel enforces (C3.b): the gap closes to somebody’s cost and no further, and the desk with the lowest cost is the one that closes it.',
  },
};
