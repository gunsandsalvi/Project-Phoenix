/**
 * WHAT A COMPANY'S BOOKS PRODUCED, AND WHO MAY READ IT — one read per fact, for everybody who asks.
 *
 * @spec Reporting A1 Reporting A1.a Reporting A2 Reporting A2.a Reporting A4 Reporting A4.a Reporting B1 Reporting B4 Reporting G2 Reporting G5 Corporate Credit A3 Corporate Credit A3.a Corporate Credit A3.b Corporate Credit A4 Corporate Credit E9 Observer A3 Observer A4 Law 4 Law 8 Law 19
 *
 * THE OWNER'S RULE (17.0a): every company prepares FULL quarterly financials; they are PUBLISHED or
 * SHARED only when there is a reason. Full is what a reader of a quarterly report actually reads
 * (`docs/IMPLEMENTATION.md` 17.0a lists the sections and where each comes from): the income
 * statement with its lines attributed by what the money WAS, other comprehensive income, the
 * balance sheet with every position and its fair-value level, the changes in equity, the cash
 * flow statement, the segments by product and geography, the debt schedule, the leases and other
 * commitments, the derivatives and guarantees, the deals, the employees, the share capital, the
 * related parties, the subsequent events, what the party published about its own regulation, the
 * management's own outlook and key figures, and the sums a covenant reads.
 *
 * EVERY LINE IS A READ (A2). Nothing here derives anything and nothing is composed: this file
 * settles the SHAPE — what each field means — so a lender, an assessor and a covenant cannot read
 * one statement two ways (Law 4). A line the world has no mechanism for is not here, and the plan
 * says which item builds it, rather than a figure standing in for it.
 *
 * MONEY ON THE RECORD IS PIECES beside the statement's one currency (Law 8): the journal refuses a
 * `Cash` value in event data, so the statement is written as `StatementRecord` (numbers) and read
 * back as `Statement` (money) through `statementOf`, the one parser.
 *
 * WHO MAY READ IT is a separate fact: public because the company has paper anybody can buy (A1.a),
 * or disclosed to a named party because that party has a reason to see it — it lends to the
 * company, keeps its account, is asked for a loan, is paid for an opinion (§44 A5).
 */
import { asCash, type Cash } from '../core/measure.js';
import { InvalidRegistry } from '../core/errors.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import { period, type Period } from '../calendar/calendar.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';

/** The kind every company's quarterly financials are written under. Named HERE, once. */
export const REPORT = 'reporting.report';

/**
 * Reporting A2, A3, Corporate Credit A4 (17b′.1): MANAGEMENT ACCOUNTS — the same statement over a
 * span that is not a fiscal quarter, prepared because a LENDER asked to see the books.
 *
 * A company's first fiscal close is one to four quarters after this world opens (a quarter that
 * began before the epoch is a quarter with no books in it), and in that window a company has
 * nothing to show anybody. A bank underwriting a commitment in it would be lending large on
 * nothing, which nobody does.
 *
 * It is a SECOND KIND and not a second `reporting.report`, because the two are different acts: a
 * report is what a company says about a quarter that closed, and this is what it hands one lender
 * as at today. Nothing may trade on it, nothing guides off it, and the covenant test tells them
 * apart by which it read. Same fields, same parser, same vocabulary (Law 4).
 */
export const INTERIM = 'reporting.interim';
/** The kernel's record that one party showed another one of its own events (Observer A3). */
export const DISCLOSED = 'disclosed';
/** §35: the events a deal leaves on the record, read here so no module names another's. */
const DEAL_KINDS = [
  'control.tender',
  'control.contested',
  'control.failed',
  'control.acquired',
  'control.combined',
  'control.owned',
] as const;

/* --- The lines ------------------------------------------------------------------------------ */

/**
 * G2: THE INCOME STATEMENT'S LINES — what each movement of the equity account WAS, read off the legs
 * of the instruction that moved it: the receipt the wire wrote on the money (`Receipt`), a create or
 * a destroy leg, a write-off at nothing, a mark. `other` is a movement whose legs said none of
 * these, kept so the lines still sum to what the account moved by.
 */
export const INCOME_LINES = [
  'revenue',
  'gainsOnDisposal',
  'production',
  'wagesReceived',
  'rentReceived',
  'interestReceived',
  'dividendsReceived',
  'taxReceived',
  'claimsReceived',
  'otherReceipts',
  'wages',
  'rent',
  'interest',
  'tax',
  'claimsPaid',
  'dividendsPaid',
  'otherPayments',
  'wear',
  'spoilage',
  'writeOffs',
  'writeDowns',
  'writeUps',
  'other',
] as const;
export type IncomeLine = (typeof INCOME_LINES)[number];

/** The cash statement, by what the wire said each payment was; every one a sum of settled money legs. */
export const CASH_LINES = [
  'operatingIn',
  'operatingOut',
  /** Money paid for plant that came onto the book (Capital Programme A6). */
  'capex',
  /** Money paid for claims bought, and received for claims sold. */
  'securitiesBought',
  'securitiesSold',
  /** Corporate Credit A3.a: interest actually paid; principal actually repaid. */
  'interestPaid',
  'principalRepaid',
  /** What lenders and investors put in: drawings, issues, contributions. */
  'financingIn',
  /** What went back to owners: dividends, capital returned, own shares bought. */
  'financingOut',
  /** Currency D2: what the marks did to the money it holds in other currencies. */
  'fxOnCash',
  'openingCash',
  'closingCash',
] as const;
export type CashLine = (typeof CASH_LINES)[number];

/** The balance sheet at the close, and the commitments beside it. */
export const BALANCE_LINES = [
  'assets',
  'liabilities',
  'equity',
  /** Money held, in every currency, stated in the company's own. */
  'cash',
  /** Claims held that a market prices or a lender carries: securities and loans, at their marks. */
  'securities',
  /** Physical goods held that are not plant, at cost. */
  'inventories',
  /** Plant at cost, and what the wear has taken off it. */
  'plantAtCost',
  'accumulatedWear',
  /** What it owes at face on the claims it issued that are not its money: loans, paper, bonds. */
  'debt',
  /** Rent committed per period under the leases it is tenant of, plant and premises alike. */
  'leaseRentPerPeriod',
  /** What the kernel marks its standing commitments at, as their debtor (Insurers B2). */
  'commitments',
] as const;
export type BalanceLine = (typeof BALANCE_LINES)[number];

/**
 * Changes in equity over the span. The identity is `opening + earned = closing` — `earned` is the
 * WHOLE movement of the account (G2) — and the other three are components of `earned` a reader of
 * this statement wants by name, not additions to it.
 */
export const EQUITY_LINES = [
  'opening',
  'earned',
  'dividendsPaid',
  'sharesIssued',
  'sharesBoughtBack',
  'closing',
] as const;
export type EquityLine = (typeof EQUITY_LINES)[number];

/**
 * Comprehensive income: THE SAME MARKS as `revaluation` and as the income statement's write-downs
 * and write-ups, cut by WHAT was marked rather than by which way it moved. It is a second cut of one
 * set of entries taken in the same read, never a second number that can drift from them (Law 4).
 */
export const OCI_LINES = ['fxTranslation', 'unrealisedOnSecurities', 'otherMarks'] as const;
export type OciLine = (typeof OCI_LINES)[number];

/** The sums a covenant reads, each a sum of named lines above (Law 4). */
export const SUMMARY_LINES = [
  /** Before interest, tax, wear, spoilage, write-offs and the marks; distributions excluded. */
  'ebitda',
  'freeCashFlow',
  /** Interest plus principal actually paid over the span (Corporate Credit A3.a). */
  'service',
  'netDebt',
] as const;
export type SummaryLine = (typeof SUMMARY_LINES)[number];

/* --- The notes ------------------------------------------------------------------------------ */

/** A segment by product: what was sold of one kind, what it fetched, what making it consumed. */
export interface ProductSegment {
  readonly kind: string;
  readonly revenue: number;
  readonly unitsSold: number;
  readonly unitsMade: number;
  readonly inputsConsumed: number;
}
export interface GeographySegment {
  readonly region: string;
  readonly revenue: number;
}
/**
 * Corporate Credit E9, Observer A1.a: HOW THE MARK WAS STRUCK, as the fair-value hierarchy names it.
 * A read of the print's provenance and never a judgement: printed this period; a print carried from
 * an earlier period; carried at cost or unpriced. The three are named once, here.
 */
export const FAIR_VALUE = { printedThisPeriod: 1, carriedPrint: 2, atCost: 3 } as const;
export type FairValueLevel = (typeof FAIR_VALUE)[keyof typeof FAIR_VALUE];

/** One position held, at its mark, with how the mark was struck: the fair-value level is a read of the print's provenance. */
export interface HoldingNote {
  readonly instrument: string;
  readonly kind: string;
  readonly ccy: string;
  readonly units: number;
  readonly carrying: number;
  readonly level: FairValueLevel;
  readonly markedIn: number;
  readonly encumbered: number;
}
/**
 * A2, Law 9: ONE LINE OF THE DIRECT-METHOD CASH STATEMENT — what moved between this company and ONE
 * named counterparty, in one money, for one kind of reason. A counterparty is named, never bucketed,
 * and every line of it settled, so no line is a derivation.
 */
export interface CashCounterpartyNote {
  readonly counterparty: string;
  readonly instrument: string;
  readonly cause: string;
  readonly amount: number;
  readonly legs: number;
}

export interface InventoryNote {
  readonly kind: string;
  readonly units: number;
  readonly cost: number;
}
export interface PlantNote {
  readonly instrument: string;
  readonly kind: string;
  readonly region: string;
  readonly inService: string;
  readonly retires: string;
  readonly units: number;
  readonly cost: number;
  readonly carrying: number;
}
export interface DebtNote {
  readonly instrument: string;
  readonly kind: string;
  readonly ccy: string;
  readonly face: number;
  readonly nextPaymentOn: string | null;
  readonly nextPayment: number | null;
  readonly lastPrint: number | null;
  readonly lastPrintedIn: number | null;
  readonly holders: number;
  readonly traded: boolean;
}
export interface LeaseNote {
  readonly agreement: string;
  readonly kind: string;
  readonly units: number;
  readonly rentPerPeriod: number;
  readonly until: string;
}
export interface CommitmentNote {
  readonly kind: string;
  readonly what: string;
  readonly rows: number;
  readonly marked: number;
}
export interface DerivativeNote {
  readonly contract: string;
  readonly kind: string;
  readonly counterparty: string;
  readonly house: string | null;
  readonly ccy: string;
  readonly notional: number;
  readonly valueToUs: number;
  readonly margin: number | null;
}
export interface DealNote {
  readonly kind: string;
  readonly period: number;
  readonly buyer: string;
  readonly target: string;
  readonly detail: Readonly<Record<string, unknown>>;
}
export interface EmployeeNote {
  readonly occupation: string;
  readonly region: string;
  readonly headcount: number;
  readonly hours: number;
  readonly wageBill: number;
  readonly leaving: number;
}
/**
 * G5: the share line and what happened on it. NO PER-SHARE FIGURE IS HERE: earnings per share is
 * income over shares, both of them published reads, and a stated one would be an outcome written
 * down (Law 2). A reader divides.
 */
export interface ShareNote {
  readonly line: string;
  readonly issued: number;
  readonly holders: number;
  readonly dividendsDeclared: number;
  readonly dividendsPaid: number;
  readonly boughtBack: number;
}
export interface SubsequentNote {
  readonly kind: string;
  readonly period: number;
}
/** B1, B4: management's own outlook of one thing, in the numbers its decisions read. */
export interface Commentary {
  readonly on: string;
  readonly expected: number;
  readonly unit: string;
  readonly confidence: number;
  readonly formed: number;
}
export interface CurrencyExposure {
  readonly ccy: string;
  readonly held: number;
  readonly owed: number;
}
export interface RelatedParties {
  readonly controller: string | null;
  readonly subsidiaries: readonly string[];
  readonly banks: readonly string[];
  readonly largestCustomer: string | null;
  readonly largestCustomerShare: number | null;
}

/**
 * THE RECORD AS WRITTEN: every money figure in pieces of `ccy`. This is the shape the journal
 * carries and the reporting module writes; `Statement` below is the same thing read back as money.
 */
export interface StatementRecord {
  readonly company: string;
  readonly quarter: string;
  readonly from: number;
  readonly to: number;
  readonly ccy: string;
  readonly income: Readonly<Record<IncomeLine, number>>;
  readonly earned: number;
  readonly revaluation: number;
  readonly oci: Readonly<Record<OciLine, number>>;
  readonly cashFlow: Readonly<Record<CashLine, number>>;
  readonly balance: Readonly<Record<BalanceLine, number>>;
  readonly equityChanges: Readonly<Record<EquityLine, number>>;
  readonly summary: Readonly<Record<SummaryLine, number>>;
  readonly byProduct: readonly ProductSegment[];
  readonly byGeography: readonly GeographySegment[];
  readonly cash: readonly CashCounterpartyNote[];
  readonly holdings: readonly HoldingNote[];
  readonly inventories: readonly InventoryNote[];
  readonly plant: readonly PlantNote[];
  readonly debt: readonly DebtNote[];
  readonly leases: readonly LeaseNote[];
  readonly commitments: readonly CommitmentNote[];
  readonly guarantees: readonly CommitmentNote[];
  readonly derivatives: readonly DerivativeNote[];
  readonly deals: readonly DealNote[];
  readonly employees: readonly EmployeeNote[];
  readonly shares: ShareNote | null;
  readonly relatedParties: RelatedParties;
  readonly subsequent: readonly SubsequentNote[];
  /** What the party itself published about its own regulation, by event kind; nothing invented. */
  readonly regulatory: Readonly<Record<string, Readonly<Record<string, unknown>>>>;
  readonly commentary: readonly Commentary[];
  readonly currencies: readonly CurrencyExposure[];
}

/** A4: the statement as a reader holds it — the record, with its money as money. */
export interface Statement {
  readonly company: PartyId;
  readonly quarter: string;
  readonly from: Period;
  readonly to: Period;
  /** Reporting A4: the period it was prepared in, which is after the books closed. */
  readonly preparedIn: Period;
  /** G5, Law 19: how many periods the quarter covered, derived at the read from its two dates. */
  readonly periods: number;
  readonly ccy: CurrencyCode;
  readonly income: Readonly<Record<IncomeLine, Cash>>;
  /** G2: the movement of the equity account over the span — the income lines summed. */
  readonly earned: Cash;
  /** The part of `earned` nobody was paid: what the marks did (Clearing D4), both directions. */
  readonly revaluation: Cash;
  readonly oci: Readonly<Record<OciLine, Cash>>;
  readonly cashFlow: Readonly<Record<CashLine, Cash>>;
  readonly balance: Readonly<Record<BalanceLine, Cash>>;
  readonly equityChanges: Readonly<Record<EquityLine, Cash>>;
  readonly summary: Readonly<Record<SummaryLine, Cash>>;
  readonly notes: Omit<
    StatementRecord,
    | 'company'
    | 'quarter'
    | 'from'
    | 'to'
    | 'ccy'
    | 'income'
    | 'earned'
    | 'revaluation'
    | 'oci'
    | 'cashFlow'
    | 'balance'
    | 'equityChanges'
    | 'summary'
  >;
}

/* --- The sums, stated once ------------------------------------------------------------------ */

/** Before interest, tax, wear, spoilage, write-offs and the marks; distributions are not income (G2). */
export function ebitdaOf(income: Readonly<Record<IncomeLine, number>>): number {
  return (
    income.revenue +
    income.gainsOnDisposal +
    income.production +
    income.wagesReceived +
    income.rentReceived +
    income.dividendsReceived +
    income.taxReceived +
    income.claimsReceived +
    income.otherReceipts +
    income.wages +
    income.rent +
    income.claimsPaid +
    income.otherPayments +
    income.other
  );
}

/* --- Reading it back ------------------------------------------------------------------------ */

/** Law 4: every line of a section, or the writer's defect named at the site. */
function numbersIn<K extends string>(
  e: Event,
  row: unknown,
  keys: readonly K[],
  section: string,
): Readonly<Record<K, number>> {
  if (typeof row !== 'object' || row === null) {
    throw new InvalidRegistry('Reporting G2', `${e.kind} has no ${section}`);
  }
  const out: Partial<Record<K, number>> = {};
  for (const k of keys) {
    const v = (row as Record<string, unknown>)[k];
    if (typeof v !== 'number' || !Number.isFinite(v)) {
      throw new InvalidRegistry('Reporting G2', `${e.kind} has no finite ${section}.${k}`);
    }
    out[k] = v;
  }
  return out as Readonly<Record<K, number>>;
}

function money<K extends string>(
  row: Readonly<Record<K, number>>,
  keys: readonly K[],
  ccy: CurrencyCode,
): Readonly<Record<K, Cash>> {
  const out: Partial<Record<K, Cash>> = {};
  for (const k of keys) out[k] = asCash(row[k], ccy, `${k} as the statement said it`);
  return out as Readonly<Record<K, Cash>>;
}

const rows = (v: unknown): readonly unknown[] => (Array.isArray(v) ? (v as unknown[]) : []);

/**
 * THE ONE PARSE of a published statement, and it is TOTAL: `reporting` is the one writer of this
 * event (Law 4), so a field it did not write is ITS defect and throwing names it at the site. Every
 * reader used to drop such a record silently, which made a broken writer indistinguishable from a
 * company that had not reported — a `?? 0` in disguise, since every reader treats "not there" as
 * "never published".
 */
export function statementOf(e: Event): Statement {
  /**
   * A2, A3 (17b′.1): the two acts that produce a statement — a QUARTER a company reported on, and
   * the MANAGEMENT ACCOUNTS it prepared for a lender that asked. Same fields, same parse, one
   * vocabulary (Law 4); what tells them apart is the kind, and every reader that cares asks for the
   * one it means rather than unpacking a record twice.
   */
  if (e.kind !== REPORT && e.kind !== INTERIM) {
    throw new InvalidRegistry('Reporting A2', `${e.kind} is not a set of accounts`);
  }
  const d = e.data;
  const ccy = str(e, 'ccy') as CurrencyCode;
  const from = period(num(e, 'from'));
  const to = period(num(e, 'to'));
  return {
    company: str(e, 'company') as PartyId,
    quarter: str(e, 'quarter'),
    from,
    to,
    preparedIn: e.period,
    // Law 19: derived at the read from the two dates the writer published, never stored beside them.
    periods: to - from + 1,
    ccy,
    income: money(numbersIn(e, d['income'], INCOME_LINES, 'income'), INCOME_LINES, ccy),
    earned: asCash(num(e, 'earned'), ccy, 'what the quarter produced'),
    revaluation: asCash(num(e, 'revaluation'), ccy, 'what the marks did'),
    oci: money(numbersIn(e, d['oci'], OCI_LINES, 'oci'), OCI_LINES, ccy),
    cashFlow: money(numbersIn(e, d['cashFlow'], CASH_LINES, 'cashFlow'), CASH_LINES, ccy),
    balance: money(numbersIn(e, d['balance'], BALANCE_LINES, 'balance'), BALANCE_LINES, ccy),
    equityChanges: money(
      numbersIn(e, d['equityChanges'], EQUITY_LINES, 'equityChanges'),
      EQUITY_LINES,
      ccy,
    ),
    summary: money(numbersIn(e, d['summary'], SUMMARY_LINES, 'summary'), SUMMARY_LINES, ccy),
    notes: {
      byProduct: rows(d['byProduct']) as readonly ProductSegment[],
      byGeography: rows(d['byGeography']) as readonly GeographySegment[],
      cash: rows(d['cash']) as readonly CashCounterpartyNote[],
      holdings: rows(d['holdings']) as readonly HoldingNote[],
      inventories: rows(d['inventories']) as readonly InventoryNote[],
      plant: rows(d['plant']) as readonly PlantNote[],
      debt: rows(d['debt']) as readonly DebtNote[],
      leases: rows(d['leases']) as readonly LeaseNote[],
      commitments: rows(d['commitments']) as readonly CommitmentNote[],
      guarantees: rows(d['guarantees']) as readonly CommitmentNote[],
      derivatives: rows(d['derivatives']) as readonly DerivativeNote[],
      deals: rows(d['deals']) as readonly DealNote[],
      employees: rows(d['employees']) as readonly EmployeeNote[],
      shares: (d['shares'] ?? null) as ShareNote | null,
      relatedParties: d['relatedParties'] as RelatedParties,
      subsequent: rows(d['subsequent']) as readonly SubsequentNote[],
      regulatory: d['regulatory'] as Readonly<Record<string, Readonly<Record<string, unknown>>>>,
      commentary: rows(d['commentary']) as readonly Commentary[],
      currencies: rows(d['currencies']) as readonly CurrencyExposure[],
    },
  };
}

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

/** Corporate Credit A3.b: what it took in (marks excluded) against what its debt took; nothing where its debt took nothing. */
export function coverageOf(s: Statement): Option<number> {
  if (s.summary.service.pieces <= 0) return none<number>();
  return some(s.summary.ebitda.pieces / s.summary.service.pieces);
}

/** Debt over what it earns before interest, tax, wear and the marks; nothing where that is not positive. */
export function leverageOf(s: Statement): Option<number> {
  if (s.summary.ebitda.pieces <= 0) return none<number>();
  return some(s.balance.debt.pieces / s.summary.ebitda.pieces);
}

/* --- Who may read it ------------------------------------------------------------------------ */

/** The two doors a party has to somebody else's statement: what was published, what was shown to it. */
export interface StatementReads {
  lastPublicAbout(kind: string, subject: string): Option<Event>;
  disclosedToMe(kind: string, from: PartyId): Option<Event>;
}

/**
 * Observer A3, A4: THE LATEST STATEMENT OF A NAME THIS PARTY MAY READ — what the name showed it, or
 * what the name published; nothing where neither happened, which is a refusal and not a zero. What
 * was shown to it is asked first because it is at least as recent as what was published and may be
 * more: a lender of record sees the statement in the period it is prepared, whether or not the
 * company is public.
 */
export function statementVisibleTo(reads: StatementReads, company: PartyId): Option<Statement> {
  const shown = reads.disclosedToMe(REPORT, company);
  const published = reads.lastPublicAbout(REPORT, String(company));
  const pick = (a: Option<Event>, b: Option<Event>): Option<Event> => {
    if (!a.some) return b;
    if (!b.some) return a;
    return a.value.period >= b.value.period ? a : b;
  };
  const e = pick(shown, published);
  return e.some ? some(statementOf(e.value)) : none<Statement>();
}

/** The wire, asked for what involved a named party in a span. */
export interface DealReads {
  forSubject(kind: string, subject: string): readonly Event[];
}

/** Equity D3: the dividends a company declared over a span, per share times the shares it was on. */
const DIVIDEND_DECLARED = 'payout.declared';
export function dividendsDeclaredIn(reads: DealReads, party: PartyId, from: Period, to: Period): number {
  let declared = 0;
  for (const e of reads.forSubject(DIVIDEND_DECLARED, String(party))) {
    if (e.period < from || e.period > to) continue;
    const perShare = e.data['perShare'];
    const count = e.data['shares'];
    if (typeof perShare === 'number' && typeof count === 'number') declared += perShare * count;
  }
  return declared;
}

/** §35: every deal event naming this party from `from` on, in the order it happened. */
export function dealsInvolving(reads: DealReads, party: PartyId, from: Period): readonly DealNote[] {
  const out: DealNote[] = [];
  for (const kind of DEAL_KINDS) {
    for (const e of reads.forSubject(kind, String(party))) {
      if (e.period < from) continue;
      const buyer = e.data['buyer'];
      const target = e.data['target'];
      out.push({
        kind,
        period: e.period,
        buyer: typeof buyer === 'string' ? buyer : '',
        target: typeof target === 'string' ? target : '',
        detail: e.data,
      });
    }
  }
  return out.sort((a, b) => a.period - b.period);
}
