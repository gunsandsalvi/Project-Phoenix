/**
 * How fine the smallest piece of each kind of unit is, in this world.
 *
 * @spec Law 2 Law 8 Money A2 Goods A1 Labour A1 Equity A2 Register A1.c
 *
 * Every unit declares its own grid (`UnitDecl.tickExponent`), and the numbers are here rather than
 * scattered through the modules because they are one decision taken together: a RESOLUTION (Law 2),
 * chosen so that the world's path does not turn on it and tested by running the same world on a
 * finer and a coarser grid (test/tick.test.ts, `resolution.tickShift`).
 *
 * The rule for choosing one is that IT IS SET BY THE SMALLEST HOLDER, not the largest. A firm deals
 * in tonnes and a household member in a kilo or two of bread a week, so a grid at the kilo would
 * round a person's whole week's shopping up or down — and the sector's demand with it. Every grid
 * here is fine enough that no decision in this world turns on it, and coarse enough to be a real
 * granularity rather than the floating point's own dust: a millionth is ten orders of magnitude
 * above where the arithmetic stops being exact.
 */

/** Money, and everything denominated in it (par, loans, money-market rows): about a millionth. */
export const MONEY_GRID = 20;

/** Goods and the plant they become: about a gram of a tonne, or a millionth of a machine. */
export const GOODS_GRID = 20;

/**
 * A share: finer again, because a share here costs a few units of money and a household member
 * holds a ten-thousandth of one. Whole shares would put equity out of a household's reach.
 */
export const SHARE_GRID = 24;

/** Time: about four seconds of an hour, which is finer than any contract in this world states. */
export const TIME_GRID = 10;
