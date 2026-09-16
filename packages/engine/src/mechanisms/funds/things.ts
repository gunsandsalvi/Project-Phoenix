/**
 * WHAT A MANDATE OVER THINGS DOES: it buys what nobody promised, because it expects to be paid
 * more for it later than keeping it costs (Commodities Spot C3).
 *
 * Item 10e: named for the TERM. What puts a pool here is its blueprint admitting only `thing` —
 * the class the classification gives anything with no issuer behind it — and never a flag saying
 * what sort of fund it is.
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
import { amountOf, asPerPiece, asRatio, heldAsMoney, minus, scale } from '../../core/measure.js';
import { downTick } from '../../core/tick.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { isGoodTerms, spacePerPiece, storageRateIn } from '../../registry/physical.js';
import type { ParticipantView } from '../../world/context.js';
import type { Mandate } from './index.js';
import { admits, type Blueprint } from '../../registry/blueprint.js';
import { about } from '../../world/context.js';

/**
 * A4, Law 15, item 10e: A MANDATE OF PHYSICAL THINGS AND NOTHING ELSE — structural, never a flag.
 *
 * It used to walk the mandate's list of kind ids and ask the registry whether every one of them was
 * `physical`. The blueprint says it in one band: `classes: ['thing']` is the class the
 * classification gives to anything NOBODY PROMISED (`liabilityOfIssuer` false), which is what makes
 * a commodity fund a commodity fund. One read, and no list to walk.
 */
export function holdsThings(b: Blueprint): boolean {
  // A blueprint that states NO class band holds anything, which is not a mandate over things —
  // and `[].every(...)` is `true`, so the empty case has to be said rather than fallen into.
  return b.classes.length > 0 && b.classes.every((c) => c === 'thing');
}

/**
 * C3, B4: what it will pay, and it is ITS OWN NUMBER. What it expects a piece to fetch, less what
 * the wait costs — the room at the rate the room let for, and the part of the stock that does not
 * survive the wait. A fund with no outlook of its own has nothing to say here and says nothing,
 * which is the honest answer for a party that has never seen this book (Expectations A2.a).
 */
export function thingOrders(
  view: ParticipantView,
  mandate: Mandate,
  m: MarketDecl,
): readonly Order[] {
  const i = view.instruments.get(m.instrument);
  if (!i.status.live || !isGoodTerms(i.terms)) return [];
  // A4: and this particular good is inside its mandate, asked of the one `admits` every vehicle
  // in the world asks — so a commodity fund and a credit fund refuse things the same way.
  if (!admits(mandate.blueprint, view.classify(m.instrument), () => undefined)) return [];
  const expected = view.outlook(about({ on: 'price', instrument: m.instrument }));
  if (!expected.some) return [];
  const spoilage = view.params.ratio(i.terms.spoilage);
  const expects = asPerPiece(expected.value.expected, 'what it expects a piece to fetch');
  const survives = minus(
    expects,
    scale(expects, spoilage, 'what perishes'),
    'what a piece is worth to it at the end of the wait',
  );
  const rate = storageRateIn(view, i.terms.region);
  const carry =
    i.terms.storagePerUnit === null || rate === undefined
      ? asPerPiece(0, 'a thing nobody stores costs nothing to keep')
      : scale(
          rate,
          asRatio(
            spacePerPiece(view, i.unit, view.params.ratio(i.terms.storagePerUnit)),
            'the room a piece takes',
          ),
          'the room for a period',
        );
  const most = minus(survives, carry, 'the most it will pay and still be better off waiting');
  if (most <= 0) return [];
  // Law 6: it bids for what its OWN CASH reaches at that level and no more. Not a limit anybody set
  // — it is a quantity it has, in the way that selling units nobody holds is not a thing to do.
  const ccy = view.registry.currencyOf(view.self.region);
  const qty = downTick(
    amountOf(
      heldAsMoney(view.cash(ccy), ccy, 'the money it holds'),
      most,
      'pieces its own cash reaches',
    ),
  );
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: most, qty }];
}
