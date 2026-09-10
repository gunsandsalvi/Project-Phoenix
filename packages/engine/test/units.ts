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
 */
import { MONEY_PIECES, SHARE_PIECES, TONNE_PIECES, WHOLE_PIECES } from '../src/registry/grid.js';

/** An amount of money, in PHX, as the pieces of it the state holds. */
export const phx = (amount: number): number => Math.round(amount * MONEY_PIECES);

/** A weight of a good measured in tonnes, as the pieces of it the state holds. */
export const tonnes = (weight: number): number => Math.round(weight * TONNE_PIECES);

/** A count of a good that comes in whole things — a machine, a dwelling. */
export const machines = (count: number): number => Math.round(count * WHOLE_PIECES);

/** A count of shares. */
export const shares = (count: number): number => Math.round(count * SHARE_PIECES);

/** A price in PHX for one tonne, as the state holds a price: pieces of money per piece of good. */
export const perTonne = (price: number): number => (price * MONEY_PIECES) / TONNE_PIECES;

/** A price in PHX for one whole thing (a machine), as the state holds a price. */
export const perMachine = (price: number): number => (price * MONEY_PIECES) / WHOLE_PIECES;

/** A price in PHX for one share, as the state holds a price. */
export const perShare = (price: number): number => (price * MONEY_PIECES) / SHARE_PIECES;

/** A price in PHX for one unit of par (or of any money-denominated unit): a ratio, unchanged. */
export const perPar = (price: number): number => price;
