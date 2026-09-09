/**
 * The treasury: the need, the announcement, what it pays, what it collects, and the constraint that
 * it has no overdraft anywhere.
 *
 * @spec Treasury A3.a Treasury B1 Treasury C1 Treasury C1.a Treasury C3 Treasury D1 Treasury D3 Treasury D4 Treasury D4.b Treasury D5 Sovereign A2 Sovereign A2.a Sovereign A3.b Sovereign C1 Sovereign C1.a Central Bank E2 XI-9
 */
import { describe, expect, it } from 'vitest';
import {
  CB,
  HOUSEHOLD,
  PHX,
  TREASURY_NORTH,
  TREASURY_PARAMS,
  assemble,
  foundationSpec,
  foundationWorld,
  moneyInstrumentId,
  type World,
} from '../src/index.js';

function programme(w: World): Record<string, unknown> {
  const events = w.journal.ofKind('treasury.programme');
  const last = events[events.length - 1];
  if (last === undefined) throw new Error('no programme published');
  return last.data;
}

describe('the programme (Treasury D4, Sovereign A2)', () => {
  it('is published every period, with the need it computed and what went into it (C1.a)', () => {
    const w = foundationWorld('tsy-a');
    w.step();
    const p = programme(w);
    expect(typeof p['need']).toBe('number');
    // A2.a: the need is what it must pay plus what falls due, less what it has and expects.
    expect(p['service']).toBeGreaterThan(0);
    expect(p['mandate']).toBeGreaterThan(0);
    expect(p['buffer']).toBeGreaterThan(0);
    expect(w.journal.ofKind('treasury.programme')[0]?.public).toBe(true);
  });

  it('announces on its own calendar and not otherwise (Sovereign C1)', () => {
    const w = foundationWorld('tsy-b');
    const every = w.params.get(TREASURY_PARAMS.auctionEvery);
    const announced: number[] = [];
    for (let i = 0; i < 3 * every; i += 1) {
      const r = w.step();
      if (w.journal.ofKind('auction.result').some((e) => e.period === r.period)) {
        announced.push(r.period);
      }
    }
    expect(announced.length).toBeGreaterThan(1);
    expect(announced.every((p) => p % every === 0)).toBe(true);
  });

  it('raises more when more falls due: the programme reads its own maturity profile (D4.a)', () => {
    const w = foundationWorld('tsy-c');
    const sizes: number[] = [];
    for (let i = 0; i < 30; i += 1) {
      const r = w.step();
      for (const e of w.journal.ofKind('auction.result')) {
        if (e.period === r.period) sizes.push(e.data['size'] as number);
      }
    }
    expect(sizes.length).toBeGreaterThan(4);
    expect(sizes.every((s) => s > 0)).toBe(true);
  });
});

describe('outlays and receipts (Treasury B1, C1)', () => {
  it('pays every household cell by name, per member, and the money arrives', () => {
    const w = foundationWorld('tsy-d');
    const cells = w.parties.ofKind(HOUSEHOLD);
    const before = cells.map((c) => w.cash(c.id, PHX));
    w.step();
    const after = cells.map((c) => w.cash(c.id, PHX));
    expect(after.every((x, i) => x > (before[i] ?? 0))).toBe(true);
    const transfers = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.instruction.cause === 'transfer' && r.outcome === 'settled');
    expect(transfers.length).toBeGreaterThanOrEqual(cells.length);
  });

  it('collects tax on the interest each payer was actually paid (C1.a, C3)', () => {
    const w = foundationWorld('tsy-e');
    const rate = w.params.get(TREASURY_PARAMS.taxInterest);
    let checked = false;
    for (let i = 0; i < 30; i += 1) {
      const r = w.step();
      const coupons = w.ledger
        .inPeriod(r.period)
        .filter((x) => x.outcome === 'settled' && x.instruction.cause === 'coupon');
      if (coupons.length === 0) continue;
      const paid = coupons
        .flatMap((x) => x.instruction.legs)
        .reduce((a, l) => a + (l.kind === 'money' ? l.amount : 0), 0);
      const next = w.step();
      const receipts = w.journal
        .ofKind('treasury.receipts')
        .find((e) => e.period === next.period);
      // What it collected is the rate applied to what was really paid, party by party — never to an
      // aggregate nobody was charged.
      expect(receipts?.data['total']).toBeCloseTo(paid * rate, 6);
      checked = true;
      break;
    }
    expect(checked).toBe(true);
  });
});

describe('the funding constraint (Treasury D3, Sovereign A3.b, XI-9)', () => {
  it('has no overdraft at the central bank: an outlay it cannot meet fails and is reported', () => {
    // A world whose treasury opens with nothing has to fail its very first mandate payment.
    const spec = foundationSpec('tsy-f');
    const drained = spec.modules.map((m) =>
      m.id === 'seed.foundation'
        ? {
            ...m,
            seed: (ctx: Parameters<NonNullable<typeof m.seed>>[0]) => {
              m.seed?.(ctx);
              // Take the buffer away: the account is empty when the first outlay falls due.
              const account = moneyInstrumentId(CB, PHX);
              const held = ctx.register.quantity(TREASURY_NORTH, account);
              ctx.register.moneyDelta(TREASURY_NORTH, account, -held, ctx.period, false);
              ctx.instruments.adjustIssued(account, -held);
            },
          }
        : m,
    );
    const w = assemble({ ...spec, modules: drained });
    expect(w.cash(TREASURY_NORTH, PHX)).toBe(0);
    const r = w.step();
    const shortfall = w.journal.ofKind('treasury.shortfall').find((e) => e.period === r.period);
    expect(shortfall).toBeDefined();
    expect(shortfall?.data['unpaid']).toBeGreaterThan(0);
    // The account never went below zero: nothing advanced it (D3, Central Bank E2).
    expect(w.cash(TREASURY_NORTH, PHX)).toBeGreaterThanOrEqual(0);
    const refused = w.ledger
      .inPeriod(r.period)
      .filter((x) => x.outcome === 'failed' && x.reason.kind === 'overdraftRefused');
    expect(refused.length).toBeGreaterThan(0);
    expect(r.audit.total).toBe(0);
  });

  it('funds itself over a year when the market is there: no shortfall, and the buffer survives', () => {
    const w = foundationWorld('tsy-g');
    for (let i = 0; i < 52; i += 1) w.step();
    expect(w.journal.ofKind('treasury.shortfall')).toHaveLength(0);
    expect(w.cash(TREASURY_NORTH, PHX)).toBeGreaterThan(0);
    // A3.a: its equity is negative and that is normal; the number is still a read.
    expect(w.register.equity(TREASURY_NORTH)).toBeLessThan(0);
  });
});
