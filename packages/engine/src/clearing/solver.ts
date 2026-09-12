/**
 * The clearing engine: one solver for every market.
 *
 * @spec Central Bank C3 Clearing A2 Clearing A2.a Clearing A4 Clearing B5 Clearing C1 Clearing C2 Clearing C3 Clearing C4 Clearing C4.a Clearing C4.b Clearing C4.c Clearing C5 Clearing D1 Clearing D2 Clearing D5 Clearing E3 Sovereign C2
 *
 * Participants post schedules: limit orders, a size at a level, which together form a step function
 * from price to quantity (A2). The solver finds the uniform price at which posted supply meets posted
 * demand (C1): among the posted prices, the one that executes the most volume, then the one with the
 * least imbalance, then the lowest. It never returns a price nobody posted (C4.c). Rationing at the
 * marginal price is pro rata (C3). It adds nothing to either side (B5) and is a pure function of the
 * orders (C5).
 */
import { impossible } from '../core/assert.js';
import type { PartyId } from '../core/ids.js';
import {
  atMost,
  finite,
  sum,
  zeroIfNone,
} from '../core/num.js';
import { asQty, splitOnTick, type Qty } from '../core/tick.js';

export type Side = 'buy' | 'sell';

/**
 * A level, or 'market': a participant that posts a quantity and no level (Central Bank C3). A market
 * order is resolved before price formation to the worst level the other side actually posted — the
 * most a taker can be asked to pay, or the least it can be offered — so it is a price somebody
 * posted (C4.c) and never a bracket. With nothing posted on the other side there is no level to
 * take and the order is not in the book.
 */
export type OrderPrice = number | 'market';

export interface Order {
  readonly party: PartyId;
  readonly side: Side;
  /** A buy: the most it will pay per unit. A sell: the least it will accept. */
  readonly price: OrderPrice;
  /**
   * Law 8: TOTAL PIECES — a cell's per-member count times its weight. It is a `Qty` and not a
   * number, so a schedule built by dividing money by a price cannot reach a book without its author
   * saying which way it rounds (core/tick.ts). That decision is the author's: what a party CAN do
   * rounds down, what it MUST do rounds up, and the kernel never picks for it.
   */
  readonly qty: Qty;
}

/** An order with its level fixed: what the solver works on. */
export interface LimitOrder extends Order {
  readonly price: number;
}

export interface Fill {
  readonly party: PartyId;
  readonly side: Side;
  readonly qty: Qty;
  /** The level this order posted, which is what a tail is measured against (Sovereign C4). */
  readonly at: number;
}

export type Outcome =
  | {
      readonly kind: 'cleared';
      readonly price: number;
      readonly volume: number;
      readonly fills: readonly Fill[];
      /** The side that posted more at the clearing price and was rationed (C3), if any. */
      readonly rationed: Side | 'none';
      readonly demandAtPrice: number;
      readonly supplyAtPrice: number;
    }
  | { readonly kind: 'noDemand' }
  | { readonly kind: 'noSupply' }
  | { readonly kind: 'noOverlap'; readonly bestBid: number; readonly bestAsk: number };

export type Rationing = 'proRata';

/** A cleared session, for readers that must tell the outcomes apart (Law 15's dispatch). */
export type Cleared = Extract<Outcome, { kind: 'cleared' }>;
export const isCleared = (o: Outcome): o is Cleared => o.kind === 'cleared';

/**
 * Which level a tie is struck at when several execute the same volume with the same imbalance: a
 * stated rule of the venue, like the rationing rule (C3), and never a bound of a search (C4.c).
 *
 * - `sellersCompete`: the lowest such level. An open book where supply exceeds demand across the
 *   range is a crowd of sellers undercutting each other, and the price falls to where one of them
 *   is content.
 * - `marginalBid`: the highest such level, which is the lowest bid still allotted — the STOP-OUT a
 *   uniform-price sealed-bid auction allots at (Sovereign C2). An issuer selling below a level a
 *   bidder actually posted would be handing away paper nobody asked it to discount.
 */
export type PriceRule = 'sellersCompete' | 'marginalBid';

/**
 * Resolve every 'market' order against the levels the other side posted (C3, C4.c). Orders that
 * find no level are dropped: nobody posted a price for them to take.
 *
 * **THE RULE IS STATED DELIBERATELY, AND A2.a ALREADY SAID WHAT IT IS.** "A market expressed as
 * 'here is the quantity I want' has no level, only a shape, and forces every venue to invent its
 * own rule." This is that invented rule, in one place rather than one per venue, and naming it is
 * the point: a market order is not a schedule (A2), and the participant that posts one has answered
 * "how much" without answering *"and if it were cheaper?"* — which is the only question the
 * mechanism asks.
 *
 * Given that it exists, it resolves to the WORST level the other side actually posted: a market buy
 * to the highest ask, a market sell to the lowest bid. That is the least-bad invention available,
 * because the level it takes is one somebody really asked for — the price still comes out of posted
 * supply meeting posted demand (Law 3), and nothing here invents a level to clear at. Resolving to
 * the BEST level instead would make an order with no limit into a price-sensitive one, which is a
 * different order from the one that was posted.
 *
 * What it costs is A4: an order with no level is a price-taker of a price this mechanism has not
 * yet produced, and a large enough one is the marginal order that sets it. Seventeen sites post one
 * today and the consequential one is `central-bank-omo` (`13b.1-10`). The fix is at the POSTER, not
 * here: a participant with a reason to be in the room has a level at which its reason stops.
 */
export function resolveMarketOrders(orders: readonly Order[]): {
  readonly resolved: readonly LimitOrder[];
  readonly unpriced: readonly Order[];
} {
  const limits = orders.filter((o): o is LimitOrder => o.price !== 'market');
  let highestAsk: number | undefined;
  let lowestBid: number | undefined;
  for (const o of limits) {
    if (o.side === 'sell' && (highestAsk === undefined || o.price > highestAsk)) highestAsk = o.price;
    if (o.side === 'buy' && (lowestBid === undefined || o.price < lowestBid)) lowestBid = o.price;
  }
  const resolved: LimitOrder[] = [];
  const unpriced: Order[] = [];
  for (const o of orders) {
    const posted = o.price;
    if (posted !== 'market') {
      resolved.push({ ...o, price: posted });
      continue;
    }
    const level = o.side === 'buy' ? highestAsk : lowestBid;
    if (level === undefined) unpriced.push(o);
    else resolved.push({ ...o, price: level });
  }
  return { resolved, unpriced };
}

function validate(orders: readonly LimitOrder[]): void {
  for (const o of orders) {
    impossible(
      finite(o.price, 'order price') >= 0,
      'Law 6',
      `a price cannot be negative: ${o.price}`,
    );
    impossible(
      finite(o.qty, 'order qty') > 0,
      'Clearing A2',
      `an order has a positive size, got ${o.qty}`,
    );
    // Law 8 is not checked here, and that is the point: `Order.qty` is a `Qty`, so a size that is
    // not a whole number of the unit's pieces cannot be built at all (core/tick.ts). A check in
    // this solver would be the same rule written a second time, and the one that fires last.
  }
}

/** The executable volume at a price is bounded by whichever side posted less: arithmetic, not a decision. */
function executable(demand: Qty, supply: Qty): Qty {
  return atMost(demand, supply, 'what is not offered cannot be bought');
}

export function clear(
  posted: readonly Order[],
  rationing: Rationing,
  priceRule: PriceRule = 'sellersCompete',
): Outcome {
  const { resolved: orders } = resolveMarketOrders(posted);
  validate(orders);
  const ration = RATIONERS[rationing];
  const buys = orders.filter((o) => o.side === 'buy');
  const sells = orders.filter((o) => o.side === 'sell');
  if (buys.length === 0) return { kind: 'noDemand' };
  if (sells.length === 0) return { kind: 'noSupply' };

  const candidates = [...new Set(orders.map((o) => o.price))].sort((a, b) => a - b);
  let best: { price: number; volume: Qty; imbalance: number; d: Qty; s: Qty } | undefined;
  for (const p of candidates) {
    // Law 8: adding counts of pieces gives a count of pieces — no rounding is involved, so the
    // sum is still a quantity and says so.
    const d = asQty(sum(buys.filter((o) => o.price >= p).map((o) => o.qty)).value, 'demand at a level');
    const s = asQty(sum(sells.filter((o) => o.price <= p).map((o) => o.qty)).value, 'supply at a level');
    const v = executable(d, s);
    const imbalance = Math.abs(d - s);
    const tie = v === best?.volume && imbalance === best.imbalance;
    if (
      best === undefined ||
      v > best.volume ||
      (v === best.volume && imbalance < best.imbalance) ||
      (tie && PRICE_RULES[priceRule](p, best.price))
    ) {
      best = { price: p, volume: v, imbalance, d, s };
    }
  }
  if (best === undefined || best.volume === 0) {
    let bestBid = zeroIfNone(buys[0]?.price);
    for (const o of buys) if (o.price > bestBid) bestBid = o.price;
    let bestAsk = zeroIfNone(sells[0]?.price);
    for (const o of sells) if (o.price < bestAsk) bestAsk = o.price;
    return { kind: 'noOverlap', bestBid, bestAsk };
  }

  const fills: Fill[] = [
    ...ration(buys, best.price, best.volume, 'buy'),
    ...ration(sells, best.price, best.volume, 'sell'),
  ];
  const rationed: Side | 'none' = best.d > best.s ? 'buy' : best.s > best.d ? 'sell' : 'none';
  return {
    kind: 'cleared',
    price: best.price,
    volume: best.volume,
    fills,
    rationed,
    demandAtPrice: best.d,
    supplyAtPrice: best.s,
  };
}

type Rationer = (
  orders: readonly LimitOrder[],
  price: number,
  volume: number,
  side: Side,
) => Fill[];

/**
 * Fill one side in price priority (best prices first), pro rata within the marginal price level (C3).
 * Deterministic: ties broken by party id, then by posting order.
 */
function proRata(
  orders: readonly LimitOrder[],
  price: number,
  volume: number,
  side: Side,
): Fill[] {
  const eligible: { o: LimitOrder; i: number }[] = orders
    .map((o, i) => ({ o, i }))
    .filter(({ o }) => (side === 'buy' ? o.price >= price : o.price <= price))
    .sort((a, b) => {
      const byPrice = side === 'buy' ? b.o.price - a.o.price : a.o.price - b.o.price;
      if (byPrice !== 0) return byPrice;
      if (a.o.party < b.o.party) return -1;
      if (a.o.party > b.o.party) return 1;
      return a.i - b.i;
    });
  const fills: Fill[] = [];
  let remaining = volume;
  let idx = 0;
  while (idx < eligible.length && remaining > 0) {
    const level = eligible[idx]?.o.price;
    if (level === undefined) break;
    const group: { o: LimitOrder; i: number }[] = [];
    while (idx < eligible.length && eligible[idx]?.o.price === level) {
      const e = eligible[idx];
      if (e !== undefined) group.push(e);
      idx += 1;
    }
    const groupQty = sum(group.map((g) => g.o.qty)).value;
    if (groupQty <= remaining) {
      for (const g of group) fills.push({ party: g.o.party, side, qty: g.o.qty, at: g.o.price });
      remaining = finite(remaining - groupQty, 'remaining');
    } else {
      // C3, Law 8: the marginal level, pro rata — and pro rata over a quantity that has a smallest
      // piece is the largest-remainder split, not a multiplication by a share. Ten shares between
      // three bidders is four, three and three: the parts sum to exactly what there was, and WHO
      // gets the odd piece is the stated rule (`splitOnTick`, ties to the earlier claimant) rather
      // than a rounding that happens later somewhere else. Multiplying by `remaining / groupQty`
      // gave every one of them a fraction of a piece and left the residual with no holder (Law 2).
      const each = splitOnTick(remaining, group.map((g) => g.o.qty));
      group.forEach((g, at) => {
        const mine = each[at];
        if (mine === undefined || mine <= 0) return;
        fills.push({ party: g.o.party, side, qty: mine, at: g.o.price });
      });
      remaining = 0;
    }
  }
  return fills;
}

/** The stated rationing rules (C3): one profile per rule, never a branch (Law 15). */
const RATIONERS: Readonly<Record<Rationing, Rationer>> = Object.freeze({ proRata });

/** The stated tie rules: does this candidate replace the one held, at equal volume and imbalance? */
const PRICE_RULES: Readonly<Record<PriceRule, (candidate: number, held: number) => boolean>> =
  Object.freeze({
    sellersCompete: (candidate, held) => candidate < held,
    marginalBid: (candidate, held) => candidate > held,
  });
