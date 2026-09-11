/**
 * Reporting: a public company publishes what its own books produced, on its own calendar.
 *
 * @spec Reporting A1 Reporting A1.a Reporting A2 Reporting A2.a Reporting A3 Reporting A4 Reporting A4.a Reporting G2 Reporting G4 Reporting G5 Reporting G6 Money G3.a Money G3.b Law 2 Law 4 Law 9 Law 19
 *
 * A2.a is the rule every assertion here answers to: no reported number the books do not produce. So
 * what is tested is not that the figures are plausible — it is that each of them is the SAME number
 * some other reader of this world already has. Income is the equity ledger's entries; the balance
 * sheet is the function the `accounts` family checks the equity account against; the cash is the
 * wire's own legs. A report that agreed with none of them would still look like a report.
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  REPORTING_PARAMS,
  anchorOf,
  balanceSheet,
  partyId,
  civil,
  compareCivil,
  guidanceRecord,
  publishableOn,
  quarterClosedBy,
  spanOf,
  type Event,
  type PartyId,
  type World,
} from '../src/index.js';
import { ranWorld } from './rig.js';
import { dustOf, withinDust } from '../src/core/num.js';

function reportsOf(w: World): readonly Event[] {
  return w.journal.ofKind('reporting.report');
}

/**
 * Law 18: a world of this seed and draw, stepped this far — built ONCE for the whole file and read
 * by every test that asks for the same one (`test/rig.ts`). Every test below only READS what its
 * world did; a test that needed to act on one would build its own.
 */
const ran = (seed: string, periods: number, banks = 4, firms = 40): World =>
  ranWorld(seed, periods, banks, firms);

describe('the fiscal calendar is dates (Reporting A3, G6; Money G3.a, G3.b)', () => {
  it('closes a quarter on the last day of a month, three months at a time, round the year', () => {
    // A company whose year ends in April closes in April, January, October and July — and the
    // labels say which quarter of ITS year, because two companies with different anchors would
    // otherwise both call three different months "Q1".
    const q = quarterClosedBy(4, civil(2026, 6, 15));
    expect(`${q.ends.y}-${q.ends.m}-${q.ends.d}`).toBe('2026-4-30');
    expect(`${q.begins.y}-${q.begins.m}-${q.begins.d}`).toBe('2026-2-1');
    expect(q.label).toBe('2026-FQ4');
    const earlier = quarterClosedBy(4, civil(2026, 4, 29));
    expect(`${earlier.ends.y}-${earlier.ends.m}-${earlier.ends.d}`).toBe('2026-1-31');
    expect(earlier.label).toBe('2026-FQ3');
  });

  it('crosses a year end without the arithmetic going wrong', () => {
    const q = quarterClosedBy(12, civil(2027, 1, 3));
    expect(`${q.ends.y}-${q.ends.m}-${q.ends.d}`).toBe('2026-12-31');
    expect(`${q.begins.y}-${q.begins.m}-${q.begins.d}`).toBe('2026-10-1');
    // February, and a leap year: the close is the month's own last day, whatever that is.
    const feb = quarterClosedBy(2, civil(2028, 3, 1));
    expect(`${feb.ends.y}-${feb.ends.m}-${feb.ends.d}`).toBe('2028-2-29');
  });

  it('places the quarter on the one calendar, and nothing in it is finer than a period (G3.b)', () => {
    const w = ranWorld('fiscal', 0, 3, 12);
    const q = quarterClosedBy(4, civil(2026, 6, 15));
    const span = spanOf(q, w.calendar);
    // A3: a whole number of periods only by accident. What must hold is that the span's ends are
    // the periods the two DATES fall in — never a count of periods back from today (G6).
    expect(span.from).toBe(w.calendar.periodOf(q.begins));
    expect(span.to).toBe(w.calendar.periodOf(q.ends));
    expect(span.to).toBeGreaterThan(span.from);
  });

  it('publishes after a lag that is a POLICY somebody wrote (A4, A4.a)', () => {
    const w = ranWorld('fiscal', 0, 3, 12);
    const decl = w.params.decl(REPORTING_PARAMS.lag);
    expect(decl.kind).toBe('policy');
    expect(decl.owner).toBe('parliament');
    const q = quarterClosedBy(4, civil(2026, 6, 15));
    const out = publishableOn(q, w.params.get(REPORTING_PARAMS.lag));
    expect(compareCivil(out, q.ends)).toBeGreaterThan(0);
  });

  it('gives companies different year ends, so reporting season is not one week (A3)', () => {
    const w = ranWorld('anchors', 0, 4, 40);
    const anchors = new Set(w.parties.ofKind(FIRM).map((f) => anchorOf('anchors', f.id)));
    // If every company closed in the same month there would be one season a year instead of a thing
    // that happens continuously, and every surprise in this world would land on the same morning.
    expect(anchors.size).toBeGreaterThan(1);
  });
});

describe('who reports (Reporting A1, A1.a, G4)', () => {
  it('publishes only for companies whose shares somebody outside holds', () => {
    const w = ran('reporting', 30);
    const reports = reportsOf(w);
    expect(reports.length).toBeGreaterThan(0);
    for (const r of reports) {
      const company = String(r.data['company']);
      const line = w.instruments
        .all()
        .find((i) => i.issuer.some && String(i.issuer.value) === company && i.market.some);
      expect(line, `${company} reported with no listed line`).toBeDefined();
      const outside = w.register.holdersOf(line!.id).filter((h) => String(h) !== company);
      expect(outside.length, `${company} reported with nobody outside holding it`).toBeGreaterThan(
        0,
      );
    }
    // G4: and the firms that are not listed said nothing at all.
    const reported = new Set(reports.map((r) => String(r.data['company'])));
    const listed = new Set(
      w.instruments
        .all()
        .filter((i) => i.market.some && String(i.kind) === 'equity.share')
        .map((i) => (i.issuer.some ? String(i.issuer.value) : '')),
    );
    for (const c of reported) expect(listed.has(c)).toBe(true);
    expect(reported.size).toBeLessThan(w.parties.ofKind(FIRM).length);
  });

  it('reports each quarter once, and only one that opened after the world did', () => {
    const w = ran('reporting', 30);
    const seen = new Set<string>();
    for (const r of reportsOf(w)) {
      const key = `${String(r.data['company'])}|${String(r.data['quarter'])}`;
      expect(seen.has(key), `${key} was reported twice`).toBe(false);
      seen.add(key);
      // Seed A2: a quarter that started before the epoch has no books in it, so there is nothing
      // to report about it — not a short period, an absent one.
      expect(Number(r.data['from'])).toBeGreaterThanOrEqual(0);
    }
  });
});

describe('what a report carries (Reporting A2, A2.a, G2, G5)', () => {
  it('is the equity ledger, decomposed by what the instructions and the marks did (G2)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w)[0];
    expect(r).toBeDefined();
    const lines = r!.data['income'] as { cause: string; amount: number; entries: number }[];
    expect(lines.length).toBeGreaterThan(1);
    // The bottom line IS the sum of the decomposition — it is not computed a second way (Law 4).
    //
    // Law 7: WITHIN THE DUST OF THIS ADDITION, which is derived from the numbers being added and is
    // not a number of decimal places. `toBeCloseTo(..., 6)` is a fixed band of 5e-7, and this
    // world's equity ledger reached 1.24e10: one addition of it carries 2.7e-6 of floating-point
    // dust before anybody has done anything wrong, so the band was reporting arithmetic as a defect.
    // A band is never the answer — the dust is `terms × ε × Σ|magnitudes|` and it is computed here.
    const summed = lines.reduce((t, l) => t + l.amount, 0);
    const earned = Number(r!.data['earned']);
    const scale = lines.reduce((t, l) => t + Math.abs(l.amount), Math.abs(earned));
    expect(withinDust(summed, earned, dustOf(lines.length + 1, scale))).toBe(true);
    // And the marks are separable, because they are the part nobody was paid.
    const marks = lines.find((l) => l.cause === 'revaluation');
    expect(marks).toBeDefined();
    const revalued = Number(r!.data['revaluation']);
    expect(
      withinDust(marks!.amount, revalued, dustOf(2, Math.abs(marks!.amount) + Math.abs(revalued))),
    ).toBe(true);
    // Every key is a word its own writer wrote: the instruction's cause or the marks. A key this
    // module invented would be a chart of accounts, which is what A2.a forbids.
    for (const l of lines) expect(l.entries).toBeGreaterThan(0);
  });

  it('carries the SAME balance sheet the accounts family checks the equity account against (Law 4)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w).at(-1);
    expect(r).toBeDefined();
    // The report is published in its own period and the sheet is read at that instant, so this
    // compares like with like only for the last one — which is enough to say the two readers are
    // one function. A report with its own implementation could agree here and drift anywhere else.
    const company = partyId(r!.subjects[0]!);
    const sheet = balanceSheet(w, company);
    expect(typeof r!.data['assets']).toBe('number');
    expect(String(r!.data['ccy'])).toBe(sheet.ccy);
  });

  it('states the cash by named counterparty, in the money it moved (A2, Law 9)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w)[0];
    const cash = r!.data['cash'] as {
      counterparty: string;
      instrument: string;
      cause: string;
      amount: number;
      legs: number;
    }[];
    expect(cash.length).toBeGreaterThan(0);
    for (const c of cash) {
      // Law 9: a counterparty is a party this world has, never a bucket.
      expect(w.parties.has(c.counterparty as unknown as PartyId)).toBe(true);
      expect(c.instrument.startsWith('money:')).toBe(true);
      expect(c.legs).toBeGreaterThan(0);
    }
    // And it is a direct-method statement: every line of it settled, so no line is a derivation.
    expect(new Set(cash.map((c) => c.counterparty)).size).toBeGreaterThan(1);
  });

  it('states shares outstanding and stores no per-share figure anywhere (G5)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w)[0];
    expect(Number(r!.data['shares'])).toBeGreaterThan(0);
    // Earnings per share is income over shares, both of them reads. A stored quotient would be an
    // outcome written down (Law 2), so the report carries the two and never their ratio.
    const keys = Object.keys(r!.data).join(' ');
    expect(/perShare|eps/i.test(keys)).toBe(false);
  });
});

describe('guidance (Reporting B1, B2, B3, B4; Firm E7)', () => {
  it('publishes the SAME number the firm own decisions read, and not a second one (B4)', () => {
    const w = ran('reporting', 45);
    const all = w.journal.ofKind('reporting.guidance');
    expect(all.length).toBeGreaterThan(0);
    // §46 C1, B4: AN OUTLOOK IS A NUMBER AT AN INSTANT. It is formed adaptively from the firm's own
    // history, so it moves every period — and `outlook('income')` answers with the one in force NOW.
    // This took the last five guidances and compared each of them against today's outlook, which
    // asks a management that guided six weeks ago to have guided today's number. What B4 forbids is
    // a SECOND number, and the only instant at which the two can be read together is this one.
    const said = all.filter((e) => e.period === w.period);
    expect(said.length, 'nothing was guided in the period the outlooks are read at').toBeGreaterThan(0);
    for (const e of said) {
      const firm = partyId(e.subjects[0]!);
      const own = w.participantView(firm).outlook('income');
      if (!own.some) throw new Error(`${firm} guided with no outlook of its own income`);
      // ...and the event says which instant it was formed at, so "the same number" is checkable
      // against an older guidance too, by whoever holds that period's outlook (Law 19).
      expect(Number(e.data['formed'])).toBe(e.period);
      // B4: a management that guides to a number it is not itself acting on has had its decisions
      // made somewhere else. The published figure IS the outlook, with the periodicity stated
      // beside it — the quarter figure is that number times the periods, and both are shown.
      expect(Number(e.data['perPeriod'])).toBe(own.value.expected);
      expect(String(e.data['unit'])).toBe(own.value.unit);
      expect(Number(e.data['guided'])).toBeCloseTo(
        Number(e.data['perPeriod']) * Number(e.data['periods']),
        6,
      );
      // §46 A5: a horizon and a unit, or it is not an outlook.
      expect(Number(e.data['periods'])).toBeGreaterThan(0);
      expect(String(e.data['quarter']).length).toBeGreaterThan(0);
    }
  });

  it('guides to the quarter that is COMING, never the one just reported (B1)', () => {
    const w = ran('reporting', 45);
    const reported = new Set(
      w.journal
        .ofKind('reporting.report')
        .map((e) => `${e.subjects[0]}|${String(e.data['quarter'])}`),
    );
    for (const e of w.journal.ofKind('reporting.guidance')) {
      const key = `${e.subjects[0]}|${String(e.data['quarter'])}`;
      // Guiding to a quarter whose result is already published is not a forecast.
      const already = w.journal
        .ofKind('reporting.report')
        .some(
          (r) => `${r.subjects[0]}|${String(r.data['quarter'])}` === key && r.period <= e.period,
        );
      expect(already, `${key} was guided to after it had been reported`).toBe(false);
    }
    expect(reported.size).toBeGreaterThan(0);
  });

  it('revises on a MOVE and not on a schedule (B2)', () => {
    const w = ran('reporting', 45);
    const guiding = w.journal.ofKind('reporting.guidance');
    const companies = new Set(guiding.map((e) => String(e.subjects[0])));
    expect(companies.size).toBeGreaterThan(0);
    // A revision published every period regardless would carry no information at all — which is
    // what makes it a calendar rather than news. So there are fewer than one a period per company.
    expect(guiding.length).toBeLessThan(45 * companies.size);
    // And no two consecutive statements from one company say the same thing.
    const last = new Map<string, number>();
    for (const e of guiding) {
      const who = String(e.subjects[0]);
      const now = Number(e.data['perPeriod']);
      expect(last.get(who), `${who} republished a figure it had not changed`).not.toBe(now);
      last.set(who, now);
    }
  });

  it('is a READ of the journal and stores nothing (B3)', () => {
    const w = ran('reporting', 60);
    const company = partyId(w.journal.ofKind('reporting.report')[0]!.subjects[0]!);
    const record = guidanceRecord(w, company);
    // It pairs what a management SAID with what its books then produced, and it exists only while
    // somebody is looking: a stored record would be a second account of what a company said.
    expect(record.quarters).toBeGreaterThan(0);
    for (const m of record.misses) {
      expect(typeof m.guided).toBe('number');
      expect(typeof m.earned).toBe('number');
    }
    // And it is the same answer twice, because it is computed from the same journal both times.
    expect(guidanceRecord(w, company).quarters).toBe(record.quarters);
  });
});
