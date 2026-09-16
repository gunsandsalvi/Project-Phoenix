/**
 * Factoring: a seller that wants its money now sells the receivable to somebody who will wait.
 *
 * @spec Trade Credit A3 Trade Credit B2 Trade Credit C1.a Small-Business Pools A4 Small-Business Pools A5 Corporate Credit A4 Law 3 Law 4 Law 19
 *
 * A3 says the early-payment discount *"makes the discount an implicit interest rate and therefore a
 * price. Without it there is no rate, and no factoring market can exist."* That is this file's whole
 * construction, read both ways.
 *
 * WHAT THE FACTOR PAYS is the receivable discounted at what that factor requires of the name that
 * OWES it — which is the buyer, because a receivable is a claim on the buyer and a factor that
 * priced the seller would be pricing the wrong credit (Corporate Credit A4: an opinion is somebody's,
 * and it is about somebody). It is read off what each bank published it requires of that name
 * (`registry/banking.ts`), never re-derived here, and discounted through the one discounting the
 * world has (`prices/curve.ts priceAt`). A name no bank has priced has no factor, and that is a
 * refusal rather than a price.
 *
 * WHAT THE SELLER COMPARES IT WITH is two alternatives it already has, and neither is a gate anybody
 * chose. The first is its own discount: it has published what it will pay to be paid early — that IS
 * the discount on its own invoice — so a factor has to beat its own customer. The second is what
 * money costs the seller itself, which is what a lender published it requires of the SELLER's name.
 *
 * That second comparison is why factoring exists and why §42 A5's tier uses it most: selling a
 * receivable is borrowing against your CUSTOMER's credit instead of your own, so it is worth doing
 * exactly when your customer is the better name. A strong seller with a weak customer borrows; a
 * small firm shipping a large one factors. Nothing states which is which — both numbers are the
 * same banks' published opinions of two names (A4).
 */
import { minus, type PerPiece, type Ratio, asRatio } from '../../core/measure.js';

/**
 * A3: THE FLOOR UNDER A FACTOR'S BID, and the seller put it there itself.
 *
 * What a unit of this row is worth to its seller if it is paid early on its own terms — par less
 * what it offered to take off. A factor offering less than that is offering less than the seller's
 * own customer already can.
 */
export function ownPrice(discount: Ratio): Ratio {
  return minus(asRatio(1, 'the face'), discount, 'what its own early payment leaves it');
}

/**
 * B2, C1.a: WHETHER THIS BID IS WORTH TAKING. One comparison, per unit, both sides in the same
 * dimension: what the factor offers against what the seller's own terms already offer.
 */
export function worthSelling(bid: PerPiece, discount: Ratio): boolean {
  return bid > ownPrice(discount);
}

/**
 * §42 A5, Corporate Credit A4, XI-4: WHETHER BORROWING AGAINST THE CUSTOMER BEATS BORROWING AGAINST
 * ITSELF. The factor waits for the buyer and charges what it requires of the buyer; a lender would
 * wait for the seller and charge what it requires of the seller. The seller takes the cheaper, which
 * is the whole of why a small firm shipping a large one sells its invoices and a large one shipping
 * a small one does not.
 */
export function cheaperThanBorrowing(onTheBuyer: Ratio, onTheSeller: Ratio): boolean {
  return onTheBuyer < onTheSeller;
}
