/**
 * The credit default index: protection on a stated list of names at stated weights, as one line.
 *
 * @spec CDS A5 CDS A5.a CDS A5.b CDS D2 CDS D2.a Derivative D1 Derivative D1.b Derivative D3 Derivative D7.b Derivative D12 Indices A1 Indices A1.a Indices B1 Law 3 Law 15 Law 19
 *
 * A5: THE NAMES ARE FIXED AT THE ROLL. A series is a list somebody published on a date, and it does
 * not change because a constituent got worse — that is the whole point of a series, and it is why
 * there is a new one every roll rather than a running edit of the old one. What changes within a
 * series is only who is left: a name that defaults SETTLES ITS WEIGHT, once, for every contract on
 * the line, and the line then runs on over the survivors.
 *
 * A5.b: it has its own book, so the index has its own cleared spread — and the difference between
 * that and what its constituents' own books are printing is a READ (the index-against-single-name
 * basis), never a check and never an arbitrage somebody closes for free.
 *
 * THE MARK NEVER LOOKS UP A SETTLED NAME. A name whose weight has been paid is out of the running
 * spread leg from that period on, which is how "the line runs on" is arithmetic rather than an
 * edit: the terms still list it, the payoff already happened, and what is left is the survivors.
 */
import type { Period } from '../../calendar/calendar.js';
import type { DerivativeClassDecl } from '../../world/module.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId } from '../../core/ids.js';
import { mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { creditState } from './contract.js';
import { BASIS_POINT, CDS_DAY_COUNT, PROTECTED } from './data.js';
import { cdsIndexOrders } from './participants.js';

export const CDS_INDEX = derivativeKindId('cds.index');

/** A1.a, A5.a: whose opinion the series was built on, published in advance with the names. */
export interface SeriesName {
  readonly reference: PartyId;
  readonly obligation: InstrumentId;
  /** Indices B1: a weight is a COUNT of a real thing — the face of this name in the series. */
  readonly weight: number;
}

export interface CdsIndexTerms extends ContractTerms {
  readonly kind: typeof CDS_INDEX;
  /** A5: the series — a grade, a roll and a list, all of them fixed when it was published. */
  readonly series: string;
  readonly grade: string;
  readonly names: readonly SeriesName[];
  readonly book: InstrumentId;
  readonly maturity: Period;
  readonly tenorYears: number;
  readonly buysProtection: boolean;
  readonly window: number;
}

export const isCdsIndex = (t: ContractTerms): t is CdsIndexTerms =>
  'series' in t && 'names' in t && 'buysProtection' in t;

export const seriesMarketOf = (series: string, tenorYears: number): MarketId =>
  marketId(`mkt.cds.index.${series}.${tenorYears}y`);

export const seriesLineOf = (series: string, tenorYears: number): InstrumentId =>
  instrumentId(`cds.index:${series}:${tenorYears}y`);

/** The share of the series one name is, as a fraction of what the whole line protects (B1). */
export function weightShare(t: CdsIndexTerms, name: SeriesName): number {
  const whole = sum(t.names.map((n) => n.weight)).value;
  return whole > 0 ? name.weight / whole : 0;
}

/** A5: which names are still running — the ones whose weight has not already been paid out. */
export function survivors(t: CdsIndexTerms, reads: ContractReads): readonly SeriesName[] {
  return t.names.filter((n) => !creditState(n.reference, reads).settled);
}

/** The share of the line that is still running, which is what the spread leg is on. */
export function runningShare(t: CdsIndexTerms, reads: ContractReads): number {
  const whole = sum(t.names.map((n) => n.weight)).value;
  if (whole <= 0) return 0;
  return sum(survivors(t, reads).map((n) => n.weight)).value / whole;
}

function yearsLeft(t: CdsIndexTerms, at: Period, reads: ContractReads): number {
  if (at >= t.maturity) return 0;
  return yearFraction(CDS_DAY_COUNT, reads.calendar.startOf(at), reads.calendar.startOf(t.maturity));
}

/**
 * A3, A5: what it is worth to `a` — the spread the LINE cleared against the spread it was struck
 * at, on the part of the line that is still running, plus what each defaulted name is worth that
 * nobody has settled yet.
 */
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isCdsIndex(c.terms)) return 0;
  const t = c.terms;
  const print = reads.print(t.book, at);
  const running = runningShare(t, reads);
  const spreadLeg = print.some
    ? mul(
        mul(
          sub(print.value.price, c.struckAt, 'the spread now against the spread struck'),
          mul(c.notional, running, 'on the part of the line still running'),
          'over the notional',
        ),
        yearsLeft(t, at, reads),
        'over the years it has left',
      )
    : 0;
  // A name that has defaulted and not yet settled is worth par less what its own debt is worth,
  // on its share of the line — the same read a single-name row makes (D2.a: no fixed recovery).
  const claims: number[] = [];
  const whole = sum(t.names.map((n) => n.weight)).value;
  for (const n of t.names) {
    const state = creditState(n.reference, reads);
    if (!state.defaulted || state.settled) continue;
    const recovery = reads.mark(n.obligation, at);
    if (!recovery.some || whole <= 0) continue;
    claims.push(
      mul(
        sub(1, recovery.value, 'par less recovery'),
        mul(c.notional, n.weight / whole, 'on this name’s share of the line'),
        'what this name owes the line',
      ),
    );
  }
  const worth = sub(spreadLeg, -sum(claims).value, 'the spread leg and the claims on it');
  return t.buysProtection ? worth : -worth;
}

/** D2, E1: why a party is in the series. The index itself is priced by its own book (Law 3). */
export const cdsIndexClass: DerivativeClassDecl = {
  kind: CDS_INDEX,
  orders: (view, m) => cdsIndexOrders(view, m),
};

export const cdsIndexKind: DerivativeKindProfile = {
  id: CDS_INDEX,
  unit: PROTECTED,
  priceTick: BASIS_POINT,
  quotedAs: 'rate',
  // D3: the line settles against the credit events of its constituents. The first name IS the
  // underlying the kernel checks exists; the rest are checked by this module's own family.
  underlying: (c) => ({
    kind: 'event',
    party: isCdsIndex(c.terms) ? (c.terms.names[0]?.reference ?? c.a) : c.a,
    event: 'credit.default',
  }),
  validateTerms: (t) => {
    if (!isCdsIndex(t)) throw new Error('not credit index terms');
    if (t.names.length === 0) throw new Error(`series ${t.series} has no names in it`);
    if (t.names.some((n) => !(n.weight > 0))) throw new Error(`series ${t.series} has a weightless name`);
  },
  displayName: (c) =>
    isCdsIndex(c.terms)
      ? `${c.terms.series} ${c.terms.grade} ${c.terms.tenorYears}y`
      : String(c.id),
  mark: markOf,
  flip: (t) => (isCdsIndex(t) ? { ...t, buysProtection: !t.buysProtection } : t),
  /**
   * A2, A5: the premium runs on the part of the line that is still running. A buyer that has been
   * paid out on a name does not go on paying for protection on it.
   */
  legs: (c, at, reads): readonly ContractPayment[] => {
    if (!isCdsIndex(c.terms)) return [];
    const t = c.terms;
    if (at >= t.maturity) return [];
    const running = runningShare(t, reads);
    if (running <= 0) return [];
    const from = t.buysProtection ? c.a : c.b;
    const to = t.buysProtection ? c.b : c.a;
    const accrual = yearFraction(CDS_DAY_COUNT, reads.calendar.startOf(at), reads.calendar.endOf(at));
    const amount = mul(
      mul(c.struckAt, mul(c.notional, running, 'on what is still running'), 'the spread on it'),
      accrual,
      'this period',
    );
    return amount > 0
      ? [{ from, to, ccy: c.ccy, amount, date: reads.calendar.endOf(at), why: 'the index premium' }]
      : [];
  },
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isCdsIndex(c.terms)) return none();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none();
    return some(
      mul(
        mul(move.value, mul(c.notional, runningShare(c.terms, reads), 'still running'), 'over it'),
        yearsLeft(c.terms, at, reads),
        'over the years it has left',
      ),
    );
  },
  closeOut: markOf,
  expires: (c, at): boolean => isCdsIndex(c.terms) && at >= c.terms.maturity,
};
