/**
 * What a test's numbers mean, in the units the state actually holds.
 *
 * @spec Law 8 Money A2 Goods A1 Equity A2
 *
 * The engine holds COUNTS OF INDIVISIBLE PIECES — cents, grams, whole machines, whole shares — and
 * a test that wrote a bare `85` would be saying eighty-five cents when it meant eighty-five PHX.
 * So a test says what it means: `phx(85)`, `tonnes(10)`, `perTonne(1.2)`. These are the same
 * conversions the seed uses at its own boundary (`registry.pieces`, `registry.priceOf`), written
 * once here so the tests read like the world they are about.
 *
 * Law 8: the ones that produce a QUANTITY say so in their type (`Qty`), so a test cannot hand the
 * engine a fraction of a cent any more than a mechanism can — this file is the tests' own door onto
 * the grid, and there is one of it. The ones that produce a PRICE are ratios and are not quantities:
 * pieces of money per piece of a good is a real number and always was.
 */
import { asQty, type Qty } from '../src/core/tick.js';
import {
  MONEY_PIECES,
  SHARE_PIECES,
  TIME_PIECES,
  TONNE_PIECES,
  WHOLE_PIECES,
} from '../src/registry/grid.js';

/** An amount of money, in PHX, as the pieces of it the state holds. */
export const phx = (amount: number): Qty => asQty(Math.round(amount * MONEY_PIECES));

/** A weight of a good measured in tonnes, as the pieces of it the state holds. */
export const tonnes = (weight: number): Qty => asQty(Math.round(weight * TONNE_PIECES));

/** A count of a good that comes in whole things — a machine, a dwelling. */
export const machines = (count: number): Qty => asQty(Math.round(count * WHOLE_PIECES));

/**
 * An amount of a money-denominated unit — par of a bond, a loan's principal — as the pieces of it
 * the state holds. A unit of par is divided like the money it is denominated in.
 */
export const par = (amount: number): Qty => asQty(Math.round(amount * MONEY_PIECES));

/** A count of shares. */
export const shares = (count: number): Qty => asQty(Math.round(count * SHARE_PIECES));

/** A price in PHX for one tonne, as the state holds a price: pieces of money per piece of good. */
export const perTonne = (price: number): number => (price * MONEY_PIECES) / TONNE_PIECES;

/** A price in PHX for one whole thing (a machine), as the state holds a price. */
export const perMachine = (price: number): number => (price * MONEY_PIECES) / WHOLE_PIECES;

/** A price in PHX for one share, as the state holds a price. */
export const perShare = (price: number): number => (price * MONEY_PIECES) / SHARE_PIECES;

/** A price in PHX for one unit of par (or of any money-denominated unit): a ratio, unchanged. */
export const perPar = (price: number): number => price;

/**
 * A technology stated per NAMED unit, as the state holds it: what one PIECE of a good takes. A
 * machine for a tonne a week is a millionth of a machine for a gram a week.
 */
export const machinesPerTonne = (units: number): number => (units * WHOLE_PIECES) / TONNE_PIECES;

/** Hours of labour for a tonne, as the minutes a piece of it takes. */
export const minutesPerTonne = (hours: number): number => (hours * TIME_PIECES) / TONNE_PIECES;

/** A wage in PHX for one hour, as the state holds a price: pieces of money per minute. */
export const perHour = (phxPerHour: number): number => (phxPerHour * MONEY_PIECES) / TIME_PIECES;

/** A span of time in hours, as the minutes the state counts it in. */
export const minutes = (hours: number): Qty => asQty(Math.round(hours * TIME_PIECES));
