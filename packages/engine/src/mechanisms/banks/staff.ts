/**
 * The people a bank employs, and what they cost the borrower.
 *
 * @spec Banks Lending C1 Banks Lending C1.d Labour A1 Labour A2 Labour A3 Labour D1 XI-14 Law 2 Law 5 Law 19
 *
 * C1.d's operating cost was `loan.operatingCost`: half a per cent a year on every principal, added
 * into every quote and PAID TO NOBODY. A wage bill charged and never paid is margin wearing the
 * clothes of a cost (Law 5), and stating it as a share of the principal is a cost expressed as a
 * share of money, which is what `phoenix/no-value-recipe` refuses on the production side. It was a
 * placeholder that named 13d in its own reason, and this is 13d.
 *
 * SO A BANK EMPLOYS PEOPLE. It posts openings in the same venue every other employer does, in a
 * trade of its own (Labour A3: a lending officer is not a baker), at what an hour is worth to IT —
 * what its book took in over the last period, over the hours that book takes to service. It is
 * matched, or not, by the same rule as everybody else, and it pays the wage through the same
 * instruction. Nothing here is a labour market of its own.
 *
 * AND THE COST IN THE QUOTE IS A READ. What servicing costs per annum on a principal is the hours
 * its book takes, times what it is paying for an hour, over the principal it is servicing. That
 * makes a SMALL loan dearer than a large one out of the same arithmetic — a loan costs about the
 * same to make whatever its size — which is a real fact about lending that no parameter could have
 * expressed, and it moves the day the wage moves (Law 19).
 */
import {
  asAmount,
  asPerPiece,
  asRatio,
  heldAsMoney,
  over,
  type PerPiece,
  pricedAt,
  ratioOf,
  type Ratio,
  scale,
  valueAt,
} from '../../core/measure.js';
import { findVenue, type VenueDecl } from '../../clearing/venue.js';
import type { Order } from '../../clearing/solver.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { sum } from '../../core/num.js';
import { downTick, asQty, type Qty } from '../../core/tick.js';
import { isLoan } from '../../registry/credit.js';
import type { ParticipantView } from '../../world/context.js';
import { yearFraction } from '../../calendar/daycount.js';
import { payrollSince, wageFacing as facingIn } from '../../registry/wages.js';
import { period as periodOf } from '../../calendar/calendar.js';

/** A3: the trade a bank's lending staff are in. Data, and the venue is found by it. */
export const BANKING = 'banking';

export const STAFF_PARAMS = {
  hoursPerLoanPeriod: paramId('bank.hoursPerLoanPeriod'),
  hoursPerLinePeriod: paramId('bank.hoursPerLinePeriod'),
  hoursPerProcess: paramId('bank.hoursPerProcess'),
} as const satisfies Record<string, ParamId>;

/** A3: the trade a bank's dealers are in. A market maker is not a lending officer (Labour A3). */
export const DEALING = 'dealing';

/** What this bank's own book is: the live loans it holds, and what they come to. */
function bookOf(view: ParticipantView): { readonly rows: number; readonly principal: Qty } {
  let rows = 0;
  const principal: Qty[] = [];
  for (const h of view.holdings()) {
    if (!view.instruments.has(h.instrument)) continue;
    const i = view.instruments.get(h.instrument);
    // C2, D4, XI-11: what costs a bank to run is the book it HOLDS. A row it sold is somebody
    // else's to administer and one it bought is its own — and since this walks its own holdings,
    // holding it is the whole of the test. The name on the terms is who wrote it, not whose it is.
    if (!i.status.live || !isLoan(i.terms)) continue;
    const units = sum(h.lots.map((l) => l.qty));
    if (units.value <= 0) continue;
    rows += 1;
    principal.push(units.value);
  }
  return { rows, principal: sum(principal).value };
}

/** C1.d: the hours this bank's book takes to service in a period. A read of its own rows. */
export function hoursNeeded(view: ParticipantView): Qty {
  // The hours one row takes is the TECHNOLOGY and the rows are a read of its own book, so what the
  // book takes is that technology scaled by how many rows there are — a count, not a second unit.
  const perRow = asAmount<'piece'>(
    view.params.count(STAFF_PARAMS.hoursPerLoanPeriod),
    'the hours one row takes',
  );
  return scale(perRow, asRatio(bookOf(view).rows, 'the rows it has'), 'the hours its book takes');
}

/**
 * Labour D1.c, Expectations A2.a: what an hour of this trade costs, as this bank can see it, asked
 * of the venue it would hire in.
 *
 * ITEM 10e.4, FINDING `E-19`: THIS WAS A SECOND COPY AND IT HAD THE WRONG KEY. It looked the going
 * rate up under `region|occupation`; `publishGoingRate` writes it under the VENUE's id. So the
 * fallback could never hit, and a bank that had never met a payroll concluded it could not price an
 * hour and put NO staff cost into any quote it made — for as long as it had no staff of its own,
 * which is exactly when the published rate is the only thing there is to go on. The read is the
 * registry's now, and every employer in this world asks the same one (Law 4).
 */
export function wageFacing(view: ParticipantView, occupation: string): PerPiece | undefined {
  const venue = findVenue(view.venues, { region: String(view.self.region), occupation });
  if (venue === undefined) return undefined;
  const facing = facingIn(view, view.period, venue.id);
  return facing.some ? facing.value : undefined;
}

/**
 * C1.d, Law 19: WHAT SERVICING COSTS, per annum on a principal, and every term is a read. A bank
 * with no book yet has nothing to service and no cost to add; one that has staff but no principal
 * would be dividing by nothing, and that is not a large number, it is no answer.
 */
export function operatingCostOf(view: ParticipantView): Ratio {
  const book = bookOf(view);
  if (book.rows <= 0 || book.principal <= 0) return asRatio(0, 'a bank with no book services nothing');
  const wage = wageFacing(view, BANKING);
  if (wage === undefined) return asRatio(0, 'a bank that cannot price an hour');
  const ofAYear = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(periodOf(view.period + 1)),
  );
  if (ofAYear <= 0) return asRatio(0, 'a period of no length costs nothing');
  const perPeriod = valueAt(wage, hoursNeeded(view), 'what its staff cost it a period');
  return ratioOf(
    over(perPeriod, asRatio(ofAYear, 'the fraction of a year that was'), 'a year of it'),
    // A loan's units ARE money, so what servicing costs per unit of principal is a pure rate.
    heldAsMoney(book.principal, 'the principal it is servicing'),
    'what servicing costs, per unit of principal per annum',
  );
}

/**
 * Labour A1, A2, D1: THE OPENING. It wants the hours its book takes, and it bids what an hour is
 * worth to it — what the book took in over the last period, over those hours. A bank whose book
 * earns nothing bids nothing and hires nobody, which is how a shrinking bank sheds staff without
 * anybody writing a rule for it (C3: the separation is the venue's, and it costs severance).
 */
export function staffOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.key['occupation'] !== BANKING) return [];
  if (venue.key['region'] !== String(view.self.region)) return [];
  const hours = hoursNeeded(view);
  if (hours <= 0) return [];
  const took = view.earned(1);
  if (took <= 0) return [];
  // What an hour of this is worth to it: what the book took in over the hours the book actually
  // takes. That read is exact and stays exact — it is a price, and prices have their own grid.
  const worth = pricedAt(took, hours, 'what an hour of this is worth to it');
  if (worth <= 0) return [];
  /**
   * Law 8, item 0 (stop 9): AN HOUR IS THE PIECE, so what it BIDS FOR is whole hours.
   *
   * `hoursNeeded` is a technology (0.6 of an hour a row) times a count of rows, so a book of two
   * rows takes 1.2 hours — and `asQty` refused it, which stopped the world at period 23 the first
   * time a bank's book was an odd size. The fraction below an hour is not an order that went
   * missing: it is a quantity no venue trades (ARCHITECTURE 4.11a), and a bank whose whole book
   * takes less than an hour hires nobody for it, which is what it means to employ people.
   */
  const want = downTick(hours);
  if (want <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth, qty: asQty(want) }];
}

/** The venue this bank's staff are hired in, or none because this world has no such trade. */
export const staffVenue = (view: ParticipantView): VenueDecl | undefined =>
  findVenue(view.venues, { region: String(view.self.region), occupation: BANKING });

/**
 * Dealer Desks D1, D4, XI-13 (13d): HOW MANY LINES A DESK CAN ACTUALLY QUOTE, and it is people.
 *
 * A desk quotes a line by watching it: somebody prices it, somebody carries the position, somebody
 * answers the phone. So what a desk can cover is the hours it EMPLOYS over the hours one line takes
 * — a read of what it actually paid for, in the same event a firm costs a unit from — and a desk
 * that sheds staff drops lines. Nothing states a coverage; it is a count of people over a technology.
 *
 * A bank that has never paid a wage has no hours and covers nothing, which is a real answer: a desk
 * with nobody on it is not a desk, and a line nobody quotes journals `market.noView` and says so.
 */
export function linesCovered(view: ParticipantView): number {
  const own = view.lastOwnSince('labour.wages', payrollSince(view.period));
  if (!own.some) return 0;
  const hours = own.value.data['hours'];
  if (typeof hours !== 'number' || hours <= 0) return 0;
  const per = asAmount<'piece'>(
    view.params.count(STAFF_PARAMS.hoursPerLinePeriod),
    'the hours one line takes',
  );
  if (per <= 0) return 0;
  // Hours it employs over hours one line takes: two amounts of the same unit, so a pure count.
  return Math.floor(
    ratioOf(
      asAmount<'piece'>(hours, 'the hours it actually paid for'),
      per,
      'the lines its people can cover',
    ),
  );
}


/* --------------------------------------------------------------------------------------------
 * THE FIFTH BUSINESS LINE: CORPORATE FINANCE (item 10f.4)
 *
 * §35 B4, §29 D1: *"formal exit processes and m&a processes lead by IBD departments"*. A sale of a
 * company is not a bilateral tender that appears from nowhere — a seller appoints a bank, the bank
 * invites bidders, the book clears, and the bank is paid for the work.
 *
 * WHAT A BANK CAN RUN AT ONCE IS THE PEOPLE IT EMPLOYS. It is the same read a dealing desk makes
 * about how many lines it can quote (`linesCovered`) and a manager makes about how many pools it
 * can run: hours paid for, over the hours one takes. A bank that sheds its bankers runs fewer
 * processes, and nothing states a capacity anywhere.
 *
 * AND WHAT IT CHARGES IS WHAT THE WORK COSTS IT. Not a percentage of the deal — a percentage of an
 * outcome is a fee with no work in it (Law 2) — but the hours at what an hour of that trade costs
 * it, which differs by bank because what a bank pays differs. The seller appoints the cheapest bank
 * that has room, so the fee falls to what the keenest of them can do it for and nothing bounds the
 * fall (Law 6: the refusal is the mechanism — a bank that cannot cover its people stops bidding).
 * ------------------------------------------------------------------------------------------ */

/** A3: the trade a bank's corporate-finance people are in. A banker is not a dealer (Labour A3). */
export const ADVISORY = 'advisory';

/** The venue this bank's corporate-finance people are hired in, where this world has that trade. */
export const advisoryVenue = (view: ParticipantView): VenueDecl | undefined =>
  findVenue(view.venues, { region: String(view.self.region), occupation: ADVISORY });

/**
 * §35 B4 (10f.4): HOW MANY SALES THIS BANK CAN RUN AT ONCE — its hours over what one takes.
 *
 * The hours are what it actually PAID FOR (`labour.wages`), which is the same event
 * `linesCovered` reads and for the same reason: a capacity is a count of people and a count of
 * people is what a payroll is. A bank that has never met a wage runs nothing, which is a real
 * answer rather than a zero with a rule behind it.
 */
export function processesRun(view: ParticipantView): number {
  const own = view.lastOwnSince('labour.wages', payrollSince(view.period));
  if (!own.some) return 0;
  const hours = own.value.data['hours'];
  if (typeof hours !== 'number' || hours <= 0) return 0;
  const per = asAmount<'piece'>(
    view.params.count(STAFF_PARAMS.hoursPerProcess),
    'the hours one process takes',
  );
  if (per <= 0) return 0;
  return Math.floor(
    ratioOf(
      asAmount<'piece'>(hours, 'the hours it actually paid for'),
      per,
      'the processes its people can run',
    ),
  );
}

/**
 * §35 B4 (10f.4): WHAT RUNNING ONE COSTS IT, which is what it charges for it.
 *
 * Hours at what an hour of that trade costs it (Labour D1.c), and nothing else: no percentage of
 * the deal, no minimum, no retainer. A bank that cannot price an hour of the trade cannot quote for
 * the work, which is a refusal and not a zero (App A).
 */
export function costOfAProcess(view: ParticipantView): PerPiece | undefined {
  const wage = wageFacing(view, ADVISORY);
  if (wage === undefined) return undefined;
  const hours = asAmount<'piece'>(
    view.params.count(STAFF_PARAMS.hoursPerProcess),
    'the hours one process takes',
  );
  if (hours <= 0) return undefined;
  const cost = valueAt(wage, asQty(hours), 'what running one costs it');
  return cost > 0 ? asPerPiece(cost, 'what it charges to run one') : undefined;
}

/**
 * Labour A1, A2, D1: THE OPENING, in the advisory trade. It wants the hours the sales it is running
 * take, and it bids what an hour is worth to it — what its business took in over the last period,
 * over those hours. A bank nobody appointed last period wants nobody this period, which is how a
 * corporate-finance department shrinks when the deals stop without anybody writing a rule for it.
 */
export function advisoryOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.key['occupation'] !== ADVISORY) return [];
  if (venue.key['region'] !== String(view.self.region)) return [];
  const ran = view.lastOwnSince('advisory.ran', payrollSince(view.period));
  if (!ran.some) return [];
  const count = ran.value.data['processes'];
  if (typeof count !== 'number' || count <= 0) return [];
  const per = view.params.count(STAFF_PARAMS.hoursPerProcess);
  const hours = asQty(scale(asAmount<'piece'>(per, 'the hours one takes'), asRatio(count, 'the ones it ran'), 'the hours they took'));
  if (hours <= 0) return [];
  const took = view.earned(1);
  if (took <= 0) return [];
  const worth = pricedAt(took, hours, 'what an hour of this is worth to it');
  if (worth <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth, qty: hours }];
}
