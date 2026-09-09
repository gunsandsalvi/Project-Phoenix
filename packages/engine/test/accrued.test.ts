/**
 * Accrued interest travels with the paper (Bond N9.b): the buyer pays the seller what accrued since
 * the last coupon, so the coupon that arrives next is not a windfall to whoever holds it on the day.
 *
 * @spec Bond N9.b Sovereign F1 Sovereign F2 Register E1.a Audit B5
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  BANK_A,
  FIRM,
  GOV_LINE,
  MONTHLY,
  PHX,
  assemble,
  civil,
  foundationSpec,
  partyId,
  sovereignBill,
  sovereignBond,
  yearFraction,
  type SovereignBondTerms,
  type Order,
  type SystemModule,
  type World,
} from '../src/index.js';

const FIRM_1 = partyId('firm.1');
const QTY = 50;
const CLEAN = 0.98;

/** bank.a offers, firm.1 bids, both at one level, in one stated period. */
function oneTrade(at: number): SystemModule {
  const side = (
    view: { self: { id: string }; period: number },
    m: { instrument: string },
  ): readonly Order[] => {
    if (view.period !== at || m.instrument !== GOV_LINE) return [];
    if (view.self.id === FIRM_1)
      return [{ party: FIRM_1, side: 'buy', price: CLEAN, qty: QTY }];
    if (view.self.id === BANK_A) return [{ party: BANK_A, side: 'sell', price: CLEAN, qty: QTY }];
    return [];
  };
  return {
    id: 'test.onetrade',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [
      { partyKind: FIRM, orders: (v, m) => side(v, m) },
      { partyKind: BANK, orders: (v, m) => side(v, m) },
    ],
    families: [],
  };
}

/**
 * The kernel and the opening state with one trade in it and nothing else: what accrued interest
 * does is a property of the wire, so the test drives the wire and no mechanism drives the test.
 */
function world(at: number): World {
  const spec = foundationSpec('seed-accrued');
  const kernelOnly = spec.modules.filter(
    (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation' || m.id === 'bank-lending',
  );
  return assemble({ ...spec, modules: [...kernelOnly, oneTrade(at)] });
}

/** The settled trades in one line this period: what this test is about, and nothing else. */
function tradesOf(w: World, instrument: string): ReturnType<World['ledger']['inPeriod']> {
  return w.ledger
    .inPeriod(w.period)
    .filter(
      (r) =>
        r.outcome === 'settled' &&
        r.instruction.cause === 'trade' &&
        r.instruction.legs.some((l) => l.kind === 'asset' && l.instrument === instrument),
    );
}

/** Step until the world is in `to`; the world starts sealed at period zero. */
function stepTo(w: World, to: number): void {
  while (w.period < to - 1) w.step();
}

describe('what has accrued (Bond N9.b)', () => {
  it('is the coupon rate over the fraction of the period since the last payment, on the line own day count', () => {
    const w = world(-1);
    const i = w.instruments.get(GOV_LINE);
    const terms = i.terms as SovereignBondTerms;
    const issue = terms.issueDate;
    const firstCoupon = w.calendar.advance(issue, terms.couponPeriodicity);
    // Nothing accrues before the line exists, or on its issue date.
    expect(sovereignBond.accrued(i, issue, w.calendar)).toBe(0);
    const midway = w.calendar.advance(issue, MONTHLY);
    expect(sovereignBond.accrued(i, midway, w.calendar)).toBeCloseTo(
      terms.coupon.amount * yearFraction(terms.dayCount, issue, midway),
      15,
    );
    // Just after a coupon it starts again from nothing.
    expect(sovereignBond.accrued(i, firstCoupon, w.calendar)).toBe(0);
    const after = w.calendar.advance(firstCoupon, MONTHLY);
    expect(sovereignBond.accrued(i, after, w.calendar)).toBeCloseTo(
      terms.coupon.amount * yearFraction(terms.dayCount, firstCoupon, after),
      15,
    );
  });

  it('is nothing on a bill, which accretes against its own price instead (Sovereign F2)', () => {
    const w = world(-1);
    const bill = w.instruments.all().find((x) => x.kind === 'sovereign.bill');
    if (bill === undefined) throw new Error('no bill in the opening profile');
    expect(sovereignBill.accrued(bill, civil(2026, 6, 1), w.calendar)).toBe(0);
    expect(w.registry.instrumentKind(bill.kind).accrued(bill, civil(2026, 6, 1), w.calendar)).toBe(
      0,
    );
  });
});

describe('a trade in the middle of a coupon period', () => {
  it('settles dirty, holds the basis clean, and leaves the audit green', () => {
    const at = 20;
    const w = world(at);
    stepTo(w, at);
    const buyerCashBefore = w.cash(FIRM_1, PHX);
    const carrying = w.prices.latest(GOV_LINE, w.period);
    if (!carrying.some) throw new Error('no mark before the trade');
    const report = w.step();
    expect(w.period).toBe(at);
    const acc = w.accruedPerUnit(GOV_LINE, w.period);
    expect(acc).toBeGreaterThan(0);
    expect(report.audit.total).toBe(0);
    // The cash that moved is the dirty amount: clean plus what accrued.
    expect(buyerCashBefore - w.cash(FIRM_1, PHX)).toBeCloseTo(QTY * (CLEAN + acc), 9);
    const trades = tradesOf(w, GOV_LINE);
    expect(trades).toHaveLength(1);
    const money = trades[0]?.instruction.legs.filter((l) => l.kind === 'money');
    expect(money?.[0]?.kind === 'money' && money[0].amount).toBeCloseTo(QTY * (CLEAN + acc), 9);
    // What the seller earned is two separable things: what the paper did against its mark, and the
    // interest it had earned and is paid for in cash (N9.b).
    const sellerEffect = trades
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === BANK_A);
    expect(sellerEffect[0]?.delta).toBeCloseTo(QTY * (CLEAN - carrying.value.price) + QTY * acc, 9);
    // The lot the buyer holds carries the clean price, so its mark is the print and not the print
    // plus somebody else's interest.
    const h = w.register.holding(FIRM_1, GOV_LINE);
    expect(h.some && h.value.lots.some((l) => l.basisPerUnit === CLEAN)).toBe(true);
    // The ledger records what travelled, so nobody has to re-derive it (Law 19).
    const legs = trades
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'asset' && l.instrument === GOV_LINE);
    expect(legs.some((l) => l.kind === 'asset' && l.accruedPerUnit.some)).toBe(true);
  });

  it('makes the next coupon no windfall: the buyer keeps only what accrued while it held', () => {
    // Trade the period before the line's first coupon date, then step through that date.
    const at = 24;
    const w = world(at);
    stepTo(w, at);
    w.step();
    expect(w.period).toBe(at);
    const accruedAtTrade = w.accruedPerUnit(GOV_LINE, w.period);
    const effectsAtTrade = tradesOf(w, GOV_LINE)
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === FIRM_1);
    expect(effectsAtTrade).toHaveLength(1);
    expect(effectsAtTrade[0]?.delta).toBeCloseTo(-QTY * accruedAtTrade, 9);
    const couponReport = w.step();
    expect(couponReport.audit.total).toBe(0);
    const coupons = w.ledger
      .inPeriod(w.period)
      .filter(
        (r) =>
          r.outcome === 'settled' &&
          r.instruction.cause === 'coupon' &&
          r.instruction.reason.includes(GOV_LINE),
      )
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === FIRM_1);
    const terms = w.instruments.get(GOV_LINE).terms as SovereignBondTerms;
    const couponPerUnit =
      terms.coupon.amount *
      yearFraction(
        terms.dayCount,
        terms.issueDate,
        w.calendar.advance(terms.issueDate, terms.couponPeriodicity),
      );
    expect(coupons[0]?.delta).toBeCloseTo(QTY * couponPerUnit, 9);
    // What it kept over the two events is the interest of the days it actually owned the paper.
    const kept = QTY * (couponPerUnit - accruedAtTrade);
    expect((coupons[0]?.delta ?? 0) + (effectsAtTrade[0]?.delta ?? 0)).toBeCloseTo(kept, 9);
    expect(kept).toBeGreaterThan(0);
    expect(kept).toBeLessThan(QTY * couponPerUnit);
  });
});
