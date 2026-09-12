/**
 * A FUND THAT HOLDS THE THING ITSELF (Commodities Spot C3).
 *
 * @spec Commodities Spot C3 Commodities Spot B4 Commodities Spot D3 Clearing A2 Clearing A3 Expectations A2 Law 3 Law 6
 *
 * The third reason a fund is in a book, and a fund has exactly one of them: a money fund puts spare
 * cash to work at a yield it requires, a tracker holds an index whatever it costs, and this one
 * holds A PHYSICAL THING because it expects to be paid more for it later than it costs to keep it
 * until then. Nothing tells it what the price will do; what it has is its own outlook, formed from
 * what it has itself seen (Expectations A2), and what it is up against is the carry — which since
 * 13c is a real price somebody charges for the room, not a number anybody wrote down.
 *
 * IT IS THE PARTY THE STORAGE MARKET WAS MISSING. Every producer in this world opens with room for
 * exactly what it holds, so nobody is short and nobody has spare and the venue has nothing to
 * clear. An investor wanting to hold a thing it did not make is short of room by construction, and
 * a market with a party on each side of it is what makes the carry an outcome rather than a rate.
 *
 * WHAT MAKES IT A COMMODITY FUND IS ITS MANDATE, not a flag: every kind it may hold is PHYSICAL
 * (Goods A1 — nobody issued it and nobody owes it). A mandate of paper is a money fund's and this
 * never speaks for one (Law 4, Law 15).
 */
import { div, mul, sub } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { isGoodTerms, spacePerPiece, storageRateIn } from '../../registry/physical.js';
import type { ParticipantView } from '../../world/context.js';
import type { FundDecl } from './data.js';

/** A4, Law 15: a mandate of physical things and nothing else. Structural, never a flag on the row. */
export function holdsPhysical(view: ParticipantView, d: FundDecl): boolean {
  if (d.eligible.length === 0) return false;
  return d.eligible.every((k) => {
    return view.registry.instrumentKinds.get(k as never)?.physical === true;
  });
}

/**
 * C3, B4: what it will pay, and it is ITS OWN NUMBER. What it expects a piece to fetch, less what
 * the wait costs — the room at the rate the room let for, and the part of the stock that does not
 * survive the wait. A fund with no outlook of its own has nothing to say here and says nothing,
 * which is the honest answer for a party that has never seen this book (Expectations A2.a).
 */
export function physicalOrders(
  decls: readonly FundDecl[],
  view: ParticipantView,
  m: MarketDecl,
): readonly Order[] {
  const d = decls.find((row) => row.fund === String(view.self.id));
  if (d === undefined || !holdsPhysical(view, d)) return [];
  const i = view.instruments.get(m.instrument);
  if (!i.status.live || !isGoodTerms(i.terms)) return [];
  if (!d.eligible.includes(String(i.kind))) return [];
  const expected = view.outlook(`price.${m.instrument}`);
  if (!expected.some) return [];
  const spoilage = view.params.ratio(i.terms.spoilage);
  const survives = sub(expected.value.expected, mul(expected.value.expected, spoilage, 'what perishes'), 'what a piece is worth to it at the end of the wait');
  const rate = storageRateIn(view, i.terms.region);
  const carry =
    i.terms.storagePerUnit === null || rate === undefined
      ? 0
      : mul(rate, spacePerPiece(view, i.unit, view.params.ratio(i.terms.storagePerUnit)), 'the room for a period');
  const most = sub(survives, carry, 'the most it will pay and still be better off waiting');
  if (most <= 0) return [];
  // Law 6: it bids for what its OWN CASH reaches at that level and no more. Not a limit anybody set
  // — it is a quantity it has, in the way that selling units nobody holds is not a thing to do.
  const ccy = view.registry.region(view.self.region).ccy;
  const qty = downTick(div(view.cash(ccy), most, 'pieces its own cash reaches'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: most, qty }];
}
