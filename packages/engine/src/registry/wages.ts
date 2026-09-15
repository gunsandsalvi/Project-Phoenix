/**
 * WHAT AN HOUR COSTS THE PARTY LOOKING AT IT — one read, for every employer in this world.
 *
 * @spec Labour D1.c Labour E2 Expectations A2.a Law 4 Law 19
 *
 * WHY THIS IS KERNEL DATA AND NOT THE LABOUR MODULE'S. Three modules have to ask the same question
 * — a firm costing a unit, a bank costing a loan, a manager costing a pool — and none of them may
 * import the module that answers it (`phoenix/no-cross-module-import`). It is the same decision
 * `registry/physical.ts` already took for `storageRateIn`, for the same reason: what a market
 * PUBLISHED is public, and a read of a public fact belongs where everybody can reach it.
 *
 * IT WAS WRITTEN TWICE AND THE TWO COPIES DID NOT AGREE (item 10e.4, finding `E-19`). The firm's
 * copy keyed the going rate by the VENUE's id, which is what `publishGoingRate` actually writes;
 * the bank's keyed it by `region|occupation`, which is a key that has never existed — so a bank
 * that had never met a payroll fell through to a lookup that could not hit, concluded it could not
 * price an hour, and put NO staff cost in any quote it made. That is what a second copy of a
 * formula is for, and it is the third time this module's rules have caught one.
 *
 * Expectations A2.a: what a party faces is ITS OWN experience first — what its own last wage bill
 * came to, over the hours it paid for — and what the market published only where it has none. An
 * employer with neither has never met a wage and cannot price an hour, which is a refusal and not
 * a zero (App A).
 */
import { period as periodOf, type Period } from '../calendar/calendar.js';
import { asAmount, asCash, asPerPiece, type Cash, type PerPiece, pricedAt } from '../core/measure.js';
import type { Qty } from '../core/tick.js';
import type { RegionId, VenueId } from '../core/ids.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';

/** What the labour module publishes about a payroll, and about what an hour cleared at. */
const OWN_PAYROLL = 'labour.wages';
const GOING_RATE = 'labour.goingRate';
const PRINTED_WAGE = 'labour.print';

/** The narrow door: a party's own last event of a kind, and the last public one. */
export interface WageReads {
  lastOwnSince(kind: string, since: Period): Option<Event>;
  lastPublic(kind: string): Option<Event>;
}

/**
 * A-33, Law 8: THE PERIOD A WAGE BILL IS STILL CURRENT IN.
 *
 * `labour.pay` settles in cycle 2, so a reader in an earlier cycle of period p means the bill of
 * p−1 and one after it means p's own; either is current. Anything OLDER belongs to an employer that
 * has employed nobody since — `payWages` writes an event only for an employer with rows — and
 * reading it as current is how a firm that shed its last worker went on force-selling stock to
 * cover a payroll of nobody for the rest of the run (A-60).
 */
export function payrollSince(at: Period): Period {
  return at > 0 ? periodOf(at - 1) : at;
}

/** What a party's own last wage bill came to, and over how many hours. */
export interface OwnPayroll {
  /** Labour C2: the hours it has UNDER CONTRACT — with a hiring lag, the hours that can work now. */
  readonly hours: Qty;
  /** Labour D1: what that bill came to, which is what it is committed to pay out. */
  readonly due: Cash;
}

/**
 * Labour C2, D1, Law 4, Law 19: A PARTY'S OWN LAST PAYROLL, extracted ONCE.
 *
 * Five call sites in three modules used to reach for `labour.wages` themselves and pull `hours` or
 * `due` out of its data — five copies of one extraction, each with its own idea of what a missing
 * field meant. The kinds a module may not name are named here (`phoenix/no-cross-module-event-read`),
 * and so is what the fields mean when they are read back.
 *
 * A party with no bill since `payrollSince` has employed nobody since, and that is NOTHING rather
 * than a zero (App A): a caller that wants a zero says so at its own site and says why.
 */
export function ownPayroll(reads: WageReads, at: Period): Option<OwnPayroll> {
  const own = reads.lastOwnSince(OWN_PAYROLL, payrollSince(at));
  if (!own.some) return none<OwnPayroll>();
  const hours = own.value.data['hours'];
  const due = own.value.data['due'];
  if (typeof hours !== 'number' || typeof due !== 'number') return none<OwnPayroll>();
  // Item 16: a published number re-enters the type system here, through the dimension's own door.
  return some({
    hours: asAmount<'piece'>(hours, 'the hours it paid for'),
    due: asCash(due, 'what its wage bill came to'),
  });
}

/**
 * D1.c, E2: what an hour costs the party asking, in the venue it would hire in. Its own wage bill
 * over its own hours where it has one; what that venue last cleared at where it has not; and
 * NOTHING where it has neither, which is an employer that has never met a wage.
 */
export function wageFacing(reads: WageReads, at: Period, venue: VenueId): Option<PerPiece> {
  const own = ownPayroll(reads, at);
  if (own.some && own.value.hours > 0) {
    // Item 16: its own wage bill re-enters here — what it paid, over the hours it paid for.
    return some(pricedAt(own.value.due, own.value.hours, 'what an hour cost it'));
  }
  const published = reads.lastPublic(GOING_RATE);
  if (!published.some) return none<PerPiece>();
  const rates = published.value.data['wagePerHour'];
  if (typeof rates !== 'object' || rates === null) return none<PerPiece>();
  // Law 4: keyed by the VENUE, because that is what `publishGoingRate` writes. One region's trade
  // is one venue, so the venue IS the (region, occupation) this employer would hire in.
  const rate = (rates as Record<string, unknown>)[String(venue)];
  // Item 16: the going rate re-enters here — what an hour cleared at where this employer is.
  return typeof rate === 'number' && rate > 0
    ? some(asPerPiece(rate, 'what an hour cleared at where it is'))
    : none<PerPiece>();
}

/* --------------------------------------------------------------------------------------------
 * THE SAME QUESTIONS, ASKED ABOUT A NAMED PARTY.
 *
 * A participant asks about itself and gets `ownPayroll`. A phase asks about a party it is running
 * for and has only the journal, so it asked the wire directly — and the treasury module ended up
 * with its own `currentPayroll`, its own `payrollSince`, and its own `wageFacing`, which is the
 * THIRD copy of the formula this file exists to hold (Law 4). These are that same read with the
 * other door: what a phase has is a journal, so the journal is what they take.
 * ------------------------------------------------------------------------------------------ */

/** The phase-scoped door: the wire, asked about a named party or a named kind. */
export interface PartyWageReads {
  forSubject(kind: string, subject: string): readonly Event[];
  ofKind(kind: string): readonly Event[];
  lastOf(kind: string, subject: string): Event | undefined;
}

/** Labour C2, D1, A-33: a named party's own last payroll, if it is still current. */
export function ownPayrollOf(reads: PartyWageReads, who: string, at: Period): Option<OwnPayroll> {
  const events = reads.forSubject(OWN_PAYROLL, who);
  const last = events[events.length - 1];
  if (last === undefined || last.period < payrollSince(at)) return none<OwnPayroll>();
  const hours = last.data['hours'];
  const due = last.data['due'];
  if (typeof hours !== 'number' || typeof due !== 'number') return none<OwnPayroll>();
  // Item 16: a published number re-enters the type system here, through the dimension's own door.
  return some({
    hours: asAmount<'piece'>(hours, 'the hours it paid for'),
    due: asCash(due, 'what its wage bill came to'),
  });
}

/**
 * Labour E2, Expectations A2.a: what the labour venue LAST PRINTED where a party is.
 *
 * Region-scoped, because an hour is hired in a place: a state reading the last print anywhere was
 * reading another country's wage whenever that country printed later, which is a wage in the wrong
 * money as often as not (0e′.1). A region that has never printed has nothing, which is a refusal.
 */
export function wagePrintedIn(reads: PartyWageReads, region: RegionId): Option<PerPiece> {
  let last: PerPiece | undefined;
  for (const e of reads.ofKind(PRINTED_WAGE)) {
    if (e.data['region'] !== String(region)) continue;
    const wage = e.data['wagePerHour'];
    // Item 16: a wage re-enters from what the labour venue published — a level per hour.
    if (typeof wage === 'number' && wage > 0) last = asPerPiece(wage, 'what an hour cleared at');
  }
  return last === undefined ? none<PerPiece>() : some(last);
}

/** D1.c, E2: `wageFacing` for a party a phase names — its own bill first, its region's print after. */
export function wageFacingParty(
  reads: PartyWageReads,
  who: string,
  at: Period,
  region: RegionId,
): Option<PerPiece> {
  const own = ownPayrollOf(reads, who, at);
  if (own.some && own.value.hours > 0) {
    return some(pricedAt(own.value.due, own.value.hours, 'what an hour cost it'));
  }
  return wagePrintedIn(reads, region);
}

/** What a payroll SETTLED to this period: what went out, and the hours that can make something. */
export interface PayrollSettled {
  readonly paid: Cash;
  readonly productive: Qty;
}

/**
 * Labour C2, Goods B1.c: the payroll a named party actually PAID in a period — a different fact
 * from what it owes, and the one a firm making something this period runs on.
 */
export function payrollSettledIn(
  reads: PartyWageReads,
  who: string,
  at: Period,
): Option<PayrollSettled> {
  const events = reads.forSubject(OWN_PAYROLL, who).filter((e) => e.period === at);
  const last = events[events.length - 1];
  if (last === undefined) return none<PayrollSettled>();
  const paid = last.data['paid'];
  const productive = last.data['productive'];
  if (typeof paid !== 'number' || typeof productive !== 'number') return none<PayrollSettled>();
  // Item 16: two published numbers re-enter the type system here, through their own doors.
  return some({
    paid: asCash(paid, 'what it paid its people'),
    productive: asAmount<'piece'>(productive, 'the hours that can make something'),
  });
}

/**
 * §35 A4, §29 A5: WHETHER A PARTY HAS EVER MET A PAYROLL — the whole test of whether it could run
 * a business rather than merely own one. A fact off the wire, not a label (Law 15).
 */
export function hasEverMetAPayroll(reads: PartyWageReads, who: string): boolean {
  return reads.lastOf(OWN_PAYROLL, who) !== undefined;
}
