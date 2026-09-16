/**
 * What this world states about a seller's terms, and it states a WIDTH rather than a number.
 *
 * @spec Trade Credit A3 Trade Credit B5 Seed B1 Seed B4 Law 2 Law 15
 *
 * A3 says terms are *"how long, and often a discount for paying early"*, and B5 says the seller
 * DECIDES them. One number for the whole world said neither: a mill and a corner shop wrote the
 * same invoice, and a sector of equals produces no market (Seed B4). So what is stated here is the
 * width each seller's own terms are drawn from, once, under its own name — and what is written down
 * by hand is why the world is that wide, never what any one seller offers.
 *
 * THE TWO DAY-COUNTS DO NOT OVERLAP, and that is the one thing this table has to guarantee: the
 * early-payment window is inside the term or there is no early payment, and a pair of independent
 * draws that could cross would be a seller whose discount expires after its invoice falls due.
 * `assertTermsAreOrdered` is that check, run at assembly rather than trusted.
 */
import { InvalidRegistry } from '../../core/errors.js';
import { paramId, type ParamId } from '../../core/ids.js';
import type { Spread } from '../../rng/spread.js';

/** A3, B5: one seller's own terms, declared in the parameter register under its own name (XI-14). */
export const sellerParam = (seller: string, what: string): ParamId =>
  paramId(`tradeCredit.${what}.${seller}`);

export interface TermsDispersion {
  readonly days: Spread;
  readonly discountDays: Spread;
  readonly discount: Spread;
}

export const TERMS_SPREAD: TermsDispersion = {
  days: {
    low: 21,
    high: 60,
    why: 'Trade Credit A3, B5: how long THIS seller gives a buyer to pay. A month is what most of the world writes, and the width is what the trades on either side of it write: a business shipping perishables wants its money in three weeks and one shipping machinery gives two months. It is the seller\'s own preference and not a convention of the world, because B5 says the seller decides its terms — and a world where every seller wrote the same number had no terms to compete on, which is B2\'s whole reason for offering them.',
  },
  discountDays: {
    low: 5,
    high: 12,
    why: 'Trade Credit A3: how long this seller\'s early-payment window is. Shorter than any term in the width above, always, so the discount is a real option on every invoice this world writes rather than one that expires after the money was due. A week or so is what a seller can afford to wait for cash it wanted now.',
  },
  discount: {
    low: 0.005,
    high: 0.03,
    why: 'Trade Credit A3: what this seller takes off for being paid inside its window. It is not a fee and not a preference about prices: WITH THE TWO DATES IT IS AN INTEREST RATE, and A3 says so — the rate at which the seller is willing to buy its own money back early. The width is what makes two sellers\' implicit rates differ, which is what a buyer choosing between paying early and holding its cash is choosing between.',
  },
};

/**
 * A3: the window is inside the term. Checked here rather than assumed, because the two are drawn
 * independently and a seller whose discount outlived its invoice would be offering nothing.
 */
export function assertTermsAreOrdered(s: TermsDispersion): void {
  if (s.discountDays.high >= s.days.low) {
    throw new InvalidRegistry(
      'Trade Credit A3',
      `an early-payment window of up to ${String(s.discountDays.high)} days would outlast a term of ${String(s.days.low)}`,
    );
  }
}
