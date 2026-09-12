/**
 * Who is in a currency market, and why.
 *
 * @spec Spot FX B1 Spot FX B2 Spot FX B5 Spot FX C5 Spot FX D1 Spot FX D2 Spot FX D3 Spot FX D4 Spot FX D5 Law 3 Law 6 Law 19 XI-13
 *
 * XI-13 is why this file has more than one function in it. A market whose only participant is a
 * dealer quoting off the last print is a fixed point: both sides come from one view, the view
 * follows the print, and the print follows the view. A currency market needs somebody there for a
 * reason that is not the rate — and there are two of them, and neither is a speculation:
 *
 *   B1 A PARTY THAT OWES A MONEY IT HAS NOT GOT. It is not deciding whether the rate is good; it
 *      has a coupon in another country's money falling due. It posts a size and no level (Clearing
 *      C3), because it has no choice, and what it pays is what the other side posted.
 *   B2 A PARTY HOLDING A MONEY IT HAS NO USE FOR. The mirror: a balance in a money nothing it owes
 *      is denominated in, which it will part with at a level it is content with.
 *
 * And then B5, the dealer, which quotes both ways and is the reason the first two find anybody.
 */
import { pairOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { CurrencyCode } from '../../core/ids.js';
import {
  add,
  atMost,
  div,
  mul,
  sub,
} from '../../core/num.js';
import { asQty, downTick, type Qty } from '../../core/tick.js';
import type { ParticipantView } from '../../world/context.js';
import type { FxDeskDecl } from './data.js';
import { fxParam } from './data.js';

/**
 * B1, B2: what a party brings to a pair because of what it owes and what it holds.
 *
 * ITS OWN MONEY IS ONE SIDE OF IT OR IT HAS NO BUSINESS HERE. A party that is short of yen wants
 * yen for its own money; a party sitting on yen it has no use for wants its own money back for
 * them. Neither wants to be somewhere else afterwards, so the pair it deals in is the one between
 * that money and its own — and a cross between two moneys that are both foreign to it is a trade
 * it has no reason to make (XI-12: nothing is ROUTED through a third money; the cross is its own
 * market for the parties whose money is in it). It is also what stops one balance being offered in
 * three pairs at once, which would be a party committing the same money three times over.
 *
 * A SIZE AND NO LEVEL (Clearing C3). It is not shopping: it has a coupon in another country's
 * money falling due, or a balance in one that nothing it owes is denominated in, and what it pays
 * or takes is what the desks on the other side posted. That is a cleared price — somebody quoted it
 * and somebody took it (Law 3) — and it is the only reason a pair has two sides in a world where
 * nobody yet has a view of a rate (XI-13).
 *
 * Law 19: both come from one read of its own state. `owedIn` is what its own liabilities say falls
 * due in that money less what it holds of it: positive it must buy, negative it has too much.
 */
export function needOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const pair = pairOf(m);
  if (pair === undefined || !view.self.status.alive) return [];
  const home = view.registry.region(view.self.region).ccy;
  if (pair.base === home) return ownMoneyIsTheBase(view, m, pair.quote);
  if (pair.quote === home) return ownMoneyIsTheQuote(view, pair.base);
  return [];
}

/**
 * Its own money is what the pair is priced IN, so the foreign money is the base and the sizes are
 * already in the units this market trades. Short of it, it buys; holding too much, it sells.
 */
function ownMoneyIsTheQuote(view: ParticipantView, foreign: CurrencyCode): readonly Order[] {
  const position = view.owedIn(foreign);
  if (position > 0) {
    const want = downTick(position);
    return want > 0 ? [{ party: view.self.id, side: 'buy', price: 'market', qty: want }] : [];
  }
  // It can only sell what it actually holds, which is arithmetic and not a limit (Law 6).
  const spare = least(-position, view.cash(foreign));
  return spare > 0 ? [{ party: view.self.id, side: 'sell', price: 'market', qty: spare }] : [];
}

/**
 * Its own money is the BASE, so what it is short of or sitting on is the quote, and the size has to
 * be carried across at the rate in force to be a size in this book at all.
 */
function ownMoneyIsTheBase(
  view: ParticipantView,
  m: MarketDecl,
  foreign: CurrencyCode,
): readonly Order[] {
  const pair = pairOf(m);
  if (pair === undefined) return [];
  const print = view.print(m.instrument);
  if (!print.some || print.value.price <= 0) return [];
  const rate = print.value.price;
  const position = view.owedIn(foreign);
  if (position > 0) {
    // Short of the quote: it sells its own money to raise it, and never more than it holds.
    const want = least(div(position, rate, 'base it must sell'), view.cash(pair.base));
    return want > 0 ? [{ party: view.self.id, side: 'sell', price: 'market', qty: want }] : [];
  }
  // Sitting on the quote: it buys its own money back with it, and never more than that balance buys.
  const spare = least(div(-position, rate, 'base its spare quote buys'), div(view.cash(foreign), rate, 'what it holds, in base'));
  return spare > 0 ? [{ party: view.self.id, side: 'buy', price: 'market', qty: spare }] : [];
}

/**
 * B5, D1–D5: the desk's quote, and every number in it is the desk's own.
 *
 * It is the same shape as its paper book (Dealer Desks C) and for the same reasons: what it will
 * pay is the rate less what standing in the middle costs it, what it wants is the rate plus the
 * same, and the SPREAD is twice that rather than a width anybody stated (C5.a). Its own inventory
 * skews BOTH sides — long the base it bids lower and offers lower, because it wants to be shorter —
 * which is how order flow moves a rate with nobody deciding that it should (D4).
 *
 * D4, Currency D2: WHAT ITS POSITION IN A PAIR IS. A desk's balance in its OWN money is not a
 * position: it is the funding of everything it does, and a bank that counted its whole reserve
 * account as a long dollar book would be a desk permanently too long to bid for dollars — which is
 * a market with one side (XI-13). Its position is what it holds of the moneys that are NOT its own,
 * expressed in the base: long the base, short the quote, or a cross of two foreign balances, which
 * is a real position however little either leg is home.
 *
 * D1, D2: and it is bounded by what it will risk and by what it HOLDS. A desk cannot sell a money
 * it has not got and cannot pay with one it has not got: that is arithmetic, not a limit (Law 6),
 * and it is why a currency with no seller left simply has no offer rather than an infinitely
 * elastic one.
 */
export function dealerOrders(
  view: ParticipantView,
  m: MarketDecl,
  d: FxDeskDecl | undefined,
): readonly Order[] {
  const pair = pairOf(m);
  if (pair === undefined || d === undefined || !view.self.status.alive) return [];
  const print = view.print(m.instrument);
  if (!print.some || print.value.price <= 0) return [];
  const rate = print.value.price;
  const edge = mul(rate, view.params.ratio(fxParam(d.bank, 'edge')), 'what standing in the middle costs it');
  // D1: what it will have behind a position in this pair — its own share of its own capital, which
  // is a read of its own account and is in its OWN money, carried across to the base at the rate
  // in force so the room is a size in the units this book trades in.
  const home = view.registry.region(view.self.region).ccy;
  const risk = mul(view.equity(), view.params.ratio(fxParam(d.bank, 'inventoryLimit')), 'what it will risk');
  const room = downTick(mul(risk, view.rateIn(home, pair.base), 'in the base'));
  if (room <= 0) return [];
  // D4: how far its book is from flat, as a share of the room it has — signed, because a desk can
  // be either way round. Flat is where a desk wants to be: it is paid for turning the position
  // over, not for holding it.
  const held = sub(
    positionIn(view, pair.base, home),
    div(positionIn(view, pair.quote, home), rate, 'the quote it is carrying, in base'),
    'its book in this pair, in the base',
  );
  const skew = mul(div(held, room, 'how much of its room its book uses'), edge, 'what its own position does to both sides');
  const bid = sub(sub(rate, edge, 'what it will pay'), skew, 'less what it is carrying');
  const offer = sub(add(rate, edge, 'what it wants'), skew, 'less what it wants to shed');
  const out: Order[] = [];
  // D2, XI-2, Clearing C3: PAST ITS OWN LIMIT IT IS NOT QUOTING, IT IS GETTING OUT. A desk that
  // decided how much it would risk and is carrying more than that does not sit on the excess at a
  // price it likes — it goes to the other desks with a size and no level, and what it gets is what
  // they posted. That is the same rung its paper book has (`urgentSale`), and it is what moves a
  // rate when everybody is on the same side: a limit that nobody crosses is a desk waiting, and a
  // market full of desks waiting is a market with one side (XI-13).
  const over = sub(absolute(held), room, 'how far past its own limit it is');
  if (over > 0) {
    const side = held > 0 ? 'sell' : 'buy';
    const size = side === 'sell'
      ? least(over, view.cash(pair.base))
      : least(over, div(view.cash(pair.quote), rate, 'what its quote balance buys'));
    if (size > 0) out.push({ party: view.self.id, side, price: 'market', qty: size });
    return out;
  }
  // What it can BUY is the room it has left on the long side, and never more than the quote money
  // it has to pay with; what it can SELL is the room it has left on the short side, and never more
  // than the base it holds. Neither pair is a cap: one side is a decision it made about its own
  // capital and the other is the arithmetic of delivery.
  const buy = least(
    sub(room, held, 'room left long the base'),
    div(view.cash(pair.quote), rate, 'what its quote balance buys'),
  );
  const sell = least(add(room, held, 'room left short the base'), view.cash(pair.base));
  if (bid > 0 && buy > 0) out.push({ party: view.self.id, side: 'buy', price: bid, qty: buy });
  if (offer > 0 && sell > 0) out.push({ party: view.self.id, side: 'sell', price: offer, qty: sell });
  return out;
}

/** How big a position is, whichever way round it is. Not a bound: a size has no sign (Law 6). */
function absolute(x: number): number {
  return x < 0 ? sub(0, x, 'the size of it') : x;
}

/**
 * D4, Currency D2: what a party is CARRYING in a money — what it holds of one that is not its own.
 * Its balance in its own money funds everything it does and is not a view of anything; a balance in
 * somebody else's is a position it took and has to close.
 */
function positionIn(view: ParticipantView, ccy: CurrencyCode, home: CurrencyCode): number {
  return ccy === home ? 0 : view.cash(ccy);
}

/** Law 8, Law 6: the smaller of two sizes, on the grid. Arithmetic of delivery, not a limit. */
function least(a: number, b: number): Qty {
  return downTick(atMost(a, b, 'the smaller of the two is how far both reach'));
}


/**
 * C2.a, E3, Clearing F1: the arbitrage legs this desk decided on this period, read back from what
 * it published. One decision, three books, and the book that is asking gets the leg that is its own.
 *
 * A size and no level: the desk is taking the round trip at whatever the three books give it, which
 * is what makes the gap close TO its cost and not past it — a level would be the desk deciding
 * where the rate should be, and nobody decides that (Law 3).
 */
export function arbitrageOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const said = view.lastOwn('fx.arbitrage');
  if (!said.some || said.value.period !== view.period) return [];
  const legs = said.value.data['legs'];
  if (!Array.isArray(legs)) return [];
  const out: Order[] = [];
  for (const row of legs as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const leg = row as { market?: unknown; side?: unknown; qty?: unknown };
    if (leg.market !== m.id) continue;
    if (leg.side !== 'buy' && leg.side !== 'sell') continue;
    if (typeof leg.qty !== 'number' || leg.qty <= 0) continue;
    out.push({
      party: view.self.id,
      side: leg.side,
      price: 'market',
      qty: asQty(leg.qty, `${view.self.id}'s arbitrage leg`),
    });
  }
  return out;
}
