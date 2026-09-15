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
import { asCash, asPerPiece, asRatio, type Cash, heldAsMoney, type PerPiece, plus, pricedAt, scale } from '../core/measure.js';
import { addQty, NO_QTY, type Qty, scaleQty } from '../core/tick.js';
import type { PartyId, RegionId, VenueId } from '../core/ids.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { SettlementRecord } from '../ledger/instruction.js';
import { type EmploymentReads, type EmploymentRow, wagePerMember } from '../register/employment.js';

const GOING_RATE = 'labour.goingRate';
const PRINTED_WAGE = 'labour.print';

/**
 * 12b.1: WHAT A PARTY PAYS ITS PEOPLE IS ITS ROWS. `ownPayroll` was a read of the `labour.wages`
 * tally the labour module published once a period — hours, due, paid, productive, headcount per
 * employer — a stored aggregate of the register re-derived every period (Appendix B) and left
 * standing when the rows changed. It is the register now (`view.employs()`, `ctx.employment`), and
 * what was PAID is the ledger's.
 */
export interface WageReads {
  lastPublic(kind: string): Option<Event>;
  employs(): readonly EmploymentRow[];
}

export function payrollSince(at: Period): Period {
  return at > 0 ? periodOf(at - 1) : at;
}

export interface OwnPayroll {
  readonly hours: Qty;
  readonly due: Cash;
}

/** E1: what its rows say it has under contract and owes for it — nothing when it employs nobody. */
export function ownPayroll(reads: Pick<WageReads, 'employs'>, at: Period): Option<OwnPayroll> {
  const rows = reads.employs();
  if (rows.length === 0) return none<OwnPayroll>();
  let hours = NO_QTY;
  let due = asCash(0, 'nothing due yet');
  for (const r of rows) {
    if (r.since > at) continue;
    hours = addQty(hours, scaleQty(r.hoursPerMember, r.headcount, 'hours under contract'), 'hours');
    due = plus(due, scale(wagePerMember(r), asRatio(r.headcount, 'the people on it'), 'wage bill'), 'wages due');
  }
  return some({ hours, due });
}

export function wageFacing(reads: WageReads, at: Period, venue: VenueId): Option<PerPiece> {
  const own = ownPayroll(reads, at);
  if (own.some && own.value.hours > 0) {
    // Item 16: its own wage bill re-enters here — what it owes, over the hours it owes it for.
    return some(pricedAt(own.value.due, own.value.hours, 'what an hour costs it'));
  }
  return goingRateIn(reads, venue);
}

export function goingRateIn(reads: Pick<WageReads, 'lastPublic'>, venue: VenueId): Option<PerPiece> {
  const published = reads.lastPublic(GOING_RATE);
  if (!published.some) return none<PerPiece>();
  const rates = published.value.data['wagePerHour'];
  if (typeof rates !== 'object' || rates === null) return none<PerPiece>();
  const rate = (rates as Record<string, unknown>)[String(venue)];
  // Item 16: the going rate re-enters here — what an hour cleared at in that trade, in that place.
  return typeof rate === 'number' && rate > 0
    ? some(asPerPiece(rate, 'what an hour cleared at there'))
    : none<PerPiece>();
}

/* --------------------------------------------------------------------------------------------
 * THE SAME READS FOR A PHASE, ABOUT A NAMED PARTY
 * ------------------------------------------------------------------------------------------ */

export interface PartyWageReads {
  ofKind(kind: string): readonly Event[];
}

/** The doors a phase has: the register for the rows, the ledger for what moved, the wire for prints. */
export interface PayrollReads {
  readonly employment: Pick<EmploymentReads, 'payrollOf' | 'everEmployed'>;
  readonly journal: PartyWageReads;
  readonly ledger: { inPeriod(period: Period): readonly SettlementRecord[] };
}

export function ownPayrollOf(reads: Pick<PayrollReads, 'employment'>, who: PartyId, at: Period): Option<OwnPayroll> {
  const p = reads.employment.payrollOf(who, at);
  if (p.headcount === 0) return none<OwnPayroll>();
  return some({ hours: p.hours, due: p.due });
}

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

export function wageFacingParty(
  reads: Pick<PayrollReads, 'employment' | 'journal'>,
  who: PartyId,
  at: Period,
  region: RegionId,
): Option<PerPiece> {
  const own = ownPayrollOf(reads, who, at);
  if (own.some && own.value.hours > 0) {
    return some(pricedAt(own.value.due, own.value.hours, 'what an hour costs it'));
  }
  return wagePrintedIn(reads.journal, region);
}

export interface PayrollSettled {
  readonly paid: Cash;
  readonly productive: Qty;
}

/**
 * E1, Law 19: WHAT IT PAID ITS PEOPLE THIS PERIOD is what the wire moved — every settled wage leg
 * out of its own account — and the hours that can make something are the rows' (C2).
 */
export function payrollSettledIn(reads: PayrollReads, who: PartyId, at: Period): Option<PayrollSettled> {
  const p = reads.employment.payrollOf(who, at);
  if (p.headcount === 0) return none<PayrollSettled>();
  let paid = asCash(0, 'nothing paid yet');
  for (const r of reads.ledger.inPeriod(at)) {
    if (r.outcome !== 'settled') continue;
    for (const l of r.instruction.legs) {
      if (l.kind !== 'money' || l.receipt?.of !== 'wage' || l.from.holder !== who) continue;
      paid = plus(paid, heldAsMoney(l.amount, 'a wage it paid'), 'wages paid');
    }
  }
  return some({ paid, productive: p.productive });
}

/** Whether this party has ever employed anybody: a row of its in the register, live or ended. */
export function hasEverMetAPayroll(reads: Pick<PayrollReads, 'employment'>, who: PartyId): boolean {
  return reads.employment.everEmployed(who);
}
