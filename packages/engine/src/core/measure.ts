/**
 * A number that carries what it counts: the dimension, in the type.
 *
 * @spec Law 4 Law 7 Law 8 Money A2.b Currency C4 XI-15
 *
 * `mul(a: number, b: number, what: string)` has **950 call sites**, and at every one of them the
 * third argument is the only place the dimension lives. Nothing validates it. So:
 *
 *  - `mul(money, money, 'the premium')` compiles, and every option premium in this world is a
 *    money-squared number that does not depend on the strike (**A-65**);
 *  - `add(usd, eur, 'wealth')` compiles, and a cell's wealth is the sum of every money it has ever
 *    been paid in (**A-23**, **A-38**, **A-47**, **A-51**, **A-61**);
 *  - `mul(perMember, headcount, …)` on one side of a flow whose other side used `perMember × weight`
 *    compiles, and the wage bill capitalised into inventory is bigger than the wage that was paid,
 *    every period, for every row (**A-39** — the largest conservation break in the model);
 *  - and last period's number reads as this period's (**A-33**).
 *
 * **594 fields are typed as bare `number`.**
 *
 * `core/num.ts` already says what the fix is, in its own words: *"A caller mixing a `Qty` with a
 * plain number is refused by the compiler, which is the question the brand exists to ask."*
 * `Brand<T,B>` has existed since `core/ids.ts` was written; it is applied to identifiers and to
 * `Qty`, and to nothing else.
 *
 * THIS IS LAW 8 ENFORCED RATHER THAN REQUESTED. "Periodicity, price level and unit are part of the
 * number" is a rule the third argument asks for politely and the compiler now insists on.
 *
 * IT IS A PHANTOM TYPE AND ERASES COMPLETELY. `Measure<D>` IS a `number` at runtime: there is no
 * wrapper, no allocation and no arithmetic that was not there before, so **behaviour cannot change**
 * and Law 18's "gate on behaviour, not bits" is satisfied by construction. What changes is which
 * programs compile.
 *
 * HOW IT IS ADOPTED. Not as a rewrite. The dimension is already declared at all 950 sites; it moves
 * from the third argument into a type argument, and the compiler generates the migration list by
 * erroring at every site that was lying. A value with no dimension yet is a plain `number` and every
 * operation here accepts one, so a module that has not been migrated is not broken — it is simply
 * not yet checked, which is the state it was in before.
 */
import { finite } from './num.js';

declare const dimension: unique symbol;

/**
 * A number that knows what it is. `D` is a phantom: it exists in the type and nowhere at runtime.
 *
 * The brand is REQUIRED, and it has to be: an optional one lets a plain `number` stand in for any
 * dimension and therefore lets every dimension stand in for every other, which is the state the
 * engine is already in. So a `Measure` is CONSTRUCTED — at the door where the dimension is known —
 * and from there the compiler carries it. That is exactly how `Qty` already works.
 *
 * A `Measure<D>` IS a number where a number is wanted, so nothing that reads one has to change;
 * what cannot happen is putting one where a different dimension is wanted.
 *
 * THE PHANTOM IS A FUNCTION TYPE AND THAT IS NOT DECORATION. A plain `readonly [dimension]: D` is
 * COVARIANT, so `Measure<'money:USD'>` is assignable to `Measure<'money:USD' | 'money:EUR'>` — and
 * `plus(usd, eur, …)` then infers the union and compiles, which is the exact bug this file exists
 * to stop. `(d: D) => D` puts `D` in both a parameter and a return position, which makes it
 * INVARIANT: two dimensions are two types and inference has to pick one of them.
 */
export type Measure<D extends string> = number & { readonly [dimension]: (d: D) => D };

/**
 * Money A2.b, Currency C4: AN AMOUNT IN A NAMED CURRENCY. Two currencies are two dimensions and
 * they do not add — which is the whole of A-23, and of the five findings beside it.
 */
export type Money<C extends string> = Measure<`money:${C}`>;

/** Law 8: a count of things of a named unit. Tonnes are not hours and neither is a share. */
export type Amount<U extends string> = Measure<`amount:${U}`>;

/**
 * Law 3, Law 8: WHAT ONE OF A THING COSTS IN A MONEY. It is money PER unit, and that is why
 * `Money / Amount` gives one and `Price × Amount` gives money back.
 */
export type Price<C extends string, U extends string> = Measure<`price:${C}:${U}`>;

/**
 * A pure number: a share, a fraction, a multiple. **It can never be an amount**, which is what
 * stops a spread being used as a level (A-44) and a leverage ratio being called a cost of funds
 * (A-58).
 */
export type Ratio = Measure<'ratio'>;

/** XI-15: PER MEMBER of a cell, and the only conversion to a total is the cell's weight. */
export type PerMember<D extends string> = Measure<`perMember:${D}`>;

/** XI-15: the total across a cell's members. `PerMember` and `Total` are different types (A-1, A-39). */
export type Total<D extends string> = Measure<`total:${D}`>;

/**
 * Law 7, Law 8: two of the SAME thing, added. Two currencies do not meet here and the compiler is
 * what says so — `add(usd, eur, …)` used to compile and produce a number with no unit at all.
 */
export function plus<D extends string>(a: Measure<D>, b: Measure<D>, what: string): Measure<D> {
  return (finite(a, what) + finite(b, what)) as Measure<D>;
}

export function minus<D extends string>(a: Measure<D>, b: Measure<D>, what: string): Measure<D> {
  return (finite(a, what) - finite(b, what)) as Measure<D>;
}

/**
 * Law 8: A DIMENSION TIMES A RATIO IS THAT DIMENSION. This is the multiplication almost every one
 * of the 950 sites actually wants — a share of something, a rate applied to a balance, a fraction
 * of a population — and it is the one that cannot produce money².
 */
export function scale<D extends string>(a: Measure<D>, by: Ratio, what: string): Measure<D> {
  return (finite(a, what) * finite(by, what)) as Measure<D>;
}

/**
 * Law 3: WHAT A QUANTITY COMES TO AT A PRICE. `Price<C,U> × Amount<U> = Money<C>` — the unit
 * cancels, the currency survives, and there is no way to reach money² through it.
 */
export function valueAt<C extends string, U extends string>(
  price: Price<C, U>,
  qty: Amount<U>,
  what: string,
): Money<C> {
  return (finite(price, what) * finite(qty, what)) as Money<C>;
}

/**
 * Law 3: AND BACK. `Money<C> / Amount<U> = Price<C,U>`, which is the only way to make a price out
 * of a payment — and it is why a price cannot be made out of two payments (A-65).
 */
export function pricedAt<C extends string, U extends string>(
  paid: Money<C>,
  qty: Amount<U>,
  what: string,
): Price<C, U> {
  const q = finite(qty, what);
  if (q === 0) {
    throw new RangeError(`${what}: nothing was delivered, so there is no price it was delivered at`);
  }
  return (finite(paid, what) / q) as Price<C, U>;
}

/**
 * Law 8: TWO OF THE SAME THING, DIVIDED, IS A PURE NUMBER — a share, a ratio, a multiple. It is the
 * only way to reach `Ratio` from measured things, and it is why a ratio can never be spent.
 */
export function ratioOf<D extends string>(a: Measure<D>, b: Measure<D>, what: string): Ratio {
  const den = finite(b, what);
  if (den === 0) {
    throw new RangeError(`${what}: a share of nothing is not a number`);
  }
  return (finite(a, what) / den) as Ratio;
}

/**
 * XI-15: THE ONLY CONVERSION FROM PER-MEMBER TO TOTAL, and it is the cell's weight.
 *
 * A-1, A-18, A-39 are all one mistake made in three places: a per-member number multiplied by
 * something that was not the weight, or a total booked against a per-member account. Two types and
 * one door between them is what makes that mistake unwriteable.
 */
export function acrossMembers<D extends string>(
  perMember: PerMember<D>,
  weight: number,
  what: string,
): Total<D> {
  if (!Number.isInteger(weight) || weight < 0) {
    throw new RangeError(`${what}: a weight is a count of people, got ${weight}`);
  }
  return (finite(perMember, what) * weight) as Total<D>;
}

/** XI-15: and back, which is a division and therefore refuses a cell of nobody. */
export function eachMember<D extends string>(
  total: Total<D>,
  weight: number,
  what: string,
): PerMember<D> {
  if (!Number.isInteger(weight) || weight <= 0) {
    throw new RangeError(`${what}: a cell of ${weight} has nobody to divide between`);
  }
  return (finite(total, what) / weight) as PerMember<D>;
}

/**
 * THE DOORS. A dimension enters the type system exactly here, at the place that knows what the
 * number is — the parameter register reading a declared ratio, a market printing a price, a leg
 * carrying an amount. Everywhere else it is carried by the compiler.
 *
 * There is no `asMeasure<D>(x)` taking any dimension, deliberately: a cast that can produce any
 * dimension can produce the wrong one, and then this file is a comment rather than a check.
 */
export const asRatio = (x: number, what: string): Ratio => finite(x, what) as Ratio;

export const asMoney = <C extends string>(x: number, what: string): Money<C> =>
  finite(x, what) as Money<C>;

export const asAmount = <U extends string>(x: number, what: string): Amount<U> =>
  finite(x, what) as Amount<U>;

export const asPrice = <C extends string, U extends string>(
  x: number,
  what: string,
): Price<C, U> => finite(x, what) as Price<C, U>;

export const asPerMember = <D extends string>(x: number, what: string): PerMember<D> =>
  finite(x, what) as PerMember<D>;

export const asTotal = <D extends string>(x: number, what: string): Total<D> =>
  finite(x, what) as Total<D>;

/**
 * WHAT THE KERNEL CAN CHECK AND WHAT IT CANNOT, said out loud rather than implied by an alias.
 *
 * A currency in this world is DRAWN (Seed B1.a) and an instrument's unit is registry data, so
 * neither is a literal the compiler ever sees: `C` and `U` are `string` in every kernel signature
 * and two currencies are one type to it. What the kernel therefore checks is the DIMENSION — money
 * is not an amount, an amount is not a price, a price is not a ratio, and per-member is not a total
 * — and that is what makes A-65, A-39, A-1, A-18, A-44 and A-58 unwriteable. Cross-currency
 * addition (A-23 and the four beside it) stays a RUNTIME refusal at the leg, which is where the
 * currency is actually known, and becomes a compile error only where a module names its money as a
 * literal. The generics are still worth carrying: a kernel function generic in `C` cannot reach its
 * return currency except through the rate, whatever a caller instantiates it at.
 */
export type Cash = Money<'piece'>;

/**
 * Law 8: MONEY IN ITS NAMED UNIT — dollars rather than cents — which is what a seed states, what a
 * parameter declares and what a report shows. It is a DIFFERENT TYPE from `Cash` and that is the
 * point: the scale is part of the number (Law 8), the two differ by the currency's subdivision, and
 * the one door between them is `Registry.pieces` (`cash` in the seed). Adding a stated amount to a
 * balance was arithmetic nothing refused.
 */
export type Stated = Money<'named'>;

/**
 * Law 8: MONEY PER PIECE, which is what every price in this engine is. `Qty` is `Amount<'piece'>`
 * (`core/tick.ts`) because a count of pieces on the tick grid is exactly what an amount is here —
 * one representation of one thing (Law 4) — so `valueAt(price, qty)` takes the register's own
 * quantity with nothing in between.
 */
export type PerPiece = Price<'piece', 'piece'>;

/**
 * Law 8: MONEY FOR ONE OF A NAMED UNIT — a dollar a tonne, a wage an hour, a level per share — as a
 * recipe, a wage and an opening level are all stated. `Registry.priceOf` is the one door onto the
 * grid, and 12b.1 is the whole reason there is a type here: a stated level that never went through
 * it is a level its market could not print.
 */
export type PerNamedUnit = Price<'named', 'named'>;

/** Law 8: an amount of a named unit — tonnes, hours, shares — before it is counted in pieces. */
export type Named = Amount<'named'>;

/** The doors for the two above, so a kernel site does not have to spell the parameters out. */
export const asCash = (x: number, what: string): Cash => finite(x, what) as Cash;

export const asPerPiece = (x: number, what: string): PerPiece => finite(x, what) as PerPiece;

export const asStated = (x: number, what: string): Stated => finite(x, what) as Stated;

export const asPerNamedUnit = (x: number, what: string): PerNamedUnit =>
  finite(x, what) as PerNamedUnit;

export const asNamed = (x: number, what: string): Named => finite(x, what) as Named;

/**
 * Law 8: A DIMENSION DIVIDED BY A PURE NUMBER IS THAT DIMENSION — `scale` read the other way, and
 * it is here because the inverse is a rounding waiting to happen: `scale(x, asRatio(1 / n))` says
 * the same thing through a reciprocal nobody needed to compute. A split restating a price, a total
 * shared out, a cost spread over a run: each is one of these.
 */
export function over<D extends string>(a: Measure<D>, by: Ratio, what: string): Measure<D> {
  const den = finite(by, what);
  if (den === 0) {
    throw new RangeError(`${what}: divided into nothing`);
  }
  return (finite(a, what) / den) as Measure<D>;
}

/** Law 8: HOW BIG IT IS, which is the same dimension as the thing. A magnitude is not a ratio. */
export function absolute<D extends string>(a: Measure<D>, what: string): Measure<D> {
  return Math.abs(finite(a, what)) as Measure<D>;
}

/** Law 5: the other side of it — what is owed rather than held. A direction, not a new dimension. */
export function negated<D extends string>(a: Measure<D>, what: string): Measure<D> {
  return -finite(a, what) as Measure<D>;
}
