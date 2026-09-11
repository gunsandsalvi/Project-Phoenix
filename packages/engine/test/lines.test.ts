/**
 * A bank's own allocation: what each of its lines earned on the capital it used, and who gets the
 * room it has left.
 *
 * @spec Banks Capital B1 Banks Capital B2 Banks Capital B3 Banks Lending C1.c Dealer Desks D1 Dealer Desks F1 Dealer Desks F2 Observer A4 Law 2 Law 4 Law 6 Law 19 XI-4
 *
 * Deferred out of 11.2 with a reason — "every realised return in that world was an artefact of the
 * opening balance sheets; it is reachable once they add up" — and reachable now that they do.
 *
 * The defect it closes: two lines of one bank each sized itself against the WHOLE bank, the lending
 * line reading the capital headroom and the dealing line taking its own declared share of capital,
 * and neither knowing the other existed. Two lines drawing on one pool with no allocation between
 * them are two banks sharing an equity account, and which of them grows is nobody's decision.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  snapshot,
  type Event,
  type World,
} from '../src/index.js';
import { rigWorld } from './rig.js';
import { unexpected } from './expected.js';

const PERIODS = 26;

interface Row {
  readonly line: string;
  readonly capital: number;
  readonly earned: number;
  readonly returnOnCapital: number | null;
  readonly room: number;
}

function run(seed: string): World {
  const w = rigWorld(seed);
  for (let i = 0; i < PERIODS; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

function rowsOf(e: Event | undefined): Row[] {
  const lines = e?.data['lines'];
  return Array.isArray(lines) ? (lines as Row[]) : [];
}

function latest(w: World, bank: string): Event | undefined {
  const said = w.journal.ofKind('bank.lines').filter((e) => e.subjects.includes(bank));
  return said[said.length - 1];
}

describe("a bank's own allocation across its lines (XI-4, Dealer Desks F2)", () => {
  const w = run('lines-a');
  const banks = w.parties.ofKind(BANK).map((b) => String(b.id));

  it('is PRIVATE: what a bank makes on each line is what a rival would price against (A4)', () => {
    // Observer A4. Its CONSEQUENCE is public — a bank that stops quoting has stopped where anybody
    // can see — but the number that decided it is nobody else's.
    const said = w.journal.ofKind('bank.lines');
    expect(said.length).toBeGreaterThan(0);
    for (const e of said) expect(e.public).toBe(false);
    for (const bank of banks) {
      const seen = snapshot(w, { kind: 'party', party: bank as never }, 4000).journal.filter(
        (e) => e.kind === 'bank.lines',
      );
      // A party sees its own; nobody sees another's.
      expect(seen.length).toBeGreaterThan(0);
      for (const e of seen) expect(e.subjects).toContain(bank);
    }
  });

  it('reports a return on capital that is a read, and Missing where a line used none', () => {
    for (const bank of banks) {
      const rows = rowsOf(latest(w, bank));
      expect(rows.length).toBe(2);
      for (const r of rows) {
        if (r.capital > 0) {
          // Law 19: it is what the line EARNED over what it USED, and neither is a stated rate.
          expect(r.returnOnCapital).not.toBeNull();
          expect(r.returnOnCapital).toBeCloseTo(r.earned / r.capital, 9);
        } else {
          // Appendix A: a return on nothing is not a number, and it is not zero either.
          expect(r.returnOnCapital).toBeNull();
        }
      }
    }
  });

  it('names what neither line claims instead of sweeping it into one of them (Law 2)', () => {
    // A residual with no holder is a defect. What an instruction did to a bank's equity that
    // neither line moved — a coupon on the paper its treasury holds for liquidity, a deposit rate
    // it paid — is reported under its own name, so the two returns are what those two lines did
    // and nothing else has been folded into either.
    for (const bank of banks) {
      const e = latest(w, bank);
      expect(typeof e?.data['unattributed']).toBe('number');
    }
  });

  it('gives the room to the higher earner first, and the other can get nothing (no floor)', () => {
    for (const bank of banks) {
      const e = latest(w, bank);
      const rows = rowsOf(e);
      const headroom = e?.data['headroom'];
      expect(typeof headroom).toBe('number');
      const given = rows.reduce((a, r) => a + r.room, 0);
      // Law 6: what is shared out is exactly the room the bank has, and no line is held up to a
      // minimum. A line behind the other in a period when the room ran out gets nothing at all.
      if (typeof headroom === 'number' && headroom > 0) {
        expect(given).toBeLessThanOrEqual(headroom + 1);
      } else {
        expect(given).toBe(0);
      }
      const ranked = [...rows].sort(
        (a, b) => (b.returnOnCapital ?? 0) - (a.returnOnCapital ?? 0),
      );
      const first = ranked[0];
      const second = ranked[1];
      expect(first).toBeDefined();
      expect(second).toBeDefined();
      // The higher earner is served first: the one behind it never has room the one in front
      // was refused.
      if (first !== undefined && second !== undefined && second.room > 0) {
        expect(first.room).toBeGreaterThan(0);
      }
    }
  });

  it('reaches the lines that spend it: both read the allocation rather than the whole bank', () => {
    // Law 4: one decider, one number. The dealing line's published room and the credit decision's
    // capital constraint are the same allocation read twice, not two derivations of it.
    const dealing = w.journal.ofKind('bank.dealing').filter((e) => e.period === w.period);
    expect(dealing.length).toBeGreaterThan(0);
    for (const e of dealing) {
      const bank = String(e.subjects[0]);
      const rows = rowsOf(latest(w, bank));
      const room = rows.find((r) => r.line === 'dealing')?.room;
      expect(typeof room).toBe('number');
      // A line allotted nothing keeps the book it has and does not grow it: what it has room for
      // is never more than what it carries plus what it was given.
      const book = e.data['book'];
      const left = e.data['roomLeft'];
      if (typeof book === 'number' && typeof left === 'number' && typeof room === 'number') {
        expect(book + left).toBeLessThanOrEqual(book + room + 1);
      }
    }
  });

  it('holds every audit family over the run it was measured on (Part XII)', () => {
    // `run` asserts it every period; this is the statement that the run happened at all, so a
    // world that stopped early cannot quietly pass the rest of this file.
    expect(w.period).toBe(PERIODS);
    expect(w.parties.ofKind(BANK).length).toBeGreaterThan(0);
  });
});
