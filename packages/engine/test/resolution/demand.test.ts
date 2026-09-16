/**
 * Resolution: how finely a party posts its own demand curve is not supposed to be an answer.
 *
 * @spec Goods C1 Clearing A2 Clearing A2.a Equity B1 Households D5 Law 2 Law 7 Law 11
 *
 * Law 2 says a RESOLUTION is a number tested by invariance: change it and the answer must not move.
 * `households.demand.steps` is declared one, and what it must leave alone is the CURVE — how much
 * the party wants at a price. A book adds up the limit orders at or above the level it is testing,
 * so what it sees at a level is the whole of what the party said about that level; if refining the
 * grid moved that number, the count would be deciding how much the party wanted, and it would be a
 * SHAPE wearing a resolution's name.
 *
 * Two failures this guards, and both were real. A ladder whose rungs sat at `k/(steps+1)` of the
 * party's own opinion topped out BELOW that opinion and crept up as the count rose — a haircut on
 * what a saver would pay that nobody had stated, enough to keep a whole market from ever crossing.
 * And then (`A-32`) a ladder at `top × k/steps`, whose BOTTOM was `top / steps`: the count decided
 * how far down the party bid at all, a grid of five and a grid of seven shared only their top
 * level, and the span of a saver's own demand was being set by a number declared a resolution.
 */
import { USD } from '../../src/seeds/foundation.js';
import { asCash, asPerPiece } from '../../src/core/measure.js';
import { describe, expect, it } from 'vitest';
import { downTick, levelsBelow, levelsUpTo, rungsOver, rungsUpTo, sum, type Rung } from '../../src/index.js';

/**
 * Law 8: a budget is a count of the smallest pieces of money there are, and what it buys is a count
 * of the smallest pieces of the thing. It used to be 37.5 — three dozen pieces — and at that size
 * the whole-piece curve and the real-number one are visibly different numbers; at a real budget they
 * differ by at most one piece in millions, which is what "a piece is the smallest thing there is"
 * means when the thing is money. The SIZE of the budget is this test's own resolution.
 */
const BUDGET = asCash(37_500_000, USD, 'what a member has to place');
const OPINION = asPerPiece(2.75, 'what it thinks a unit is worth');
const WIDTH = asPerPiece(0.5, 'how wrong it has been about this line');
const GRAINS = [1, 2, 5, 10, 50, 200];

/** What a book sees at a level: every order at or above it, added up (Clearing A2). */
function demandAt(rungs: readonly Rung[], price: number): number {
  return sum(rungs.filter((r) => r.price >= price).map((r) => r.qty)).value;
}

describe('a party own demand curve at every grain (Law 2)', () => {
  it('is the same curve however finely it is posted', () => {
    const coarse = levelsBelow(OPINION, WIDTH, GRAINS[0] ?? 1);
    for (const steps of GRAINS) {
      const rungs = rungsOver(levelsBelow(OPINION, WIDTH, steps), BUDGET);
      // Every grain reaches the party's own top price, and never goes above it: that is the most it
      // will pay, and a level above it is not a price it would ever take (Equity B1).
      const top = Math.max(...rungs.map((r) => r.price));
      expect(top).toBeCloseTo(OPINION, 12);
      // At every level of the coarsest grid — which every finer grid contains — the book sees
      // exactly the budget divided by the price, IN WHOLE PIECES, which is the curve itself: there
      // is nothing between two pieces for the grid to have moved it to (Law 8).
      for (const level of coarse) {
        expect(demandAt(rungs, level)).toBe(downTick(BUDGET.pieces / level));
      }
      // Law 7: and the money it committed is its budget, to within the one piece that a whole
      // number of them at a price cannot reach. That is not a tolerance — it is the piece.
      expect(Math.abs(demandAt(rungs, top) * top - BUDGET.pieces)).toBeLessThanOrEqual(top);
    }
  });

  /**
   * A-32: the span is the party's own, at every grain. This is the test the old ladder failed —
   * its bottom was `top / steps`, so one grain bid down to 55% of its opinion and another to 1.4%.
   */
  it('spans the party own width at every grain, and the count only samples it', () => {
    for (const steps of GRAINS) {
      const levels = levelsBelow(OPINION, WIDTH, steps);
      expect(Math.max(...levels)).toBeCloseTo(OPINION, 12);
      expect(Math.min(...levels)).toBeCloseTo(OPINION - WIDTH, 12);
      expect(levels.length).toBe(steps + 1);
    }
  });

  it('adds levels as it is refined and moves none of the ones it had', () => {
    const five = rungsOver(levelsBelow(OPINION, WIDTH, 5), BUDGET);
    const twenty = rungsOver(levelsBelow(OPINION, WIDTH, 20), BUDGET);
    expect(twenty.length).toBeGreaterThan(five.length);
    // A refinement is a superset: every level the coarse grid posted is still a level.
    for (const r of five) {
      expect(twenty.some((x) => Math.abs(x.price - r.price) < 1e-12)).toBe(true);
    }
  });

  it('posts nothing at all for an opinion of nothing, rather than a price of nothing', () => {
    expect(levelsBelow(asPerPiece(0, 'no opinion at all'), WIDTH, 5)).toEqual([]);
    expect(rungsOver(levelsBelow(OPINION, WIDTH, 5), asCash(0, USD, 'no money at all'))).toEqual([]);
    expect(
      rungsOver([asPerPiece(0, 'nothing'), asPerPiece(-1, 'less than nothing')], BUDGET),
    ).toEqual([]);
  });

  /**
   * Securitisation C4: the other ladder, for a bidder whose limit is a SIZE and not a price. Its
   * span is `(0, top]` — fixed — and what stops the quantity diverging as the grid refines is the
   * party's own published limit per name, through `rungsUpTo`.
   */
  describe('a bidder stopped by a size rather than a price', () => {
    const LIMIT = 4_000_000;
    it('tops out at its own reservation and takes more the cheaper it is, up to its limit', () => {
      for (const steps of GRAINS) {
        const levels = levelsUpTo(OPINION, steps);
        expect(Math.max(...levels)).toBeCloseTo(OPINION, 12);
        const rungs = rungsUpTo(levels, BUDGET, LIMIT);
        const deepest = Math.min(...rungs.map((r) => r.price));
        expect(demandAt(rungs, deepest)).toBeLessThanOrEqual(downTick(LIMIT));
      }
    });
    it('converges on the limit as the grid is refined rather than diverging', () => {
      const coarse = rungsUpTo(levelsUpTo(OPINION, 5), BUDGET, LIMIT);
      const fine = rungsUpTo(levelsUpTo(OPINION, 200), BUDGET, LIMIT);
      const bottom = (rungs: readonly Rung[]): number =>
        demandAt(rungs, Math.min(...rungs.map((r) => r.price)));
      expect(bottom(fine)).toBeGreaterThanOrEqual(bottom(coarse));
      expect(bottom(fine)).toBeLessThanOrEqual(downTick(LIMIT));
    });
  });
});
