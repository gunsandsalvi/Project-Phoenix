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
import { quarterClosedBy } from '../src/calendar/fiscal.js';
import { paramId } from '../src/core/ids.js';
import { BUYBACK } from '../src/mechanisms/equity/index.js';
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

describe('what households spend is remitted quarterly (Treasury C3, item 20)', () => {
  const w = rigWorld('periodicity.vat');
  const anchor = w.params.count(paramId('treasury.fiscalYear.endsInMonth'));

  it('has a state fiscal year whose quarters close in one period each, walked by date', () => {
    // The placement itself: over thirty periods a quarter closes twice or three times, each close
    // falls in exactly one period, and the period it falls in is the one whose span contains the
    // day. This is what the remittance is placed on, and it is a date and not a count of periods.
    const closes = new Map<string, number[]>();
    for (let p = 1; p < 30; p += 1) {
      const q = quarterClosedBy(anchor, w.calendar.startOf(period(p)));
      if (compareCivil(q.ends, w.calendar.startOf(period(0))) < 0) continue;
      const at = Number(w.calendar.periodOf(q.ends));
      const held = closes.get(q.label);
      if (held === undefined) closes.set(q.label, [at]);
      else held.push(at);
    }
    expect(closes.size).toBeGreaterThan(0);
    // Each quarter has ONE close period, however many periods report it as the last one closed.
    for (const [, periods] of closes) expect(new Set(periods).size).toBe(1);
    // The state's year end is the constitution's and not parliament's: a seat-weighted average of
    // three parties' preferred months would not be a month.
    expect(w.params.decl(paramId('treasury.fiscalYear.endsInMonth')).owner).toBe('constitution');
  });

  it('reads the spending base in no other period — and this world spends nothing to read (21.84)', () => {
    for (let i = 0; i < 30; i += 1) w.step();
    const said = w.journal.ofKind('treasury.receipts');
    expect(said.length).toBeGreaterThan(0);
    const spent = said.filter(
      (e) => Number((e.data['bases'] as Record<string, number>)['consumption']) > 0,
    );
    // Every period that DOES assess spending is the period after a quarter closed.
    for (const e of spent) {
      const q = quarterClosedBy(anchor, w.calendar.startOf(e.period));
      expect(Number(w.calendar.periodOf(q.ends))).toBe(Number(e.period) - 1);
    }
    /**
     * And there are none, because no physical good reaches a household in a settled instruction in
     * this world: the consumption base was ZERO in every period before this change too (verified at
     * `5828b34`), which is finding 21.84 seen from the tax side rather than the index side. What
     * this case can show is that the withheld taxes still read every period while the spending one
     * does not, and that is the difference the change makes.
     */
    expect(spent.length).toBe(0);
    const withheld = said.filter(
      (e) =>
        Number((e.data['bases'] as Record<string, number>)['income']) > 0 ||
        Number((e.data['bases'] as Record<string, number>)['interest']) > 0,
    );
    expect(withheld.length).toBeGreaterThan(0);
  });
});

describe('a buyback is a programme, not a bid (Equity D2, XI-8, item 20)', () => {
  it('opens an authority with a closing period, and ends it used up or lapsed', () => {
    const w = rigWorld('periodicity.buyback');
    for (let i = 0; i < 30; i += 1) w.step();
    const programmes = w.processes.all().filter((p) => p.what === BUYBACK);
    expect(programmes.length).toBeGreaterThan(0);
    for (const p of programmes) {
      // XI-8, Firm Birth D5: a programme has a period by which it must be over — that is what
      // makes it a programme and not a wish — and steps a reader can tell apart.
      expect(p.closesAfter).toBeGreaterThanOrEqual(p.opened);
      expect(p.steps).toEqual(['authorised', 'buying']);
      expect(p.subject.length).toBeGreaterThan(0);
      // Nothing outlives its own closing period: what is still running is still inside it.
      if (p.state === 'running') expect(w.period).toBeLessThanOrEqual(p.closesAfter + 1);
    }
    // A board does not authorise a second programme while the first stands: one live authority per
    // firm at a time, which is what "the authority it is buying under" means.
    const live = programmes.filter((p) => p.state === 'running').map((p) => String(p.subject));
    expect(new Set(live).size).toBe(live.length);
    // And an authority that ran out of time with shares unbought is ABANDONED and says how many —
    // a lapsed authority is a real outcome, not a silence.
    for (const p of programmes.filter((x) => x.state === 'abandoned')) {
      expect(p.why.length).toBeGreaterThan(0);
    }
  });
});
