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
 * ONE READ. The parse is `registry/statements.ts`'s — the file that settles what a statement's
 * fields MEAN, so the writer and every reader share one vocabulary (17.0a) — and this is the
 * kernel's READ FACE onto it. It is total, and a record that does not parse THROWS rather than
 * disappearing: `reporting` is the one writer of this event and a malformed one is its defect, not
 * a fact about the company (Law 4, and Missing is Missing — a dropped report is a `?? 0` in
 * disguise, since every reader treats "not there" as "never published").
 *
 * IT IS NOT A SECOND COPY. Nothing is stored: this reads the journal the writer wrote, at the read
 * (Law 19). And it carries only what `record(..., isPublic: true)` published, which is why a
 * PARTICIPANT may hold one — it is on `KernelReads`, so a bank valuing a borrower sees exactly what
 * the company told everybody and nothing else (Observer A3, A4).
 */
// Item 16: A PUBLISHED NUMBER RE-ENTERS THE TYPE SYSTEM HERE. `reporting` wrote it knowing what it
// was and a journal carries `unknown`, so the field this reads into goes through the dimension's
// own door and says which it is.
import { asCash, type Cash } from '../core/measure.js';
import { INTERIM, REPORT, type Statement, statementOf } from '../registry/statements.js';
import type { Period } from '../calendar/calendar.js';
import { InvalidRegistry } from '../core/errors.js';
import { partyId, type CurrencyCode, type PartyId } from '../core/ids.js';
import type { Event, Journal } from './journal.js';

/**
 * Reporting G2, G5: a set of accounts as the company published it. It is `registry/statements.ts`'s
 * `Statement` under the name its readers already had: what a statement IS belongs with the
 * vocabulary its writer writes it in, and a second definition here would be a second answer to what
 * a line means (Law 4).
 */
export type PublishedStatement = Statement;

/** Reporting B1: what a management said it expects, before the quarter it says it about closes. */
export interface PublishedGuidance {
  readonly company: PartyId;
  readonly quarter: string;
  /** What it guided the company will make IN A PERIOD. Money, like the statement's `earned`. */
  readonly perPeriod: Cash;
  /** What it guided for the whole span. Money. */
  readonly guided: Cash;
  readonly periods: number;
  readonly at: Period;
}

type JournalRead = Pick<Journal, 'ofKind' | 'lastOf'>;

export interface PublishedReads {
  /** A2: the last accounts this company published, or nothing if it never has. */
  lastStatement(company: PartyId): PublishedStatement | undefined;
  /**
   * A2, A3 (17b′.1): the last MANAGEMENT ACCOUNTS it prepared for a lender — a snapshot as at a
   * date that is not a fiscal close, which a company in its first year has and a report it has not.
   */
  lastInterim(company: PartyId): PublishedStatement | undefined;
  /**
   * Corporate Credit A4, Law 4 (17b′.2): THE FRESHEST BOOKS THIS COMPANY HAS, whichever act
   * produced them. *"What this company's books say"* is one question, and a lender that read the
   * older of two sets because it asked the wrong door would be pricing a name on stale accounts.
   */
  latestAccounts(company: PartyId): PublishedStatement | undefined;
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
    lastInterim: (company: PartyId) => {
      const e = journal.lastOf(INTERIM, company);
      return e === undefined ? undefined : statementOf(e);
    },
    latestAccounts: (company: PartyId) => {
      const report = journal.lastOf(STATEMENT, company);
      const interim = journal.lastOf(INTERIM, company);
      if (report === undefined) return interim === undefined ? undefined : statementOf(interim);
      if (interim === undefined) return statementOf(report);
      // The later of the two, and on the same date the QUARTER wins: a closed set of accounts is
      // what a snapshot was standing in for until it existed.
      return statementOf(interim.period > report.period ? interim : report);
    },
    statements: (company?: PartyId) =>
      forSubject(journal.ofKind(STATEMENT), company).map(statementOf),
    lastGuidance: (company: PartyId) => {
      const e = journal.lastOf(GUIDANCE, company);
      return e === undefined ? undefined : guidanceOf(e);
    },
    guidances: (company?: PartyId) => forSubject(journal.ofKind(GUIDANCE), company).map(guidanceOf),
  });
}

const STATEMENT = REPORT;
const GUIDANCE = 'reporting.guidance';

function forSubject(events: readonly Event[], company: PartyId | undefined): readonly Event[] {
  if (company === undefined) return events;
  return events.filter((e) => e.subjects[0] === String(company));
}

function guidanceOf(e: Event): PublishedGuidance {
  const ccy = str(e, 'ccy') as CurrencyCode;
  return {
    company: subject(e),
    quarter: str(e, 'quarter'),
    perPeriod: asCash(num(e, 'perPeriod'), ccy, 'what it guided it makes in a period'),
    guided: asCash(num(e, 'guided'), ccy, 'what it guided for the span'),
    periods: num(e, 'periods'),
    at: e.period,
  };
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
