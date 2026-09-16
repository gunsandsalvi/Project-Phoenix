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
import { asCash, type Cash, asRatio, type Ratio } from '../core/measure.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { Period } from '../calendar/calendar.js';
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

/** D3.a: whether a named benchmark has ever fixed at all. No fixing, no floating leg. */
export function hasFixed(reads: WireReads, named: string): boolean {
  return reads.ofKind(BENCHMARK).some((e) => e.subjects.includes(named));
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
export function gradesOn(reads: WireReads, party: string): ReadonlyMap<string, string> {
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
