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
 * AND WHAT THE SMALLEST INCREMENT OF A PRICE IS, which is the same decision about the other grid.
 *
 * A price is money per named unit and a market quotes it on a grid of its own: the tick. The rule
 * for choosing one is the rule above, read for a price — IT IS THE SMALLEST INCREMENT ANYBODY
 * REALLY QUOTES — and it is not derived from the money's pieces, because the two conventions are
 * genuinely different: a share moves in cents and a bond in ten-thousandths of its own face, and
 * both are paid for in the same cents.
 *
 * AND IT IS A TECHNOLOGY (Law 2), NOT A RESOLUTION, which is what the measurement said rather than
 * what this item expected. A finer PIECE rounds an amount, so its effect shrinks as the piece does
 * and the path converges (test/tick.test.ts). A finer TICK moves the LEVEL a decision is taken at,
 * and a coarser one pulls a bid down and an ask up until books that used to cross no longer do — so
 * it changes WHO TRADES, and the money stock of this world moves three per cent between one tick
 * grid and another without converging at all. That is not an artefact to be minimised: it is what a
 * tick does in a real venue, which is exactly why exchanges and their regulators argue about tick
 * sizes. So it is a fact about the market, declared like any other, and `markets.tickShift` runs the
 * world at a finer one to show that every STRUCTURAL invariant still holds exactly — never that the
 * path is the same, because it is not and should not be.
 */

/** A share, a fund's published value, a tonne of a commodity: the cent. */
export const CENT_TICK = 0.01;

/**
 * Government paper: a ten-thousandth of its own face, which is a basis point of price. A cent of
 * face would be a whole percentage point of a bond and no sovereign book has ever quoted in those.
 */
export const FACE_TICK = 0.0001;

/** A machine: the whole money. Capital goods are not haggled over in cents. */
export const WHOLE_MONEY_TICK = 1;

/**
 * A PIP: the smallest increment a rate quoted in a money moves by. A ten-thousandth of that money
 * nearly everywhere, and a hundredth where one unit of it buys so little that a ten-thousandth is
 * beneath anyone's notice. It is a fact about the money a price is IN, not about the pair.
 */
export const PIP = 0.0001;
export const PIP_YEN = 0.01;

/**
 * Time: THE HOUR. Labour is contracted, supplied and paid for by the hour in this world and no wage
 * in it is struck for part of one, so an hour is the smallest piece of somebody's time there is.
 */
export const TIME_PIECES = 1;
