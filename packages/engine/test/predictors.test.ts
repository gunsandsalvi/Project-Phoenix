/**
 * Two predictors per variable, and the party follows the one that has surprised it less; the
 * aggregate moves after the surprises, never before (§46 B1, B1.b, E2, 12d.2).
 *
 * @spec Expectations B1 Expectations B1.b Expectations B2 Expectations E2 Expectations D4 Law 17
 */
import { describe, expect, it } from 'vitest';
import { rigWorld } from './rig.js';

interface Held {
  readonly expected: number;
  readonly memory: number;
  readonly anchored: number | null;
  readonly follows: 'adaptive' | 'anchored';
}
type Book = Record<string, Record<string, Held>>;

const mean = (xs: readonly number[]): number => xs.reduce((s, x) => s + Math.abs(x), 0) / xs.length;

describe('a party follows the predictor that has surprised it less (12d.2)', () => {
  it('keeps an anchored predictor where the variable has a public level, none where it has not, and follows the narrower track', () => {
    const w = rigWorld('predict');
    for (let i = 0; i < 20; i += 1) w.step();
    const book = w.stateSlots()['expectations/outlooks'] as Book;
    const surprises = w.journal.ofKind('expectations.surprise');
    let anchored = 0;
    let switched = 0;
    let decisions = 0;
    for (const [party, forParty] of Object.entries(book)) {
      for (const [variable, h] of Object.entries(forParty)) {
        // B1.b: no public level for what is the party's own — what reached it and what it made.
        if (variable === 'income' || variable === 'earnings' || variable.startsWith('sold.') || variable.startsWith('bought.')) {
          expect(h.anchored).toBeNull();
          expect(h.follows).toBe('adaptive');
          continue;
        }
        if (h.anchored === null) continue;
        anchored += 1;
        if (h.follows === 'anchored') switched += 1;
        // B4, Law 19: the switch at the top of each period is a read of the two tracks AS THEY
        // STOOD THEN — replayed here off the recorded surprises, each of which carries both tracks'
        // entries and the predictor the party followed when it took it.
        const own: number[] = [];
        const pub: number[] = [];
        let follows: 'adaptive' | 'anchored' = 'adaptive';
        const window = Math.max(1, Math.round(h.memory));
        const mine = surprises.filter((e) => e.subjects[0] === party && e.data['variable'] === variable);
        for (const e of mine) {
          if (own.length > 0 && pub.length > 0) {
            const a = mean(own);
            const b = mean(pub);
            if (b < a) follows = 'anchored';
            else if (a < b) follows = 'adaptive';
          }
          expect(e.data['follows']).toBe(follows);
          decisions += 1;
          const o = e.data['own'];
          const p = e.data['anchored'];
          if (typeof o === 'number' && o !== 0) own.push(o);
          if (typeof p === 'number' && p !== 0) pub.push(p);
          if (own.length > window) own.splice(0, own.length - window);
          if (pub.length > window) pub.splice(0, pub.length - window);
        }
      }
    }
    expect(anchored).toBeGreaterThan(0);
    expect(decisions).toBeGreaterThan(0);
    // A3: not everybody follows the same predictor — the histories differ, so the choices do.
    expect(switched).toBeGreaterThan(0);
    expect(switched).toBeLessThan(anchored);
  });
});

describe('the aggregate moves after the surprises, never before (E2, Law 17 — the killer of 12d.2)', () => {
  it('every change in a variable’s published dispersion follows a surprise on it, or a party arriving or leaving it', () => {
    const w = rigWorld('killer');
    for (let i = 0; i < 52; i += 1) w.step();
    const published = w.journal.ofKind('expectations.dispersion');
    expect(published.length).toBeGreaterThan(10);
    const surprises = w.journal.ofKind('expectations.surprise');
    const surprisedAt = new Map<string, Set<number>>();
    for (const e of surprises) {
      const v = String(e.data['variable']);
      const set = surprisedAt.get(v) ?? new Set<number>();
      set.add(e.period);
      surprisedAt.set(v, set);
    }
    // Who held a view of each variable, period by period, read off the surprises' subjects and the
    // weight events: a value that joins or leaves the aggregate moves it with no surprise behind it.
    const arrivals = new Set(w.journal.ofKind('weight').map((e) => Number(e.period)));
    const entries = new Set([...w.journal.ofKind('party.entered'), ...w.journal.ofKind('party.ceased')].map((e) => Number(e.period)));
    let moved = 0;
    const unexplained: string[] = [];
    for (let i = 1; i < published.length; i += 1) {
      const before = published[i - 1]?.data['dispersion'] as Record<string, number>;
      const now = published[i]?.data['dispersion'] as Record<string, number>;
      const about = Number(published[i]?.data['of']);
      for (const [variable, value] of Object.entries(now)) {
        const was = before[variable];
        if (was === undefined || Math.abs(value - was) <= 1e-9 * (Math.abs(value) + Math.abs(was))) continue;
        moved += 1;
        // E2: the statistic about period `about` moved because of what happened IN period `about`.
        const explained = surprisedAt.get(variable)?.has(about) === true || arrivals.has(about) || entries.has(about);
        if (!explained) unexplained.push(`${variable} at ${String(about)}`);
      }
    }
    expect(moved).toBeGreaterThan(0);
    expect(unexplained).toEqual([]);
  });
});
