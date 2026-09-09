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
  PHX,
  assemble,
  civil,
  foundationSpec,
  partyId,
  sovereignBill,
  sovereignBond,
  yearFraction,
  type Order,
  type SystemModule,
  type World,
} from '../src/index.js';

const FIRM_1 = partyId('firm.1');
const QTY = 50;
const CLEAN = 0.98;

/** bank.a offers, firm.1 bids, both at one level, in one stated period. */
function oneTrade(at: number): SystemModule {
  const side = (view: { self: { id: string }; period: number }): readonly Order[] => {
    if (view.period !== at) return [];
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
    units: [],
    params: [],
    phases: [],
    participants: [
      { partyKind: FIRM, orders: (v) => side(v) },
      { partyKind: BANK, orders: (v) => side(v) },
    ],
    families: [],
  };
}

function world(at: number): World {
  const spec = foundationSpec('seed-accrued');
  return assemble({ ...spec, modules: [...spec.modules, oneTrade(at)] });
}

/** Step until the world is in `to`; the world starts sealed at period zero. */
function stepTo(w: World, to: number): void {
  while (w.period < to - 1) w.step();
}

describe('what has accrued (Bond N9.b)', () => {
  it('is the coupon rate over the fraction of the period since the last payment, on the line own day count', () => {
    const w = world(-1);
    const i = w.instruments.get(GOV_LINE);
    // The line was issued 2026-03-15 and pays semi-annually; nothing accrues before it exists.
    expect(sovereignBond.accrued(i, civil(2026, 3, 1), w.calendar)).toBe(0);
    expect(sovereignBond.accrued(i, civil(2026, 3, 15), w.calendar)).toBe(0);
    expect(sovereignBond.accrued(i, civil(2026, 6, 15), w.calendar)).toBeCloseTo(
      0.02 * yearFraction('ACT/ACT', civil(2026, 3, 15), civil(2026, 6, 15)),
      15,
    );
    // Just after a coupon it starts again from nothing.
    expect(sovereignBond.accrued(i, civil(2026, 9, 15), w.calendar)).toBe(0);
    expect(sovereignBond.accrued(i, civil(2026, 9, 22), w.calendar)).toBeCloseTo(
      0.02 * yearFraction('ACT/ACT', civil(2026, 9, 15), civil(2026, 9, 22)),
      15,
    );
  });

  it('is nothing on a bill, which accretes against its own price instead (Sovereign F2)', () => {
    const w = world(-1);
    const i = w.instruments.get(GOV_LINE);
    expect(sovereignBill.accrued(i, civil(2026, 6, 15), w.calendar)).toBe(0);
  });
});

describe('a trade in the middle of a coupon period', () => {
  it('settles dirty, holds the basis clean, and leaves the audit green', () => {
    const at = 30;
    const w = world(at);
    stepTo(w, at);
    const buyerCashBefore = w.cash(FIRM_1, PHX);
    const report = w.step();
    expect(w.period).toBe(at);
    const acc = w.accruedPerUnit(GOV_LINE, w.period);
    expect(acc).toBeGreaterThan(0);
    expect(report.audit.total).toBe(0);
    // The cash that moved is the dirty amount: clean plus what accrued.
    expect(buyerCashBefore - w.cash(FIRM_1, PHX)).toBeCloseTo(QTY * (CLEAN + acc), 9);
    const moneyLegs = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'trade')
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'money');
    expect(moneyLegs).toHaveLength(1);
    expect(moneyLegs[0]?.kind === 'money' && moneyLegs[0].amount).toBeCloseTo(
      QTY * (CLEAN + acc),
      9,
    );
    // The seller sold at its own carrying value, so what it earned on the trade is the accrued
    // interest and nothing else: the coupon it will not now collect, paid to it in cash.
    const sellerEffect = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'trade')
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === BANK_A);
    expect(sellerEffect[0]?.delta).toBeCloseTo(QTY * acc, 9);
    // The lot the buyer holds carries the clean price, so its mark is the print and not the print
    // plus somebody else's interest.
    const h = w.register.holding(FIRM_1, GOV_LINE);
    expect(h.some && h.value.lots.some((l) => l.basisPerUnit === CLEAN)).toBe(true);
    // The ledger records what travelled, so nobody has to re-derive it (Law 19).
    const legs = w.ledger
      .inPeriod(w.period)
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'asset' && l.instrument === GOV_LINE);
    expect(legs.some((l) => l.kind === 'asset' && l.accruedPerUnit.some)).toBe(true);
  });

  it('makes the next coupon no windfall: the buyer keeps only what accrued while it held', () => {
    // The line pays on 2026-09-15. Trade the period before that date, then step through it.
    const at = 35;
    const w = world(at);
    stepTo(w, at);
    w.step();
    expect(w.period).toBe(at);
    const accruedAtTrade = w.accruedPerUnit(GOV_LINE, w.period);
    const effectsAtTrade = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'trade')
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === FIRM_1);
    expect(effectsAtTrade).toHaveLength(1);
    expect(effectsAtTrade[0]?.delta).toBeCloseTo(-QTY * accruedAtTrade, 9);
    const couponReport = w.step();
    expect(couponReport.audit.total).toBe(0);
    const coupons = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'coupon')
      .flatMap((r) => (r.outcome === 'settled' ? r.equity : []))
      .filter((e) => e.party === FIRM_1);
    const couponPerUnit =
      0.02 * yearFraction('ACT/ACT', civil(2026, 3, 15), civil(2026, 9, 15));
    expect(coupons[0]?.delta).toBeCloseTo(QTY * couponPerUnit, 9);
    // What it kept over the two events is the interest of the days it actually owned the paper.
    const kept = QTY * (couponPerUnit - accruedAtTrade);
    expect((coupons[0]?.delta ?? 0) + (effectsAtTrade[0]?.delta ?? 0)).toBeCloseTo(kept, 9);
    expect(kept).toBeGreaterThan(0);
    expect(kept).toBeLessThan(QTY * couponPerUnit);
  });
});
