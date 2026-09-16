/**
 * What every corporate line — fixed or floating — is counted in and promises.
 *
 * @spec Corporate Credit B2 Corporate Credit B2.a Corporate Credit B4 Bond N9 Law 4
 *
 * Both kinds this module declares are par paper with covenants on them, and they must agree about
 * what a unit IS and what a promise is (Law 4). So the two facts they share live here, and neither
 * file imports the other to get them.
 */
import { unitId } from '../../core/ids.js';
import type { Ratio } from '../../core/measure.js';

/** N9: quoted as a fraction of its own face, like any other bond. */
export const CORPORATE_PAR = unitId('corporate.par');

/**
 * B2, B2.a: WHAT THIS ISSUER PROMISED ITS LENDERS, struck when it borrowed and stated on the paper.
 *
 * Two lines, because they are the two questions a lender actually asks and they fail in different
 * worlds: how much it owes against what it has (a balance-sheet test, which a fall in asset prices
 * breaks), and what it earns against what falls due (an income test, which a bad year breaks). A
 * firm can pass either while failing the other, and which one goes says what went wrong.
 *
 * Neither is a parameter. They are TERMS — this issuer's own commitment at this issue — and what a
 * given firm promised is an outcome of what it had to promise to be lent to.
 */
export interface Covenants {
  /** B2: the most it may owe against what it holds, as the issuer's own published accounts read. */
  readonly leverage: Ratio;
  /** B2: the least it must earn against what falls due, on the same published accounts. */
  readonly coverage: Ratio;
}
