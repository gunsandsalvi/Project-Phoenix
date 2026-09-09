/**
 * The auction: the obligation that makes it hard to fail, and the limit that keeps failure real.
 *
 * @spec Sovereign C2 Sovereign C3 Sovereign C3.a Sovereign C4 Sovereign C5 Sovereign C6 Sovereign C7 Sovereign B3.a Treasury D2.a Treasury D5 Treasury D5.a Treasury E4
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  PHX,
  TREASURY_NORTH,
  assemble,
  foundationSpec,
  foundationWorld,
  moneyInstrumentId,
  type World,
} from '../src/index.js';

interface AuctionRow {
  readonly period: number;
  readonly line: string;
  readonly size: number;
  readonly allotted: number;
  readonly cover: number;
  readonly stopOut: number | null;
}

function auctions(w: World): AuctionRow[] {
  return w.journal.ofKind('auction.result').map((e) => ({
    period: e.period,
    line: String(e.data['line']),
    size: e.data['size'] as number,
    allotted: e.data['allotted'] as number,
    cover: e.data['cover'] as number,
    stopOut: e.data['stopOut'] as number | null,
  }));
}

describe('the obligation (Sovereign C3)', () => {
  it('brings dealers to every auction, so the paper is placed and the cash reaches the issuer', () => {
    const w = foundationWorld('auc-a');
    let cashBefore = w.cash(TREASURY_NORTH, PHX);
    for (let i = 0; i < 12; i += 1) {
      const before = cashBefore;
      const r = w.step();
      cashBefore = w.cash(TREASURY_NORTH, PHX);
      const row = auctions(w).find((a) => a.period === r.period);
      if (row === undefined) continue;
      expect(row.cover).toBeGreaterThan(0);
      expect(row.allotted).toBeGreaterThan(0);
      // C6: the proceeds reach the account, so the period it auctions in is a period it gains cash.
      expect(cashBefore).toBeGreaterThan(before - row.size);
      expect(row.stopOut).not.toBeNull();
    }
    expect(auctions(w).length).toBeGreaterThan(1);
  });

  it('re-opens the line the tenor lands on rather than making a new one beside it (B3.a)', () => {
    const w = foundationWorld('auc-b');
    for (let i = 0; i < 30; i += 1) w.step();
    const byLine = new Map<string, number>();
    for (const a of auctions(w)) byLine.set(a.line, (byLine.get(a.line) ?? 0) + 1);
    // At least one line was brought more than once: that is a re-opening, not a second instrument.
    expect([...byLine.values()].some((n) => n > 1)).toBe(true);
  });
});

describe('when the dealers step back (C3.a, Treasury D5.a)', () => {
  it('fails: nobody absorbs the remainder, and the treasury is left lower than the plan assumed', () => {
    // Banks with no cash cannot bid, however obliged they are — and with no central bank buying in
    // the market there is nothing to put reserves back into their hands either.
    const spec = foundationSpec('auc-c');
    const withoutCentralBank = spec.modules.filter((m) => m.id !== 'central-bank-omo');
    const broke = withoutCentralBank.map((m) =>
      m.id === 'seed.foundation'
        ? {
            ...m,
            seed: (ctx: Parameters<NonNullable<typeof m.seed>>[0]) => {
              m.seed?.(ctx);
              // Nobody has any money: the dealers cannot bid however obliged they are, and the
              // treasury cannot spend any into their hands either.
              for (const party of [...ctx.parties.ofKind(BANK), ctx.parties.get(TREASURY_NORTH)]) {
                const account = moneyInstrumentId(party.bank, PHX);
                const held = ctx.register.quantity(party.id, account);
                if (held <= 0) continue;
                ctx.register.moneyDelta(party.id, account, -held, ctx.period, false);
                ctx.instruments.adjustIssued(account, -held);
              }
            },
          }
        : m,
    );
    const w = assemble({ ...spec, modules: broke });
    let failed = false;
    for (let i = 0; i < 12; i += 1) {
      const r = w.step();
      const row = auctions(w).find((a) => a.period === r.period);
      if (row?.allotted === 0) {
        failed = true;
        expect(row.cover).toBe(0);
        break;
      }
    }
    expect(failed).toBe(true);
    expect(w.last?.audit.total).toBe(0);
  });
});

describe('what the market reads out of it (C4)', () => {
  it('reports the cover and the tail, and the tail is what the winners bid over the stop-out', () => {
    const w = foundationWorld('auc-d');
    for (let i = 0; i < 12; i += 1) w.step();
    const rows = w.journal.ofKind('auction.result');
    expect(rows.length).toBeGreaterThan(0);
    for (const e of rows) {
      expect(typeof e.data['cover']).toBe('number');
      if ((e.data['allotted'] as number) > 0) {
        expect(e.data['tail']).not.toBeNull();
        expect(e.data['tail'] as number).toBeGreaterThanOrEqual(-1e-12);
      }
      expect((e.data['withdrawn'] as number) + (e.data['allotted'] as number)).toBeCloseTo(
        e.data['size'] as number,
        9,
      );
    }
  });
});
