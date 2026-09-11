/**
 * The auction: the obligation that makes it hard to fail, and the limit that keeps failure real.
 *
 * @spec Sovereign C2 Sovereign C3 Sovereign C3.a Sovereign C4 Sovereign C5 Sovereign C6 Sovereign C7 Sovereign B3.a Treasury D2.a Treasury D5 Treasury D5.a Treasury E4
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  FIRM,
  USD,
  TREASURY_US,
  assemble,
  moneyInstrumentId,
  type World,
} from '../src/index.js';
import { rigWorld, rigSpec, withDependencies } from './rig.js';
import { negQty } from '../src/core/tick.js';

interface AuctionRow {
  readonly period: number;
  readonly line: string;
  readonly size: number;
  readonly allotted: number;
  readonly cover: number;
  readonly stopOut: number | null;
}

/**
 * Polity A1, PLAN §7: THE AUCTIONS OF ONE ISSUER. This world has four states selling paper (12d:
 * the United States, Europe, the United Kingdom and Japan), and they auction in the same periods —
 * so "the auction this period" was whichever of the four the journal happened to record first, and
 * a test about the US treasury's cash was reading Japan's book. The issuer is on the event.
 */
function auctions(w: World, issuer: string = TREASURY_US): AuctionRow[] {
  return w.journal
    .ofKind('auction.result')
    .filter((e) => e.subjects.includes(issuer))
    .map((e) => ({
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
    const w = rigWorld('auc-a');
    let cashBefore = w.cash(TREASURY_US, USD);
    let placed = 0;
    for (let i = 0; i < 12; i += 1) {
      const before = cashBefore;
      const r = w.step();
      cashBefore = w.cash(TREASURY_US, USD);
      const row = auctions(w).find((a) => a.period === r.period);
      if (row === undefined) continue;
      // C3: THE OBLIGATION IS TO BID, and it is discharged at every auction — the dealers turn up.
      expect(row.cover).toBeGreaterThan(0);
      // C7, D5.a: what they bid is their own, and the issuer never sells below its reservation, so
      // an auction CAN fail — it does at period 12 of this run, on the long line, with a cover of
      // 1.4 and every bid under the reserve. This asserted every auction places its paper, which is
      // the one thing the next test is about. What follows a STRUCK stop-out is the placing.
      if (row.stopOut === null) {
        expect(row.allotted).toBe(0);
        continue;
      }
      expect(row.allotted).toBeGreaterThan(0);
      // C6: the proceeds reach the account, so the period it auctions in is a period it gains cash.
      expect(cashBefore).toBeGreaterThan(before - row.size);
      placed += 1;
    }
    expect(auctions(w).length).toBeGreaterThan(1);
    expect(placed, 'no auction in this run placed any paper at all').toBeGreaterThan(0);
  });

  it('re-opens the line the tenor lands on rather than making a new one beside it (B3.a)', () => {
    const w = rigWorld('auc-b');
    for (let i = 0; i < 30; i += 1) w.step();
    const byLine = new Map<string, number>();
    for (const a of auctions(w)) byLine.set(a.line, (byLine.get(a.line) ?? 0) + 1);
    // At least one line was brought more than once: that is a re-opening, not a second instrument.
    expect([...byLine.values()].some((n) => n > 1)).toBe(true);
  });
});

describe('when the dealers step back (C3.a, Treasury D5.a)', () => {
  it('fails: nobody absorbs the remainder, and the treasury is left lower than the plan assumed', () => {
    // A world with no money in it. The obligation still brings a bid — that is what an obligation
    // is — but what a dealer bids is its OWN price, and a dealer with nothing, in a market where
    // nothing has traded and nothing can, does not reach what the issuer will take. With no central
    // bank buying in the market there is nothing to put money back into anybody's hands either.
    const spec = rigSpec('auc-c');
    const withoutCentralBank = withDependencies(spec.modules, (m) =>
m.id !== 'central-bank-omo');
    const stepped = withoutCentralBank.map((m) =>
      m.id === 'banks'
        ? {
            ...m,
            // Dealer Desks D1, D4, D4.a: the dealers step back because their own limit binds — a
            // book of nothing is all they will carry — which is the reason D1-D3 give and the one
            // that makes a failed auction possible. A dealer that will not take a position on is a
            // legitimate, representable state, and this is what it looks like.
            params: m.params.map((p) =>
              p.id.startsWith('bank.dealing.limit.') ? { ...p, value: 0 } : p,
            ),
          }
        : m,
    );
    const broke = stepped.map((m) =>
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding'
        ? {
            ...m,
            seed: (ctx: Parameters<NonNullable<typeof m.seed>>[0]) => {
              m.seed?.(ctx);
              // Nobody has any money and nobody is about to be paid any: the banks cannot bid
              // however obliged they are, the treasury cannot spend any into their hands, and the
              // firms cannot pay a wage that would put some into a household's. A world with no money in it is the one in which an auction has to
              // fail, and this is that world.
              for (const party of [
                ...ctx.parties.ofKind(BANK),
                ...ctx.parties.ofKind(FIRM),
                ctx.parties.get(TREASURY_US),
              ]) {
                const account = moneyInstrumentId(party.bank, USD);
                const held = ctx.register.quantity(party.id, account);
                if (held <= 0) continue;
                ctx.register.moneyDelta(party.id, account, negQty(held), ctx.period, false);
                ctx.instruments.adjustIssued(account, negQty(held));
              }
            },
          }
        : m,
    );
    const w = assemble({ ...spec, modules: broke });
    let failed = false;
    // Sovereign B3: WHEN this world's treasury next comes to market is its own funding programme's
    // decision, and the programme moved when the seed's scale was derived (11.5) — twelve periods
    // used to reach an auction of the long line and now reaches the bill that still places. What
    // the clause is about is the auction that fails, so the run is long enough to contain one.
    for (let i = 0; i < 16; i += 1) {
      const r = w.step();
      const row = auctions(w).find((a) => a.period === r.period);
      if (row?.allotted === 0) {
        failed = true;
        // C3: the obligation was discharged — the dealers bid, and between them for the whole size.
        // C7, D5.a: and it still failed. Nothing was struck, nothing was placed, and the remainder
        // is withdrawn rather than absorbed: there is no buyer of last resort behind this book.
        expect(row.cover).toBeGreaterThan(0);
        expect(row.stopOut).toBeNull();
        expect(row.size - row.allotted).toBeCloseTo(row.size, 9);
        break;
      }
    }
    expect(failed).toBe(true);
    expect(w.last?.audit.total).toBe(0);
  });
});

describe('what the market reads out of it (C4)', () => {
  it('reports the cover and the tail, and the tail is what the winners bid over the stop-out', () => {
    const w = rigWorld('auc-d');
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
