/**
 * What a company published, read once and typed.
 *
 * @spec Reporting A2 Reporting A2.a Reporting G2 Reporting G5 Observer A3 Observer A4 Law 4 Law 19
 */
import { describe, expect, it } from 'vitest';
import { partyId } from '../src/core/ids.js';
import { publishedReads } from '../src/journal/published.js';
import { Journal } from '../src/journal/journal.js';
import { Calendar, period } from '../src/calendar/calendar.js';
import { ranWorld } from './rig.js';
import {
  BALANCE_LINES,
  CASH_LINES,
  EQUITY_LINES,
  INCOME_LINES,
  OCI_LINES,
  SUMMARY_LINES,
} from '../src/registry/statements.js';

/** Any calendar: the read does not consult one, and a cycle is only constructible from one. */
const CYCLE = new Calendar({ epoch: { y: 2026, m: 1, d: 1 }, periodDays: 7, cyclesPerPeriod: 2 }).cycle(0);

/**
 * A statement as `reporting` writes one: every section, every line (17.0a). The test states it in
 * full rather than the four fields the old readers happened to want — which is the point of there
 * being one parse.
 */
const zeros = (keys: readonly string[]): Record<string, number> =>
  Object.fromEntries(keys.map((k) => [k, 0]));

const FULL = {
  company: 'firm.1',
  quarter: '2026Q1',
  from: 0,
  to: 12,
  ccy: 'USD',
  earned: 500,
  revaluation: 20,
  income: { ...zeros(INCOME_LINES), revenue: 500, writeUps: 20 },
  oci: { ...zeros(OCI_LINES), otherMarks: 20 },
  cashFlow: { ...zeros(CASH_LINES), operatingIn: 500 },
  balance: { ...zeros(BALANCE_LINES), assets: 9000, liabilities: 4000 },
  equityChanges: { ...zeros(EQUITY_LINES), earned: 500 },
  summary: { ...zeros(SUMMARY_LINES), ebitda: 500 },
  byProduct: [],
  byGeography: [],
  holdings: [],
  inventories: [],
  plant: [],
  debt: [],
  leases: [],
  commitments: [],
  guarantees: [],
  derivatives: [],
  deals: [],
  employees: [],
  shares: { line: 'share.firm.1', issued: 100, holders: 2, dividendsDeclared: 0, dividendsPaid: 0, boughtBack: 0, earningsPerShare: 5 },
  relatedParties: { controller: null, subsidiaries: [], banks: [], largestCustomer: null, largestCustomerShare: null },
  subsequent: [],
  regulatory: {},
  commentary: [],
  currencies: [],
};

function withReport(data: Record<string, unknown>): Journal {
  const j = new Journal();
  j.record(period(3), CYCLE, 'reporting.report', ['firm.1'], data, true);
  return j;
}

describe('one parse of what a company published (Law 4)', () => {
  it('reads every line the writer wrote, and derives the span rather than storing it', () => {
    const said = publishedReads(withReport(FULL)).lastStatement(partyId('firm.1'));
    expect(said?.quarter).toBe('2026Q1');
    expect(said?.earned.pieces).toBe(500);
    expect(said?.balance.assets.pieces).toBe(9000);
    expect(said?.notes.shares?.issued).toBe(100);
    // Law 19: 13 periods, from the two dates the writer published. Not a stored quotient (Law 2).
    expect(said?.periods).toBe(13);
    // A2: WHEN it published, which is not when the quarter closed.
    expect(said?.preparedIn).toBe(3);
    // G2: the lines are NAMED, so a reader asks for one rather than parsing a sentence.
    expect(said?.income.revenue.pieces).toBe(500);
  });

  it('a company that has never reported is not a company that reported nothing', () => {
    expect(publishedReads(new Journal()).lastStatement(partyId('firm.1'))).toBeUndefined();
    expect(publishedReads(new Journal()).statements()).toEqual([]);
  });

  it('THROWS on a record its writer did not fill, rather than dropping it', () => {
    /**
     * This is the change. Eight sites checked `typeof earned !== 'number'` by hand and four of them
     * did `continue` — so a statement whose assets went missing vanished from the covenant test and
     * stayed visible to the analyst, and nothing anywhere said so. `reporting` is the ONE WRITER of
     * this event (Law 4), so a field it did not write is its defect: naming it at the site is the
     * difference between a broken writer and a company that has not reported.
     */
    for (const field of ['earned', 'revaluation', 'from', 'to']) {
      const missing = { ...FULL, [field]: undefined };
      expect(() => publishedReads(withReport(missing)).lastStatement(partyId('firm.1'))).toThrow(
        new RegExp(field),
      );
    }
    for (const field of ['quarter', 'ccy', 'company']) {
      const missing = { ...FULL, [field]: undefined };
      expect(() => publishedReads(withReport(missing)).lastStatement(partyId('firm.1'))).toThrow(
        new RegExp(field),
      );
    }
    // And a SECTION its writer did not fill is the same defect, named by its own name.
    for (const section of ['income', 'balance', 'cashFlow', 'summary']) {
      expect(() =>
        publishedReads(withReport({ ...FULL, [section]: undefined })).lastStatement(partyId('firm.1')),
      ).toThrow(new RegExp(section));
    }
    // Missing is Missing: a NaN is not a number here either, whatever `typeof` says.
    expect(() =>
      publishedReads(withReport({ ...FULL, earned: Number.NaN })).lastStatement(partyId('firm.1')),
    ).toThrow(/earned/);
  });

  it('the last one is the last one, and the history is in order', () => {
    const j = new Journal();
    j.record(period(1), CYCLE, 'reporting.report', ['firm.1'], { ...FULL, quarter: 'a' }, true);
    j.record(period(2), CYCLE, 'reporting.report', ['firm.2'], { ...FULL, quarter: 'x' }, true);
    j.record(period(3), CYCLE, 'reporting.report', ['firm.1'], { ...FULL, quarter: 'b' }, true);
    const r = publishedReads(j);
    expect(r.lastStatement(partyId('firm.1'))?.quarter).toBe('b');
    expect(r.statements(partyId('firm.1')).map((s) => s.quarter)).toEqual(['a', 'b']);
    // No company named: every company's, which is the read `research` uses to find who to cover.
    expect(r.statements().length).toBe(3);
  });
});

describe('what a real world publishes, through the one read', () => {
  it('every statement it publishes parses, which is the assertion the hand-parses could not make', () => {
    /**
     * The old readers dropped what they could not parse, so a malformed statement was INVISIBLE:
     * no test could see one because every reader agreed to look away. The read throws now, so
     * merely asking for them is the check — and it covers every field, not the three or four each
     * site happened to want.
     */
    const w = ranWorld('reporting', 30, 4, 40);
    const all = w.published.statements();
    expect(all.length).toBeGreaterThan(0);
    for (const said of all) {
      expect(said.periods).toBeGreaterThan(0);
      expect(said.quarter.length).toBeGreaterThan(0);
      // G5: shares in issue where there is a listed line, so a reader can divide.
      if (said.notes.shares !== null) expect(said.notes.shares.issued).toBeGreaterThan(0);
      // A2.a: the accounts are ONE set. What it published is what its equity account says, so the
      // bottom line and the part nobody was paid are the same number the audit proves.
      expect(Number.isFinite(said.earned.pieces)).toBe(true);
      expect(Number.isFinite(said.revaluation.pieces)).toBe(true);
    }
  });

  it('a participant may hold it, and it carries nothing that was not published (Observer A3, A4)', () => {
    const w = ranWorld('reporting', 30, 4, 40);
    const anyone = w.parties.all().find((p) => p.status.alive);
    if (anyone === undefined) return;
    const view = w.participantView(anyone.id);
    // The door is on the KERNEL read, so a bank valuing a borrower sees what the company told
    // everybody — and the read reaches two event kinds, both of them recorded public.
    expect(view.published.statements().length).toBe(w.published.statements().length);
  });
});
