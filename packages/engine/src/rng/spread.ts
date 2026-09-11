/**
 * A WIDTH, and how a draw lands inside it.
 *
 * @spec Seed B1 Seed B1.a Seed B4 Law 2 Law 4
 *
 * Seed B4 asks for dispersion — a sector of equals never produces a market — and B1.a asks that how
 * many of a type there are be a property of the world rather than of a file. Between them they say
 * that what a seed states about a population of anything is a WIDTH and a COUNT, and never a row per
 * member. This is that width, and there is one of it (Law 4): banks, firms and managements all vary
 * the same way, and two structures with the same three fields would be two names for one thing.
 *
 * A spread carries its own REASON. The number that comes out of it is declared in the parameter
 * register under the drawn party's own name (XI-14), so the register prints what each one is like;
 * what is written down by hand is why the world is that wide.
 */
import type { Prng } from './prng.js';

export interface Spread {
  readonly low: number;
  readonly high: number;
  readonly why: string;
}

/** Uniform across the width. One draw, so a caller's sequence is its own (Audit D3). */
export function between(rng: Prng, s: Spread): number {
  return s.low + rng.next() * (s.high - s.low);
}

/** The same, to a whole number of whatever the spread counts in (periods, machines, people). */
export function betweenWhole(rng: Prng, s: Spread): number {
  return Math.round(between(rng, s));
}

/**
 * Seed B4: HOW UNEQUAL A POPULATION IS, as the exponent of the distribution its members' weights are
 * drawn from. It is the other shape a seed can state about a population, and it is not a width: a
 * banking system, a firm sector and a distribution of wealth are none of them a few members of
 * similar size with a spread on them — each is a handful of very large ones and a long tail of
 * small ones, and no uniform draw between two ends produces that at any width.
 *
 * One is the heaviest tail short of divergence; larger exponents pull the draw back towards the
 * smallest member. The largest of `n` draws lands near `n^(1/concentration)` times the smallest, so
 * the exponent is what says how much of a sector its biggest members are — and that is a claim
 * about the answer, which is why every one of these is a SHAPE with a death named beside it.
 */
export interface Tail {
  readonly concentration: number;
  readonly why: string;
}

/** A weight, as a multiple of the smallest member's. Never zero, so nothing drawn is nothing. */
export function drawSize(rng: Prng, t: Tail): number {
  return Math.pow(1 - rng.next(), -1 / t.concentration);
}
