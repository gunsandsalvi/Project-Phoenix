/**
 * PUBLIC NOTICES, READ BY NAME — what a market, an assessor or an agent announced about a party.
 *
 * @spec Indices A1 Indices A1.a Central Bank D3.a Sovereign D4 Corporate Credit E5 Prime Brokerage B2 Prime Brokerage C2 Ratings A5.a Law 4 Law 15 Law 19
 *
 * The fourth file on the shape 0e′.1 settled. What is left after `wages`, `banking` and `funding`
 * are the one-off announcements: an overnight fixing, an auction's terms, an offer of paper, a
 * rating action, an estate closing, an advisory quote, a margin call. Each is written by one module
 * and read by one or two others, and each reader was naming the event kind and unpacking the fields
 * itself (`phoenix/no-cross-module-event-read`).
 *
 * They are small and they do not resemble each other, which is exactly why they are here rather
 * than spread over four more files: what they have in common is that they are PUBLIC and they are
 * ABOUT A NAMED PARTY, and that is the whole of what a reader needs to know.
 *
 * NOTHING HERE DERIVES ANYTHING. A notice that was not published is NOTHING, never a zero (App A).
 */
import { paramId, type ParamId } from '../core/ids.js';
import { asCash, type Cash, asRatio, type Ratio } from '../core/measure.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import { middleGrade, type Grade } from './grades.js';
import { period, type Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import { yearFraction } from '../calendar/daycount.js';
import type { CurrencyCode } from '../core/ids.js';

const BENCHMARK = 'index.benchmark';
const AUCTION = 'auction.announced';
const OFFERED = 'bond.offered';
const RATING = 'rating.action';
const ESTATE_CLOSED = 'estate.closed';
const ADVISORY_QUOTED = 'advisory.quoted';
const ADVISORY_RAN = 'advisory.ran';
const PRIME_WANTED = 'prime.wanted';
const PRIME_CALL = 'prime.call';
const PRIME_LINE = 'prime.line';

/** The phase's door: the wire, by kind, by period or about a named subject. */
export interface WireReads {
  ofKind(kind: string): readonly Event[];
  ofKindIn(kind: string, at: Period): readonly Event[];
  lastOf(kind: string, subject: string): Event | undefined;
}

/** The participant's door. */
export interface PartyReads {
  lastPublic(kind: string): Option<Event>;
  lastPublicAbout(kind: string, subject: string): Option<Event>;
  lastOwnSince(kind: string, since: Period): Option<Event>;
}

/* --- What an overnight book fixed at (Indices A1) ------------------------------------------ */

function rateOf(e: Event | undefined, named: string): Option<Ratio> {
  if (e?.subjects.includes(named) !== true) return none<Ratio>();
  const rate = e.data['rate'];
  // Item 16: a published rate re-enters the type system here, through its dimension's own door.
  return typeof rate === 'number'
    ? some(asRatio(rate, 'what the benchmark printed'))
    : none<Ratio>();
}

/**
 * Indices A1, Central Bank D3.a: WHAT A NAMED BENCHMARK LAST FIXED AT. A money whose overnight book
 * has never traded has no benchmark, and a derivative on a benchmark that does not exist is a
 * derivative on nothing — so the refusal here is the mechanism, not a missing default.
 */
export function fixingOf(reads: PartyReads, named: string): Option<Ratio> {
  const said = reads.lastPublicAbout(BENCHMARK, named);
  return said.some ? rateOf(said.value, named) : none<Ratio>();
}

/** The same fixing, for a reader that has only the last public notice of a kind. */
export function lastFixing(reads: Pick<PartyReads, 'lastPublic'>, named: string): Option<Ratio> {
  const said = reads.lastPublic(BENCHMARK);
  return said.some ? rateOf(said.value, named) : none<Ratio>();
}

/**
 * Indices A1, D3 (17.1): EVERY BENCHMARK THAT FIXED IN A PERIOD, by the name it was published
 * under. A book that did not trade published nothing and is not here — carrying the last one
 * forward would be the posted benchmark Appendix B forbids.
 */
export function benchmarksFixedIn(
  reads: Pick<WireReads, 'ofKind'>,
  at: Period,
): readonly { readonly named: string; readonly rate: number }[] {
  const out: { named: string; rate: number }[] = [];
  for (const e of reads.ofKind(BENCHMARK)) {
    if (e.period !== at) continue;
    const named = e.subjects[1];
    const rate = e.data['rate'];
    if (typeof named !== 'string' || typeof rate !== 'number') continue;
    out.push({ named, rate });
  }
  return out;
}

/**
 * Indices A1, D3 (17.1): WHAT A NAMED BENCHMARK LAST FIXED AT, read off the public record itself
 * rather than through a party — a fixing is public, so a module that needs one needs nobody's view
 * of it (Observer A3, A4).
 */
export function lastBenchmarkFix(reads: Pick<WireReads, 'lastOf'>, named: string): Option<Ratio> {
  const said = reads.lastOf(BENCHMARK, named);
  return said === undefined ? none<Ratio>() : rateOf(said, named);
}

/** D3.a: whether a named benchmark has ever fixed at all. No fixing, no floating leg. */
export function hasFixed(reads: WireReads, named: string): boolean {
  return reads.ofKind(BENCHMARK).some((e) => e.subjects.includes(named));
}

/**
 * XI-7, Indices D3.a, Corporate Credit B4, Law 3 (17d.1): A TERM RATE, AND EVERY INPUT TO IT IS A
 * RATE SOMEBODY PAID.
 *
 * The owner's rule is *"Xm SOFR, with X based on the coupon frequency"*, and this world's benchmark
 * is OVERNIGHT: what the overnight book actually cleared at, period by period, transacted or
 * nothing (D3.a). A term rate over those is the one shape in which it stays transacted — the
 * fixings COMPOUNDED over the tenor that just ended, in arrears, which is what a SOFR-based loan
 * actually pays and is not a forecast of anything. A forward-looking term rate would be a price for
 * a market that does not meet here, and posting one is what Appendix B forbids.
 *
 * WHAT IT RETURNS IS THE SHARE OF PAR THE SPAN EARNED, not a rate per annum: a coupon in arrears IS
 * the compounding, and annualising it only to scale it back down by the same span would be two
 * crossings where the arithmetic wants none (Law 8).
 *
 * A GAP IS A REFUSAL. A period inside the span in which nobody borrowed overnight has no fixing,
 * and there is no rate to put in its place — not the last one, which would be the posted benchmark,
 * and not zero, which would be a week of free money nobody lent. The answer is that this span has
 * no term rate, and a loan that cannot fix cannot float (D3.a).
 */
export interface TermFixing {
  /** The share of par the fixings compounded to over the span — what a coupon in arrears IS. */
  readonly over: Ratio;
  readonly from: Period;
  readonly to: Period;
  /** How many fixings went into it, which is how many periods the span covered. */
  readonly fixings: number;
}

export function termFixing(
  reads: Pick<WireReads, 'ofKind'>,
  named: string,
  from: Period,
  to: Period,
  /**
   * Law 8: A FIXING IS PER ANNUM and a span is days, so the CALENDAR turns each one into the share
   * of par its own period earned before they are multiplied out. Compounding the rates themselves
   * would be compounding a year's rate once a week, which is what this did until it was read
   * against the money market's own accrual (`money-market/rows.ts`: interest is the rate scaled by
   * the span of a year) — and the answer came out at ninety-nine per cent.
   */
  calendar: { startOf(p: Period): Civil },
): Option<TermFixing> {
  if (to < from) return none<TermFixing>();
  const byPeriod = new Map<number, number>();
  for (const e of reads.ofKind(BENCHMARK)) {
    if (e.period < from || e.period > to || !e.subjects.includes(named)) continue;
    const rate = rateOf(e, named);
    if (rate.some) byPeriod.set(e.period, rate.value);
  }
  let compounded = 1;
  for (let at: number = from; at <= to; at += 1) {
    const perAnnum = byPeriod.get(at);
    // D3.a: a period nobody borrowed in has no fixing, and nothing stands in for one.
    if (perAnnum === undefined) return none<TermFixing>();
    const ofAYear = yearFraction(
      'ACT/365F',
      calendar.startOf(period(at)),
      calendar.startOf(period(at + 1)),
    );
    if (ofAYear <= 0) return none<TermFixing>();
    compounded *= 1 + perAnnum * ofAYear;
  }
  return some({
    over: asRatio(compounded - 1, `what ${named} compounded to over the span`),
    from,
    to,
    fixings: byPeriod.size,
  });
}

/* --- What an auction announced, and who offered paper -------------------------------------- */

/**
 * Sovereign D4: THE SHARE A DEALERSHIP OBLIGES A BANK TO BID FOR in an announced auction. It is an
 * obligation of the dealership and not a preference, which is why it is read off the announcement
 * rather than decided at the desk.
 */
export function dealershipShare(reads: PartyReads, instrument: string, at: Period): Option<Ratio> {
  const said = reads.lastPublicAbout(AUCTION, instrument);
  if (!said.some || said.value.period !== at) return none<Ratio>();
  const share = said.value.data['dealershipShare'];
  return typeof share === 'number'
    ? some(asRatio(share, 'the share its dealership obliges it to bid for'))
    : none<Ratio>();
}

/** E5: whether anybody offered this borrower's paper in a period — a channel it already used. */
export function paperOfferedIn(reads: WireReads, borrower: string, at: Period): boolean {
  return reads.ofKindIn(OFFERED, at).some((e) => e.subjects.includes(borrower));
}

/* --- What the assessors said, and whether an estate is closed ------------------------------ */

/**
 * Ratings A5.a: EVERY ASSESSOR'S LATEST GRADE FOR A NAME, one per assessor. The assessors disagree
 * and the disagreement is the point (§46 A3); what a reader does with several grades is its own
 * business, so this hands back all of them rather than one.
 */
export function gradesOn(reads: Pick<WireReads, 'ofKind'>, party: string): ReadonlyMap<string, string> {
  const latest = new Map<string, string>();
  for (const e of reads.ofKind(RATING)) {
    if (!e.subjects.includes(party)) continue;
    const assessor = e.subjects[0];
    const grade = e.data['grade'];
    if (typeof assessor !== 'string' || typeof grade !== 'string') continue;
    latest.set(assessor, grade);
  }
  return latest;
}

/**
 * Ratings C2, A5.a: THE GRADE A NAME CARRIES, which is the middle of what its assessors have
 * published about it. One read, where everybody reads it: a bank weighting a claim, an index
 * deciding whether a name is in its basket, and anybody else who has to know what the market has
 * been told. Nothing where nobody has graded it, which is what unrated means.
 */
export function gradeOn(reads: Pick<WireReads, 'ofKind'>, name: string): Option<Grade> {
  const grade = middleGrade([...gradesOn(reads, name).values()]);
  return grade === undefined ? none<Grade>() : some(grade);
}

/** XI-8: whether an estate has closed — which is when what it owed is finally known. */
export function estateClosed(reads: WireReads, estate: string): boolean {
  return reads.ofKind(ESTATE_CLOSED).some((e) => e.subjects.includes(estate));
}

/* --- What an adviser quoted, and how much of it it ran ------------------------------------- */

/** §35: a bank's quote to run one process — what it charges, and how many it can run at once. */
export interface AdvisoryQuote {
  readonly bank: string;
  readonly fee: Cash;
  readonly capacity: number;
  readonly ccy: string;
}

/** §35 B4: every advisory quote made in a period. A seller takes the keenest it can still get. */
export function advisoryQuotesIn(reads: WireReads, at: Period): readonly AdvisoryQuote[] {
  const out: AdvisoryQuote[] = [];
  for (const e of reads.ofKindIn(ADVISORY_QUOTED, at)) {
    const { bank, fee, capacity, ccy } = e.data;
    if (typeof bank !== 'string' || typeof fee !== 'number') continue;
    if (typeof capacity !== 'number' || typeof ccy !== 'string') continue;
    // Item 16: a fee re-enters the type system here, as the money it is.
    out.push({
      bank,
      fee: asCash(fee, ccy as CurrencyCode, 'what it charges to run one'),
      capacity,
      ccy,
    });
  }
  return out;
}

/** §35 B4: how many processes this bank published it ran — its corporate-finance department's work. */
export function processesItRan(reads: PartyReads, since: Period): Option<number> {
  const ran = reads.lastOwnSince(ADVISORY_RAN, since);
  if (!ran.some) return none<number>();
  const count = ran.value.data['processes'];
  return typeof count === 'number' && count > 0 ? some(count) : none<number>();
}

/* --- What a prime broker and its client said to each other --------------------------------- */

/** B2: what a client published it wants to draw. A client that has asked for nothing is lent none. */
export function wantedToDraw(reads: WireReads, client: string): Option<Cash> {
  const said = reads.lastOf(PRIME_WANTED, client);
  if (said === undefined) return none<Cash>();
  const wants = said.data['wants'];
  const ccy = said.data['ccy'];
  // Item 16: what a client published it wants to draw re-enters here as the money it is (16.0: named).
  return typeof wants === 'number' && typeof ccy === 'string'
    ? some(asCash(wants, ccy as CurrencyCode, 'what it asked to draw'))
    : none<Cash>();
}

/** What a broker published about a client's line: the book it finances, and the client's own equity. */
export interface PrimeLine {
  readonly portfolio: number;
  readonly equity: number;
}

/** B2: the line as the broker published it. A client with no equity in it has no line. */
export function primeLineOf(reads: WireReads, client: string): Option<PrimeLine> {
  const said = reads.lastOf(PRIME_LINE, client);
  if (said === undefined) return none<PrimeLine>();
  const portfolio = said.data['portfolio'];
  const equity = said.data['equity'];
  if (typeof portfolio !== 'number' || typeof equity !== 'number' || equity <= 0) {
    return none<PrimeLine>();
  }
  return some({ portfolio, equity });
}

/** What a broker called and the client could not pay, and the period it called in. */
export interface PrimeCall {
  readonly period: Period;
  readonly unmet: number;
}

/**
 * C2: the last margin call on a client. The broker re-calls every period it is over the line, so
 * the last one is the current one — and a call older than the period before is one this client has
 * since met, or a broker that has stopped looking.
 */
export function primeCallOn(reads: WireReads, client: string): Option<PrimeCall> {
  const said = reads.lastOf(PRIME_CALL, client);
  if (said === undefined) return none<PrimeCall>();
  const unmet = said.data['unmet'];
  return typeof unmet === 'number' && unmet > 0
    ? some({ period: said.period, unmet })
    : none<PrimeCall>();
}

/**
 * Central Bank B1, B2 (18a.3): THE NAME OF A MONEY'S ADMINISTERED RATE.
 *
 * The rate itself is the money market's to set and to publish; the NAME is public, the way a
 * benchmark's or an index's is, because everything that prices against it has to be able to ask for
 * it — a central bank's own desk, a bank's board, a borrower choosing between fixed and floating.
 * Naming it here is what keeps those readers out of each other's modules (Law 15).
 */
export const policyRateOf = (ccy: string): ParamId => paramId(`centralBank.policyRate.${ccy}`);
