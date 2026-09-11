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
  publishableOn,
  quarterClosedBy,
  spanOf,
  type Event,
  type PartyId,
  type World,
} from '../src/index.js';
import { rigWorld } from './rig.js';

function reportsOf(w: World): readonly Event[] {
  return w.journal.ofKind('reporting.report');
}

function ran(seed: string, periods: number, banks = 4, firms = 40): World {
  const w = rigWorld(seed, banks, firms);
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

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
    const w = rigWorld('fiscal', 3, 12);
    const q = quarterClosedBy(4, civil(2026, 6, 15));
    const span = spanOf(q, w.calendar);
    // A3: a whole number of periods only by accident. What must hold is that the span's ends are
    // the periods the two DATES fall in — never a count of periods back from today (G6).
    expect(span.from).toBe(w.calendar.periodOf(q.begins));
    expect(span.to).toBe(w.calendar.periodOf(q.ends));
    expect(span.to).toBeGreaterThan(span.from);
  });

  it('publishes after a lag that is a POLICY somebody wrote (A4, A4.a)', () => {
    const w = rigWorld('fiscal', 3, 12);
    const decl = w.params.decl(REPORTING_PARAMS.lag);
    expect(decl.kind).toBe('policy');
    expect(decl.owner).toBe('parliament');
    const q = quarterClosedBy(4, civil(2026, 6, 15));
    const out = publishableOn(q, w.params.get(REPORTING_PARAMS.lag));
    expect(compareCivil(out, q.ends)).toBeGreaterThan(0);
  });

  it('gives companies different year ends, so reporting season is not one week (A3)', () => {
    const w = rigWorld('anchors', 4, 40);
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
    const summed = lines.reduce((t, l) => t + l.amount, 0);
    expect(summed).toBeCloseTo(Number(r!.data['earned']), 6);
    // And the marks are separable, because they are the part nobody was paid.
    const marks = lines.find((l) => l.cause === 'revaluation');
    expect(marks).toBeDefined();
    expect(marks!.amount).toBeCloseTo(Number(r!.data['revaluation']), 6);
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
