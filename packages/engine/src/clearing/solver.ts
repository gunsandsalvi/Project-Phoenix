/**
 * The clearing engine: one solver for every market.
 *
 * @spec Clearing A2 Clearing A2.a Clearing A4 Clearing B5 Clearing C1 Clearing C2 Clearing C3 Clearing C4 Clearing C4.a Clearing C4.b Clearing C4.c Clearing C5 Clearing D1 Clearing D2 Clearing D5 Clearing E3 Sovereign C2
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
import { finite, sum, zeroIfNone } from '../core/num.js';

export type Side = 'buy' | 'sell';

export interface Order {
  readonly party: PartyId;
  readonly side: Side;
  /** A buy: the most it will pay per unit. A sell: the least it will accept. */
  readonly price: number;
  /** Total units (a cell's per-member size times its weight). */
  readonly qty: number;
}

export interface Fill {
  readonly party: PartyId;
  readonly side: Side;
  readonly qty: number;
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

function validate(orders: readonly Order[]): void {
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
  }
}

/** The executable volume at a price is bounded by whichever side posted less: arithmetic, not a decision. */
function executable(demand: number, supply: number): number {
  return demand < supply ? demand : supply;
}

export function clear(orders: readonly Order[], rationing: Rationing): Outcome {
  validate(orders);
  const ration = RATIONERS[rationing];
  const buys = orders.filter((o) => o.side === 'buy');
  const sells = orders.filter((o) => o.side === 'sell');
  if (buys.length === 0) return { kind: 'noDemand' };
  if (sells.length === 0) return { kind: 'noSupply' };

  const candidates = [...new Set(orders.map((o) => o.price))].sort((a, b) => a - b);
  let best: { price: number; volume: number; imbalance: number; d: number; s: number } | undefined;
  for (const p of candidates) {
    const d = sum(buys.filter((o) => o.price >= p).map((o) => o.qty)).value;
    const s = sum(sells.filter((o) => o.price <= p).map((o) => o.qty)).value;
    const v = executable(d, s);
    const imbalance = Math.abs(d - s);
    if (
      best === undefined ||
      v > best.volume ||
      (v === best.volume && imbalance < best.imbalance)
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

type Rationer = (orders: readonly Order[], price: number, volume: number, side: Side) => Fill[];

/**
 * Fill one side in price priority (best prices first), pro rata within the marginal price level (C3).
 * Deterministic: ties broken by party id, then by posting order.
 */
function proRata(orders: readonly Order[], price: number, volume: number, side: Side): Fill[] {
  const eligible = orders
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
    const group: { o: Order; i: number }[] = [];
    while (idx < eligible.length && eligible[idx]?.o.price === level) {
      const e = eligible[idx];
      if (e !== undefined) group.push(e);
      idx += 1;
    }
    const groupQty = sum(group.map((g) => g.o.qty)).value;
    if (groupQty <= remaining) {
      for (const g of group) fills.push({ party: g.o.party, side, qty: g.o.qty });
      remaining = finite(remaining - groupQty, 'remaining');
    } else {
      // The marginal level: pro rata.
      const share = remaining / groupQty;
      for (const g of group)
        fills.push({ party: g.o.party, side, qty: finite(g.o.qty * share, 'pro rata fill') });
      remaining = 0;
    }
  }
  return fills;
}

/** The stated rationing rules (C3): one profile per rule, never a branch (Law 15). */
const RATIONERS: Readonly<Record<Rationing, Rationer>> = Object.freeze({ proRata });
