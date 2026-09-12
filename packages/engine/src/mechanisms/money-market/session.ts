/**
 * The session: what every bank posts out of its own position, what clears, and the rows it writes.
 *
 * @spec Money Market A1 Money Market A1.a Money Market A1.b Money Market A2 Money Market A2.a Money Market A3 Money Market A3.a Money Market B1 Money Market B2 Money Market B2.a Money Market B3 Money Market B3.c Money Market B4 Money Market B5 Money Market B5.a Money Market B6 Money Market B7 Money Market C1 Money Market C1.a Money Market C2 Money Market C4 Money Market C4.a Money Market C4.b Money Market E1 Banks Funding C4 Banks Funding D1 Central Bank D1 Clearing B2 Clearing C5 XI-15
 *
 * A1: NOBODY DECIDED THE POSITION. A bank's reserve balance moved because its customers paid other
 * banks' customers, and the net of that is read off the wire's own interbank legs (Law 19). A1.a and
 * A1.b then hold by construction: those legs sum to zero across the banks, so one bank's deficit IS
 * another's surplus and the market has two sides without anybody being told to take one.
 *
 * A3, A3.a: THE SESSION IS AFTER THE FLOWS. It runs in a later cycle than every market, every wage
 * and every invoice in the period, because a session held before them cannot see the thing it
 * exists to fund.
 *
 * B1: EVERY BANK POSTS A SCHEDULE. Who lends and who borrows is the outcome of the schedules
 * meeting, not a rule about surplus banks — and a bank above its buffer that finds no name it will
 * lend to at a rate it will accept simply parks at the floor, which is B5.a from the other side.
 *
 * B2 needs the rate to be THE NAME'S: a book with two borrowers in it and one uniform price cannot
 * make a doubted name pay more. So a session is held per borrowing name, in each of B6's books, and
 * what a lender posts there is what it requires of THAT name — its own published reservation, which
 * is the number its bond schedules and its loan quotes are struck from too (Law 4).
 */
import { compareCivil, addDays, type Civil } from '../../calendar/civil.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { findVenue } from '../../clearing/venue.js';
import {
  currencyUnit,
  venueId,
  type CurrencyCode,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { add, atMost, div, mul, sub, sum } from '../../core/num.js';
import { downTick, upTick, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { advances, borrowingPower, windowAdvances, type Advance } from './collateral.js';
import { BOOKS, type BookDecl } from './data.js';
import { INTERBANK, isRow, REPO, rowId, type Pledged, type RowTerms } from './rows.js';

/** Clearing B2, C5: one book for one name, declared like a market and public like one. */
export function sessionVenue(book: BookDecl, borrower: PartyId): VenueId {
  return venueId(`mm.${book.id}.${borrower}`);
}

export function venueKey(book: BookDecl, borrower: PartyId): Record<string, string> {
  return { market: 'money', book: book.id, tenor: book.tenor, secured: String(book.secured), borrower };
}

export function venuesOf(ccy: CurrencyCode, borrowers: readonly PartyId[]): VenueDecl[] {
  const out: VenueDecl[] = [];
  for (const book of BOOKS) {
    for (const borrower of borrowers) {
      out.push({
        id: sessionVenue(book, borrower),
        name: `${borrower} ${book.tenor} ${book.secured ? 'secured' : 'unsecured'}`,
        clearedBy: 'money-market',
        unit: currencyUnit(ccy),
        ccy,
        key: venueKey(book, borrower),
      });
    }
  }
  return out;
}

/** B5: how somebody who is not a bank finds the book it wants to lend into, without knowing us. */
export function findSession(
  venues: readonly VenueDecl[],
  book: string,
  borrower: PartyId,
): VenueDecl | undefined {
  return findVenue(venues, { market: 'money', book, borrower });
}

/**
 * C1, B5.a: the same ladder from the LENDER's side — what is owed to this bank at the start of the
 * next period. A bank that parked its spare cash at the floor has not stopped holding it: it holds
 * an overnight claim on the central bank that turns back into its account balance tomorrow morning,
 * and that is as liquid as an asset gets. Read off the rows it holds, never off what it parked.
 */
export function fallsDueToIt(ctx: MechanismContext, lender: PartyId, ccy: CurrencyCode): number {
  const next = ctx.period + 1;
  const terms: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isRow(i.terms) || i.terms.lender !== lender || i.ccy !== ccy) continue;
    if (ctx.calendar.periodOf(i.terms.maturity) !== next) continue;
    const held = ctx.register.quantity(lender, i.id);
    if (held <= 0) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    const flows = profile.cashFlows(i, ctx.calendar.startOf(ctx.period), ctx.calendar);
    const perUnit = sum(flows.map((f) => f.perUnit)).value;
    terms.push(mul(held, perUnit, 'what comes back tomorrow'));
  }
  return sum(terms).value;
}

/** A1: the net of what this bank's customers paid other banks' customers this period (Law 19). */
export function netReserveFlow(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.reserveLegs) {
      if (leg.bank !== bank || leg.ccy !== ccy) continue;
      terms.push(leg.amount);
    }
  }
  return sum(terms).value;
}

/** The corridor's two levels, which are the policy rate plus what the central bank chose (C3). */
export interface Corridor {
  readonly policy: number;
  readonly floor: number;
  readonly ceiling: number;
}

/** What a lender has to place, and what it will take for it. */
export interface Posted {
  readonly orders: readonly Order[];
  /** B7: names that found no bid at all, with why, so the refusal is a recorded outcome. */
  readonly refusals: readonly { readonly lender: PartyId; readonly why: string }[];
}

/**
 * C4, C4.a, C4.b, Central Bank D1: the window's seat in the session. It lends at the top of the
 * corridor, against collateral, and only as much as the borrower's own unencumbered eligible paper
 * covers — so a bank prefers the market (C4.a) and a bank out of paper cannot draw (C4.b).
 */
export function windowOffer(
  cbView: ParticipantView,
  borrowerView: ParticipantView,
  book: BookDecl,
  corridor: Corridor,
  on: Civil,
  policyHaircut: number,
): readonly Order[] {
  // C4: only against paper. The window has no unsecured seat at all, which is C5 in the one place
  // it would otherwise be quietly broken.
  if (!book.secured) return [];
  // Law 8: what the collateral is worth after its haircut is a valuation, and what the window
  // lends is money — whole cents of it, and down, because lending the cent above what the paper
  // covers is lending unsecured (C4).
  const power = downTick(borrowingPower(windowAdvances(cbView, borrowerView, on, policyHaircut)));
  if (power <= 0) return [];
  return [{ party: cbView.self.id, side: 'sell', price: corridor.ceiling, qty: power }];
}

/** What cleared in one book for one name (B4), and what it means for the row that gets written. */
export interface Struck {
  readonly lender: PartyId;
  readonly borrower: PartyId;
  /** Law 8: money, as a count of the money's own smallest piece. */
  readonly amount: Qty;
  readonly rate: number;
  readonly book: BookDecl;
}

/**
 * B4: the rate clears from the schedules meeting each other. Lenders undercut one another, so the
 * level is the lowest that clears the volume — which is why a borrower with two eager lenders pays
 * less than one with a single grudging one, and why nothing here writes a spread.
 */
export function strike(
  posted: readonly Order[],
  borrower: PartyId,
  book: BookDecl,
): readonly Struck[] {
  const outcome = clear(posted, 'proRata', 'sellersCompete');
  if (!isCleared(outcome)) return [];
  const out: Struck[] = [];
  for (const f of outcome.fills) {
    // Law 8: money is lent in whole pieces of itself. Rationing gives a lender a share of what it
    // offered, and the share below the piece it could actually hand over is not lent.
    const amount = downTick(f.qty);
    if (f.side !== 'sell' || amount <= 0) continue;
    out.push({ lender: f.party, borrower, amount, rate: outcome.price, book });
  }
  return out;
}

/** The collateral a borrower puts up for one row, at the lender's own valuation (B3.b). */
export function coverFor(available: readonly Advance[], amount: number): readonly Pledged[] {
  const out: Pledged[] = [];
  let left = amount;
  for (const a of available) {
    // Law 8: a lien binds WHOLE PIECES of the paper. What it takes to cover the rest is rounded up
    // — collateral a piece short is collateral that does not cover — unless the parcel runs out
    // first, in which case it binds every whole piece of it there is.
    if (left <= 0) break;
    const want = div(left, a.valuePerUnit, 'units to cover');
    const enough = upTick(want);
    const units = atMost(enough, downTick(a.free), 'it binds no more of a parcel than there is of it');
    if (units <= 0) continue;
    out.push({ instrument: a.instrument, qty: units, valuedAt: a.valuePerUnit });
    left = sub(left, mul(units, a.valuePerUnit, 'covered'), 'left to cover');
  }
  return out;
}

/**
 * The row itself: the borrower issues it to the lender and the money moves the other way, in one
 * instruction (XI-5). A secured row binds its collateral in the SAME instruction (B3.c): paper
 * bound in one pass and lent against in another was briefly nobody's.
 */
export function writeRow(
  ctx: MechanismContext,
  s: Struck,
  n: number,
  ccy: CurrencyCode,
  cover: readonly Pledged[],
): Option<string> {
  const what = s.book.secured ? 'repo' : 'interbank';
  const id = rowId(what, s.lender, s.borrower, n);
  const drawn = ctx.calendar.startOf(ctx.period);
  const maturity = addDays(drawn, s.book.periods * ctx.calendar.periodDays);
  const terms: RowTerms = {
    kind: s.book.secured ? REPO : INTERBANK,
    lender: s.lender,
    borrower: s.borrower,
    rate: s.rate,
    drawn,
    maturity,
    dayCount: 'ACT/365F',
    tenor: s.book.tenor,
    collateral: cover,
  };
  ctx.issue({ id, kind: terms.kind, issuer: some(s.borrower), ccy, terms, market: none() });
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: s.borrower,
      to: s.lender,
      instrument: id,
      qty: s.amount,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: ctx.accountOf(s.lender, ccy),
      to: ctx.accountOf(s.borrower, ccy),
      ccy,
      amount: s.amount,
      fromCell: none(),
      toCell: none(),
    },
    ...cover.map(
      (c): Leg => ({
        kind: 'pledge',
        pledgor: s.borrower,
        beneficiary: s.lender,
        instrument: c.instrument,
        qty: c.qty,
        secures: String(id),
        pledgorCell: none(),
      }),
    ),
  ];
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${s.lender} lends ${s.amount} to ${s.borrower} ${s.book.tenor}`,
  });
  return r.outcome === 'settled' ? some(String(id)) : none<string>();
}

/** Every bank that is alive and that this module knows how to fund. */
export function banksOf(ctx: MechanismContext): readonly PartyId[] {
  return ctx.parties
    .ofKind(BANK)
    .filter((b) => b.status.alive)
    .map((b) => b.id);
}

/** Whether a date is at or after another, for the reads that ask whether a row has run out. */
export const notBefore = (a: Civil, b: Civil): boolean => compareCivil(a, b) >= 0;

/** The whole amount of a schedule's asks, which is what a standing bid at the floor answers (C1). */
export function offered(orders: readonly Order[]): number {
  return sum(orders.filter((o) => o.side === 'sell').map((o) => o.qty)).value;
}

/** What a borrower still needs after part of it has been raised (B7: the rest is a refusal). */
export const stillNeeded = (need: number, raised: number): number =>
  need > raised ? sub(need, raised, 'still short') : 0;

/** The rate a book actually struck for a name, for the reads and the publication (B4, C3). */
export function struckRate(rows: readonly Struck[]): Option<number> {
  const first = rows[0];
  return first === undefined ? none<number>() : some(first.rate);
}

/** B6.a: what the two tenors printed, so the gap between them can be read rather than stated. */
export function averageRate(rows: readonly Struck[]): Option<number> {
  if (rows.length === 0) return none<number>();
  const weight = sum(rows.map((r) => r.amount)).value;
  if (weight <= 0) return none<number>();
  return some(div(sum(rows.map((r) => mul(r.amount, r.rate, 'weighted'))).value, weight, 'rate'));
}

/** Everything a lender could advance to this borrower, most valuable parcel first (C4.b). */
export function collateralFor(
  ctx: MechanismContext,
  lender: PartyId,
  borrower: PartyId,
  on: Civil,
): readonly Advance[] {
  return advances(ctx.participant(lender), ctx.participant(borrower), on);
}

/** The total a schedule bid for, which is what the window's own seat is measured against. */
export function bidFor(orders: readonly Order[]): number {
  return sum(orders.filter((o) => o.side === 'buy').map((o) => o.qty)).value;
}

/** Money Market C1: the standing bid at the floor, for whatever the session offers it. */
export function floorBid(cb: PartyId, amount: Qty, corridor: Corridor): readonly Order[] {
  return amount > 0 ? [{ party: cb, side: 'buy', price: corridor.floor, qty: amount }] : [];
}

/** The corridor from the policy rate and the two spreads the central bank chose (C3, B3). */
export function corridorOf(policy: number, floorSpread: number, ceilingSpread: number): Corridor {
  return {
    policy,
    floor: sub(policy, floorSpread, 'the floor'),
    ceiling: add(policy, ceilingSpread, 'the ceiling'),
  };
}
