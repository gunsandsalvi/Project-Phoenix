/**
 * What a company published, read once and typed, instead of eight times and by hand.
 *
 * @spec Reporting A2 Reporting A2.a Reporting B1 Reporting G2 Reporting G5 Observer A3 Law 4 Law 19
 *
 * §48's output is not private — `publish` records the whole statement to the JOURNAL, and the
 * journal is a kernel store anybody may read. What was missing is a TYPE for it. So eight sites
 * pulled `unknown` out of `e.data`, checked `typeof earned !== 'number'` by hand, and each recovered
 * a different subset of the same fact:
 *
 * | site | what it took | what it did with a record it could not parse |
 * | --- | --- | --- |
 * | `world.ts:worthReads` | earned, from, to | `none()` |
 * | `control/index.ts` | earned, from, to | `undefined` |
 * | `corporate-bond/index.ts` | quarter, assets, liabilities, earned | `undefined` |
 * | `reporting/guidance.ts` | quarter, earned | `continue` |
 * | `research/index.ts` (×2) | subject; earned, from, to | `continue` |
 * | `research/estimate.ts` (×2) | earned, from, to; quarter, earned | `continue` |
 *
 * That is A-52's shape — a fact recovered by taking loosely-typed data apart — and Law 4's: eight
 * parses of one thing, four of which silently drop a report they cannot read, and no two agreeing
 * on what a readable one is. A statement whose `assets` went missing would vanish from the covenant
 * test and stay visible to the analyst, and nothing anywhere would say so.
 *
 * ONE READ. The parse is here, it is total, and a record that does not parse THROWS rather than
 * disappearing: `reporting` is the one writer of this event and a malformed one is its defect, not
 * a fact about the company (Law 4, and Missing is Missing — a dropped report is a `?? 0` in
 * disguise, since every reader treats "not there" as "never published").
 *
 * IT IS NOT A SECOND COPY. Nothing is stored: this reads the journal the writer wrote, at the read
 * (Law 19). And it carries only what `record(..., isPublic: true)` published, which is why a
 * PARTICIPANT may hold one — it is on `KernelReads`, so a bank valuing a borrower sees exactly what
 * the company told everybody and nothing else (Observer A3, A4).
 */
import type { Period } from '../calendar/calendar.js';
import { InvalidRegistry } from '../core/errors.js';
import { partyId, type CurrencyCode, type PartyId } from '../core/ids.js';
import type { Event, Journal } from './journal.js';

/** G2: one cause of the equity account's movement, with how many entries made it up. */
export interface PublishedIncomeLine {
  readonly cause: string;
  readonly amount: number;
  readonly entries: number;
}

/** Reporting G2, G5: a set of accounts as the company published it, with the date it published. */
export interface PublishedStatement {
  readonly company: PartyId;
  /** B1: the period it covers, named the way the company names it. */
  readonly quarter: string;
  readonly from: Period;
  readonly to: Period;
  /** G5: how many periods the quarter covered, so a reader can annualise without re-deriving it. */
  readonly periods: number;
  /** G2: the bottom line, and the part of it nobody was paid. */
  readonly earned: number;
  readonly revaluation: number;
  /** G2: the decomposition, in the words its writers used. What the lines say is not re-derived. */
  readonly income: readonly PublishedIncomeLine[];
  readonly assets: number;
  readonly liabilities: number;
  readonly ccy: CurrencyCode;
  /** G5: shares in issue, so a reader can divide. The quotient is never stored (Law 2). */
  readonly shares: number;
  /** When it was published, which is not when the quarter closed (A2: reporting has a lag). */
  readonly at: Period;
}

/** Reporting B1: what a management said it expects, before the quarter it says it about closes. */
export interface PublishedGuidance {
  readonly company: PartyId;
  readonly quarter: string;
  readonly perPeriod: number;
  readonly guided: number;
  readonly periods: number;
  readonly at: Period;
}

type JournalRead = Pick<Journal, 'ofKind' | 'lastOf'>;

export interface PublishedReads {
  /** A2: the last accounts this company published, or nothing if it never has. */
  lastStatement(company: PartyId): PublishedStatement | undefined;
  /** Every set it has published, oldest first. Pass no company for every company's. */
  statements(company?: PartyId): readonly PublishedStatement[];
  lastGuidance(company: PartyId): PublishedGuidance | undefined;
  guidances(company?: PartyId): readonly PublishedGuidance[];
}

export function publishedReads(journal: JournalRead): PublishedReads {
  return Object.freeze({
    lastStatement: (company: PartyId) => {
      const e = journal.lastOf(STATEMENT, company);
      return e === undefined ? undefined : statementOf(e);
    },
    statements: (company?: PartyId) => forSubject(journal.ofKind(STATEMENT), company).map(statementOf),
    lastGuidance: (company: PartyId) => {
      const e = journal.lastOf(GUIDANCE, company);
      return e === undefined ? undefined : guidanceOf(e);
    },
    guidances: (company?: PartyId) => forSubject(journal.ofKind(GUIDANCE), company).map(guidanceOf),
  });
}

const STATEMENT = 'reporting.report';
const GUIDANCE = 'reporting.guidance';

function forSubject(events: readonly Event[], company: PartyId | undefined): readonly Event[] {
  if (company === undefined) return events;
  return events.filter((e) => e.subjects[0] === String(company));
}

function statementOf(e: Event): PublishedStatement {
  const from = num(e, 'from');
  const to = num(e, 'to');
  return {
    company: subject(e),
    quarter: str(e, 'quarter'),
    from: from as Period,
    to: to as Period,
    // Law 19: derived at the read from the two dates the writer published, never stored beside them.
    periods: to - from + 1,
    earned: num(e, 'earned'),
    revaluation: num(e, 'revaluation'),
    income: lines(e),
    assets: num(e, 'assets'),
    liabilities: num(e, 'liabilities'),
    ccy: str(e, 'ccy') as CurrencyCode,
    shares: num(e, 'shares'),
    at: e.period,
  };
}

function guidanceOf(e: Event): PublishedGuidance {
  return {
    company: subject(e),
    quarter: str(e, 'quarter'),
    perPeriod: num(e, 'perPeriod'),
    guided: num(e, 'guided'),
    periods: num(e, 'periods'),
    at: e.period,
  };
}

/** G2: the income lines, each read the same total way as every other field (Law 4). */
function lines(e: Event): readonly PublishedIncomeLine[] {
  const rows = e.data['income'];
  if (!Array.isArray(rows)) {
    throw new InvalidRegistry('Reporting G2', `${e.kind} has no income lines`);
  }
  return rows.map((r) => {
    const row = r as Record<string, unknown>;
    const cause = row['cause'];
    const amount = row['amount'];
    const entries = row['entries'];
    if (typeof cause !== 'string' || typeof amount !== 'number' || typeof entries !== 'number') {
      throw new InvalidRegistry('Reporting G2', `${e.kind} has an income line it did not write`);
    }
    return { cause, amount, entries };
  });
}

/**
 * Law 4: `reporting` is the one writer of these events, so a field it did not write is ITS defect
 * and throwing names it at the site. Every reader used to drop such a record silently, which made
 * a broken writer indistinguishable from a company that had not reported.
 */
function num(e: Event, field: string): number {
  const v = e.data[field];
  if (typeof v !== 'number' || !Number.isFinite(v)) {
    throw new InvalidRegistry('Reporting A2', `${e.kind} has no finite ${field}: ${String(v)}`);
  }
  return v;
}

function str(e: Event, field: string): string {
  const v = e.data[field];
  if (typeof v !== 'string' || v.length === 0) {
    throw new InvalidRegistry('Reporting A2', `${e.kind} has no ${field}: ${String(v)}`);
  }
  return v;
}

function subject(e: Event): PartyId {
  const who = e.subjects[0];
  if (who === undefined) {
    throw new InvalidRegistry('Reporting A2', `${e.kind} names no company`);
  }
  return partyId(who);
}
