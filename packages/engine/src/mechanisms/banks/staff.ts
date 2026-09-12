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
import { findVenue, type VenueDecl } from '../../clearing/venue.js';
import type { Order } from '../../clearing/solver.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { div, mul, sum } from '../../core/num.js';
import { asQty } from '../../core/tick.js';
import { isLoan } from '../../registry/credit.js';
import type { ParticipantView } from '../../world/context.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period as periodOf } from '../../calendar/calendar.js';

/** A3: the trade a bank's lending staff are in. Data, and the venue is found by it. */
export const BANKING = 'banking';

export const STAFF_PARAMS = {
  hoursPerLoanPeriod: paramId('bank.hoursPerLoanPeriod'),
} as const satisfies Record<string, ParamId>;

/** What this bank's own book is: the live loans it holds, and what they come to. */
function bookOf(view: ParticipantView): { readonly rows: number; readonly principal: number } {
  let rows = 0;
  const principal: number[] = [];
  for (const h of view.holdings()) {
    if (!view.instruments.has(h.instrument)) continue;
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !isLoan(i.terms) || i.terms.lender !== view.self.id) continue;
    const units = sum(h.lots.map((l) => l.qty));
    if (units.value <= 0) continue;
    rows += 1;
    principal.push(units.value);
  }
  return { rows, principal: sum(principal).value };
}

/** C1.d: the hours this bank's book takes to service in a period. A read of its own rows. */
export function hoursNeeded(view: ParticipantView): number {
  return mul(
    bookOf(view).rows,
    view.params.count(STAFF_PARAMS.hoursPerLoanPeriod),
    'the hours its book takes',
  );
}

/**
 * Labour D1.c, Expectations A2.a: what an hour of this trade costs, as this bank can see it — what
 * its OWN wage bill came to per hour where it has one, and the published going rate where it has
 * not. A bank that has neither has no idea what staff cost and cannot put a cost in its quote.
 */
export function wageFacing(view: ParticipantView, occupation: string): number | undefined {
  const own = view.lastOwn('labour.wages');
  if (own.some) {
    const due = own.value.data['due'];
    const hours = own.value.data['hours'];
    if (typeof due === 'number' && typeof hours === 'number' && hours > 0) {
      return div(due, hours, 'what an hour cost it');
    }
  }
  const published = view.lastPublic('labour.goingRate');
  if (!published.some) return undefined;
  const rates = published.value.data['wagePerHour'];
  if (typeof rates !== 'object' || rates === null) return undefined;
  const here = (rates as Record<string, unknown>)[`${String(view.self.region)}|${occupation}`];
  return typeof here === 'number' && here > 0 ? here : undefined;
}

/**
 * C1.d, Law 19: WHAT SERVICING COSTS, per annum on a principal, and every term is a read. A bank
 * with no book yet has nothing to service and no cost to add; one that has staff but no principal
 * would be dividing by nothing, and that is not a large number, it is no answer.
 */
export function operatingCostOf(view: ParticipantView): number {
  const book = bookOf(view);
  if (book.rows <= 0 || book.principal <= 0) return 0;
  const wage = wageFacing(view, BANKING);
  if (wage === undefined) return 0;
  const ofAYear = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(periodOf(view.period + 1)),
  );
  if (ofAYear <= 0) return 0;
  const perPeriod = mul(hoursNeeded(view), wage, 'what its staff cost it a period');
  return div(
    div(perPeriod, ofAYear, 'a year of it'),
    book.principal,
    'per unit of the principal it is servicing',
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
  const worth = div(took, hours, 'what an hour of this is worth to it');
  if (worth <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth, qty: asQty(hours) }];
}

/** The venue this bank's staff are hired in, or none because this world has no such trade. */
export const staffVenue = (view: ParticipantView): VenueDecl | undefined =>
  findVenue(view.venues, { region: String(view.self.region), occupation: BANKING });
