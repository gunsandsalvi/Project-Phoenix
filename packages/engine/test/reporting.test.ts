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
import { about } from '../src/world/context.js';

function reportsOf(w: World): readonly Event[] {
  return w.journal.ofKind('reporting.report');
}

/** A1.a: the ones anybody may read — the companies with paper outside holders hold. */
function publicReportsOf(w: World): readonly Event[] {
  return reportsOf(w).filter((e) => e.public);
}

/**
 * 17.0a: one named line of one section of a statement. The parse is the kernel's (`published`);
 * what a test reads off the RECORD it reads by name, because a section is a fixed vocabulary and a
 * line that is not there is the writer's defect rather than a zero (Missing is Missing).
 */
function lineIn(e: Event, section: string, line: string): number {
  const rows = e.data[section] as Record<string, unknown> | undefined;
  const v = rows === undefined ? undefined : rows[line];
  if (typeof v !== 'number') throw new Error(`${e.kind} has no ${section}.${line}`);
  return v;
}

const sectionOf = (e: Event, name: string): Record<string, number> => {
  const rows = e.data[name];
  if (typeof rows !== 'object' || rows === null) throw new Error(`${e.kind} has no ${name}`);
  return Object.fromEntries(
    Object.entries(rows as Record<string, unknown>).filter(([, v]) => typeof v === 'number'),
  ) as Record<string, number>;
};

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
    const out = publishableOn(q, w.params.days(REPORTING_PARAMS.lag));
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

describe('who reports, and who may read it (Reporting A1, A1.a, G4; Observer A3, A4)', () => {
  it('has every company prepare, and publishes only what somebody outside holds paper of (17.0a)', () => {
    const w = ran('reporting', 30);
    const reports = reportsOf(w);
    expect(reports.length).toBeGreaterThan(0);
    // A1.a: PUBLIC is a state read from the register, and it is not "listed" — a company whose
    // BONDS somebody outside holds owes its holders the same accounts (17.0a, the owner's rule).
    for (const r of publicReportsOf(w)) {
      const company = String(r.data['company']);
      const outside = w.instruments
        .all()
        .filter(
          (i) =>
            i.issuer.some &&
            String(i.issuer.value) === company &&
            i.market.some &&
            w.registry.instrumentKind(i.kind).pricing === 'cleared',
        )
        .flatMap((i) => w.register.holdersOf(i.id))
        .filter((h) => String(h) !== company);
      expect(outside.length, `${company} published with nobody outside holding its paper`)
        .toBeGreaterThan(0);
    }
    // And every company prepares one, published or not: the books exist either way, which is what
    // makes a private company's accounts something it can OPEN to a lender (17.0a).
    const prepared = new Set(reports.map((r) => String(r.data['company'])));
    const published = new Set(publicReportsOf(w).map((r) => String(r.data['company'])));
    expect(prepared.size).toBeGreaterThan(published.size);
  });

  it('shows a private company’s accounts to its lenders of record and to nobody else (A4)', () => {
    const w = ran('reporting', 30);
    const privateReport = reportsOf(w).find((e) => !e.public);
    expect(privateReport, 'no company kept its accounts to itself').toBeDefined();
    const company = partyId(String(privateReport!.data['company']));
    // The disclosures name the two sides and nobody else: a party that was not shown it cannot
    // reach it, however public the journal is (Observer A4).
    const shown = w.journal
      .ofKind('disclosed')
      .filter((e) => e.data['from'] === String(company) && e.data['kind'] === 'reporting.report');
    expect(shown.length, 'a private company showed its books to nobody').toBeGreaterThan(0);
    // The REASON is read at the state it was read at: a loan since repaid and an account since
    // closed are both real, and a disclosure made when the relationship stood is not retracted by
    // its ending. So the reason is checked for the disclosures of the last period, against the
    // state that stands now — like with like.
    const latest = w.journal
      .ofKind('disclosed')
      .filter((e) => e.data['kind'] === 'reporting.report' && e.period === w.period);
    expect(latest.length, 'nobody was shown anything this period').toBeGreaterThan(0);
    for (const d of latest) {
      const from = partyId(String(d.data['from']));
      const to = partyId(String(d.data['to']));
      // Banks Lending A5: whoever it owes a bilateral claim to, or the bank that keeps its account.
      const lends = w.instruments
        .all()
        .some(
          (i) =>
            i.issuer.some &&
            String(i.issuer.value) === String(from) &&
            !i.market.some &&
            w.register.holdersOf(i.id).some((h) => String(h) === String(to)),
        );
      const banksIt = w.register.holdingsOf(from).some((h) => {
        const i = w.instruments.get(h.instrument);
        return (
          w.registry.instrumentKind(i.kind).pricing === 'money' &&
          i.issuer.some &&
          String(i.issuer.value) === String(to)
        );
      });
      // Ratings A5: and the assessor it PAYS for an opinion is shown them too — that is what the
      // fee buys, and an assessor grading books nobody opened would be reading private state.
      const rates = w.journal
        .ofKind('rating.action')
        .some((e) => e.subjects[0] === String(to) && e.subjects.includes(String(from)));
      expect(
        lends || banksIt || rates,
        `${String(from)} showed its books to ${String(to)} for no reason`,
      ).toBe(true);
    }
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
    // 17.0a: the lines are NAMED — what the money WAS, from the receipt the wire wrote on it — so
    // a reader asks for revenue or for wages rather than parsing the sentence somebody typed.
    const income = sectionOf(r!, 'income');
    const lines = Object.entries(income).map(([cause, amount]) => ({ cause, amount }));
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
    // And the marks are separable, because they are the part nobody was paid: the write-downs and
    // write-ups are the same entries the comprehensive-income section cuts by WHAT was marked, and
    // both come to what `revaluation` says (Law 4: one set of entries, read once, cut two ways).
    const marks = lineIn(r!, 'income', 'writeDowns') + lineIn(r!, 'income', 'writeUps');
    const revalued = Number(r!.data['revaluation']);
    expect(withinDust(marks, revalued, dustOf(3, Math.abs(marks) + Math.abs(revalued)))).toBe(true);
    const byWhat =
      lineIn(r!, 'oci', 'fxTranslation') +
      lineIn(r!, 'oci', 'unrealisedOnSecurities') +
      lineIn(r!, 'oci', 'otherMarks');
    expect(withinDust(byWhat, revalued, dustOf(4, Math.abs(byWhat) + Math.abs(revalued)))).toBe(
      true,
    );
  });

  it('carries the statement of changes in equity, and its identity holds (G2)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w).at(-1);
    const opening = lineIn(r!, 'equityChanges', 'opening');
    const closing = lineIn(r!, 'equityChanges', 'closing');
    const earned = Number(r!.data['earned']);
    // opening + earned = closing. The dividend and share lines are components OF earned, named
    // because a reader wants them by name — never additions to it.
    const scale = Math.abs(opening) + Math.abs(earned) + Math.abs(closing);
    expect(withinDust(opening + earned, closing, dustOf(3, scale))).toBe(true);
  });

  it('states the sums a covenant reads, each of its own named lines (Corporate Credit A3.b)', () => {
    const w = ran('reporting', 30);
    for (const r of reportsOf(w)) {
      // A3.a: what its debt took is what it PAID — interest plus principal, both real payments.
      const service = -lineIn(r, 'cashFlow', 'interestPaid') - lineIn(r, 'cashFlow', 'principalRepaid');
      const said = lineIn(r, 'summary', 'service');
      expect(withinDust(said, service, dustOf(3, Math.abs(said) + Math.abs(service)))).toBe(true);
      const net = lineIn(r, 'balance', 'debt') - lineIn(r, 'balance', 'cash');
      const netSaid = lineIn(r, 'summary', 'netDebt');
      expect(withinDust(netSaid, net, dustOf(3, Math.abs(netSaid) + Math.abs(net)))).toBe(true);
    }
  });

  it('carries the SAME balance sheet the accounts family checks the equity account against (Law 4)', () => {
    // The report is published in its own period and the sheet is read at that instant, so this
    // compares like with like only for one published in THIS period — which is enough to say the
    // two readers are one function. A report with its own implementation could agree here and
    // drift anywhere else. Its OWN world, stepped until a quarter closes: somebody's does within
    // the thirteen periods a quarter takes.
    const w = ranWorld('reporting-sheet', 20, 4, 40);
    let r: Event | undefined;
    for (let i = 0; i < 14 && r === undefined; i += 1) {
      w.step();
      r = reportsOf(w).filter((e) => e.period === w.period).at(-1);
    }
    expect(r, 'no company reported in fourteen periods').toBeDefined();
    const company = partyId(r!.subjects[0]!);
    const sheet = balanceSheet(w, company);
    const assets = lineIn(r!, 'balance', 'assets');
    expect(String(r!.data['ccy'])).toBe(sheet.ccy);
    expect(
      withinDust(
        assets,
        sheet.assets.value.pieces,
        dustOf(2, Math.abs(assets) + Math.abs(sheet.assets.value.pieces)),
      ),
    ).toBe(true);
  });

  it('states the cash by named counterparty, in the money it moved (A2, Law 9)', () => {
    const w = ran('reporting', 30);
    // A quarter in which nothing moved between this company and anybody has no lines, which is an
    // answer; what is asserted is what a statement that HAS them says.
    const r = reportsOf(w).find((e) => (e.data['cash'] as unknown[]).length > 0);
    expect(r, 'no company moved money with anybody in a whole quarter').toBeDefined();
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
    const r = publicReportsOf(w).find((e) => e.data['shares'] !== null);
    expect(r, 'no listed company published').toBeDefined();
    const shares = r!.data['shares'] as { issued: number; holders: number };
    expect(shares.issued).toBeGreaterThan(0);
    // Earnings per share is income over shares, both of them reads. A stored quotient would be an
    // outcome written down (Law 2), so the report carries the two and never their ratio.
    const keys = JSON.stringify(r!.data);
    expect(/perShare|earningsPerShare|"eps"/i.test(keys)).toBe(false);
  });

  it('carries every section a reader of a quarterly report opens it for (17.0a)', () => {
    const w = ran('reporting', 30);
    const r = reportsOf(w).at(-1);
    expect(r).toBeDefined();
    // The sections, by name. A report missing one is not a shorter report — it is a set of accounts
    // whose reader has to go and find the fact somewhere else, which is what 17.0a removed.
    for (const section of [
      'income',
      'oci',
      'cashFlow',
      'balance',
      'equityChanges',
      'summary',
      'byProduct',
      'byGeography',
      'cash',
      'holdings',
      'inventories',
      'plant',
      'debt',
      'leases',
      'commitments',
      'guarantees',
      'derivatives',
      'deals',
      'employees',
      'relatedParties',
      'subsequent',
      'regulatory',
      'commentary',
      'currencies',
    ]) {
      expect(r!.data[section], `a statement with no ${section}`).toBeDefined();
    }
    // A4.a: what management says about what comes next is its OWN outlook, in the numbers its own
    // decisions read (B4) — never a second figure composed for the audience.
    const commentary = r!.data['commentary'] as { on: string; expected: number }[];
    expect(Array.isArray(commentary)).toBe(true);
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
      const own = w.participantView(firm).outlook(about({ on: 'income' }));
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
    // B1: a management guides where its shares trade — that is the audience the guidance is for.
    const guided = w.journal.ofKind('reporting.guidance')[0];
    expect(guided, 'no management guided at all').toBeDefined();
    const company = partyId(guided!.subjects[0]!);
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
