/**
 * The smallest indivisible amount of a unit, and the arithmetic of splitting one.
 *
 * @spec Law 1 Law 2 Law 7 Law 8 Money A2 Money C1 Register A1.c Clearing C3 Appendix A
 *
 * A unit of anything real has a smallest piece. There is no half-cent, no thousandth of a share
 * certificate, and no gram of a cargo measured in tonnes: a quantity is a WHOLE NUMBER OF TICKS and
 * anything finer does not exist. Law 1 asks for the real mechanism, and continuous money is not it —
 * what continuous money produces instead is a residue of floating-point dust that every check then
 * has to be told to forgive, and a residual that belongs to nobody (Law 2 calls that a defect).
 *
 * TICKS ARE POWERS OF TWO, and that is the whole of why this works. A grid of 10^-2 is not
 * representable in binary floating point, so sums of "exact" decimal amounts drift off their own
 * grid and the dust comes back with extra steps. On a binary grid every sum and difference of whole
 * ticks is EXACT, up to 2^53 of them — so a balance moved a million times is exactly the balance,
 * and the checks that compare it need no tolerance at all rather than a derived one.
 *
 * WHAT THE GRID IS is a RESOLUTION (Law 2): change it and the world's path must not move. It is
 * declared per unit, as the exponent of its tick, and shifted globally by one parameter so that the
 * invariance can be tested by running the same world on a finer and a coarser grid.
 *
 * SPLITTING IS WHERE THE REAL MECHANISM SHOWS. Ten cents shared three ways is four, three and three:
 * somebody is rounded up and somebody down, the parts sum to exactly what was there, and WHO gets
 * the odd tick is a stated rule rather than an accident. `splitOnTick` is that rule — largest
 * remainder, ties to the earlier claimant — and it is what makes "no residual with no holder"
 * something the arithmetic cannot violate instead of something the audit reports afterwards.
 */
import { Impossible } from './errors.js';
import { finite } from './num.js';

/** A tick is 2^-exponent of its unit: exponent 0 is whole units, 20 is about a millionth. */
export function tickFromExponent(exponent: number): number {
  if (!Number.isInteger(exponent)) {
    throw new Impossible('Law 2', `a tick exponent is a whole number of halvings, got ${exponent}`);
  }
  const tick = Math.pow(2, -exponent);
  if (!Number.isFinite(tick) || tick <= 0) {
    throw new Impossible('Law 2', `tick exponent ${exponent} is not a usable grid`);
  }
  return tick;
}

/** Law 7: only a power of two gives exact sums, so only a power of two is a tick. */
export function isTick(tick: number): boolean {
  if (!Number.isFinite(tick) || tick <= 0) return false;
  const e = Math.round(Math.log2(tick));
  return Math.pow(2, e) === tick;
}

/** Whether this quantity exists: a whole number of ticks and nothing finer. */
export function onTick(value: number, tick: number): boolean {
  return Number.isInteger(finite(value, 'quantity') / tick);
}

/**
 * The nearest quantity that exists, ties away from zero. Used where a computed amount BECOMES a
 * payment or a delivery: what leaves an account is what the account can hold.
 */
export function toTick(value: number, tick: number): number {
  const n = finite(value, 'quantity') / tick;
  const whole = n < 0 ? -Math.round(-n) : Math.round(n);
  return whole * tick;
}

/**
 * The nearest quantity that exists, toward zero. Used where the number is what somebody CAN do —
 * deliver, pay, pledge — because rounding that up invents a tick nobody has.
 */
export function downTick(value: number, tick: number): number {
  const n = finite(value, 'quantity') / tick;
  const whole = n < 0 ? -Math.floor(-n) : Math.floor(n);
  return whole * tick;
}

/**
 * The nearest quantity that exists, away from zero. Used where the number is what somebody MUST
 * put up — the input a recipe draws, the collateral a claim needs — because a requirement met with
 * the piece below is a requirement not met.
 */
export function upTick(value: number, tick: number): number {
  const n = finite(value, 'quantity') / tick;
  const whole = n < 0 ? -Math.ceil(-n) : Math.ceil(n);
  return whole * tick;
}

/** How many whole ticks this quantity is. Exact, because the grid is binary. */
export function ticks(value: number, tick: number): number {
  return Math.round(finite(value, 'quantity') / tick);
}

/**
 * Clearing C3, Law 2: split a total into parts in the ratio of the weights, in whole ticks, summing
 * to EXACTLY the total. The floor of each share is given first and the ticks left over go to the
 * largest remainders, one each, ties to the earlier claimant — so the last tick has a holder and
 * the same claimants always get it.
 *
 * The total must already exist (be a whole number of ticks); the weights need not.
 */
export function splitOnTick(
  total: number,
  weights: readonly number[],
  tick: number,
): readonly number[] {
  if (weights.length === 0) return [];
  if (!onTick(total, tick)) {
    throw new Impossible('Law 8', `${total} is not a whole number of ${tick} to split`);
  }
  let whole = ticks(total, tick);
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
  return parts.map((n) => (negative ? -n : n) * tick);
}

/**
 * XI-15, Law 8: the smallest amount two parties can actually exchange.
 *
 * A cell is a count of identical members, and each of them is a real holder whose share is a whole
 * number of ticks — so a cell of five hundred deals in five hundred ticks at a time. Two parties can
 * therefore only exchange a multiple of the least common multiple of their weights, in ticks: a
 * quantity finer than that would leave one side's members holding a fraction of the smallest piece
 * there is, which is not a small difference but a thing that does not exist.
 *
 * Between named parties both weights are one and the grain is the tick itself.
 */
export function commonGrain(tick: number, a: number, b: number): number {
  const wa = whole(a);
  const wb = whole(b);
  return (wa / gcd(wa, wb)) * wb * tick;
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
