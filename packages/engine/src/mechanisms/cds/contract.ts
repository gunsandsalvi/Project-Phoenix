/**
 * The credit default swap: one side pays a running spread, the other pays par less recovery if a
 * named party defaults.
 *
 * @spec CDS A1 CDS A1.a CDS A1.b CDS A1.c CDS A2 CDS A3 CDS A4 CDS A4.a CDS C2 CDS D1 CDS D2 CDS D2.a CDS D2.b Derivative D1 Derivative D1.b Derivative D3 Derivative D3.a Derivative D4 Derivative D5 Derivative D7 Derivative D7.b Derivative D8 Derivative D11 Derivative D11.a Derivative D12 Derivative Layer D1 Derivative Layer G2 Law 3 Law 19
 *
 * THE UNDERLYING IS AN EVENT this world records (D3, A1.a): the named reference failing to pay,
 * which item 5 writes as `credit.default` from a settlement that failed. It is not a probability
 * and there is no model of one here.
 *
 * THE MARK IS TWO READS AND NO MODEL (A3, C2). Before the event it is the spread this reference's
 * own book cleared against the spread this contract was struck at, over the years it has left —
 * what the protection is now worth against what it is being paid for. After the event it is par
 * less what the reference's own defaulted debt is worth, which is a PRICE this world clears
 * (D2.a forbids a fixed recovery, and this is the read that means there is not one). An implied
 * default probability is what you get by dividing the first by the second, which is why C2 says it
 * is DERIVED: nothing here computes one, and nothing here needs one.
 */
import type { Calendar, Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { derivativeKindId } from '../../core/ids.js';
import { div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { BASIS_POINT, CDS_DAY_COUNT, PROTECTED } from './data.js';
import { cdsOrders } from './participants.js';
import { cdsMeasures } from './measures.js';

export const CDS = derivativeKindId('cds');

export interface CdsTerms extends ContractTerms {
  readonly kind: typeof CDS;
  /** A1.a, A4: the named party whose default this is about. */
  readonly reference: PartyId;
  /**
   * A4, A4.a, D2: the reference's own debt — what defines the event and what prices the recovery.
   * A reference with none is a reference nobody can observe failing and nobody can settle against.
   */
  readonly obligation: InstrumentId;
  /** Where this contract's own tenor prints: the curve point it was struck off (A1.d). */
  readonly book: InstrumentId;
  readonly maturity: Period;
  readonly tenorYears: number;
  /** Which way `a` is. The book writes the buyer as `a` (Derivative Layer B1); `flip` writes false. */
  readonly buysProtection: boolean;
  /** Derivative Layer D1: how many periods of this book's own prints the margin is measured over. */
  readonly window: number;
}

export const isCds = (t: ContractTerms): t is CdsTerms =>
  'reference' in t && 'obligation' in t && 'buysProtection' in t;

/** D1, D2.b: has this reference defaulted, and has its estate finished paying out? */
export function creditState(
  reference: PartyId,
  reads: Pick<ContractReads, 'lastEvent'>,
): { readonly defaulted: boolean; readonly settled: boolean } {
  const failed = reads.lastEvent('credit.default', String(reference));
  if (!failed.some) return { defaulted: false, settled: false };
  const opened = reads.lastEvent('estate.opened', String(reference));
  if (!opened.some) return { defaulted: true, settled: false };
  const estate = opened.value.data['estate'];
  if (typeof estate !== 'string') return { defaulted: true, settled: false };
  return { defaulted: true, settled: reads.lastEvent('estate.closed', estate).some };
}

/** A1.b, D2: par less what a unit of the reference's own defaulted debt is worth. */
function payoff(c: Contract, t: CdsTerms, at: Period, reads: ContractReads): number {
  const recovery = reads.mark(t.obligation, at);
  // Law 19: nobody assumes a recovery. Until the defaulted line is worth something somebody can
  // read, what protection pays is not knowable and the mark says so rather than inventing a rate.
  if (!recovery.some) return 0;
  return mul(sub(1, recovery.value, 'par less recovery'), c.notional, 'over the notional protected');
}

/** The years this contract has left to run, from the calendar rather than a count of periods. */
function yearsLeft(t: CdsTerms, at: Period, calendar: Calendar): number {
  if (at >= t.maturity) return 0;
  return yearFraction(CDS_DAY_COUNT, calendar.startOf(at), calendar.startOf(t.maturity));
}

/** A3, C2: what it is worth to `a`, from two prints and nothing else. */
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isCds(c.terms)) return 0;
  const t = c.terms;
  const state = creditState(t.reference, reads);
  if (state.defaulted) {
    const owed = payoff(c, t, at, reads);
    return t.buysProtection ? owed : -owed;
  }
  const now = reads.print(t.book, at);
  if (!now.some) return 0;
  const richer = sub(now.value.price, c.struckAt, 'the spread now against the spread struck');
  const worth = mul(
    mul(richer, c.notional, 'over the notional'),
    yearsLeft(t, at, reads.calendar),
    'over the years it has left',
  );
  return t.buysProtection ? worth : -worth;
}

export const cdsKind: DerivativeKindProfile = {
  id: CDS,
  unit: PROTECTED,
  priceTick: BASIS_POINT,
  // A1.c, D7.b: what clears is the SPREAD that makes the contract worth nothing at inception.
  quotedAs: 'rate',
  underlying: (c) => ({
    kind: 'event',
    party: isCds(c.terms) ? c.terms.reference : c.a,
    event: 'credit.default',
  }),
  validateTerms: (t) => {
    if (!isCds(t)) throw new Error('not credit default swap terms');
    if (!(t.tenorYears > 0)) throw new Error(`a credit default swap over ${t.tenorYears} years`);
  },
  // Law 9: a market names protection by whose credit it is on and for how long.
  displayName: (c) =>
    isCds(c.terms)
      ? `${c.terms.reference} ${c.terms.tenorYears}y protection`
      : String(c.id),
  mark: markOf,
  // A3: the same contract as the other side wrote it — who is buying protection is what turns over.
  flip: (t) => (isCds(t) ? { ...t, buysProtection: !t.buysProtection } : t),
  /**
   * A2, D4, D5: THE PREMIUM, in cash, and it STOPS ON THE EVENT.
   *
   * A protection buyer pays the spread it struck at on the notional it protects, every period, for
   * as long as the reference is performing. The period after the reference defaults it pays
   * nothing — what it is owed then is the payoff, and a buyer still paying for protection it has
   * already claimed would be paying twice.
   */
  legs: (c, at, reads): readonly ContractPayment[] => {
    if (!isCds(c.terms)) return [];
    const t = c.terms;
    if (at >= t.maturity) return [];
    if (creditState(t.reference, reads).defaulted) return [];
    const from = t.buysProtection ? c.a : c.b;
    const to = t.buysProtection ? c.b : c.a;
    const accrual = yearFraction(CDS_DAY_COUNT, reads.calendar.startOf(at), reads.calendar.endOf(at));
    const amount = mul(mul(c.struckAt, c.notional, 'the spread on the notional'), accrual, 'this period');
    if (amount <= 0) return [];
    return [
      { from, to, ccy: c.ccy, amount, date: reads.calendar.endOf(at), why: 'the protection premium' },
    ];
  },
  // D7.b: struck at par. The cleared spread IS what makes it worth nothing, so nothing is paid now.
  premiumPerUnit: () => 0,
  /**
   * Derivative Layer D1, G2: the spread's OWN measured move over the life the contract has left.
   *
   * A spread that moves by a basis point costs the side it moved against a basis point a year for
   * every year it has left, so that is what the margin is. None when the book has not printed
   * enough to have a record: G2 takes a refusal over a number nobody can stand behind.
   */
  initialMargin: (c, at, reads): Option<number> => {
    if (!isCds(c.terms)) return none();
    /**
     * Derivative Layer D1, G2: THE MOVE OF THIS CREDIT, measured wherever this world has a record
     * of it. A protection book that has not printed yet has no record of its own — but the credit
     * has been trading all along, in the reference's own debt, and what that line has done IS what
     * a position in this name could do. It is the same credit and the same read (Law 19), not a
     * number stood in for a missing one: a name whose debt has not moved either has no honest
     * margin and the layer refuses the trade (G2).
     */
    const own = reads.measuredMove(c.terms.book, c.terms.window);
    const move = own.some ? own : reads.measuredMove(c.terms.obligation, c.terms.window);
    if (!move.some) return none();
    const life = yearsLeft(c.terms, at, reads.calendar);
    return some(
      mul(mul(move.value, c.notional, 'over the notional'), life, 'over the years it has left'),
    );
  },
  // D11.a: the stated close-out value is what it is worth now.
  orders: (view, m) => cdsOrders(view, m),
  measures: (m, reads) => cdsMeasures(m, reads),
  closeOut: markOf,
  /**
   * D11, D2.b: the term runs out — UNLESS the reference has defaulted and its estate has not
   * finished. A contract whose event has fired is held past its maturity until there is a realised
   * recovery to settle at, because a protection buyer whose contract expired the week its
   * reference failed bought nothing.
   */
  expires: (c, at): boolean => {
    if (!isCds(c.terms)) return false;
    return at >= c.terms.maturity;
  },
};

/** D2.b: whether this row is being held open past its maturity waiting on an estate. */
export function heldPastMaturity(c: Contract, at: Period, reads: ContractReads): boolean {
  if (!isCds(c.terms)) return false;
  const state = creditState(c.terms.reference, reads);
  return at >= c.terms.maturity && state.defaulted && !state.settled;
}

/** A reader's read: the implied default probability, DERIVED (C2) and stored nowhere. */
export function impliedDefaultRate(spread: number, recovery: number): Option<number> {
  const loss = sub(1, recovery, 'loss given default');
  return loss > 0 ? some(div(spread, loss, 'the implied default rate')) : none<number>();
}
