/**
 * 0g.1: THE LADDER'S INVARIANTS, on the rung the suite can afford. The full ladder is a measurement
 * and runs as `npm run ladder` (Law 11); what is asserted here is what is invariant by
 * construction at any scale — a count of people is a count at every grain, a finer grain occupies
 * no fewer keys — and the numbers are reported, never asserted (Law 18: gate on behaviour).
 */
import { describe, expect, it } from 'vitest';
import { line, runRung } from './ladder.js';

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
});
