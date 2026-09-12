/**
 * Numeric primitives.
 *
 * @spec Law 7 Audit A4 Audit A4.a Register B2.b
 *
 * Every number entering the state is finite (NonFinite otherwise). Every identity check derives its
 * own tolerance — the dust — from the arithmetic that produced the number: count × ε × Σ|terms|.
 * There is no global epsilon and no percentage band anywhere in the engine.
 *
 * This is the only module in which Math.min / Math.max may appear (Law 6): here they express
 * arithmetic, never a decision.
 */
import { Impossible, NonFinite } from './errors.js';

export const EPS: number = Number.EPSILON;

/** The smallest double that still carries a full mantissa; below it there is no relative precision. */
const SMALLEST_NORMAL: number = Number.MIN_VALUE / EPS;

/** Validate that a number is finite and normalise -0 to 0. Throws NonFinite otherwise. */
export function finite(x: number, what: string): number {
  if (!Number.isFinite(x)) {
    throw new NonFinite('Law 7', `${what} is ${String(x)}`, { what, value: x });
  }
  return x === 0 ? 0 : x;
}

/** A sum with the arithmetic dust it is entitled to (Law 7). */
export interface Sum {
  readonly value: number;
  /** The tolerance any comparison involving this sum may use: terms × ε × Σ|terms|. */
  readonly dust: number;
  readonly terms: number;
  /** Σ|terms|: the magnitude the arithmetic passed through, which is rarely |value|. */
  readonly magnitude: number;
}

export const ZERO_SUM: Sum = Object.freeze({ value: 0, dust: 0, terms: 0, magnitude: 0 });

/**
 * Neumaier compensated summation. The returned dust is the specification's bound,
 * (number of terms) × ε × (sum of absolute magnitudes), not the (smaller) compensated error.
 */
export function sum(terms: Iterable<number>): Sum {
  let s = 0;
  let c = 0;
  let mag = 0;
  let n = 0;
  for (const raw of terms) {
    const t = finite(raw, 'sum term');
    n += 1;
    mag += Math.abs(t);
    const u = s + t;
    if (Math.abs(s) >= Math.abs(t)) c += s - u + t;
    else c += t - u + s;
    s = u;
  }
  const value = finite(s + c, 'sum');
  return { value, dust: n * EPS * mag, terms: n, magnitude: mag };
}

/**
 * A balance reached by accumulation: a total stated once and then moved, one named event at a time,
 * for as long as its owner exists — an equity account, a money balance.
 *
 * Law 7's dust is the dust of the arithmetic that produced the number, and for such a balance that
 * is every rounding since it was stated: each one landing at the magnitude of the balance it made,
 * not at the size of the move that made it. A check comparing a fresh read against a balance moved
 * a thousand times is comparing against a thousand roundings, so the walk travels with the number
 * rather than being guessed back — badly — by whoever reads it.
 */
export interface Running {
  readonly value: number;
  /** Σ ε|balance after each move|: what the walk itself is entitled to. */
  readonly dust: number;
  /** How many moves it took to get here. */
  readonly moves: number;
}

/** State a balance: the one rounding is the statement of the number itself. */
export function opened(value: number, what: string): Running {
  const v = finite(value, what);
  return { value: v, dust: moveDust(v, 0), moves: 0 };
}

/**
 * State a balance that was COMPUTED, carrying the dust its arithmetic earned (Law 7).
 *
 * `opened` is right for a balance somebody stated — the one rounding is the statement of it. It is
 * wrong for a balance somebody worked out, because Law 7's dust is the dust of the TERMS, never of
 * the answer: a fund's equity is zero by construction (Fund Shares A3) and that zero is a
 * contribution of four hundred billion minus a book of four hundred billion, so `opened(0)` charges
 * it ε × 0 and the first check against it reports the real residue of the subtraction — measured at
 * 0.00008869, which is ε × 4e11 to the digit — as a violation of an identity nothing is wrong with.
 *
 * Widening anything is forbidden and nothing here is widened: the balance opens with what its own
 * arithmetic did, which is the same rule `moved` applies to every step after it.
 */
export function openedFrom(terms: Sum, what: string): Running {
  const v = finite(terms.value, what);
  return { value: v, dust: terms.dust + moveDust(v, 0), moves: 0 };
}

/**
 * Move a balance by one event; the rounding lands at the magnitude the addition passed through.
 *
 * `through` is that magnitude when it is BIGGER than the move itself: an instruction that takes a
 * party's equity down by a thousand and back up by a thousand moves it by nothing, and the rounding
 * it left behind is the rounding of a thousand, not of nothing. A balance that is zero by
 * construction — a fund's equity (Fund Shares A3) — is nothing BUT that residue, so a walk that
 * only saw the net would call every one of them a defect (Law 7: the tolerance is what the
 * arithmetic did, not what the answer looks like).
 */
export function moved(balance: Running, delta: number, what: string, through = 0): Running {
  const value = finite(balance.value + finite(delta, what), what);
  const passed = Math.abs(through) > Math.abs(delta) ? through : delta;
  return { value, dust: balance.dust + moveDust(balance.value, passed), moves: balance.moves + 1 };
}

/** What one more move onto a running balance costs it: the rounding of that one addition (Law 7). */
export function moveDust(balance: number, delta: number): number {
  return dustOf(1, Math.abs(balance) + Math.abs(delta));
}

/** Dust for a comparison of two quantities computed from `terms` terms of total magnitude `magnitude`. */
export function dustOf(terms: number, magnitude: number): number {
  const d = terms * EPS * Math.abs(magnitude);
  // Law 7, and the one thing floating point does that arithmetic does not: below the smallest
  // NORMAL double there is no relative precision left. A magnitude down there has a dust of its
  // own that underflows to zero, so every identity in the wire becomes EXACT at exactly the scale
  // where the representation is least exact — a per-member share multiplied back by a weight stops
  // giving the total again, and the settlement refuses a leg nothing is wrong with. This is not a
  // widened band and not a bound on anything the model decides (Law 6): nothing is clamped to it
  // and no flow is cut short at it. It says that a quantity below the representation's own floor
  // is not distinguishable from nothing, which is true, and it is stated once here.
  return d > SMALLEST_NORMAL ? d : SMALLEST_NORMAL;
}

/**
 * The dust of a balance CARRIED from one end of a period to the other (Law 7).
 *
 * Two records of one walk are being compared: a balance read at each end, and the legs that moved
 * it in between. They are not two readings of one number — the balance was carried from the first
 * to the second by applying those legs ONE AT A TIME, every application rounding at the magnitude
 * the balance passed through rather than at the size of the leg — and each end was itself read over
 * `reads` terms (the lots a holding is held in, or one for a single running total).
 *
 * Derived any smaller, as though two readings of one balance, a busy account reports a violation
 * the moment it is moved more than a handful of times in a period. This is the one derivation, used
 * by every family that compares a book against the wire that moved it.
 */
export function carriedDust(before: number, now: number, reads: number, legs: Sum): number {
  const ends = Math.abs(before) + Math.abs(now);
  return (
    dustOf(reads + 2, ends) + dustOf(legs.terms, Math.abs(before) + legs.magnitude) + legs.dust
  );
}

/** Combine the dust of several sums that are then compared or added. */
export function combineDust(...sums: readonly Sum[]): number {
  let d = 0;
  let n = 0;
  let mag = 0;
  for (const s of sums) {
    d += s.dust;
    n += s.terms;
    mag += Math.abs(s.value);
  }
  // The comparison itself adds one rounding per term it touches.
  return d + n * EPS * mag;
}

/** |a − b| within the dust. */
export function withinDust(a: number, b: number, dust: number): boolean {
  return Math.abs(finite(a, 'a') - finite(b, 'b')) <= dust;
}

/** A count: a non-negative integer. A fractional or negative count is arithmetic impossibility. */
export function count(x: number, what: string): number {
  const v = finite(x, what);
  if (!Number.isInteger(v) || v < 0) {
    throw new Impossible('Law 6', `${what} must be a non-negative integer, got ${v}`, {
      what,
      value: v,
    });
  }
  return v;
}

/** A strictly positive count (a weight, a seat count). */
export function positiveCount(x: number, what: string): number {
  const v = count(x, what);
  if (v === 0) throw new Impossible('Law 6', `${what} must be positive`, { what });
  return v;
}

/** Multiply with validation. */
export function mul(a: number, b: number, what: string): number {
  return finite(finite(a, what) * finite(b, what), what);
}

/** Divide with validation; division by zero is non-finite and throws. */
export function div(a: number, b: number, what: string): number {
  return finite(finite(a, what) / finite(b, what), what);
}

export function add(a: number, b: number, what: string): number {
  return finite(finite(a, what) + finite(b, what), what);
}

export function sub(a: number, b: number, what: string): number {
  return finite(finite(a, what) - finite(b, what), what);
}

/**
 * LAW 6'S ONE ADMISSIBLE CASE, SAID OUT LOUD: what a party CAN do, not what it is allowed to want.
 *
 * A bank can only sell paper it holds; a firm can only hire the hours there are; a member can only
 * post margin out of the money in its account. Each of those is arithmetic impossibility — the
 * thing on the other side does not exist — and Law 6 admits exactly that and nothing else.
 *
 * The engine wrote it thirty-one times as `a < b ? a : b`, which `phoenix/no-bounds` cannot tell
 * from a cap somebody chose (it forbade `Math.min`, a spelling, and the ternary is the same
 * operation in different clothes). So a reviewer grepping for bounds found nothing and a reviewer
 * grepping for ternaries found thirty-one with no way to sort them. **The third argument is the
 * point of these two functions**: it names the thing that is not there, so a bound with a weak
 * reason reads as one, and the rule can forbid the bare comparison-ternary shape entirely.
 *
 * They are NOT a place to put a cap. If the reason cannot be written as "there is no more of it",
 * the number is a decision or a missing mechanism and Law 6 says so.
 */
/**
 * Law 8: IT IS GENERIC SO THE GRID SURVIVES IT. Two counts of whole pieces meeting each other give
 * a count of whole pieces — neither of them was rounded to get here, so nothing has left the grid,
 * and a caller that had a `Qty` still has one. A caller mixing a `Qty` with a plain number is
 * refused by the compiler, which is the question the brand exists to ask.
 */
export function atMost<T extends number>(value: T, limit: T, becauseThereIsNoMore: string): T {
  finite(value, becauseThereIsNoMore);
  finite(limit, becauseThereIsNoMore);
  return value < limit ? value : limit;
}

/** The same, the other way: what it cannot go below because there is nothing below it. */
export function atLeast<T extends number>(value: T, floor: T, becauseThereIsNoLess: string): T {
  finite(value, becauseThereIsNoLess);
  finite(floor, becauseThereIsNoLess);
  return value > floor ? value : floor;
}

/** Largest of a non-empty list — arithmetic, used for reporting worst instances (Audit D2). */
export function largest(values: readonly number[], what: string): number {
  let m: number | undefined;
  for (const v of values) {
    const f = finite(v, what);
    if (m === undefined || f > m) m = f;
  }
  if (m === undefined) throw new Impossible('Audit D2', `largest of nothing: ${what}`);
  return m;
}

/**
 * A quantity that is absent is none of it: a party that holds no units of an instrument holds zero
 * (Register). This is the one place absence becomes zero, and only for quantities, never for a
 * value, a price or a rate (Appendix A: a missing number is missing).
 */
export function zeroIfNone(q: number | undefined): number {
  return q === undefined ? 0 : finite(q, 'quantity');
}

/**
 * Invert a strictly decreasing function by bisection: the x at which f(x) = target.
 *
 * Arithmetic, not a decision: it inverts a function somebody else stated. The bracket is a bracket
 * and is never reported as an answer (Clearing C4.c is about prices; this returns an x it found).
 *
 * `undefined` WHERE THERE IS NO SUCH X, and that is an answer rather than a failure. `f` is defined
 * on part of the line and the walk stops at the edge of it; a target outside what the function
 * reaches there has no inverse, which is a true thing about the target — a bill priced above what
 * it redeems for, days from redeeming, has no yield — and the caller is the one that knows what to
 * do about it. It used to march past that edge and ask `f` for a value there; the answer was
 * Infinity, `finite` threw from inside the walk, and the run stopped on what looked like an
 * arithmetic defect and was a price.
 */
function answered(f: (x: number) => number, x: number): number | undefined {
  const y = f(x);
  return Number.isFinite(y) ? y : undefined;
}

export function invertDecreasing(
  f: (x: number) => number,
  target: number,
  what: string,
): number | undefined {
  // Find a bracket by walking outward from zero. A bracket is arithmetic and is never an answer:
  // what comes back is the x where f(x) meets the target, not an end of the search.
  let a = 0;
  let b = 1;
  // A WALK STOPS AT THE EDGE OF WHAT THE FUNCTION MEANS. `f` is only defined on part of the line —
  // a present value is a discount factor and a discount factor of a rate at or below minus one is
  // not a number — and the walk outward reaches that edge before it reaches an answer whenever the
  // target is beyond the range. It used to keep going and ask `f` for a value there: the answer was
  // Infinity, `finite` threw inside the walk, and what a reader saw was `yield of ust.bill... is
  // Infinity` rather than the truth, which is that a bill priced above what it redeems for, days
  // from redeeming, has no yield at all. So the walk keeps the last x that ANSWERED, and the check
  // below reports a target outside the range as what it is.
  let fa = answered(f, a);
  for (let i = 0; i < 64 && fa !== undefined && fa < target; i += 1) {
    const next = a === 0 ? -0.5 : (a - 1) / 2;
    const at = answered(f, next);
    if (at === undefined) break;
    a = next;
    fa = at;
  }
  let fb = answered(f, b);
  for (let i = 0; i < 64 && fb !== undefined && fb > target; i += 1) {
    const at = answered(f, b * 2);
    if (at === undefined) break;
    b *= 2;
    fb = at;
  }
  if (fa === undefined || fb === undefined || fa < target || fb > target) return undefined;
  // Halve the bracket until it is narrower than the dust of the numbers being compared, and no
  // more than a stated number of times, so the loop terminates on any input.
  for (let i = 0; i < 128; i += 1) {
    const mid = (a + b) / 2;
    if (b - a <= dustOf(2, Math.abs(a) + Math.abs(b))) break;
    if (f(mid) > target) a = mid;
    else b = mid;
  }
  return finite((a + b) / 2, what);
}

/**
 * Whether a computed quantity is a real one or just the dust of the arithmetic that produced it.
 * A difference of two numbers of size `magnitude` is not a decision to act on when it is smaller
 * than the rounding of that subtraction: posting an order for it, or booking a lot of it, invents a
 * position out of floating point. Law 7 applied at the point a number becomes an action.
 */
export function material(value: number, terms: number, magnitude: number): boolean {
  return Math.abs(finite(value, 'material')) > dustOf(terms, magnitude);
}

/** Add a term into a keyed accumulator; a key with no terms yet has accumulated nothing. */
export function addTo<K>(acc: Map<K, number>, key: K, delta: number): void {
  const cur = acc.get(key) ?? 0;
  acc.set(key, finite(cur + delta, 'accumulator'));
}
