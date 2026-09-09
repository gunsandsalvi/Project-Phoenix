/**
 * The reads that come off a share register and a share price.
 *
 * @spec Equity B4 Equity B4.a Equity C1.b Equity G3
 *
 * WHERE THE OPINION IS NOT. What a share is worth to one holder is that holder's own arithmetic and
 * lives in that holder's own module (Households D5, portfolio.ts): a participant's reason belongs
 * to the participant, and a module that computed other people's reservations would be handing the
 * market its answer (Clearing A3, XI-13). What lives here is what anybody may read once a session
 * has printed — and G3 is the rule about all of it: a derived statistic is computed FROM the
 * cleared price and is never used to set it.
 */
import { mul, sub } from '../../core/num.js';

/**
 * B4: market capitalisation is a READ — shares times price. Nothing compares it against shares
 * times price and calls that a check (B4.a): that is a tautology and cannot fail. It is here so
 * that whoever reports it reports the one derivation, and so that its two inputs are named.
 */
export function marketCapitalisation(shares: number, price: number): number {
  return mul(shares, price, 'market capitalisation');
}

/**
 * C1.b: the free float — what is genuinely tradeable. Issued less what the holders who will not
 * sell are holding. It is a read of the register and of who those holders are, never a stored
 * number and never a fraction anybody stated.
 */
export function freeFloat(issued: number, strategic: number): number {
  return sub(issued, strategic, 'the free float');
}
