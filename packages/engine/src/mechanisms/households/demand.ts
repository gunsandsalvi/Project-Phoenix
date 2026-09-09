/**
 * One budget, posted as the step function a book takes.
 *
 * @spec Goods C1 Clearing A2 Clearing A2.a Households D5 Equity B1 Law 2 Law 4
 *
 * A cell spends the same money whatever the price, so the quantity it wants at a price is the
 * budget divided by it — a curve, not a point. A book takes a set of limit orders, and the size on
 * a limit order is the EXTRA that level adds: what the book sees at any posted level is then
 * exactly the curve at that level, and never more, however many levels there are.
 *
 * Law 2: that is what makes the number of levels a RESOLUTION. Refine it and every level that was
 * already there still says what it said; what changes is only how finely the book can answer. A
 * schedule whose top level moved when the count changed would be a bound on what the cell will pay
 * that nobody stated, and it would be a shape wearing a resolution's name.
 *
 * Law 4: goods and shares are two different reasons to want something at a price — what a consumer
 * expects to be charged (§46 B3) against what a saver thinks a claim is worth (Equity B3) — and the
 * levels come from each of those separately. What they share is this, and it lives in one place.
 */
import { div, material, mul, sub } from '../../core/num.js';

/** One limit order of a curve: a level, and the extra this level adds to what the cell wants. */
export interface Rung {
  readonly price: number;
  readonly qty: number;
}

/**
 * Clearing A2, A2.a: the curve `budget / price` sampled at these levels, highest first, as the
 * increments a book adds up. Levels at or below zero are not prices and are dropped.
 */
export function rungsOver(levels: readonly number[], budget: number): Rung[] {
  if (budget <= 0) return [];
  const out: Rung[] = [];
  let taken = 0;
  for (const price of [...levels].sort((a, b) => b - a)) {
    if (price <= 0) continue;
    const wants = div(budget, price, 'units it would take at that price');
    const extra = sub(wants, taken, 'the extra this level adds');
    taken = wants;
    // Law 7: an increment that is the rounding of the subtraction is not a size it asked for.
    if (!material(extra, 2, wants)) continue;
    out.push({ price, qty: extra });
  }
  return out;
}

/**
 * Equity B1, B3: the levels a saver posts for a claim it has valued — its own opinion, and down.
 *
 * Its opinion is the MOST it will pay: above it the claim is worth less to it than the money, so a
 * level above it is not a price it would ever take. Below it it wants more of the thing, which is
 * the curve. Where the sampling stops is the resolution and nothing else: every level of a coarser
 * grid is a level of a finer one, so refining adds answers and moves none.
 */
export function levelsBelow(opinion: number, steps: number): number[] {
  if (opinion <= 0 || steps < 1) return [];
  const out: number[] = [];
  for (let step = steps; step >= 1; step -= 1) {
    out.push(mul(opinion, div(step, steps, 'this level of the grid'), 'a level it would pay'));
  }
  return out;
}
