/**
 * The interest-rate swap: two legs on one notional, and only the net moves.
 *
 * @spec IRS A1 IRS A1.a IRS A1.b IRS A1.c IRS A1.d IRS A2 IRS A3 IRS A4 IRS C1 IRS C1.a IRS C2 IRS C3 IRS E1 IRS E2 IRS E3 Derivative D1 Derivative D1.b Derivative D3 Derivative D3.a Derivative D4 Derivative D5 Derivative D6 Derivative D6.a Derivative D7 Derivative D7.b Derivative D8 Derivative D11 Derivative D12 Derivative Layer D1 Law 3 Law 19
 *
 * THE UNDERLYING IS THE OVERNIGHT BOOK'S OWN FIXING (A1.a, E3): what banks actually paid each other
 * for money this period, published by the index system as `index.benchmark`. It is a transacted
 * rate, not a posted one, which is what makes a floating leg a read of something that happened.
 *
 * THE FIXED RATE CLEARS (A1.c, D7.b, E2). What the two sides agree is the rate that makes the swap
 * worth nothing today, and it is found the way every other level in this world is found — a session
 * with schedules on both sides of it. E2 forbids the other way round, and the way you can tell this
 * module does not do it is that there is no `parRate()` anywhere in it: nothing here can compute a
 * fixed rate from a discount curve, because nothing here has a discount curve to compute it from.
 *
 * E1: NO NOTIONAL IS EVER EXCHANGED. `legs` returns the NET of the two accruals and nothing else —
 * there is no path through this file that moves the notional, which is what E1 asks and what the
 * test that reads every leg of a year of swaps checks.
 */
import {
  absolute,
  noCash,
  asPerPiece,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  negated,
  scale,
  type PerPiece,
  valueAt,
} from '../../core/measure.js';
import type { Period } from '../../calendar/calendar.js';
import { ratePrint } from '../../prices/price-store.js';
import type { DerivativeClassDecl } from '../../world/module.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId } from '../../core/ids.js';
import { derivativeKindId, unitId } from '../../core/ids.js';

import { none, some, type Option } from '../../core/option.js';
import { FACE_TICK } from '../../registry/grid.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { rateLevel } from '../../registry/derivatives.js';
import { irsOrders } from './participants.js';
import { irsMeasures } from './measures.js';

export const IRS = derivativeKindId('irs');
/** D2: the notional is an amount of money the two legs accrue on, and never changes hands (E1). */
export const NOTIONAL = unitId('notional');
export const IRS_DAY_COUNT = 'ACT/360';

export interface IrsTerms extends ContractTerms {
  readonly kind: typeof IRS;
  readonly ccy: CurrencyCode;
  /** A1.a, E3: which overnight book the floating leg fixes on — the one that transacted. */
  readonly benchmark: string;
  /** Where this tenor's fixed rate prints (C1: the swap curve is the set of them). */
  readonly book: InstrumentId;
  readonly maturity: Period;
  readonly tenorYears: number;
  /** A2: each leg has its own periodicity, in periods, and they need not match. */
  readonly fixedEvery: number;
  readonly floatEvery: number;
  /** Which way `a` is: paying fixed and receiving floating, or the other way. */
  readonly paysFixed: boolean;
  readonly window: number;
}

export const isIrs = (t: ContractTerms): t is IrsTerms =>
  'benchmark' in t && 'paysFixed' in t && 'fixedEvery' in t;

/**
 * A3, E3: THE FLOATING FIXING — what the overnight book itself last printed. A read of what
 * happened, per annum, and none at all when the book did not trade (D3.a: a benchmark this world
 * did not produce is not one this contract may use).
 */
export function floatingRate(
  t: IrsTerms,
  reads: Pick<ContractReads, 'lastEvent'>,
): Option<PerPiece> {
  const e = reads.lastEvent('index.benchmark', t.benchmark);
  if (!e.some) return none<PerPiece>();
  const rate = e.value.data['rate'];
  // Item 16: the fixing re-enters here. A swap's LEVEL is a rate on a unit of notional, which is
  // the same dimension its struck rate is in — which is why the two legs can be netted at all.
  return typeof rate === 'number'
    ? some(asPerPiece(rate, `the fixing of ${t.benchmark}`))
    : none<PerPiece>();
}

function yearsLeft(t: IrsTerms, at: Period, reads: ContractReads): number {
  if (at >= t.maturity) return 0;
  return yearFraction(
    IRS_DAY_COUNT,
    reads.calendar.startOf(at),
    reads.calendar.startOf(t.maturity),
  );
}

/**
 * A3, D8: what it is worth to `a` — the fixed rate the curve is now clearing at against the one
 * this contract pays, over what it has left. A payer of fixed gains when rates rise.
 */
function markOf(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isIrs(c.terms)) return noCash(c.ccy);
  const t = c.terms;
  const now = reads.print(t.book, at);
  if (!now.some) return noCash(c.ccy);
  // Law 8, E-10, E-11 (18.0): a swap book clears a RATE, and both sides of this subtraction now SAY
  // so — `rateLevel` about this contract's own level and `ratePrint` about the print. A print used
  // to be a `PerPiece` on every book in this world whatever it had cleared, and the only thing
  // between two per cent and two cents was this comment. A rate over a notional is money per year.
  const richer = minus(
    ratePrint(now.value, 'the rate this book last printed'),
    rateLevel(c.struckAt, 'an interest-rate swap is struck at a rate'),
    'the rate now against the rate struck',
  );
  const worth = scale(
    scale(heldAsMoney(c.notional, c.ccy, 'the notional it is on'), richer, 'over the notional'),
    asRatio(yearsLeft(t, at, reads), 'the years it has left'),
    'over the years it has left',
  );
  return t.paysFixed ? worth : negated(worth, 'and the other side of it');
}

/** B2, C3: why a party swaps, and what the cleared fixed rate says against the sovereign's own. */
export const irsClass: DerivativeClassDecl = {
  kind: IRS,
  orders: (view, m) => irsOrders(view, m),
  measures: (m, reads) => irsMeasures(m, reads),
};

export const irsKind: DerivativeKindProfile = {
  id: IRS,
  unit: NOTIONAL,
  priceTick: FACE_TICK,
  // A1.c, D7.b: what clears is the FIXED RATE that makes the swap worth nothing at inception.
  quotedAs: 'rate',
  // D3, A1.a: the overnight book's own fixing — an event this world records every period it trades.
  underlying: (c) => ({
    kind: 'event',
    party: c.a,
    event: 'index.benchmark',
  }),
  validateTerms: (t) => {
    if (!isIrs(t)) throw new Error('not interest-rate swap terms');
    if (!(t.tenorYears > 0)) throw new Error(`a swap over ${t.tenorYears} years`);
    if (!(t.fixedEvery > 0) || !(t.floatEvery > 0)) throw new Error('a leg that never pays');
  },
  displayName: (c) =>
    isIrs(c.terms) ? `${c.terms.ccy} ${c.terms.tenorYears}y swap` : String(c.id),
  mark: markOf,
  // A3: the same contract as the other side wrote it — which leg it pays is what turns over.
  flip: (t) => (isIrs(t) ? { ...t, paysFixed: !t.paysFixed } : t),
  /**
   * A1.b, A2, A4, E1: ONLY THE NET MOVES, and the notional never does.
   *
   * Each leg accrues on its own periodicity and its own convention (A2, D6.a), and on a date both
   * are due the difference is one payment in one direction. A swap that paid both legs gross would
   * be two payments that mostly cancel, which is not what either side agreed to and is not what
   * either side's cash actually does (A4).
   */
  legs: (c, at, reads): readonly ContractPayment[] => {
    if (!isIrs(c.terms)) return [];
    const t = c.terms;
    if (at >= t.maturity) return [];
    const fixedDue = (at - c.opened) % t.fixedEvery === 0;
    const floatDue = (at - c.opened) % t.floatEvery === 0;
    if (!fixedDue && !floatDue) return [];
    const float = floatingRate(t, reads);
    // D3.a: a period the overnight book did not trade has no fixing, so the floating leg has
    // nothing to accrue on. Nothing is carried forward and nothing is assumed.
    if (!float.some) return [];
    const accrualFor = (every: number): number =>
      yearFraction(
        IRS_DAY_COUNT,
        reads.calendar.startOf((at - every + 1) as Period),
        reads.calendar.endOf(at),
      );
    const fixedLeg = fixedDue
      ? scale(
          scale(
            heldAsMoney(c.notional, c.ccy, 'the notional it is on'),
            rateLevel(c.struckAt, 'an interest-rate swap is struck at a rate'),
            'the fixed rate on the notional',
          ),
          asRatio(accrualFor(t.fixedEvery), 'this period of a year'),
          'accrued',
        )
      : noCash(c.ccy);
    const floatLeg = floatDue
      ? scale(
          scale(
            heldAsMoney(c.notional, c.ccy, 'the notional it is on'),
            asRatio(float.value, 'the fixing this period'),
            'the fixing on the notional',
          ),
          asRatio(accrualFor(t.floatEvery), 'this period of a year'),
          'accrued',
        )
      : noCash(c.ccy);
    const net = minus(floatLeg, fixedLeg, 'what the floating leg owes over the fixed');
    if (net.pieces === 0) return [];
    // A1.b: one payment, in the direction the net went. The payer of fixed receives when the
    // floating leg came out higher.
    const gainsOnFloat = t.paysFixed ? c.a : c.b;
    const other = t.paysFixed ? c.b : c.a;
    const from = net.pieces > 0 ? other : gainsOnFloat;
    const to = net.pieces > 0 ? gainsOnFloat : other;
    return [
      {
        from,
        to,
        ccy: c.ccy,
        amount: absolute(net, 'the net of the two legs'),
        date: reads.calendar.endOf(at),
        why: 'the net of the two legs',
      },
    ];
  },
  // D7.b: struck at par. The cleared fixed rate IS what makes it worth nothing today.
  premiumPerUnit: (): PerPiece => asPerPiece(0, 'struck at par, so nothing changes hands'),
  initialMargin: (c, at, reads): Option<Cash> => {
    if (!isIrs(c.terms)) return none<Cash>();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none<Cash>();
    return some(
      scale(
        valueAt(move.value, c.notional, c.ccy, 'over the notional'),
        asRatio(yearsLeft(c.terms, at, reads), 'the years it has left'),
        'over the years it has left',
      ),
    );
  },
  closeOut: markOf,
  expires: (c, at): boolean => isIrs(c.terms) && at >= c.terms.maturity,
};
