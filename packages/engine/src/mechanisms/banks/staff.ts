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
  asCash,
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
import { asQty, type Qty } from '../../core/tick.js';
import { isLoan } from '../../registry/credit.js';
import type { ParticipantView } from '../../world/context.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period as periodOf } from '../../calendar/calendar.js';

/** A3: the trade a bank's lending staff are in. Data, and the venue is found by it. */
export const BANKING = 'banking';

export const STAFF_PARAMS = {
  hoursPerLoanPeriod: paramId('bank.hoursPerLoanPeriod'),
  hoursPerLinePeriod: paramId('bank.hoursPerLinePeriod'),
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
 * Labour D1.c, Expectations A2.a: what an hour of this trade costs, as this bank can see it — what
 * its OWN wage bill came to per hour where it has one, and the published going rate where it has
 * not. A bank that has neither has no idea what staff cost and cannot put a cost in its quote.
 */
export function wageFacing(view: ParticipantView, occupation: string): PerPiece | undefined {
  const own = view.lastOwn('labour.wages');
  if (own.some) {
    const due = own.value.data['due'];
    const hours = own.value.data['hours'];
    if (typeof due === 'number' && typeof hours === 'number' && hours > 0) {
      // Item 16: its own wage bill re-enters here — what it paid, over the hours it paid for.
      return pricedAt(
        asCash(due, 'what its wage bill came to'),
        asAmount<'piece'>(hours, 'the hours it paid for'),
        'what an hour cost it',
      );
    }
  }
  const published = view.lastPublic('labour.goingRate');
  if (!published.some) return undefined;
  const rates = published.value.data['wagePerHour'];
  if (typeof rates !== 'object' || rates === null) return undefined;
  const here = (rates as Record<string, unknown>)[`${String(view.self.region)}|${occupation}`];
  return typeof here === 'number' && here > 0
    ? asPerPiece(here, 'what an hour cleared at where it is')
    : undefined;
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
  const worth = pricedAt(took, hours, 'what an hour of this is worth to it');
  if (worth <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth, qty: asQty(hours) }];
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
  const own = view.lastOwn('labour.wages');
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
