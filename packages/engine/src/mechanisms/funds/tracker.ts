/**
 * A fund that tracks an index, and the trade a rebalance forces on it.
 *
 * @spec Fund Shares A4 Fund Shares D1 Indices C1 Indices C2 Indices C2.a Equity C2.c Clearing C3 XI-2 Law 3 Law 8 Law 19
 *
 * C2, Equity C2.c: A TRACKER DOES NOT PRICE. Its mandate is the index's rule, so what it should
 * hold is whatever the index says is in it at whatever the index weighs it, scaled to what the fund
 * is worth. It posts a size and no level (Clearing C3) because it is not deciding anything: it is
 * the one participant in an equity market that will pay whatever the book asks and sell at whatever
 * the book bids, and that is exactly what makes it a transmission channel rather than an investor.
 *
 * C2.a: A REBALANCE IS THE INDEX ANSWERING DIFFERENTLY — a line listed, a line gone, a weight moved
 * — and the trade it forces happens in the SAME SESSION, against whoever is on the other side. A
 * line that has left the index is sold outright, however good it is, because holding it is no
 * longer the mandate. Nothing here is enforced or corrected afterwards: what the fund ends up
 * holding is what the sessions gave it, and the gap between that and the index is its tracking
 * error, which is a read (E4) and not a defect.
 *
 * Law 19: the target is read from the index, the holding from the register, and the price from the
 * print. Nothing is remembered between periods, so a fund that was rationed last session simply
 * comes back with the same difference to close.
 */
import { delivers, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { InstrumentId } from '../../core/ids.js';
import { atLeast, atMost, div, material, mul, sub, sum } from '../../core/num.js';
import { downTick, type Qty } from '../../core/tick.js';
import type { ParticipantView } from '../../world/context.js';
import type { EtfDecl } from './data.js';

export function trackerOrders(
  etfs: readonly EtfDecl[],
  view: ParticipantView,
  m: MarketDecl,
): readonly Order[] {
  const d = etfs.find((e) => e.fund === String(view.self.id));
  if (d === undefined || !view.self.status.alive) return [];
  const subject = delivers(m);
  if (!subject.some) return [];
  const line = subject.value;
  const read = view.index(d.tracks);
  // D5.a: no index yet is no mandate yet. A fund that traded against an index with no level would
  // be trading against nothing.
  if (!read.some) return [];
  const held = view.free(line);
  // C1: MEMBERSHIP IS THE RULE'S, not this period's prints. A line that did not trade this week has
  // not left the index, and selling it because nobody happened to trade it would be a rebalance
  // nobody declared — every quiet line out and back in again, every week.
  const inIndex = read.value.basket.find((c) => c.instrument === line);
  if (inIndex === undefined) {
    // C2.a, XI-2: OUT OF THE INDEX, OUT OF THE FUND, at whatever the book gives. This is the one
    // order here with no level on it, and it is a forced SELLER — which is a real thing (XI-2). A
    // forced BUYER is not (Appendix B), which is why the other side of every rebalance below
    // carries a price.
    return held > 0 ? [{ party: view.self.id, side: 'sell', price: 'market', qty: held }] : [];
  }
  const basket = priced(view, read.value.basket);
  const own = basket.find((c) => c.instrument === line);
  if (own === undefined) return [];
  const target = targetUnits(view, basket, own.weight);
  if (target === undefined) return [];
  const difference = sub(target, held, 'what it is away from its own mandate');
  // Law 7: a difference below what one session could have made is not a rebalance, it is the
  // rounding of the last one. `material` is the same test every other order in this world takes.
  if (!material(Math.abs(difference), 2, atLeast(target, held, 'the larger of the two is what the difference is measured against'))) return [];
  // C2, Appendix B: AT WHAT THE LINE IS WORTH. A tracker does not decide a price — it pays what the
  // market last said and takes what the market last said — but it does not pay ANYTHING either: an
  // order with no level on the buy side is a buyer of last resort, and one of those in a thin book
  // walks the line away from every reason anybody has to own it (XI-13, and it is what the equity
  // index did the first time this was written).
  const price = own.price;
  if (price <= 0) return [];
  if (difference > 0) {
    // Law 6: and never more than its own cash buys. A fund holding a book and no money is not a
    // bidder for anything — that is arithmetic, not a limit, and a bid it could not have paid for
    // is a trade that fails to settle every session and a level nobody pays.
    const affordable = div(view.cash(view.registry.region(view.self.region).ccy), price, 'what its cash buys');
    const qty = downTick(atMost(difference, affordable, 'it buys with the money it has'));
    return qty > 0 ? [{ party: view.self.id, side: 'buy', price, qty }] : [];
  }
  const wants = downTick(-difference);
  const qty: Qty = atMost(wants, held, 'it cannot sell what it does not hold');
  return qty > 0 ? [{ party: view.self.id, side: 'sell', price, qty }] : [];
}

/**
 * A2, Law 19: the basket at what THIS FUND can see of it — the last print of each line at or before
 * now. Not the index's own `from`, which is this period's prints and therefore depends on which
 * sessions have already run: a target that moved with the running order of the markets would have
 * the fund holding a different thing depending on where in the period it was asked.
 */
function priced(
  view: ParticipantView,
  basket: readonly { readonly instrument: InstrumentId; readonly weight: number }[],
): readonly { instrument: InstrumentId; weight: number; price: number }[] {
  const out: { instrument: InstrumentId; weight: number; price: number }[] = [];
  for (const c of basket) {
    const p = view.print(c.instrument);
    if (!p.some || p.value.price <= 0) continue;
    out.push({ instrument: c.instrument, weight: c.weight, price: p.value.price });
  }
  return out;
}

/**
 * B1, C2: how many units of this line the mandate asks for — the fund's whole book spread over the
 * index's basket at the index's own weights, which is what "tracking" means. The weights are counts
 * of the line (shares in issue), so what one basket costs is the weights at the prints, and what
 * the fund can hold is what it is worth divided by that.
 */
function targetUnits(
  view: ParticipantView,
  basket: readonly { readonly instrument: InstrumentId; readonly price: number; readonly weight: number }[],
  weight: number,
): number | undefined {
  const cost = sum(basket.map((c) => mul(c.weight, c.price, 'what one basket costs'))).value;
  if (cost <= 0) return undefined;
  const worth = bookValue(view, basket);
  if (worth <= 0) return undefined;
  const baskets = div(worth, cost, 'how many of the index basket it can hold');
  return mul(baskets, weight, 'units of this line that comes to');
}

/** What the fund has to put to work: what it holds of the basket, at the same prints, plus its cash. */
function bookValue(
  view: ParticipantView,
  basket: readonly { readonly instrument: InstrumentId; readonly price: number }[],
): number {
  const terms = basket.map((c) => mul(view.quantity(c.instrument), c.price, 'what it holds of this'));
  const ccy = view.registry.region(view.self.region).ccy;
  terms.push(view.cash(ccy));
  return sum(terms).value;
}
