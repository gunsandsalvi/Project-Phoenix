/**
 * The workout: what a creditor does with a claim that stopped performing, and what it does with one
 * that is about to fall due and has not.
 *
 * @spec Banks Lending E3 Banks Lending C1 Banks Lending C3 Banks Lending D1 Corporate Credit A4 Law 2 Law 3 Law 6 Law 19
 *
 * E3 says restructure, extend or enforce, and that EACH IS A DECISION WITH A COST. So the decision
 * is written as the two paths a creditor can actually take with a row it holds, each valued in the
 * same unit — what a unit of what is owed brings this creditor — out of numbers its own credit view
 * already publishes. Nothing here is declared: the loss it expects on a default is its own record of
 * estates (C1.b), how often it has seen the name fail is its own record of the name, and what its
 * capital costs is its own published charge.
 *
 * ENFORCING brings what the estate returns: the security at the market's price of it, and its own
 * historical recovery on whatever the security does not cover. It brings it NOW, and the capital the
 * row consumed is released with it.
 *
 * AGREEING brings par, less what it still expects to lose on a name that has just failed it — and it
 * brings it at the new maturity, so the row goes on consuming this bank's capital until then, at the
 * charge the bank itself requires on capital. THAT IS THE COST OF THE DECISION, and it is why a bank
 * that recovers well from estates enforces while one that recovers nothing agrees: with nothing
 * recovered, enforcing brings nothing at all.
 *
 * Neither number is a PRICE and neither is discounted (Law 3): a loan has no market (A1.a) and is
 * carried at what its holder expects to recover (D1). These are two readings of that one expectation,
 * taken over two different futures, and the bank takes the bigger.
 */
import { asRatio, minus, scale, type Ratio } from '../../core/measure.js';
import type { NameView } from './credit-view.js';

/** E3: what a unit of what is owed brings this creditor down each of the two paths open to it. */
export interface Paths {
  /** What the estate returns on it: the security at its price, and its own recovery on the rest. */
  readonly enforcing: Ratio;
  /** Par less what it still expects to lose, less the capital carrying it that long costs. */
  readonly agreeing: Ratio;
}

/** One unit of what is owed, which is what both paths are measured against. */
const WHOLE: Ratio = asRatio(1, 'the whole of a unit owed');

/**
 * E3, C1.b: the two paths, per unit owed.
 *
 * `lossOnDefault` is what this bank expects to lose of a unit of THIS row if the borrower fails —
 * its own recovery record, already netted against the market's price of whatever is pledged
 * (`lossGivenDefault`). `failsPerYear` is how often it has seen this name fail (C1.b). `capitalCharge`
 * is what the capital a unit of it consumes costs this bank, per annum (C1.c), and `years` is how
 * long the new terms would keep consuming it.
 */
export function pathsOf(
  lossOnDefault: Ratio,
  failsPerYear: Ratio,
  capitalCharge: Ratio,
  years: Ratio,
): Paths {
  const enforcing = minus(WHOLE, lossOnDefault, 'what the estate returns of a unit');
  const stillAtRisk = scale(failsPerYear, lossOnDefault, 'what it still expects to lose');
  const carrying = scale(capitalCharge, years, 'what carrying it that long costs');
  return {
    enforcing,
    agreeing: minus(
      minus(WHOLE, stillAtRisk, 'par less what it still expects to lose'),
      carrying,
      'and less what the capital costs until then',
    ),
  };
}

/**
 * E3: THE DECISION. The bigger of the two, and a tie enforces — a creditor that is indifferent takes
 * the money rather than the promise, because the promise is the one that already broke.
 */
export function takes(p: Paths): 'agree' | 'enforce' {
  return p.agreeing > p.enforcing ? 'agree' : 'enforce';
}

/**
 * C3, 21.59: WHETHER THIS LENDER WOULD WRITE THIS LINE AGAIN TODAY, which is the whole of whether it
 * rolls one that is performing and reaching its maturity.
 *
 * A relationship lender does not call in a loan that is being paid: it agrees another term, at what
 * its view of the name now requires — so a borrower whose credit has deteriorated pays more for the
 * extension, and one its own standard now turns away does not get it and has to find the money. It
 * is the same credit decision as the original, taken on the same day against the same standard, and
 * the only thing that makes it a roll rather than a loan is that the row already exists.
 *
 * What it costs is stated and real: the lender gives up being repaid, and carries the name for
 * another term at a rate struck today instead of having its money back to lend elsewhere.
 */
export function rolls(view: NameView): boolean {
  return !view.declines.some;
}
