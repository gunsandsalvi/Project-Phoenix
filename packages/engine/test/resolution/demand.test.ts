/**
 * Resolution: how finely a cell posts its own demand curve is not supposed to be an answer.
 *
 * @spec Goods C1 Clearing A2 Clearing A2.a Equity B1 Households D5 Law 2 Law 7 Law 11
 *
 * Law 2 says a RESOLUTION is a number tested by invariance: change it and the answer must not move.
 * `households.demand.steps` is declared one, and what it must leave alone is the CURVE — how much
 * the cell wants at a price. A book adds up the limit orders at or above the level it is testing,
 * so what it sees at a level is the whole of what the cell said about that level; if refining the
 * grid moved that number, the count would be deciding how much the cell wanted, and it would be a
 * SHAPE wearing a resolution's name.
 *
 * The failure this guards is not hypothetical. A ladder whose rungs sat at `k/(steps+1)` of the
 * cell's own opinion topped out BELOW that opinion and crept up as the count rose — a haircut on
 * what a saver would pay that nobody had stated and no clause asked for, and it was enough to keep
 * a whole market from ever crossing.
 */
import { describe, expect, it } from 'vitest';
import { levelsBelow, rungsOver, sum, type Rung } from '../../src/index.js';

const BUDGET = 37.5;
const OPINION = 2.75;
const GRAINS = [1, 2, 5, 10, 50, 200];

/** What a book sees at a level: every order at or above it, added up (Clearing A2). */
function demandAt(rungs: readonly Rung[], price: number): number {
  return sum(rungs.filter((r) => r.price >= price).map((r) => r.qty)).value;
}

describe('a cell own demand curve at every grain (Law 2)', () => {
  it('is the same curve however finely it is posted', () => {
    const coarse = levelsBelow(OPINION, GRAINS[0] ?? 1);
    for (const steps of GRAINS) {
      const rungs = rungsOver(levelsBelow(OPINION, steps), BUDGET);
      // Every grain reaches the cell's own opinion, and never goes above it: that is the most it
      // will pay, and a level above it is not a price it would ever take (Equity B1).
      const top = Math.max(...rungs.map((r) => r.price));
      expect(top).toBeCloseTo(OPINION, 12);
      // At every level of the coarsest grid — which every finer grid contains — the book sees
      // exactly the budget divided by the price, which is the curve itself.
      for (const level of coarse) {
        expect(demandAt(rungs, level)).toBeCloseTo(BUDGET / level, 9);
      }
      // Law 7: and the money it committed is its budget, whatever the session then clears at.
      expect(demandAt(rungs, top) * top).toBeCloseTo(BUDGET, 9);
    }
  });

  it('adds levels as it is refined and moves none of the ones it had', () => {
    const five = rungsOver(levelsBelow(OPINION, 5), BUDGET);
    const twenty = rungsOver(levelsBelow(OPINION, 20), BUDGET);
    expect(twenty.length).toBeGreaterThan(five.length);
    // A refinement is a superset: every level the coarse grid posted is still a level.
    for (const r of five) {
      expect(twenty.some((x) => Math.abs(x.price - r.price) < 1e-12)).toBe(true);
    }
  });

  it('posts nothing at all for an opinion of nothing, rather than a price of nothing', () => {
    expect(levelsBelow(0, 5)).toEqual([]);
    expect(rungsOver(levelsBelow(OPINION, 5), 0)).toEqual([]);
    expect(rungsOver([0, -1], BUDGET)).toEqual([]);
  });
});
