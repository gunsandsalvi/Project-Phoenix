/**
 * Commercial paper: a firm or a bank borrows for weeks, and must ask again when the weeks are up.
 *
 * @spec Short-Term Debt A1 Short-Term Debt A2 Short-Term Debt A2.a Short-Term Debt A3 Short-Term Debt D1 Short-Term Debt D2 Short-Term Debt E2 Bond N4 Bond N5.c Bond N11 Bond N12 Bond N13 Bond N13.a Corporate Credit G2 Law 3 Law 4 Law 8 Law 9
 *
 * A1: IT SATISFIES THE BOND CONTRACT AND ANSWERS FOUR OF ITS NODES ITS OWN WAY. No coupon — issued
 * at a discount, redeemed at par, and the discount is the whole return (A1.a, N5.c). Under a year,
 * weeks to months (A1.b). Senior unsecured, ranking with the issuer's other senior debt (A1.c,
 * N13.a). No early-termination regime at all, because it is too short to be worth an option (A1.d,
 * N11) — and that is a stated answer, not an omission.
 *
 * WHY THIS IS NOT THE SOVEREIGN BILL, AND WHY IT IS NOT A SECOND COPY OF IT EITHER. The promise is
 * identical and is written once, in the kernel (`DiscountSchedule`): one payment of par on one day.
 * What differs is the same three things that make §7 a different subject from §8, and they are the
 * only three: this issuer can FAIL, so a missed payment is a default and there is an estate with a
 * ranking to read; the claim RANKS, senior unsecured, on the number the waterfall already orders by;
 * and one miss makes the rest DUE (Corporate Credit G2), where a sovereign's does not (Sovereign
 * G3). A3 is what that adds up to: the state, a bank and a firm issue the same instrument, and the
 * TYPE IS THE CREDIT — which is a fact about the issuer, never a branch in a mechanism (Law 15).
 *
 * A2, E2: ITS PRICE IS WHAT IT CLEARS AT. The yield is derived from the price and the days left, in
 * that direction and never the other — a discount computed off a curve nobody traded is Law 3's
 * defect at the short end, and E2 names it separately because the short end is where it is most
 * tempting.
 */
import { asCash, minus, type PerPiece, type Ratio, ratioOf } from '../../core/measure.js';
import { formatCivil, type Civil } from '../../calendar/civil.js';
import { instrumentId, instrumentKindId, type InstrumentId, type PartyId } from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import {
  type DiscountSchedule,
  discountDue,
  discountFlows,
  validateDates,
} from '../../registry/claims.js';
import { FACE_TICK } from '../../registry/grid.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { unitId } from '../../core/ids.js';
import { issuerName } from '../../registry/naming.js';

export const COMMERCIAL_PAPER = instrumentKindId('commercial.paper');

/** N9: quoted as a fraction of its own face, like every other piece of paper in this world. */
export const PAPER_PAR = unitId('paper.par');

export interface PaperTerms extends Terms, DiscountSchedule {
  readonly kind: typeof COMMERCIAL_PAPER;
  readonly issuer: PartyId;
  /**
   * A1.c, N13.a: WHERE IT RANKS, as the number the waterfall already orders by. It is senior
   * unsecured and carries the same number a firm's senior bond carries, because "ranking with the
   * issuer's other senior debt" is not a resemblance — it is the same rank, and two claims at one
   * rank share what is there (Corporate Credit G5).
   */
  readonly seniority: number;
}

/**
 * Law 15: structural, because nothing may branch on a kind id. Paper is the only thing an ISSUER
 * promised that RANKS and pays no COUPON: a corporate bond has all three with the coupon, a
 * tranche ranks but names a vehicle rather than an issuer, and a bank's subordinated line names an
 * issuer but no rank.
 */
export const isPaper = (t: Terms): t is PaperTerms =>
  'issuer' in t && 'seniority' in t && !('coupon' in t);

export function paperTerms(i: Instrument): PaperTerms {
  if (!isPaper(i.terms)) {
    throw new InvalidRegistry('Short-Term Debt A1', `${i.id} is not commercial paper`);
  }
  return i.terms;
}

/**
 * Law 9: a market names short paper by its issuer and the day it is due, because at this tenor
 * there is nothing else to say about it — no coupon to quote, and nobody holds it long enough for
 * anything but the date to matter. One line per issuer per maturity, so an issuer coming back to a
 * date it already has is selling more of the same paper (C8, as item 10 established for bonds).
 */
export const paperId = (issuer: PartyId, maturity: Civil): InstrumentId =>
  instrumentId(`cp:${issuer}:${formatCivil(maturity)}`);

export const commercialPaper: InstrumentKindProfile = {
  id: COMMERCIAL_PAPER,
  // A2, E2: cleared, and there is no other answer. A discount off an untraded curve is the defect.
  pricing: 'cleared',
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3, XI-3: THE ISSUER OWES THE FACE. Paper falling because the market has doubts about
  // the name is the HOLDER's loss and never the issuer's gain — the same reason a corporate bond
  // says so, and it matters more here: this is the instrument an issuer is supposed to fail on.
  owes: 'face',
  unit: () => PAPER_PAR,
  ranking: (i) => ({
    seniority: isPaper(i.terms) ? i.terms.seniority : 0,
    secured: [],
    claim: 'an unsecured claim on the estate, ranking with the issuer’s other senior debt',
  }),
  /** N12: a payment fell due and the issuer did not make it. At this tenor that is the whole story. */
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'paper matured and the issuer did not pay it' }
      : undefined,
  /**
   * Corporate Credit G2: AND IT IS DUE ON THE OTHERS TOO. An issuer that cannot repay a week of
   * borrowing has not got a week's problem, and its other lenders do not wait their turn while the
   * estate empties. This is the clause that makes B3.b's run reach the rest of the balance sheet.
   */
  accelerates: true,
  validateTerms: (t) => {
    if (!isPaper(t)) throw new InvalidRegistry('Short-Term Debt A1', 'not commercial paper terms');
    validateDates(t.issueDate, t.maturity, 'commercial paper');
  },
  // Law 9: the issuer and the day it is due. There is no coupon to name it by.
  displayName: (i, namer) =>
    isPaper(i.terms)
      ? `${issuerName(namer, i.id)} CP ${formatCivil(i.terms.maturity)}`
      : `${issuerName(namer, i.id)} CP`,
  // N5.c, Law 4: the kernel's one statement of what discount paper promises.
  due: (i, period, cal) => (isPaper(i.terms) ? discountDue(i.terms, period, cal) : []),
  // F2: it accretes against its own cleared price; nothing accrues on the paper, so nothing
  // travels with a trade beyond the price itself.
  accrued: () => 0,
  cashFlows: (i, after) => (isPaper(i.terms) ? discountFlows(i.terms, after) : []),
};

/**
 * A2: THE YIELD, DERIVED FROM THE PRICE AND THE DAYS LEFT — in that direction only (Law 3, E2).
 *
 * It is a read and nothing stores it. A2.a is why the year fraction is an argument rather than a
 * constant: the same price on the same paper is a different yield under two conventions, and the
 * convention is the line's own (`DiscountSchedule.dayCount`).
 *
 * Both degenerate cases THROW rather than answering zero. Paper that is worth nothing has no yield
 * — it has a default — and paper with no time left has no yield either, it has a payment due today;
 * answering 0 for both would put a number where there is an event (Law 2, "missing is Missing").
 */
export function yieldOn(price: PerPiece, over: number, what: string): Ratio {
  if (price <= 0) {
    throw new RangeError(`${what}: paper worth nothing has a default, not a yield`);
  }
  if (over <= 0) {
    throw new RangeError(`${what}: paper with no time left has a payment due, not a yield`);
  }
  return ratioOf(minus(asCash(1, 'par'), asCash(price, what), 'the discount'), asCash(price * over, what), what);
}
