/**
 * One budget, posted as the step function a book takes.
 *
 * @spec Goods C1 Clearing A2 Clearing A2.a Households D5 Equity B1 Banks Capital B3 Law 2 Law 4
 *
 * A party spends the same money whatever the price, so the quantity it wants at a price is the
 * budget divided by it — a curve, not a point. A book takes a set of limit orders, and the size on
 * a limit order is the EXTRA that level adds: what the book sees at any posted level is then
 * exactly the curve at that level, and never more, however many levels there are.
 *
 * Law 2: that is what makes the number of levels a RESOLUTION. Refine it and every level that was
 * already there still says what it said; what changes is only how finely the book can answer. A
 * schedule whose SPAN moved when the count changed would be a bound on what the party will pay that
 * nobody stated, and it would be a SHAPE wearing a resolution's name — which is what `levelsBelow`
 * was until `A-32` (its bottom was `opinion / steps`, so the count decided how far down the party
 * bid at all, and a grid of five and a grid of seven shared only their top level).
 *
 * Law 4: a household buying grain, a saver buying a bill and a bank bidding for a note are three
 * reasons to want something at a price — what a consumer expects to be charged (§46 B3), what a
 * saver thinks a claim is worth (Equity B3), what a note is worth to a lender (XI-11's buyer) —
 * and the levels come from each of those separately. What they SHARE is this, and it lives in one
 * place. It is in the kernel beside the solver rather than inside `households`, because a module
 * never imports another module and three of them need it.
 */
import {
  type Cash,
  type PerPiece,
  amountOf,
  asRatio,
  minus,
  ratioOf,
  scale,
} from '../core/measure.js';
import { material, sub } from '../core/num.js';
import type { Option } from '../core/option.js';
import { downTick, NO_QTY, subQty, type Qty } from '../core/tick.js';

/** One limit order of a curve: a level, and the extra this level adds to what the party wants. */
export interface Rung {
  readonly price: PerPiece;
  readonly qty: Qty;
}

/**
 * Clearing A2, A2.a: the curve `budget / price` sampled at these levels, highest first, as the
 * increments a book adds up. Levels at or below zero are not prices and are dropped.
 */
export function rungsOver(levels: readonly PerPiece[], budget: Cash): Rung[] {
  if (budget.pieces <= 0) return [];
  const out: Rung[] = [];
  let taken = NO_QTY;
  for (const price of [...levels].sort((a, b) => b - a)) {
    if (price <= 0) continue;
    // Law 8, XI-15: a piece is the smallest thing there is, and what a CELL posts has to be whole
    // pieces for every member of it — so the budget here is one member's and the count it reaches
    // is whole. Down: what somebody CAN buy never rounds up, or the last rung is a piece it has
    // not got the money for.
    const wants = downTick(amountOf(budget, price, 'units it would take at that price'));
    const extra = subQty(wants, taken, 'the extra this level adds');
    taken = wants;
    // Law 7: an increment that is the rounding of the subtraction is not a size it asked for.
    if (!material(extra, 2, wants)) continue;
    out.push({ price, qty: extra });
  }
  return out;
}

/**
 * 13c.2, Goods C1: THE SAME CURVE, UNDER A WANT.
 *
 * A household does not buy grain by the lorry-load because it is cheap: it buys what it eats, and a
 * bank does not take a whole pool because it is cheap: it takes what it will have out to one name.
 * So the size at a level is what the money it set aside would take there, or what it wanted,
 * whichever runs out first — `money / price` while the money binds, flat at the want once the price
 * has fallen far enough that it does not. What it does not spend it keeps, and that is a saving
 * nobody decided on separately: it is what happens when a thing costs less than the party thought.
 *
 * Law 6: there is no bound in it. Both terms are quantities the party itself named — one from its
 * budget, one from its preference or its own published limit — and which is smaller at a level is
 * arithmetic, not a cap.
 */
export function rungsUpTo(levels: readonly PerPiece[], money: Cash, want: number): Rung[] {
  if (money.pieces <= 0 || want <= 0) return [];
  const out: Rung[] = [];
  let taken = NO_QTY;
  // Law 8, XI-15: whole pieces per member, and DOWN, for the reason `rungsOver` rounds down.
  const ceiling = downTick(want);
  for (const price of [...levels].sort((a, b) => b - a)) {
    if (price <= 0) continue;
    // Law 8 (14.2): what the money would take at this price is compared with what it wanted BEFORE
    // it is made a count — at a price of a few millionths a unit the money would take more pieces
    // than the grid can count, and a count it was never going to ask for is not a count to make.
    const affordable = amountOf(money, price, 'units the money it set aside would take');
    const wants = affordable >= ceiling ? ceiling : downTick(affordable);
    const extra = subQty(wants, taken, 'the extra this level adds');
    taken = wants;
    if (!material(extra, 2, wants)) continue;
    out.push({ price, qty: extra });
  }
  return out;
}

/**
 * Equity B1, B3, A-32: the levels a party posts BELOW its own top price, over a span it named.
 *
 * The most it will pay is `top`: above that the claim is worth less to it than the money, so a
 * level above it is not a price it would ever take. How far below it will still bid is `width` —
 * its own number, and for a saver it is the same width that produced the top (how wrong it has
 * been about this line), because what it is uncertain about is one thing and it is used once.
 *
 * BOTH ENDS ARE FIXED and `steps` only samples between them, which is what makes `steps` a
 * RESOLUTION: refine it and the span does not move. It used to be `top × k/steps`, whose bottom was
 * `top / steps` — so the COUNT decided how far down the party bid at all, a grid of five and a grid
 * of seven shared only their top level, and a number declared a resolution was setting the answer.
 */
export function levelsBelow(top: PerPiece, width: PerPiece, steps: number): PerPiece[] {
  if (top <= 0 || steps < 1) return [];
  if (width <= 0) return [top];
  const out: PerPiece[] = [];
  for (let k = 0; k <= steps; k += 1) {
    const price = minus(
      top,
      scale(
        width,
        ratioOf(
          asRatio(k, 'this step of the grid'),
          asRatio(steps, 'the steps there are'),
          'how far down',
        ),
        'how far below the most it will pay',
      ),
      'a level it would pay',
    );
    if (price > 0) out.push(price);
  }
  return out;
}

/**
 * XI-11, Clearing A2: the levels a party posts when what stops it is a SIZE and not a
 * price — a bidder with a published limit per name takes more of a thing the cheaper it is, up to
 * that limit, and there is no level below which it stops wanting it.
 *
 * The span is `(0, top]`, which is fixed, and `steps` samples it; the quantity is bounded by the
 * limit through `rungsUpTo`, so the curve converges as the grid is refined rather than diverging
 * the way a budget over a price would. That is why this one may run to zero and `levelsBelow` may
 * not: a saver with a budget and no want would bid unbounded size at a price near nothing.
 */
export function levelsUpTo(top: PerPiece, steps: number): PerPiece[] {
  if (top <= 0 || steps < 1) return [];
  const out: PerPiece[] = [];
  for (let k = steps; k >= 1; k -= 1) {
    out.push(
      scale(
        top,
        ratioOf(
          asRatio(k, 'this step of the grid'),
          asRatio(steps, 'the steps there are'),
          'this level',
        ),
        'a level it would pay',
      ),
    );
  }
  return out;
}

/**
 * §46 B3, Goods C1: the levels a party posts ACROSS its own range, highest first — what it expects,
 * spread by how wrong it has been about this line. Both ends are its own numbers and `steps` only
 * samples between them, which is the same statement `levelsBelow` makes on one side.
 *
 * 0h.2: A PARTY WITH NO WIDTH POSTS ONE LEVEL, and there are two ways to have none — an outlook
 * that has never been scored (`Missing`: it cannot say how wrong it has been) and one scored every
 * period and never wrong (a width of zero, which is a true reading). They come to the same order
 * and they are not the same fact, so the second is a number here and the first is not.
 */
export function pricesOver(
  expected: PerPiece,
  width: Option<PerPiece>,
  steps: number,
): PerPiece[] {
  if (!width.some || width.value <= 0 || steps <= 1) return [expected];
  const out: PerPiece[] = [];
  for (let i = 0; i < steps; i += 1) {
    // A position on a symmetric grid: two steps in, less the grid's own span, over that span — a
    // count over a count, so it is a pure number between minus one and one by construction.
    const span = asRatio(sub(steps, 1, 'steps less one'), 'the span of the grid');
    const t = ratioOf(minus(asRatio(2 * i, 'two steps in'), span, 'centred'), span, 'position');
    const price = minus(
      expected,
      scale(width.value, asRatio(t, 'how far along the grid'), 'how far from what it expects'),
      'a level it would pay',
    );
    if (price > 0) out.push(price);
  }
  return out;
}
