/**
 * 0g.1: THE LADDER'S INVARIANTS, on the rung the suite can afford. The full ladder is a measurement
 * and runs as `npm run ladder` (Law 11); what is asserted here is what is invariant by
 * construction at any scale — a count of people is a count at every grain, a finer grain occupies
 * no fewer keys — and the numbers are reported, never asserted (Law 18: gate on behaviour).
 */
import { describe, expect, it } from 'vitest';
import { line, runRung, steadyOf, type Rung } from './ladder.js';

/** `types: []` keeps the console out of the engine; the ladder is a report and this is its one door. */
declare const console: { log: (line: string) => void };

describe('the ladder (Law 18, 0g.1)', () => {
  it('holds what is invariant by construction at the first rung, at both grains', () => {
    const one = runRung(3, 12, 1);
    const two = runRung(3, 12, 2);
    // XI-15: a count of people is a count, at every grain, exactly — and so is a count of firms.
    expect(two.people).toBe(one.people);
    expect(two.smallFirms).toBe(one.smallFirms);
    // A finer grain occupies at least as many keys, never fewer.
    expect(two.cells).toBeGreaterThanOrEqual(one.cells);
    console.log(['LADDER', line(one), line(two)].join('\n'));
  });

  /**
   * 0g.17 (21.118): THE STATISTIC, not the machine. `msPerPeriodAt` is CUMULATIVE elapsed over the
   * mark, so it carries the opening periods — a world still filling up, 636 ms against a steady 206
   * — into every figure after them. Three identical runs of the (12, 48) rung read 302, 257 and 247
   * on that statistic and 203, 207 and 206 on the steady median. Six 0g records reported deltas of
   * 1–4% against the noisy one. What is asserted here is the SHAPE of the read, not a duration:
   * the steady window is the tail, its median lies inside its own range, and it excludes the warm.
   */
  it('reads a steady median from the tail, never from the periods a world is still filling up', () => {
    const r = runRung(3, 12, 1, 10);
    expect(r.each.length).toBe(10);
    const all = steadyOf(r, 0);
    const tail = steadyOf(r, 6);
    expect(all.n).toBe(10);
    expect(tail.n).toBe(4);
    for (const s of [all, tail]) {
      expect(s.median).toBeGreaterThanOrEqual(s.lo);
      expect(s.median).toBeLessThanOrEqual(s.hi);
    }
    // The opening periods are the expensive ones, so dropping them cannot widen the range.
    expect(tail.hi - tail.lo).toBeLessThanOrEqual(all.hi - all.lo);
    // A window past the end is empty rather than a number nobody measured (Appendix A).
    expect(steadyOf(r, 10).n).toBe(0);
  });

  it('judges a rung on its shape, which is what Law 18 gates on', () => {
    const shape = (r: Rung): readonly number[] => [r.parties, r.cells, r.people, r.smallFirms];
    expect(shape(runRung(3, 12, 1, 6))).toEqual(shape(runRung(3, 12, 1, 6)));
  });
});
