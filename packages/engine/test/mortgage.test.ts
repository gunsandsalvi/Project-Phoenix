/**
 * A mortgage is a row per (lender, cell), asked for every period a cell is short of roofs, and
 * foreclosed on a default and nothing else (Housing C1, C2, C4, XI-1, 12a.4).
 *
 * @spec Housing C1 Housing C2 Housing C4 XI-1 XI-15 Register F2
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('a cell asks for a mortgage while it is short of roofs, and a lender forecloses only on a default (12a.4)', () => {
  it('asks in more than one period, and every foreclosure names a row a default was recorded on', () => {
    const w = rigWorld('mortgage');
    const periodsAsked = new Map<string, Set<number>>();
    for (let i = 0; i < 8; i += 1) {
      w.step();
      for (const e of w.journal.ofKindIn('credit.request', w.period)) {
        const who = e.subjects[0];
        if (who === undefined || w.parties.get(who as never).kind !== HOUSEHOLD) continue;
        if (!JSON.stringify(e.data['security'] ?? []).includes('dwelling')) continue;
        const had = periodsAsked.get(who) ?? new Set<number>();
        had.add(w.period);
        periodsAsked.set(who, had);
      }
    }
    // C1, XI-15: "one mortgage at a time" is not a cell's rule — a cell short of roofs asks again.
    expect([...periodsAsked.values()].some((s) => s.size > 1)).toBe(true);
    // C4, XI-1: a foreclosure follows a default on the row, never the borrower being gone.
    const defaulted = new Set(w.journal.ofKind('credit.default').map((e) => String(e.data['instrument'])));
    for (const e of w.journal.ofKind('housing.foreclosed')) expect(defaulted.has(String(e.data['loan']))).toBe(true);
  });
});
