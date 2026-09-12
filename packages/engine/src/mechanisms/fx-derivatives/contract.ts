/**
 * The FX forward and the cross-currency swap: two moneys, two dates, and a rate for each.
 *
 * @spec FX Forwards A1 FX Forwards A1.a FX Forwards A1.b FX Forwards A1.c FX Forwards A1.d FX Forwards A2 FX Forwards A3 FX Forwards C1 FX Forwards C1.a FX Forwards C2 FX Forwards C3 FX Forwards C4 FX Forwards E1 FX Forwards E3 Derivative D1 Derivative D1.b Derivative D3 Derivative D4 Derivative D5 Derivative D7 Derivative D8 Derivative D11 Derivative D12 Derivative Layer D1 Spot FX A1 XI-5 XI-12 Law 3
 *
 * A1.d, D5: A LEG STATES ITS OWN MONEY, and the two legs of a forward are in two different ones —
 * which is the whole point, and why a forward is not a bet on a number: at maturity both notionals
 * MOVE, one each way, in one instruction (A2, E3, XI-5: delivery against payment across moneys).
 *
 * A3: THE MARK IS AGAINST THE FORWARD FOR THE TENOR LEFT, not against spot. A forward struck at
 * the market's own level is worth nothing the day it is struck and earns the carry over its life;
 * marking it against spot would call that carry a profit on day one and take it back at maturity.
 *
 * C1, C3: the cross-currency swap exchanges the notionals at the start AND at the end AT THE
 * ORIGINAL RATE, and pays interest on both legs in between. What its price is, is the basis on one
 * of those legs (C4) — which is a level that clears, not a number backed out of a parity formula.
 */
import type { Period } from '../../calendar/calendar.js';
import type { DerivativeClassDecl } from '../../world/module.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId } from '../../core/ids.js';
import { derivativeKindId, unitId } from '../../core/ids.js';
import { mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { FACE_TICK, PIP } from '../../registry/grid.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { fxForwardOrders, xccyOrders } from './participants.js';

export const FX_FORWARD = derivativeKindId('fx.forward');
export const XCCY = derivativeKindId('xccy');
/** D2: the notional is an amount of the BASE money, which is what both legs are sized from. */
export const BASE_MONEY = unitId('baseMoney');
export const FX_DAY_COUNT = 'ACT/360';

export interface FxForwardTerms extends ContractTerms {
  readonly kind: typeof FX_FORWARD;
  /** A1.d: the two moneys. The price is what one unit of `base` costs in `quote`. */
  readonly base: CurrencyCode;
  readonly quote: CurrencyCode;
  /** Where this pair and tenor prints (C1: the forward curve is the set of them). */
  readonly book: InstrumentId;
  /** The spot pair this settles against (D3: a print this world clears). */
  readonly spot: InstrumentId;
  readonly maturity: Period;
  readonly tenorYears: number;
  /** Which way `a` is: taking the base at maturity, or giving it. */
  readonly buysBase: boolean;
  readonly window: number;
}

export interface XccyTerms extends ContractTerms {
  readonly kind: typeof XCCY;
  readonly base: CurrencyCode;
  readonly quote: CurrencyCode;
  readonly book: InstrumentId;
  readonly spot: InstrumentId;
  /** C1, C3: the rate the notionals were exchanged at, and are exchanged back at. */
  readonly exchangedAt: number;
  readonly started: Period;
  readonly maturity: Period;
  readonly tenorYears: number;
  /** Which way `a` is: it took the base and pays interest on it. */
  readonly paysBase: boolean;
  /** C1.a: what each leg's interest is, per annum, on its own money. */
  readonly baseRate: number;
  readonly quoteRate: number;
  readonly payEvery: number;
  readonly window: number;
}

export const isFxForward = (t: ContractTerms): t is FxForwardTerms =>
  'buysBase' in t && 'base' in t && 'quote' in t;

export const isXccy = (t: ContractTerms): t is XccyTerms =>
  'exchangedAt' in t && 'paysBase' in t && 'baseRate' in t;

function yearsLeft(maturity: Period, at: Period, reads: ContractReads): number {
  if (at >= maturity) return 0;
  return yearFraction(FX_DAY_COUNT, reads.calendar.startOf(at), reads.calendar.startOf(maturity));
}

/** A3, D8: what a forward is worth to `a` — the forward now for the tenor it has left, against it. */
function forwardMark(c: Contract, at: Period, reads: ContractReads): number {
  if (!isFxForward(c.terms)) return 0;
  const now = reads.print(c.terms.book, at);
  if (!now.some) return 0;
  const better = sub(now.value.price, c.struckAt, 'the forward now against the one struck');
  const worth = mul(better, c.notional, 'over the base it takes');
  return c.terms.buysBase ? worth : -worth;
}

/** B1: why a party buys a money forward — an exposure it has, never a view it was given. */
export const fxForwardClass: DerivativeClassDecl = {
  kind: FX_FORWARD,
  orders: (view, m) => fxForwardOrders(view, m),
};

export const fxForwardKind: DerivativeKindProfile = {
  id: FX_FORWARD,
  unit: BASE_MONEY,
  // Law 8: a forward on a pair is quoted to the pip, like the spot pair it settles against.
  priceTick: PIP,
  underlying: (c) => ({
    kind: 'print',
    market: '' as never,
    instrument: isFxForward(c.terms) ? c.terms.spot : ('' as InstrumentId),
  }),
  validateTerms: (t) => {
    if (!isFxForward(t)) throw new Error('not FX forward terms');
    if (t.base === t.quote) throw new Error(`a forward of ${t.base} against itself`);
  },
  displayName: (c) =>
    isFxForward(c.terms)
      ? `${c.terms.base}/${c.terms.quote} ${c.terms.tenorYears}y forward`
      : String(c.id),
  mark: forwardMark,
  flip: (t) => (isFxForward(t) ? { ...t, buysBase: !t.buysBase } : t),
  /**
   * A1.b, A2, E3, XI-5: AT MATURITY BOTH NOTIONALS MOVE. Two legs in two moneys, in the period the
   * term runs out, and the layer settles them in one instruction — so either both sides deliver or
   * neither does, which is what delivery-versus-payment across two moneys means.
   */
  legs: (c, at, reads): readonly ContractPayment[] => {
    if (!isFxForward(c.terms) || at !== c.terms.maturity) return [];
    const t = c.terms;
    const taker = t.buysBase ? c.a : c.b;
    const giver = t.buysBase ? c.b : c.a;
    const date = reads.calendar.endOf(at);
    return [
      { from: giver, to: taker, ccy: t.base, amount: c.notional, date, why: 'the base, delivered' },
      {
        from: taker,
        to: giver,
        ccy: t.quote,
        amount: mul(c.notional, c.struckAt, 'the quote, at the rate they struck'),
        date,
        why: 'the quote, paid',
      },
    ];
  },
  // D7.b: struck at the market's own forward, so it is worth nothing at inception.
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isFxForward(c.terms)) return none();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none();
    return some(mul(move.value, c.notional, 'over the base it takes'));
  },
  closeOut: forwardMark,
  expires: (c, at): boolean => isFxForward(c.terms) && at >= c.terms.maturity,
};

/** C4, D8: what a cross-currency swap is worth to `a` — the basis now against the basis struck. */
function xccyMark(c: Contract, at: Period, reads: ContractReads): number {
  if (!isXccy(c.terms)) return 0;
  const now = reads.print(c.terms.book, at);
  if (!now.some) return 0;
  const better = sub(now.value.price, c.struckAt, 'the basis now against the basis struck');
  const worth = mul(
    mul(better, c.notional, 'over the notional'),
    yearsLeft(c.terms.maturity, at, reads),
    'over the years it has left',
  );
  return c.terms.paysBase ? -worth : worth;
}

/** D1: why a party swaps one money's funding for another's. */
export const xccyClass: DerivativeClassDecl = {
  kind: XCCY,
  orders: (view, m) => xccyOrders(view, m),
};

export const xccyKind: DerivativeKindProfile = {
  id: XCCY,
  unit: BASE_MONEY,
  priceTick: FACE_TICK,
  // C4: what clears is the BASIS on one leg, per annum — a rate, not a price of anything.
  quotedAs: 'rate',
  underlying: (c) => ({
    kind: 'print',
    market: '' as never,
    instrument: isXccy(c.terms) ? c.terms.spot : ('' as InstrumentId),
  }),
  validateTerms: (t) => {
    if (!isXccy(t)) throw new Error('not cross-currency swap terms');
    if (t.base === t.quote) throw new Error(`a cross-currency swap of ${t.base} against itself`);
    if (!(t.exchangedAt > 0)) throw new Error('a notional exchange at no rate at all');
  },
  displayName: (c) =>
    isXccy(c.terms) ? `${c.terms.base}/${c.terms.quote} ${c.terms.tenorYears}y basis` : String(c.id),
  mark: xccyMark,
  flip: (t) => (isXccy(t) ? { ...t, paysBase: !t.paysBase } : t),
  /**
   * C1, C1.a, C3: THE NOTIONALS GO BOTH WAYS AND COME BACK AT THE ORIGINAL RATE, with interest on
   * each leg in its own money in between (C1.a). C3 is what makes this different from a pair of
   * loans: the rate the notionals come back at is the one they went out at, so the FX risk on the
   * principal is gone and what is left is the two interest streams and the basis between them.
   */
  legs: (c, at, reads): readonly ContractPayment[] => {
    if (!isXccy(c.terms)) return [];
    const t = c.terms;
    const takerOfBase = t.paysBase ? c.a : c.b;
    const takerOfQuote = t.paysBase ? c.b : c.a;
    const quoteNotional = mul(c.notional, t.exchangedAt, 'the quote side at the rate agreed');
    const date = reads.calendar.endOf(at);
    if (at === t.started) {
      return [
        { from: takerOfQuote, to: takerOfBase, ccy: t.base, amount: c.notional, date, why: 'the base, exchanged' },
        { from: takerOfBase, to: takerOfQuote, ccy: t.quote, amount: quoteNotional, date, why: 'the quote, exchanged' },
      ];
    }
    if (at === t.maturity) {
      // C3: BACK AT THE ORIGINAL RATE. Not at spot — that is the whole instrument.
      return [
        { from: takerOfBase, to: takerOfQuote, ccy: t.base, amount: c.notional, date, why: 'the base, returned' },
        { from: takerOfQuote, to: takerOfBase, ccy: t.quote, amount: quoteNotional, date, why: 'the quote, returned' },
      ];
    }
    if ((at - t.started) % t.payEvery !== 0) return [];
    const accrual = yearFraction(
      FX_DAY_COUNT,
      reads.calendar.startOf((at - t.payEvery + 1) as Period),
      reads.calendar.endOf(at),
    );
    const out: ContractPayment[] = [];
    // C1.a: each leg pays on its own money, at its own rate, including the basis on one of them.
    const baseInterest = mul(mul(c.notional, t.baseRate, 'the base rate on it'), accrual, 'accrued');
    const quoteInterest = mul(
      mul(quoteNotional, sub(t.quoteRate, -c.struckAt, 'the quote rate plus the basis'), 'on it'),
      accrual,
      'accrued',
    );
    if (baseInterest > 0) {
      out.push({ from: takerOfBase, to: takerOfQuote, ccy: t.base, amount: baseInterest, date, why: 'interest on the base leg' });
    }
    if (quoteInterest > 0) {
      out.push({ from: takerOfQuote, to: takerOfBase, ccy: t.quote, amount: quoteInterest, date, why: 'interest on the quote leg' });
    }
    return out;
  },
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isXccy(c.terms)) return none();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none();
    return some(
      mul(
        mul(move.value, c.notional, 'over the notional'),
        yearsLeft(c.terms.maturity, at, reads),
        'over the years it has left',
      ),
    );
  },
  closeOut: xccyMark,
  expires: (c, at): boolean => isXccy(c.terms) && at >= c.terms.maturity,
};
