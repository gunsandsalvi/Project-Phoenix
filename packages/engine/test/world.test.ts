import { describe, expect, it } from 'vitest';
import {
  Forbidden,
  NotYetProduced,
  PHX,
  cellSide,
  foundationWorld,
  instrumentId,
  mergeCells,
  moneyInstrumentId,
  none,
  partyId,
  period,
  snapshot,
  some,
  splitCell,
  sum,
  totalFor,
  weightEvent,
  type InstructionDraft,
  type Order,
} from '../src/index.js';

const GOV = instrumentId('gov.north.2.0.2036-03-15');

function violations(w: ReturnType<typeof foundationWorld>): string[] {
  const r = w.last?.audit;
  if (r === undefined) return [];
  return r.families.flatMap((f) => f.violations.map((v) => `${f.family}: ${v.message}`));
}

describe('the seed (Seed A2)', () => {
  it('passes the audit at period zero, with every family built or saying it is not', () => {
    const w = foundationWorld('seed-A');
    const report = w.auditNow();
    expect(report.total).toBe(0);
    expect(report.families.map((f) => f.family)).toHaveLength(9);
    expect(report.families.filter((f) => !f.built).map((f) => f.family)).toEqual([
      'crossMarket',
      'zeroSum',
    ]);
    expect(report.reads.placeholders).toBe(1);
  });

  it('is reproducible from the seed value (Seed A5, Audit D3)', () => {
    const a = foundationWorld('seed-B');
    const b = foundationWorld('seed-B');
    a.auditNow();
    b.auditNow();
    for (let i = 0; i < 30; i += 1) {
      a.step();
      b.step();
    }
    expect(snapshot(a, { kind: 'inspector' }, 10)).toEqual(snapshot(b, { kind: 'inspector' }, 10));
  });

  it("holds nothing without an issuer: the money stock is a read of issuers' liabilities (Money A4)", () => {
    const w = foundationWorld('seed-C');
    const stock = w.moneyStock();
    const held = sum(
      w.instruments
        .all()
        .filter((i) => i.kind === 'money')
        .map((i) => w.register.heldTotal(i.id).value),
    );
    expect(stock['PHX']).toBeCloseTo(held.value, 6);
  });
});

describe('the period loop', () => {
  it('runs every phase, prints stale when nobody posts, and stays consistent over a year', () => {
    const w = foundationWorld('seed-D');
    w.auditNow();
    for (let i = 0; i < 52; i += 1) {
      const r = w.step();
      expect(violations(w)).toEqual([]);
      expect(r.markets[0]?.outcome).toBe('noDemand');
    }
    const print = w.prices.printOrThrow(GOV, w.period);
    expect(print.provenance.kind).toBe('stale');
    if (print.provenance.kind === 'stale') expect(print.provenance.from).toBe(0);
  });

  it('pays coupons to holders of record and the cash lands in named accounts (Register E1)', () => {
    const w = foundationWorld('seed-E');
    w.auditNow();
    // The central bank's own money returns to it and is destroyed (Money C4); its income is equity.
    const cbBefore = w.register.equity(partyId('cb.north'));
    const hhBefore = w.cash(partyId('hh.working.a'), PHX);
    const treasuryBefore = w.cash(partyId('treasury.north'), PHX);
    // 2026-09-15 is the first coupon date; step until its period has run.
    const target = w.calendar.periodOf({ y: 2026, m: 9, d: 15 });
    while (w.period < target) {
      w.step();
    }
    expect(violations(w)).toEqual([]);
    const coupons = w.ledger.all().filter((r) => r.instruction.cause === 'coupon');
    expect(coupons.length).toBe(7);
    expect(coupons.every((r) => r.outcome === 'settled')).toBe(true);
    expect(w.register.equity(partyId('cb.north'))).toBeGreaterThan(cbBefore);
    expect(w.cash(partyId('hh.working.a'), PHX)).toBeGreaterThan(hhBefore);
    expect(w.cash(partyId('treasury.north'), PHX)).toBeLessThan(treasuryBefore);
    // The treasury's expense equals the holders' income, exactly (Money C2.c, Audit B1).
    const paid = sum(
      coupons.flatMap((r) =>
        r.outcome === 'settled'
          ? r.equity.filter((e) => e.party === 'treasury.north').map((e) => e.delta)
          : [],
      ),
    );
    const received = sum(
      coupons.flatMap((r) =>
        r.outcome === 'settled'
          ? r.equity
              .filter((e) => e.party !== 'treasury.north')
              .map(
                (e) =>
                  e.delta *
                  (w.parties.get(e.party).representation === 'cell'
                    ? w.parties.cell(e.party).weight
                    : 1),
              )
          : [],
      ),
    );
    expect(Math.abs(paid.value + received.value)).toBeLessThanOrEqual(
      paid.dust + received.dust + 1e-9,
    );
  });

  it('a phase cannot read a print the period has not produced (Clearing F1.a)', () => {
    const w = foundationWorld('seed-F');
    w.auditNow();
    expect(() => w.prices.printOrThrow(GOV, period(1))).toThrow(NotYetProduced);
  });
});

describe('a market with reasons on both sides', () => {
  it('clears, settles paper against cash in one instruction, and revalues everyone (XI-5, D4)', () => {
    const w = foundationWorld('seed-G');
    w.auditNow();
    w.addOrderProvider((m): Order[] => {
      if (m.instrument !== GOV) return [];
      return [
        { party: partyId('firm.1'), side: 'buy', price: 0.99, qty: 100 },
        { party: partyId('bank.b'), side: 'sell', price: 0.97, qty: 100 },
      ];
    });
    const r = w.step();
    expect(violations(w)).toEqual([]);
    expect(r.markets[0]?.outcome).toBe('cleared');
    expect(r.markets[0]?.settledVolume).toBe(100);
    expect(w.register.quantity(partyId('firm.1'), GOV)).toBe(100);
    expect(w.register.quantity(partyId('bank.b'), GOV)).toBe(400);
    const print = w.prices.printOrThrow(GOV, w.period);
    expect(print.provenance.kind).toBe('traded');
    expect([0.97, 0.99]).toContain(print.price);
    // Every holder of the line now carries the new mark, and the treasury the reverse (Audit B5 holds).
    const acc = w.last?.audit.families.find((f) => f.family === 'accounts');
    expect(acc?.count).toBe(0);
  });

  it('a buyer without the cash fails the whole trade, not half of it (Register C3.b)', () => {
    const w = foundationWorld('seed-H');
    w.auditNow();
    w.addOrderProvider((m): Order[] => {
      if (m.instrument !== GOV) return [];
      return [
        { party: partyId('firm.2'), side: 'buy', price: 1.0, qty: 400 }, // firm.2 has 150 of cash
        { party: partyId('bank.a'), side: 'sell', price: 1.0, qty: 400 },
      ];
    });
    const r = w.step();
    expect(r.markets[0]?.failedTrades).toBe(1);
    expect(r.markets[0]?.settledVolume).toBe(0);
    expect(w.register.quantity(partyId('firm.2'), GOV)).toBe(0);
    expect(w.cash(partyId('firm.2'), PHX)).toBe(150);
    const failed = w.ledger.all().filter((x) => x.outcome === 'failed');
    expect(failed).toHaveLength(1);
    expect(failed[0]?.outcome === 'failed' && failed[0].reason.kind).toBe('overdraftRefused');
    expect(violations(w)).toEqual([]);
  });
});

describe('settlement contracts', () => {
  it('refuses a cell side without a per-member amount (XI-15)', () => {
    const w = foundationWorld('seed-I');
    const draft: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: partyId('bank.a') },
          to: { holder: partyId('hh.working.a'), issuer: partyId('bank.a') },
          ccy: PHX,
          amount: 10,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: 'test',
    };
    expect(() => w.settlement.settle(draft, w.period, w.cycle)).toThrow(Forbidden);
  });

  it('routes reserves between banks and none within one (Money C2.a, C2.b)', () => {
    const w = foundationWorld('seed-J');
    const cell = w.parties.cell(partyId('hh.working.b'));
    const side = cellSide(cell, 0.01);
    const draft: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: partyId('bank.a') },
          to: { holder: cell.id, issuer: partyId('bank.b') },
          ccy: PHX,
          amount: totalFor(cell, 0.01),
          fromCell: none(),
          toCell: side === undefined ? none() : some(side),
        },
      ],
      cause: 'transfer',
      reason: 'wages',
    };
    const reservesA = w.register.quantity(
      partyId('bank.a'),
      moneyInstrumentId(partyId('cb.north'), PHX),
    );
    const rec = w.settlement.settle(draft, w.period, w.cycle);
    expect(rec.outcome).toBe('settled');
    if (rec.outcome !== 'settled') return;
    expect(rec.reserveLegs).toHaveLength(2);
    expect(
      w.register.quantity(partyId('bank.a'), moneyInstrumentId(partyId('cb.north'), PHX)),
    ).toBeCloseTo(reservesA - 8, 9);
    expect(w.register.quantity(cell.id, moneyInstrumentId(partyId('bank.b'), PHX))).toBeCloseTo(
      0.26,
      12,
    );
    const report = w.auditNow();
    expect(report.total).toBe(0);

    const same: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: partyId('bank.a') },
          to: { holder: partyId('firm.2'), issuer: partyId('bank.a') },
          ccy: PHX,
          amount: 5,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: 'invoice',
    };
    const rec2 = w.settlement.settle(same, w.period, w.cycle);
    expect(rec2.outcome === 'settled' && rec2.reserveLegs).toEqual([]);
  });

  it('never adds two currencies (Money A2.b)', () => {
    const w = foundationWorld('seed-K');
    const draft: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: partyId('bank.a') },
          to: { holder: partyId('firm.2'), issuer: partyId('bank.a') },
          ccy: 'XXX' as never,
          amount: 5,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: 'bad',
    };
    expect(() => w.settlement.settle(draft, w.period, w.cycle)).toThrow();
  });
});

describe('cells (XI-15)', () => {
  it('splits exactly, conserving totals, and merges identical cells back', () => {
    const w = foundationWorld('seed-L');
    const cellId = partyId('hh.working.a');
    const before = w.register.totalQuantity(cellId, GOV);
    const fresh = splitCell(cellId, 250, 'test split', w.period, w.cycle, {
      parties: w.parties,
      register: w.register,
      journal: w.journal,
    });
    expect(w.parties.cell(cellId).weight).toBe(750);
    expect(w.parties.cell(fresh).weight).toBe(250);
    expect(
      w.register.totalQuantity(cellId, GOV) + w.register.totalQuantity(fresh, GOV),
    ).toBeCloseTo(before, 9);
    expect(w.auditNow().total).toBe(0);
    mergeCells(cellId, fresh, 'test merge', w.period, w.cycle, {
      parties: w.parties,
      register: w.register,
      journal: w.journal,
    });
    expect(w.parties.cell(cellId).weight).toBe(1000);
    expect(w.parties.get(fresh).status.alive).toBe(false);
    expect(w.register.totalQuantity(cellId, GOV)).toBeCloseTo(before, 9);
  });

  it('a weight changes only by the five events, and never to nobody', () => {
    const w = foundationWorld('seed-M');
    const deps = { parties: w.parties, register: w.register, journal: w.journal };
    weightEvent(partyId('hh.retired.b'), 'death', 100, 'test', w.period, w.cycle, deps);
    expect(w.parties.cell(partyId('hh.retired.b')).weight).toBe(500);
    expect(() => {
      weightEvent(partyId('hh.retired.b'), 'death', 500, 'test', w.period, w.cycle, deps);
    }).toThrow(Forbidden);
    expect(() => splitCell(partyId('hh.retired.b'), 500, 'test', w.period, w.cycle, deps)).toThrow(
      Forbidden,
    );
  });
});
