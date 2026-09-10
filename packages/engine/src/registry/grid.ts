/**
 * What the indivisible piece of each kind of unit IS, in this world.
 *
 * @spec Law 2 Law 8 Money A2 Goods A1 Labour A1 Equity A2 Register A1.c
 *
 * Every unit declares how many pieces it is divided into (`UnitDecl.perUnit`) and every quantity in
 * the state is a whole number of them. The numbers are here rather than scattered through the
 * modules because they are one decision taken together: a RESOLUTION (Law 2), chosen so that the
 * world's path does not turn on it and tested by declaring the same world in finer pieces
 * (test/tick.test.ts, `resolution.pieceShift`).
 *
 * The rule for choosing one is that IT IS THE SMALLEST PIECE ANYBODY REALLY DEALS IN, and that is
 * set by the SMALLEST holder rather than the largest: a person pays in cents and buys bread by the
 * gram-or-so, while a mill weighs its stock in tonnes. Nothing here is a machine epsilon dressed up
 * as a unit — a piece is a thing somebody could actually hand over.
 */

/** Money, and everything denominated in it (par, loans, money-market rows): the cent. */
export const MONEY_PIECES = 100;

/**
 * A good weighed in tonnes: the gram. A household member buys a kilo or two of bread a week, so a
 * piece at the kilo would round a person's whole week's shopping and the sector's demand with it.
 */
export const TONNE_PIECES = 1_000_000;

/** A good counted in whole things — a machine, a dwelling — is indivisible: there is no half of one. */
export const WHOLE_PIECES = 1;

/** A share: whole shares, as a register of members holds them. What one is worth is Equity's own resolution. */
export const SHARE_PIECES = 1;

/**
 * Time: THE HOUR. Labour is contracted, supplied and paid for by the hour in this world and no wage
 * in it is struck for part of one, so an hour is the smallest piece of somebody's time there is.
 */
export const TIME_PIECES = 1;
