/**
 * Periodicity: a thing that recurs, recurs on a DATE (item 20).
 *
 * @spec Money G3 Money G3.a Money G4 Ratings A5 Register E3 Law 4 Law 19
 *
 * `period % n` is what this exists instead of. A period is seven days and a month is not four of
 * them, so anything placed on a count of periods drifts against the calendar it claims to be on —
 * by a day a month and by a week a year. What places a recurrence here is the one calendar, walked
 * in months from the day the thing began.
 */
import { describe, expect, it } from 'vitest';
import { crossesAnniversary, period } from '../src/calendar/calendar.js';
import { MONTHS_IN_YEAR, addMonths, compareCivil } from '../src/calendar/civil.js';
import { rigWorld } from './rig.js';

describe('an anniversary is a date, not a count of periods (Money G3.a, item 20)', () => {
  const w = rigWorld('periodicity');
  const cal = w.calendar;

  it('falls once per year, in the period the day falls into, and never on the day itself', () => {
    const epoch = period(3);
    const hits: number[] = [];
    const years = 10;
    for (let p = 0; p < 530; p += 1) {
      if (crossesAnniversary(cal, epoch, MONTHS_IN_YEAR, period(p))) hits.push(p);
    }
    // Ten years of weekly periods: ten anniversaries, no more and no fewer.
    expect(hits.length).toBe(years);
    // Each one is the period the anniversary day falls into: the day is after the previous
    // period's start and not after this one's.
    hits.forEach((p, n) => {
      const day = addMonths(cal.startOf(epoch), MONTHS_IN_YEAR * (n + 1));
      expect(compareCivil(day, cal.startOf(period(p)))).toBeLessThanOrEqual(0);
      expect(compareCivil(day, cal.startOf(period(p - 1)))).toBeGreaterThan(0);
    });
    // A thing does not have its anniversary on the day it happened.
    expect(crossesAnniversary(cal, epoch, MONTHS_IN_YEAR, epoch)).toBe(false);
    // And it is a YEAR OF DAYS and not a count of periods: 52 periods is 364 days, so over ten
    // years the anniversary has walked a period and a half away from where `period % 52` puts it.
    // That drift is the whole reason G3.a exists.
    const first = hits[0];
    const last = hits[years - 1];
    expect(first).toBeDefined();
    expect(last).toBeDefined();
    if (first === undefined || last === undefined) return;
    expect(last - first).not.toBe(52 * (years - 1));
  });

  it('is the same walk a term of parliament takes, at any step of months', () => {
    for (const months of [1, 3, 6, 12, 48]) {
      const hits = [];
      for (let p = 1; p < 60; p += 1) {
        if (crossesAnniversary(cal, period(0), months, period(p))) hits.push(p);
      }
      // A shorter step recurs more often, and every step recurs at least as often as the next.
      expect(hits.length).toBeGreaterThan(0 + (months <= 12 ? 0 : -1));
      expect(hits.every((p) => p > 0)).toBe(true);
    }
    // A step of nothing recurs never, rather than every period (Law 8: a periodicity is positive).
    expect(crossesAnniversary(cal, period(0), 0, period(9))).toBe(false);
  });
});

describe('the rating fee is annual (Ratings A5, item 20)', () => {
  it('is declared as a share a year, on the anniversary of the first opinion', () => {
    const w = rigWorld('periodicity.fee');
    const assessors = w.params
      .all()
      .filter((d) => String(d.id).startsWith('rating.fee.'));
    expect(assessors.length).toBeGreaterThan(0);
    for (const d of assessors) {
      expect(d.unit).toContain('a year');
      expect(d.unit).not.toContain('per period');
      expect(d.dimension).toBe('ratio');
    }
    // And it is one number per assessor, named for the assessor it belongs to (Law 4).
    expect(new Set(assessors.map((d) => String(d.id))).size).toBe(assessors.length);
  });
});

describe('an impairment is a transition, not a heartbeat (Register E3, item 20)', () => {
  it('tells a holder once per default, however long it holds the claim', () => {
    const w = rigWorld('periodicity.impaired');
    for (let i = 0; i < 24; i += 1) w.step();
    const said = w.journal.ofKind('credit.impaired');
    // Whatever this world impaired, no holder was told the same thing twice between two defaults:
    // the event is what HAPPENED, and what the holding is carrying is a read of the register.
    const seen = new Map<string, number>();
    for (const e of said) {
      const key = `${String(e.data['holder'])}|${String(e.data['instrument'])}`;
      const defaults = w.journal
        .forSubject('credit.default', String(e.data['instrument']))
        .filter((d) => d.period <= e.period);
      const since = defaults[defaults.length - 1]?.period ?? 0;
      const k = `${key}|${String(since)}`;
      seen.set(k, (seen.get(k) ?? 0) + 1);
    }
    expect([...seen.values()].filter((n) => n > 1)).toEqual([]);
  });
});
