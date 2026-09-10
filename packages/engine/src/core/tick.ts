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
import { finite } from './num.js';

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
export function toTick(value: number): number {
  const n = finite(value, 'quantity');
  return n < 0 ? -Math.round(-n) : Math.round(n);
}

/**
 * The nearest quantity that exists, toward zero. Used where the number is what somebody CAN do —
 * deliver, pay, pledge — because rounding that up invents a piece nobody has.
 */
export function downTick(value: number): number {
  const n = finite(value, 'quantity');
  return n < 0 ? -Math.floor(-n) : Math.floor(n);
}

/**
 * The nearest quantity that exists, away from zero. Used where the number is what somebody MUST
 * put up — the input a recipe draws, the collateral a claim needs — because a requirement met with
 * the piece below is a requirement not met.
 */
export function upTick(value: number): number {
  const n = finite(value, 'quantity');
  return n < 0 ? -Math.ceil(-n) : Math.ceil(n);
}

/**
 * Clearing C3, Law 2: split a total into parts in the ratio of the weights, in whole pieces,
 * summing to EXACTLY the total. The floor of each share is given first and the pieces left over go
 * to the largest remainders, one each, ties to the earlier claimant — so the last piece has a
 * holder and the same claimants always get it.
 *
 * The total must already exist (be a whole number of pieces); the weights need not.
 */
export function splitOnTick(total: number, weights: readonly number[]): readonly number[] {
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
  if (weight <= 0) return weights.map(() => 0);
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
    if (next === undefined || has === undefined) break;
    parts[next.at] = has + 1;
    given += 1;
  }
  return parts.map((n) => (negative ? -n : n));
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
export function downToGrain(value: number, grain: number): number {
  return Math.floor(finite(value, 'quantity') / whole(grain)) * grain;
}

/** The same on the nearest grain, for a value that BECOMES a payment rather than a delivery. */
export function toGrain(value: number, grain: number): number {
  // Rounded to the grain ONCE. Rounding to a piece first and to the grain after would move the
  // answer by half a piece more than the grain, and the check that compares the two sides of the
  // trade would then be reporting the second rounding as a discrepancy (Law 7).
  return Math.round(finite(value, 'quantity') / whole(grain)) * grain;
}
