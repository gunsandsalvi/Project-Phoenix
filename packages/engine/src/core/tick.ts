/**
 * The indivisible piece of a unit, and the arithmetic of splitting one.
 *
 * @spec Law 1 Law 2 Law 7 Law 8 Money A2 Money C1 Register A1.c Clearing C3 Appendix A
 *
 * A unit of anything real has a smallest piece and nothing finer exists: there is no half-cent, no
 * thousandth of a share certificate, no gram of a cargo weighed in kilos. Law 1 asks for the real
 * mechanism, and continuous money is not it — what continuous money produces instead is a residue
 * of floating-point dust that every check has to be told to forgive, and a residual belonging to
 * nobody (Law 2 calls that a defect). A world with dust in it can lend a millionth of a penny,
 * refuse a bank for one, and start a run over it.
 *
 * SO A QUANTITY IS A COUNT OF PIECES, AND THE COUNT IS AN INTEGER. Not a fraction of a named unit:
 * the piece IS the number. Money is counted in cents, not in hundredths of a pound; a cargo in
 * kilos, not in thousandths of a tonne. Integers add, subtract and compare EXACTLY in binary
 * floating point up to 2^53 of them, so a balance moved a million times is exactly the balance and
 * the checks that compare it need no tolerance at all rather than a derived one — and the decimal
 * arithmetic everybody actually does (x.xx + y.yy) is exact because it is integer arithmetic on
 * cents, which is what it always was.
 *
 * HOW MANY PIECES MAKE A NAMED UNIT is the unit's own declaration (`UnitDecl.perUnit`), and how
 * fine that is, is a RESOLUTION (Law 2): declare the same world in tenths of a cent and its path
 * must not move. The registry shifts every unit's subdivision together so the invariance can be run.
 *
 * SPLITTING IS WHERE THE REAL MECHANISM SHOWS. Ten cents shared three ways is four, three and
 * three: somebody is rounded up and somebody down, the parts sum to exactly what was there, and WHO
 * gets the odd piece is a stated rule rather than an accident. `splitOnTick` is that rule — largest
 * remainder, ties to the earlier claimant — and it is what makes "no residual with no holder"
 * something the arithmetic cannot violate instead of something the audit reports afterwards.
 */
import { Impossible } from './errors.js';
import type { Brand } from './ids.js';
import { dustOf, finite } from './num.js';

/**
 * Law 8: A QUANTITY IS A COUNT OF PIECES, AND THE TYPE SAYS SO.
 *
 * Every number in this engine is a `number`, and that is exactly why a fraction of a cent kept
 * arriving: nothing distinguishes a COUNT OF PIECES from a price, a rate or a value, so every
 * author had to remember, and the ones who divided money by a price to get units did not. What was
 * caught was caught by runtime guards at the far end — settlement, the register, the order book —
 * long after the site that made it, and a published decision that never reached any of them (a
 * quote's size, a plan's batch, an allotment of room) was never caught at all.
 *
 * So a quantity has its own TYPE. It is produced by exactly four doors — `toTick`, `downTick`,
 * `upTick` and `splitOnTick` — plus `asQty` for a number that is already a count because it was
 * read out of the register or declared as one, and `asQty` throws if it is not. Everything that
 * CARRIES a quantity asks for this type: an order's size, a leg's amount, a lot, what is issued,
 * what a seed endows. A module that divides money by a price now gets a `number` and cannot put it
 * anywhere a quantity goes without saying WHICH WAY it rounds — which is the decision it was
 * skipping, and it is its own to make, never the kernel's.
 *
 * That is the difference between a rule and a habit: this one is checked by the compiler at every
 * site at once, and a new writer cannot forget it.
 */
export type Qty = Brand<number, 'Qty'>;

/**
 * A number that is ALREADY a count of pieces, said out loud. It throws if it is not — so the only
 * way past this door is a number that really is on the grid, and a caller that is wrong finds out
 * where it is wrong rather than three phases later.
 *
 * Use it for a number read out of the register, declared as a count in the parameter register, or
 * arrived at by adding and subtracting quantities. NOT for one that came out of a division: that
 * one has a rounding decision in it, and the decision belongs to whoever is making it.
 */
export function asQty(value: number, what = 'quantity'): Qty {
  const n = finite(value, what);
  if (!Number.isSafeInteger(n)) {
    throw new Impossible(
      'Law 8',
      `${what} is ${n}, which is not a whole number of the unit's pieces`,
    );
  }
  return n as Qty;
}

/** Adding and subtracting counts of pieces gives a count of pieces: no rounding is involved. */
export function addQty(a: Qty, b: Qty, what = 'quantity'): Qty {
  return asQty(finite(a + b, what), what);
}

export function subQty(a: Qty, b: Qty, what = 'quantity'): Qty {
  return asQty(finite(a - b, what), what);
}

/** So is multiplying a count by a whole number of them — a cell's per-member amount by its weight. */
export function scaleQty(a: Qty, times: number, what = 'quantity'): Qty {
  // A count times a COUNT. Multiplying by a fraction is a rounding, and a rounding is a decision
  // somebody has to make by name (`toTick`, `downTick`, `upTick`) — never one this door makes.
  if (!Number.isSafeInteger(finite(times, what))) {
    throw new Impossible('Law 8', `${what} is scaled by ${times}, which is not a whole number`);
  }
  return asQty(finite(a * times, what), what);
}

/** The other side of a count: what is owed rather than held. A direction, and no rounding in it. */
export function negQty(a: Qty, what = 'quantity'): Qty {
  return asQty(NO_QTY - a, what);
}

/** Nothing, as a quantity. The one literal a count can have without a decision behind it. */
export const NO_QTY = 0 as Qty;

/**
 * How many pieces one named unit is divided into, at this world's resolution. A subdivision is a
 * whole number — a currency is a hundred cents, never 99.5 of them — and shifting it is Law 2's
 * knob: the same world declared in cents and in tenths of a cent must follow the same path.
 */
export function piecesPerUnit(perUnit: number, shift: number): number {
  if (!Number.isInteger(perUnit) || perUnit < 1) {
    throw new Impossible('Law 8', `a unit is divided into a whole number of pieces, got ${perUnit}`);
  }
  if (!Number.isInteger(shift) || shift < 1) {
    throw new Impossible('Law 2', `the resolution multiplies a subdivision, got ${shift}`);
  }
  const made = perUnit * shift;
  if (!Number.isSafeInteger(made)) {
    throw new Impossible('Law 8', `a subdivision of ${made} pieces is past exact arithmetic`);
  }
  return made;
}

/**
 * Whether this quantity exists: a whole number of pieces, and few enough of them that the
 * arithmetic on it is still exact. Past 2^53 pieces integers stop being exact in a float, and a
 * quantity there is not a large amount — it is an amount the machine can no longer add up.
 */
export function onTick(value: number): boolean {
  return Number.isSafeInteger(finite(value, 'quantity'));
}

/**
 * The nearest quantity that exists, ties away from zero. Used where a computed amount BECOMES a
 * payment or a delivery: what leaves an account is what the account can hold.
 */
export function toTick(value: number): Qty {
  const n = finite(value, 'quantity');
  return asQty(n < 0 ? -Math.round(-n) : Math.round(n), 'the nearest whole piece');
}

/**
 * The nearest quantity that exists, toward zero. Used where the number is what somebody CAN do —
 * deliver, pay, pledge — because rounding that up invents a piece nobody has.
 */
export function downTick(value: number): Qty {
  const n = finite(value, 'quantity');
  return asQty(n < 0 ? -Math.floor(-n) : Math.floor(n), 'the whole piece below');
}

/**
 * The nearest quantity that exists, away from zero. Used where the number is what somebody MUST
 * put up — the input a recipe draws, the collateral a claim needs — because a requirement met with
 * the piece below is a requirement not met.
 */
export function upTick(value: number): Qty {
  const n = finite(value, 'quantity');
  return asQty(n < 0 ? -Math.ceil(-n) : Math.ceil(n), 'the whole piece above');
}

/**
 * Clearing C3, Law 2: split a total into parts in the ratio of the weights, in whole pieces,
 * summing to EXACTLY the total. The floor of each share is given first and the pieces left over go
 * to the largest remainders, one each, ties to the earlier claimant — so the last piece has a
 * holder and the same claimants always get it.
 *
 * The total must already exist (be a whole number of pieces); the weights need not.
 */
export function splitOnTick(total: number, weights: readonly number[]): readonly Qty[] {
  if (weights.length === 0) return [];
  if (!onTick(total)) {
    throw new Impossible('Law 8', `${total} is not a whole number of pieces to split`);
  }
  let whole = total;
  const negative = whole < 0;
  if (negative) whole = -whole;
  let weight = 0;
  for (const w of weights) {
    if (finite(w, 'split weight') < 0) {
      throw new Impossible('Clearing C3', `a share of a split is never negative, got ${w}`);
    }
    weight += w;
  }
  if (weight <= 0) return weights.map(() => NO_QTY);
  const parts: number[] = [];
  const remainders: { at: number; rest: number }[] = [];
  let given = 0;
  weights.forEach((w, at) => {
    const exact = (whole * w) / weight;
    const floor = Math.floor(exact);
    parts.push(floor);
    given += floor;
    remainders.push({ at, rest: exact - floor });
  });
  remainders.sort((a, b) => (a.rest === b.rest ? a.at - b.at : b.rest - a.rest));
  for (let i = 0; given < whole; i += 1) {
    const next = remainders[i % remainders.length];
    const has = next === undefined ? undefined : parts[next.at];
    // Clearing C3, Law 5: THE LAST PIECE HAS A HOLDER OR NOTHING DOES. Both lookups are inside
    // arrays this function built and indexed itself, so neither can be missing — and a `break`
    // here would leave the parts summing to LESS than the total with nothing saying so, which is
    // the residual with no holder this function exists to prevent. It was a silent failure written
    // to appease `noUncheckedIndexedAccess` (item 13b.1).
    if (next === undefined || has === undefined) {
      throw new Impossible(
        'Clearing C3',
        `splitting ${total} over ${weights.length} weights lost a piece at ${i}`,
      );
    }
    parts[next.at] = has + 1;
    given += 1;
  }
  return parts.map((n) => asQty(negative ? -n : n, 'a share of the split'));
}

/**
 * XI-15, Law 8: the smallest amount two parties can actually exchange.
 *
 * A cell is a count of identical members, and each of them is a real holder whose share is a whole
 * number of pieces — so a cell of five hundred deals in five hundred pieces at a time. Two parties
 * can therefore only exchange a multiple of the least common multiple of their weights: a quantity
 * finer than that would leave one side's members holding a fraction of the smallest piece there is,
 * which is not a small difference but a thing that does not exist.
 *
 * Between named parties both weights are one and the grain is a single piece.
 */
export function commonGrain(a: number, b: number): number {
  const wa = whole(a);
  const wb = whole(b);
  return (wa / gcd(wa, wb)) * wb;
}

function whole(w: number): number {
  if (!Number.isInteger(w) || w <= 0) {
    throw new Impossible('XI-15', `a weight is a positive count, got ${w}`);
  }
  return w;
}

function gcd(a: number, b: number): number {
  let x = a;
  let y = b;
  while (y !== 0) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x;
}

/**
 * XI-15, Law 8: the most of a quantity that is a whole number of GRAINS, where a grain is what two
 * parties can actually exchange (`commonGrain`). Both are counts of pieces, so this is exact.
 */
export function downToGrain(value: number, grain: number): Qty {
  return asQty(Math.floor(finite(value, 'quantity') / whole(grain)) * grain, 'whole grains below');
}

/** The same on the nearest grain, for a value that BECOMES a payment rather than a delivery. */
export function toGrain(value: number, grain: number): Qty {
  // Rounded to the grain ONCE. Rounding to a piece first and to the grain after would move the
  // answer by half a piece more than the grain, and the check that compares the two sides of the
  // trade would then be reporting the second rounding as a discrepancy (Law 7).
  return asQty(Math.round(finite(value, 'quantity') / whole(grain)) * grain, 'the nearest grain');
}

/**
 * Law 1, Law 8, Clearing C4.c: A LEVEL ON THE MARKET'S OWN GRID.
 *
 * A market quotes on a grid — cents a share, pips a rate, ten-thousandths of a bond's face — and a
 * level finer than its tick is not a level anybody can hit. It is the same argument this file makes
 * for a quantity, made about a price: what is real has a smallest piece, and nothing finer exists.
 *
 * WHICH WAY IT ROUNDS IS NOT A CHOICE, and that is the difference from a quantity. A size means two
 * things — what a party CAN do and what it MUST do — so its author says which way it goes. A LIMIT
 * means exactly one: `Order.price` is the most a buyer will pay or the least a seller will accept,
 * so a buy that cannot be at 49.7938 can only be at 49.79 and a sell can only be at 49.80. Rounding
 * either the other way would post an order its author did not agree to. The side decides, and the
 * kernel is honouring what the poster said rather than deciding for it.
 *
 * THE TICK IS NOT EXACTLY REPRESENTABLE and this does not pretend otherwise: a hundredth is not a
 * binary fraction, so `n x tick` carries one rounding. What it buys is a level that is a whole
 * number of a real market's increments instead of one with seventeen digits in it — and the
 * residue, being one multiplication rather than an accumulation, is what Law 7's dust is for.
 */
export function downToTick(price: number, tick: number): number {
  const n = ticksIn(price, tick);
  return (onIt(n) ? Math.round(n) : Math.floor(n)) * tick;
}

/** The same, upwards: the least level on the grid that is not below what was asked (Law 8). */
export function upToTick(price: number, tick: number): number {
  const n = ticksIn(price, tick);
  return (onIt(n) ? Math.round(n) : Math.ceil(n)) * tick;
}

/**
 * Law 8: the nearest level on the grid, for a price that is STATED rather than posted — a seed's
 * opening level, a published net asset value. Nobody is promising anything at it, so there is no
 * side to take the direction from and the nearest is the honest answer.
 */
export function toTickOf(price: number, tick: number): number {
  return Math.round(ticksIn(price, tick)) * tick;
}

function ticksIn(price: number, tick: number): number {
  return finite(price, 'price') / positiveTick(tick);
}

/**
 * Law 7: A LEVEL THAT IS ALREADY ON THE GRID, to the dust of asking the question.
 *
 * `1.2 / 0.0001` is 11999.999999999998, because neither a fifth nor a ten-thousandth is a binary
 * fraction — so a buy AT a tick was floored a whole tick DOWN while a sell at the same tick was
 * ceilinged UP, and a book with a bid and an ask at one level came back `noOverlap`. Two orders
 * that agreed did not trade, and nothing anywhere said so.
 *
 * The tolerance is what the arithmetic did: ONE division, over the magnitude it produced. It is not
 * a band around the grid — a level a tick away is still a tick away and is still moved — it is the
 * recognition that a division of two numbers neither of which is representable cannot answer
 * exactly, and the direction a limit rounds may not turn on which side of an integer the error fell.
 */
function onIt(n: number): boolean {
  return Math.abs(n - Math.round(n)) <= dustOf(1, Math.abs(n));
}

function positiveTick(tick: number): number {
  if (!(tick > 0) || !Number.isFinite(tick)) {
    throw new Impossible('Law 8', `a price tick is a positive increment, got ${tick}`);
  }
  return tick;
}
