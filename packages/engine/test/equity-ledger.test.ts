/**
 * The equity ledger: what moved a party's equity, kept, so income is a READ.
 *
 * @spec Reporting A2 Reporting A2.a Reporting G2 Audit A1.a Audit B5 Register E2.a XI-15 Law 4 Law 7 Law 19
 *
 * `moveEquity` always received the cause its writer wrote and always threw it away: only the running
 * balance survived. So comprehensive income was recoverable exactly — it is the movement of the
 * account — and NOTHING ABOVE THE BOTTOM LINE WAS. A report that wanted a line of it would have had
 * to parse the reason strings on money legs, recovering by inference a fact its writer knew and did
 * not record, which is the Law 19 defect this project is organised against.
 *
 * The entries are the ITEMISATION and never the balance (Law 4). `equityWalk` is still the
 * accumulator and still authoritative; these are what it is made of, so the two are two independent
 * records of one thing and the `accounts` family compares them (Audit A1.a). Summing the entries to
 * PRODUCE the balance would make that comparison a tautology and the report unfalsifiable.
 */
import { describe, expect, it } from 'vitest';
import { withinDust, type Period } from '../src/index.js';
import { rigWorld } from './rig.js';
import { unexpected } from './expected.js';

describe('the equity ledger (Reporting A2, G2)', () => {
  it('itemises every move of every account, and the count says none is missing', () => {
    const w = rigWorld('equity-ledger', 4, 16);
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    let busiest = 0;
    for (const p of w.parties.all()) {
      if (!w.register.hasEquityAccount(p.id)) continue;
      const walk = w.register.equityWalk(p.id);
      const entries = w.register.equityEntries(p.id, 0 as Period, w.period);
      // One entry for the opening statement (Seed C1) and one for every move since. A sum can be
      // made to agree by two errors; a count cannot, which is why this is the first assertion.
      expect(entries.length, String(p.id)).toBe(walk.moves + 1);
      busiest = Math.max(busiest, walk.moves);
    }
    // And the world it ran is one where accounts actually move: a party moved a handful of times
    // would pass this without testing anything.
    expect(busiest).toBeGreaterThan(100);
  });

  it('sums, over any span, to what the account did over that span', () => {
    const w = rigWorld('equity-ledger', 3, 12);
    const at = new Map<string, number[]>();
    const periods = 6;
    for (let i = 0; i < periods; i += 1) {
      w.step();
      for (const p of w.parties.all()) {
        if (!w.register.hasEquityAccount(p.id)) continue;
        const seen = at.get(String(p.id));
        const balance = w.register.equity(p.id);
        if (seen === undefined) at.set(String(p.id), [balance]);
        else seen.push(balance);
      }
    }
    // THE SPAN IS WHAT A FISCAL QUARTER IS (Reporting A3), so this is the read a report is built on
    // and not a convenience: what a company earned between two dates is the entries between them.
    let spans = 0;
    for (const p of w.parties.all()) {
      const seen = at.get(String(p.id));
      if (seen === undefined || seen.length < periods) continue;
      for (let from = 2; from <= periods; from += 1) {
        for (let to = from; to <= periods; to += 1) {
          const entries = w.register.equityEntries(p.id, from as Period, to as Period);
          const itemised = entries.reduce((t, e) => t + e.delta, 0);
          const moved = seen[to - 1]! - seen[from - 2]!;
          // Law 7: the dust is the walk's own, over the moves the span carried — never a band.
          const dust = w.register.equityWalk(p.id).dust;
          expect(
            withinDust(itemised, moved, dust),
            `${p.id} p${from}..p${to}: entries ${itemised} against a move of ${moved}`,
          ).toBe(true);
          spans += 1;
        }
      }
    }
    expect(spans).toBeGreaterThan(0);
  });

  it('carries the itemisation across a cell split, because it is per-member state (XI-15)', () => {
    // A split is one member described twice: the new cell's equity has the same history as the old
    // one's, and it did not arrive from nowhere. Copying the walk and not the entries left cells
    // whose account said it had been moved eighteen times and whose ledger carried nine — which the
    // count check above is what found.
    const w = rigWorld('equity-ledger', 4, 16);
    for (let i = 0; i < 8; i += 1) w.step();
    const split = w.parties.all().filter((p) => /\.\d+\.\d+$/.test(String(p.id)));
    expect(split.length, 'this world never split a cell, so it tests nothing').toBeGreaterThan(0);
    for (const p of split) {
      if (!w.register.hasEquityAccount(p.id)) continue;
      const walk = w.register.equityWalk(p.id);
      expect(w.register.equityEntries(p.id, 0 as Period, w.period).length).toBe(walk.moves + 1);
    }
  });

  it('is not the balance: the account is what it always was (Law 4)', () => {
    // The walk is the accumulator and stays authoritative. If the balance were ever produced BY
    // summing the entries, the family that compares them would be comparing a number with itself.
    const w = rigWorld('equity-ledger', 3, 12);
    for (let i = 0; i < 4; i += 1) w.step();
    for (const p of w.parties.all()) {
      if (!w.register.hasEquityAccount(p.id)) continue;
      expect(w.register.equity(p.id)).toBe(w.register.equityWalk(p.id).value);
    }
  });
});
