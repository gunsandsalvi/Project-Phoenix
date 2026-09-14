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
import { asAmount, asCash, asPerPiece, type PerPiece, pricedAt } from '../core/measure.js';
import type { VenueId } from '../core/ids.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';

/** What the labour module publishes about a payroll, and about what an hour cleared at. */
const OWN_PAYROLL = 'labour.wages';
const GOING_RATE = 'labour.goingRate';

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

/**
 * D1.c, E2: what an hour costs the party asking, in the venue it would hire in. Its own wage bill
 * over its own hours where it has one; what that venue last cleared at where it has not; and
 * NOTHING where it has neither, which is an employer that has never met a wage.
 */
export function wageFacing(reads: WageReads, at: Period, venue: VenueId): Option<PerPiece> {
  const own = reads.lastOwnSince(OWN_PAYROLL, payrollSince(at));
  if (own.some) {
    const due = own.value.data['due'];
    const hours = own.value.data['hours'];
    if (typeof due === 'number' && typeof hours === 'number' && hours > 0) {
      // Item 16: its own wage bill re-enters here — what it paid, over the hours it paid for.
      return some(
        pricedAt(
          asCash(due, 'what its wage bill came to'),
          asAmount<'piece'>(hours, 'the hours it paid for'),
          'what an hour cost it',
        ),
      );
    }
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

/**
 * Labour C2: the hours it has UNDER CONTRACT, read off its own last payroll. With a hiring lag of a
 * period those are exactly the hours that can do work now; nobody hired since is in it, and that is
 * right, because they are not productive yet.
 */
export function hoursPaidFor(reads: WageReads, at: Period): number {
  const own = reads.lastOwnSince(OWN_PAYROLL, payrollSince(at));
  if (!own.some) return 0;
  const hours = own.value.data['hours'];
  return typeof hours === 'number' && hours > 0 ? hours : 0;
}
