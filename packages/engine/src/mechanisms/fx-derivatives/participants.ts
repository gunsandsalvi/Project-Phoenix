/**
 * Who is in a forward book, and WHY.
 *
 * @spec FX Forwards B1 FX Forwards B2 FX Forwards B2.a FX Forwards B2.b FX Forwards B3 FX Forwards B3.a FX Forwards B3.b FX Forwards B4 FX Forwards D1 FX Forwards D2 FX Forwards D2.a FX Forwards D3 FX Forwards D4 FX Forwards E1 FX Forwards E2 FX Forwards E4 Derivative Layer E1 Spot FX B1 Spot FX B2 XI-12 XI-13 Observer A4 Law 3 Law 19
 *
 * Four reasons, and the arbitrage is one of them rather than the mechanism.
 *
 * D1: A KNOWN FOREIGN PAYMENT. A party that owes a money it does not hold is short of it on a date
 * (Spot FX B1), and a forward is how it fixes what that will cost. The size is its OWN position in
 * the money, read from its own obligations — never a notional somebody assigned it.
 *
 * D2, D2.a: A FOREIGN ASSET, ROLLED. A holder of something priced in another money is long that
 * money whether it wanted to be or not, and selling it forward is a recurring cost in its own P&L
 * rather than a free removal of risk.
 *
 * B2, B2.b: THE ARBITRAGE, TAKEN WITH A BALANCE SHEET. A bank will do the four-legged trade — borrow
 * one money, buy the other spot, lend it, sell it forward — while it pays for itself, and what
 * stops it is its own capital and its own funding, not a rule. So its reservation is its cost of
 * carry and its size is what its own balance sheet has room for. The forward then sits near the
 * funding differential BECAUSE banks take the trade (B1), and what is left over is the basis (B3) —
 * one basis, read from prints, and never a formula that sets the level.
 *
 * B3.b, E1: there is no parity formula anywhere in this module that a price comes out of. The only
 * arithmetic on the two rates is a RESERVATION — the most this bank will pay — and a reservation
 * is a reason somebody has, which is what every schedule in this world is made of.
 */
import { contractOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { CurrencyCode, UnitId } from '../../core/ids.js';
import { add, div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty } from '../../core/tick.js';
import type { ParticipantView } from '../../world/context.js';
import { isFxForward, isXccy } from './contract.js';

/** The overnight fixing this money's own book published, or nothing (D3.a: no fixing, no rate). */
export function overnightRate(view: ParticipantView, ccy: CurrencyCode): Option<number> {
  const e = view.lastPublicAbout('index.benchmark', `${String(ccy)}:secured`);
  if (!e.some) return none<number>();
  const rate = e.value.data['rate'];
  return typeof rate === 'number' ? some(rate) : none<number>();
}

/**
 * B2, B2.b: WHAT THIS BANK'S CARRY IS, per unit of base, for a tenor — its own cost of holding one
 * money against the other for that long. It is the bank's reservation and nothing else reads it.
 */
export function carryOf(
  view: ParticipantView,
  spot: number,
  base: CurrencyCode,
  quote: CurrencyCode,
  years: number,
): Option<number> {
  const rb = overnightRate(view, base);
  const rq = overnightRate(view, quote);
  if (!rb.some || !rq.some) return none<number>();
  const grownQuote = Math.pow(add(1, rq.value, 'the quote money grows'), years);
  const grownBase = Math.pow(add(1, rb.value, 'the base money grows'), years);
  return some(mul(spot, grownQuote / grownBase, 'what carrying one against the other comes to'));
}

/**
 * D1, D2, Spot FX B1, B2: WHAT THIS PARTY'S POSITION IN A MONEY IS. Positive is short of one it has
 * to pay; negative is holding one nothing it owes is in. Both are the same read of its own book.
 */
function positionIn(view: ParticipantView, ccy: CurrencyCode): number {
  return view.owedIn(ccy);
}

/** Observer A4: what it has already fixed forward in this pair, signed by the side it is on. */
function alreadyForward(view: ParticipantView, base: CurrencyCode, quote: CurrencyCode): number {
  let net = 0;
  for (const c of view.contracts.mine()) {
    if (!isFxForward(c.terms)) continue;
    if (c.terms.base !== base || c.terms.quote !== quote) continue;
    const iAmA = c.a === view.self.id;
    const iTakeBase = iAmA === c.terms.buysBase;
    net = add(net, iTakeBase ? c.notional : -c.notional, 'base it has already bought forward');
  }
  return net;
}

export function fxForwardOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isFxForward(decl.terms)) return [];
  const t = decl.terms;
  const spot = view.print(t.spot);
  if (!spot.some) return [];
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
  /**
   * B2, XI-13: ITS OWN NUMBER — what carrying one money against the other for this term costs IT,
   * out of its own funding, and spot when it cannot say. Both are reads of OTHER markets (the
   * overnight books, the spot session), so this party has a level whether or not this book has
   * ever printed.
   *
   * The HEDGER used to post at this book's own last price, which is XI-13's fixed point: a party
   * with a position to cover and nothing of its own to say posted where the market already was, so
   * a book whose members were all hedgers printed one number for ever and the cash-and-carry
   * relationship this class exists to express was live in period one and dead from period two.
   * `banks/dealing-quote.ts` reversed the same order for the same reason.
   */
  const carry = carryOf(view, spot.value.price, t.base, t.quote, t.tenorYears);
  const mine = carry.some ? carry.value : spot.value.price;
  const covered = alreadyForward(view, t.base, t.quote);
  // D1, D2: what it is short of the BASE money, less what it has already fixed. A party with a
  // position to cover is a HEDGER and posts one order for it — one party, one position, and the
  // kernel refuses anything else (Clearing A2).
  const short = sub(positionIn(view, t.base), covered, 'left uncovered');
  if (short !== 0) {
    const qty = view.registry.deliverable(unit, short > 0 ? short : -short);
    if (qty <= 0) return [];
    return [{ party: view.self.id, side: short > 0 ? 'buy' : 'sell', price: mine, qty: asQty(qty) }];
  }
  /**
   * B1, B2, B2.b: THE ARBITRAGE, and a party with nothing to hedge is the one that takes it. It
   * quotes BOTH WAYS around its own carry — a bid a tick below and an ask a tick above, which is
   * a spread and not a crossing — and the size is what its own balance sheet has room for. B2.b
   * is why it is not free: that room is its capital, and nothing raises it.
   *
   * It needs the carry ITSELF and not the hedger's fallback: a two-way quote around spot is a
   * quote around a number that is not what a forward is worth, and B3.b forbids a parity formula
   * setting a level rather than a party's own reservation naming one.
   */
  if (!carry.some) return [];
  const room = view.equity();
  if (room <= 0) return [];
  const size = view.registry.deliverable(unit, div(room, spot.value.price, 'what its capital carries'));
  if (size <= 0) return [];
  const bid = sub(carry.value, tick, 'a tick inside its carry');
  const ask = add(carry.value, tick, 'a tick outside its carry');
  if (bid <= 0) return [];
  return [
    { party: view.self.id, side: 'buy', price: bid, qty: asQty(size) },
    { party: view.self.id, side: 'sell', price: ask, qty: asQty(size) },
  ];
}

/**
 * C2: who wants a cross-currency swap — a borrower that raised in one money and needs another. Its
 * reservation is the basis it will pay to turn one into the other for the life of what it raised.
 */
export function xccyOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isXccy(decl.terms)) return [];
  const t = decl.terms;
  const last = view.print(t.book);
  if (!last.some) return [];
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  const needs = positionIn(view, t.base);
  if (needs <= 0) return [];
  const spot = view.print(t.spot);
  if (!spot.some) return [];
  const qty = view.registry.deliverable(unit, needs);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: last.value.price, qty: asQty(qty) }];
}

/**
 * B3, B3.a, B3.b: THE BASIS — the forward against what carrying one money into the other costs,
 * read from prints. ONE basis, derived at the read, stored nowhere, and never an input to either.
 */
export function basisOf(
  view: ParticipantView,
  spot: number,
  forward: number,
  base: CurrencyCode,
  quote: CurrencyCode,
  years: number,
): Option<number> {
  const carry = carryOf(view, spot, base, quote, years);
  return carry.some ? some(sub(forward, carry.value, 'the forward against the carry')) : none<number>();
}

/**
 * E4: WHAT IS LEFT OVER ON A HEDGED FOREIGN ASSET, per party, as a read — never zeroed.
 *
 * The asset's revaluation in the holder's own money and the forward's mark move against each other
 * and do not cancel: the hedge is a notional fixed on a date and the asset is a price that moves.
 * The difference is the basis and the imperfection, and the only honest thing to do with it is
 * show it.
 */
export interface HedgedResidual {
  /** D1, B1, B2: what it is short of the BASE money — positive short, negative holding one. */
  readonly exposure: number;
  /** What it has already fixed forward in this pair, signed by the side it is on. */
  readonly covered: number;
  /** E4: the difference, and it is shown rather than netted away. */
  readonly residual: number;
}

export function hedgedResidual(
  view: ParticipantView,
  base: CurrencyCode,
  quote: CurrencyCode,
): HedgedResidual {
  const exposure = positionIn(view, base);
  const covered = alreadyForward(view, base, quote);
  // E4: its PARTS travel with it. A single number would say how big the gap is and not what it is
  // made of, and what a reader has to be able to see is that the position and the hedge are two
  // different objects — a price that moves and a notional fixed on a date.
  return { exposure, covered, residual: sub(exposure, covered, 'what the hedge does not cover') };
}
