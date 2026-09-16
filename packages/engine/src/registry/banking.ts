/**
 * WHAT A BANK PUBLISHES ABOUT ITSELF — one read per fact, for everybody who has to ask.
 *
 * @spec Banks Funding C1 Banks Funding C2 Banks Funding C2.a Banks Funding F4 Money Market B1 Money Market B4 Money Market B7 Money Market D5.a Central Bank B2 Central Bank B3.a Corporate Credit A4 Currency A3 Law 4 Law 15 Law 19
 *
 * WHY THIS IS KERNEL DATA. A bank computes its own economics once and publishes them under its own
 * name: what its money costs it, what capital it has spare, what it keeps back, what it is offering
 * a class of depositor, what it requires of a name. Eleven other modules price against those facts
 * and none of them may import the module that writes them (`phoenix/no-cross-module-import`), so
 * each of them reached for the EVENT KIND and pulled the fields out itself. That is the shape
 * `registry/wages.ts` and `registry/switching.ts` already refused: a public fact is read where
 * everybody can reach it, and what the fields MEAN is settled in one place (0e′.2).
 *
 * It had already gone wrong twice the way a second copy does. `bank.costOfFunds` was unpacked in
 * `securitisation` and again in `fx-derivatives`, two walks of one event's `alsoIn` map written
 * apart. `bank.reservation`'s `required` map was unpacked in `banks/treasury.ts` and again in
 * `money-market/collateral.ts` — one bank's own credit view, read by two formulas.
 *
 * NOTHING HERE DERIVES ANYTHING. Every function is a read of a number some bank published, returned
 * in its own dimension; a bank that has published nothing yields NOTHING, which is a refusal and
 * never a zero (App A).
 */
import { asRatio, type Ratio } from '../core/measure.js';
import { asQty, type Qty } from '../core/tick.js';
import type { CurrencyCode } from '../core/ids.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { Period } from '../calendar/calendar.js';

/** The kinds a bank publishes under. Named HERE, so no mechanism has to name them. */
const COST_OF_FUNDS = 'bank.costOfFunds';
const CAPITAL = 'bank.capital';
const BUFFER = 'bank.buffer';
const DEPOSIT_RATE = 'bank.depositRate';
const RESERVATION = 'bank.reservation';
const DEALING = 'bank.dealing';
const LIQUIDITY = 'bank.liquidity';
const DEPOSIT_CLASSES = 'deposit.classes';
const CORRIDOR = 'centralBank.corridor';
const MM_PRINT = 'moneyMarket.print';
const MM_REFUSED = 'moneyMarket.refused';
const CREDIT_QUOTED = 'credit.quoted';
const CREDIT_DEFAULT = 'credit.default';

/** The participant's door: what a party can see about itself and about the public record. */
export interface BankReads {
  lastOwn(kind: string): Option<Event>;
  lastPublic(kind: string): Option<Event>;
  lastPublicAbout(kind: string, subject: string): Option<Event>;
}

/** The phase's door: the wire, asked by kind or about a named party. */
export interface WireReads {
  ofKind(kind: string): readonly Event[];
  lastOf(kind: string, subject: string): Event | undefined;
  forSubject(kind: string, subject: string): readonly Event[];
}

/** A published number, back in its own dimension, or nothing if the field is not there. */
function rateIn(row: unknown, key: string, what: string): Option<Ratio> {
  if (typeof row !== 'object' || row === null) return none<Ratio>();
  const r = (row as Record<string, unknown>)[key];
  // Item 16: a published rate re-enters the type system here, through its dimension's own door.
  return typeof r === 'number' ? some(asRatio(r, what)) : none<Ratio>();
}

/** A named row out of a published map: `{ required: { 'firm.4': 0.07 } }` asked about `firm.4`. */
function fromMap(said: Option<Event>, key: string, about: string, what: string): Option<Ratio> {
  if (!said.some) return none<Ratio>();
  return rateIn(said.value.data[key], about, what);
}

/* --- What money costs a bank (Currency A3) ----------------------------------------------- */

/**
 * Currency A3: WHAT A CURRENCY COSTS THIS BANK, per annum, as it published it.
 *
 * `publishCostOfFunds` writes one event per bank per period carrying the blend for its home money
 * and a named row under `alsoIn` for every other money it might lend in. This is a read of that,
 * never a re-derivation (Law 19) and never a table.
 */
export function costOfFundsIn(
  reads: BankReads,
  bank: string,
  ccy: CurrencyCode,
): Option<Ratio> {
  const said = reads.lastPublicAbout(COST_OF_FUNDS, bank);
  if (!said.some) return none<Ratio>();
  const data = said.value.data;
  const what = `what ${ccy} costs this bank, per annum`;
  if (data['ccy'] === ccy) return rateIn(data, 'perAnnum', what);
  const also = data['alsoIn'];
  if (typeof also !== 'object' || also === null) return none<Ratio>();
  return rateIn((also as Record<string, unknown>)[ccy], 'perAnnum', what);
}

/* --- What capital it has, and what it will have out to one name --------------------------- */

/** The three numbers a bank publishes about its own capital, plus the limit it sets per name. */
export interface PublishedCapital {
  readonly headroom: number;
  readonly minWeighted: number;
  readonly binds: string;
}

/** C2: a bank's own capital position as it published it. Nothing is computed here. */
export function capitalPublished(reads: BankReads, bank: string): Option<PublishedCapital> {
  const said = reads.lastPublicAbout(CAPITAL, bank);
  if (!said.some) return none<PublishedCapital>();
  const d = said.value.data;
  const headroom = d['headroom'];
  const minWeighted = d['minWeighted'];
  const binds = d['binds'];
  if (typeof headroom !== 'number' || typeof minWeighted !== 'number' || typeof binds !== 'string') {
    return none<PublishedCapital>();
  }
  return some({ headroom, minWeighted, binds });
}

/**
 * C4: HOW MUCH THIS BANK WILL HAVE OUT TO ANY ONE NAME, as it published it.
 *
 * It is read off the wire rather than restated wherever an exposure is taken, because a bank's
 * limit is a bank's fact and there is one writer of it (Law 4, Law 19).
 */
export function limitPerName(reads: BankReads, bank: string): Option<number> {
  const said = reads.lastPublicAbout(CAPITAL, bank);
  if (!said.some) return none<number>();
  const limit = said.value.data['limitPerName'];
  return typeof limit === 'number' && limit > 0 ? some(limit) : none<number>();
}

/* --- What it keeps back, and where that leaves it ----------------------------------------- */

/** What a bank published about its own reserve account this period. */
export interface PublishedBuffer {
  readonly reserves: Qty;
  readonly buffer: Qty;
  readonly ccy: string;
}

function bufferFrom(e: Event | undefined, at: Period): Option<PublishedBuffer> {
  if (e?.period !== at) return none<PublishedBuffer>();
  const { reserves, buffer, ccy } = e.data;
  if (typeof reserves !== 'number' || typeof buffer !== 'number' || typeof ccy !== 'string') {
    return none<PublishedBuffer>();
  }
  // Item 16: what a bank published about its own account re-enters here, in the pieces it holds.
  return some({
    reserves: asQty(reserves, 'what it published it holds'),
    buffer: asQty(buffer, 'what it published it keeps back'),
    ccy,
  });
}

/** C2.a: one named bank's published reserve position, this period. Older is not its position. */
export function bufferOf(reads: WireReads, bank: string, at: Period): Option<PublishedBuffer> {
  return bufferFrom(reads.lastOf(BUFFER, bank), at);
}

/** C2.a: every bank that published a reserve position this period, by the name it published under. */
export function buffersPublished(reads: WireReads, at: Period): ReadonlyMap<string, PublishedBuffer> {
  const out = new Map<string, PublishedBuffer>();
  for (const e of reads.ofKind(BUFFER)) {
    const bank = e.data['bank'];
    if (typeof bank !== 'string') continue;
    const said = bufferFrom(e, at);
    if (said.some) out.set(bank, said.value);
  }
  return out;
}

/* --- What it is offering a depositor ------------------------------------------------------ */

/**
 * B1.a, D5.a: what a bank is offering a CLASS, read off the last board it announced. It is the rate
 * on the board — the same fact a rival prices against and a depositor moves for, which is why it is
 * one public number and not two private ones (Law 4).
 */
export function depositRateFor(reads: WireReads, bank: string, cls: string): Option<Ratio> {
  const said = reads.forSubject(DEPOSIT_RATE, bank);
  const last = said[said.length - 1];
  if (last === undefined) return none<Ratio>();
  return rateIn(last.data['rates'], cls, 'what this bank announced for this class');
}

/**
 * §46 A2.a (12d.1): what this bank posted for this class THIS PERIOD, or nothing — the observation
 * a depositor takes the period the board goes up, and not on the weeks it stands.
 */
export function depositRatePostedAt(reads: WireReads, bank: string, cls: string, at: Period): Option<Ratio> {
  const said = reads.forSubject(DEPOSIT_RATE, bank);
  const last = said[said.length - 1];
  if (last?.period !== at) return none<Ratio>();
  return rateIn(last.data['rates'], cls, 'what this bank announced for this class');
}

/** B1.a: the whole of a bank's board this period, in one money. Nothing where it has not posted one. */
export function boardPosted(
  reads: BankReads,
  bank: string,
  at: Period,
  ccy: CurrencyCode,
): readonly number[] {
  const said = reads.lastPublicAbout(DEPOSIT_RATE, bank);
  if (!said.some || said.value.period !== at || said.value.data['ccy'] !== ccy) return [];
  const rates = said.value.data['rates'];
  if (typeof rates !== 'object' || rates === null) return [];
  return Object.values(rates as Record<string, unknown>).filter((r): r is number => typeof r === 'number');
}

/** A class of deposit as the money market declared it: who it covers and what the cover costs. */
export interface DepositClassSeen {
  readonly id: string;
  readonly insured: boolean;
  readonly premium: Ratio;
}

/**
 * B1: THE DEPOSIT CLASSES THIS WORLD RUNS, as the money market declared them. Public, because a
 * bank prices a board against classes it can see and a depositor moves between them.
 */
export function depositClassesSeen(reads: WireReads): readonly DepositClassSeen[] {
  const said = reads.ofKind(DEPOSIT_CLASSES);
  const last = said[said.length - 1];
  const rows = last?.data['classes'];
  if (!Array.isArray(rows)) return [];
  const out: DepositClassSeen[] = [];
  for (const r of rows) {
    if (typeof r !== 'object' || r === null) continue;
    const { id, insured, premium } = r as Record<string, unknown>;
      if (typeof id !== 'string' || typeof insured !== 'boolean') continue;
    if (typeof premium !== 'number') continue;
    // Item 16: what the guarantee costs re-enters the type system here, through its own door.
    out.push({ id, insured, premium: asRatio(premium, `what the guarantee on ${id} costs`) });
  }
  return out;
}

/* --- What it requires of a name (its own credit view) -------------------------------------- */

/**
 * B2: WHAT THIS BANK REQUIRES OF A NAME, out of its own credit model — the yield it wants, what it
 * expects to lose, and what the capital such a claim consumes cost it. Computed once by the module
 * that owns this bank's economics and published under its own name; a second copy of those terms
 * anywhere would be the bank pricing its book against a belief it does not hold (Law 4).
 */
export function requiredOf(reads: BankReads, about: string): Option<Ratio> {
  return fromMap(reads.lastOwn(RESERVATION), 'required', about, 'what this lender requires');
}

/** B2: what it published it expects to lose on a name. */
export function expectedLossOn(reads: BankReads, about: string): Option<Ratio> {
  return fromMap(reads.lastOwn(RESERVATION), 'expectedLoss', about, 'what it expects to lose');
}

/** B2: what it published the capital behind a claim on a name costs it. */
export function capitalCostOn(reads: BankReads, about: string): Option<Ratio> {
  return fromMap(reads.lastOwn(RESERVATION), 'capitalCost', about, 'what that capital costs it');
}

/* --- The lines its desk makes a market in -------------------------------------------------- */

/** §26: the lines this bank's own desk published that it quotes. A desk with no board quotes none. */
export function linesQuoted(reads: BankReads): readonly string[] {
  const said = reads.lastOwn(DEALING);
  if (!said.some) return [];
  const lines = said.value.data['lines'];
  return typeof lines === 'object' && lines !== null ? Object.keys(lines) : [];
}

/**
 * Banks Funding C1, F4, C2: WHAT A BANK PUBLISHED ABOUT ITS OWN LIQUIDITY. Its treasury, its
 * dealing line and its credit line all price against it and all read the one publication, because
 * a second answer to what a bank can pay with would be a second bank (Law 4).
 *
 * A field it has not published is NOTHING and not a zero: a bank that has not yet had a week does
 * not know what a bad one costs, which is a real state at the opening of the world.
 */
function liquidityField(reads: BankReads, key: string): Option<number> {
  const said = reads.lastOwn(LIQUIDITY);
  if (!said.some) return none<number>();
  const v = said.value.data[key];
  return typeof v === 'number' ? some(v) : none<number>();
}

/** C1: what this bank published it could pay with. */
export function liquidHeld(reads: BankReads): Option<number> {
  return liquidityField(reads, 'liquid');
}

/** F4: what this bank published could leave it — the uninsured money on its own books. */
export function couldLeave(reads: BankReads): Option<number> {
  return liquidityField(reads, 'couldLeave');
}

/** C2.a: what this bank published it holds in the account against a bad week. */
export function bufferHeld(reads: BankReads): Option<number> {
  return liquidityField(reads, 'buffer');
}

/** What a central bank declared its corridor to be. */
export interface PublishedCorridor {
  readonly policy: Ratio;
  readonly floor: Ratio;
  readonly ceiling: Ratio;
}

function corridorFrom(e: Event | undefined): Option<PublishedCorridor> {
  if (e === undefined) return none<PublishedCorridor>();
  const { policy, floor, ceiling } = e.data;
  if (typeof policy !== 'number' || typeof floor !== 'number' || typeof ceiling !== 'number') {
    return none<PublishedCorridor>();
  }
  // Item 16: three administered levels re-enter the type system here, through their own door.
  return some({
    policy: asRatio(policy, 'the policy rate'),
    floor: asRatio(floor, 'the floor'),
    ceiling: asRatio(ceiling, 'the ceiling'),
  });
}

/**
 * Central Bank B2, B3.a: THE CORRIDOR, READ. It is administered and published, so everybody who
 * prices against it looks it up rather than deriving it — a second copy of it anywhere would be a
 * second central bank.
 */
export function corridorSeenBy(reads: BankReads): Option<PublishedCorridor> {
  const said = reads.lastPublic(CORRIDOR);
  return corridorFrom(said.some ? said.value : undefined);
}

/** The same corridor, through a phase's door. */
export function corridorPublished(reads: WireReads): Option<PublishedCorridor> {
  const said = reads.ofKind(CORRIDOR);
  return corridorFrom(said[said.length - 1]);
}

/** One overnight trade as the book printed it: who borrowed, at what, how much, in what. */
export interface OvernightPrint {
  readonly period: Period;
  readonly borrower: string;
  readonly rate: number;
  readonly volume: number;
  readonly ccy: string;
  readonly tenor: string;
  readonly secured: boolean;
}

/**
 * Money Market B4: WHAT THE OVERNIGHT BOOKS PRINTED — the public record of every session, which a
 * bank reads to know what its funding has cost it and a benchmark reads to know what cleared. Both
 * filter it their own way; what a print IS is settled once, here (Law 4).
 */
export function overnightPrints(reads: WireReads): readonly OvernightPrint[] {
  const out: OvernightPrint[] = [];
  for (const e of reads.ofKind(MM_PRINT)) {
    const { borrower, rate, volume, ccy, tenor, secured } = e.data;
    if (typeof borrower !== 'string' || typeof rate !== 'number' || typeof volume !== 'number') continue;
    if (typeof ccy !== 'string' || typeof tenor !== 'string' || typeof secured !== 'boolean') continue;
    out.push({ period: e.period, borrower, rate, volume, ccy, tenor, secured });
  }
  return out;
}

/**
 * B7, D3: WHAT A SESSION LEFT A NAME SHORT OF. It is the book's own public refusal read back
 * (Law 19), never a second count of it.
 */
export function refusedOvernightIn(reads: WireReads, bank: string, at: Period): Option<number> {
  for (const e of reads.ofKind(MM_REFUSED)) {
    if (e.period !== at || !e.subjects.includes(bank)) continue;
    const short = e.data['short'];
    if (typeof short === 'number') return some(short);
  }
  return none<number>();
}

/** What a session refused a bank, and the cushion inside what it asked for. */
export interface OvernightRefusal {
  readonly period: Period;
  readonly short: number;
  readonly buffer: number;
}

/** B7, D3: what this bank was last refused overnight, as it saw it (Law 19). */
export function refusedOvernight(reads: BankReads): Option<OvernightRefusal> {
  const said = reads.lastOwn(MM_REFUSED);
  if (!said.some) return none<OvernightRefusal>();
  const short = said.value.data['short'];
  const buffer = said.value.data['buffer'];
  if (typeof short !== 'number' || typeof buffer !== 'number') return none<OvernightRefusal>();
  return some({ period: said.value.period, short, buffer });
}

/* --- What a lender last quoted a borrower, and what has gone wrong ------------------------- */

/**
 * Corporate Credit A4: WHAT A LENDER LAST QUOTED A NAME. It is the price of credit to that borrower
 * and it is public, so a buyer of its paper, an equity underwriter and the borrower itself all read
 * the same one (Law 4).
 */
export function creditQuotedTo(reads: WireReads, borrower: string): Option<Event> {
  const said = reads.lastOf(CREDIT_QUOTED, borrower);
  return said === undefined ? none<Event>() : some(said);
}

/** What a bank quoted, and how much it will lend at it. */
export interface CreditQuote {
  readonly rate: Ratio;
  readonly most: number;
}

/**
 * Banks Lending C3.a: THE KEENEST QUOTE THIS NAME WAS GIVEN THIS PERIOD, and how much that bank
 * will lend at it. An underwriter and a bond arranger both price a debut against it and both used
 * to unpack the event themselves, in two copies of one function under one name (Law 4, 0e′.2).
 */
export function creditQuoteThisPeriod(
  reads: WireReads,
  borrower: string,
  at: Period,
): Option<CreditQuote> {
  const said = reads.lastOf(CREDIT_QUOTED, borrower);
  if (said?.period !== at) return none<CreditQuote>();
  const rate = said.data['rate'];
  const most = said.data['most'];
  if (typeof rate !== 'number' || typeof most !== 'number') return none<CreditQuote>();
  // Item 16: two published numbers re-enter the type system here, through their own doors.
  return some({ rate: asRatio(rate, 'what its bank quoted it'), most });
}

/** Corporate Credit A4: the rate alone, from any period — what a name last cost to borrow at. */
export function costOfMoneyQuotedTo(reads: WireReads, borrower: string): Option<Ratio> {
  const said = reads.lastOf(CREDIT_QUOTED, borrower);
  return said === undefined ? none<Ratio>() : rateIn(said.data, 'rate', 'what a bank quoted it');
}

/**
 * The same rate, as the BORROWER itself sees it: the quote it was given while that quote is still
 * current, and the last one it ever had where it is not. What it was quoted is public about it.
 */
export function ownCostOfMoney(
  reads: BankReads & { lastOwnSince(kind: string, since: Period): Option<Event> },
  since: Period,
): Option<Ratio> {
  const own = reads.lastOwnSince(CREDIT_QUOTED, since);
  const quoted = own.some ? own : reads.lastOwn(CREDIT_QUOTED);
  return quoted.some
    ? rateIn(quoted.value.data, 'rate', 'what it was quoted, per annum')
    : none<Ratio>();
}

/** Every name a lender quoted in a period — who a bank was willing to put money behind. */
export function namesQuotedIn(
  reads: WireReads & { ofKindIn(kind: string, at: Period): readonly Event[] },
  at: Period,
): readonly Event[] {
  return reads.ofKindIn(CREDIT_QUOTED, at);
}

/** XI-1: every credit event this world has recorded — a loss is an event and the record is public. */
export function creditDefaults(reads: WireReads): readonly Event[] {
  return reads.ofKind(CREDIT_DEFAULT);
}
