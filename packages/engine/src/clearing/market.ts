/**
 * A market: a venue where one instrument clears at a stated point in the period, and the phase that
 * runs every market in order.
 *
 * @spec Bond N7 Bond N7.a Bond N9 Bond N9.a Clearing A1 Clearing B1 Clearing B2 Clearing C4.a Clearing C4.b Clearing D2 Clearing D3 Clearing D4 Clearing E1 Clearing E4 Clearing F1 Clearing F2 Register C3 Register C3.a XI-5 Goods C2 Sovereign C1 Sovereign C1.b Sovereign C2 Sovereign C4 Sovereign C5 Sovereign C6 Sovereign C7 Sovereign B3.a Bond N9.b Treasury D2.a Treasury D5.a
 *
 * Trades become instructions with the paper one way and the cash the other, settled together (D3,
 * XI-5). The print is written once per period (F2): traded if anything cleared, otherwise the last
 * print carried and marked stale with the reason it did not clear (C4.b, E4). A market whose
 * instrument has never printed and did not clear writes nothing: it has no price, which is what
 * Unpriced means at the reading site (XI-6).
 *
 * A PRIMARY OFFER (Sovereign C) is the issuer's own supply for one session: a size and a walk-away
 * level, posted by the issuer's module before the session and cleared by this same solver at one
 * uniform price (C2). It is not a second mechanism and not a second print: a new line and a
 * re-opening of an old one (B3.a) are the same act with the same book. Nobody absorbs what is not
 * bid for (Treasury D5.a): the unsold remainder is withdrawn and the withdrawal is an event (C7).
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { assertNever, forbid } from '../core/assert.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../core/ids.js';
import { add, div, finite, mul, sub, sum, zeroIfNone } from '../core/num.js';
import { none, type Option, some } from '../core/option.js';
import type { Journal } from '../journal/journal.js';
import type { AccountRef, InstructionDraft, Leg } from '../ledger/instruction.js';
import { cellSide, type Settlement } from '../ledger/settlement.js';
import type { Parties } from '../parties/party.js';
import { struckIn, type PriceStore, type StaleReason } from '../prices/price-store.js';
import { clear, type Fill, type Order, type Rationing } from './solver.js';

export interface MarketDecl {
  readonly id: MarketId;
  readonly name: string;
  readonly instrument: InstrumentId;
  readonly ccy: CurrencyCode;
  readonly rationing: Rationing;
}

/**
 * The issuer's supply for one session (Sovereign C1, C2, C5). The size is the issuer's choice
 * (C1.b) and the reservation its walk-away (C5, C7); the market chooses the price (Treasury D2.a).
 */
export interface PrimaryOffer {
  readonly market: MarketId;
  readonly issuer: PartyId;
  /** Units offered this session. */
  readonly size: number;
  /** The least the issuer will accept per unit; below it the paper is withdrawn (C7). */
  readonly reservation: number;
  /** C2: every winner pays the stop-out. */
  readonly allotment: 'uniformPrice';
}

/** Cash on each side settles through the party's own account (Money B1). */
export type AccountResolver = (party: PartyId, ccy: CurrencyCode) => AccountRef;

export interface MarketRunDeps {
  readonly parties: Parties;
  readonly prices: PriceStore;
  readonly settlement: Settlement;
  readonly journal: Journal;
  readonly accountOf: AccountResolver;
  /** Bond N9.b: what has accrued per unit at this session's date, from the instrument's own terms. */
  readonly accruedPerUnit: (instrument: InstrumentId, period: Period) => number;
  /** Who promised it, when somebody did: a physical thing has nobody on that side (Goods A1). */
  readonly instrumentIssuer: (instrument: InstrumentId) => Option<PartyId>;
}

export interface MarketResult {
  readonly market: MarketId;
  readonly outcome: 'cleared' | StaleReason;
  /** The level that cleared; none when nothing cleared and there was nothing to carry. */
  readonly price: Option<number>;
  readonly settledVolume: number;
  readonly failedTrades: number;
  /** Sovereign C4, when the session carried a primary offer. */
  readonly auction: Option<AuctionResult>;
}

/** What the market reads out of an auction (Sovereign C4). */
export interface AuctionResult {
  readonly issuer: PartyId;
  readonly size: number;
  /** Units the issuer actually placed and settled. */
  readonly allotted: number;
  /** Total demand posted, over the size offered: the cover ratio. */
  readonly cover: number;
  /** The average level the winning bids posted, less the stop-out, per unit (in yield it inverts). */
  readonly tail: Option<number>;
  readonly stopOut: Option<number>;
}

/** A matched trade: two named sides and a quantity (Clearing D2). */
interface Trade {
  readonly buyer: PartyId;
  readonly seller: PartyId;
  readonly qty: number;
}

export function runMarket(
  m: MarketDecl,
  posted: readonly Order[],
  offer: Option<PrimaryOffer>,
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
): MarketResult {
  const orders: Order[] = [...posted];
  if (offer.some) {
    forbid(
      offer.value.market === m.id,
      'Law 4',
      `offer for ${offer.value.market} posted into ${m.id}`,
    );
    const issuer = deps.instrumentIssuer(m.instrument);
    forbid(
      issuer.some && issuer.value === offer.value.issuer,
      'Sovereign C1.b',
      `${offer.value.issuer} does not issue ${m.instrument}`,
    );
    orders.push({
      party: offer.value.issuer,
      side: 'sell',
      price: offer.value.reservation,
      qty: offer.value.size,
    });
  }
  // Sovereign C2: a session carrying an offer is an auction, and its stated allotment is the
  // stop-out. Without one the venue is an open book and the sellers compete.
  const outcome = clear(orders, m.rationing, offer.some ? 'marginalBid' : 'sellersCompete');
  switch (outcome.kind) {
    case 'cleared': {
      const trades = pairFills(outcome.fills);
      const accrued = deps.accruedPerUnit(m.instrument, period);
      let settledVolume = 0;
      let allotted = 0;
      let failed = 0;
      for (const t of trades) {
        const record = deps.settlement.settle(
          tradeInstruction(m, t, outcome.price, accrued, deps),
          period,
          cycle,
        );
        if (record.outcome === 'settled') {
          settledVolume = finite(settledVolume + t.qty, 'settled volume');
          if (offer.some && t.seller === offer.value.issuer)
            allotted = finite(allotted + t.qty, 'allotted');
        } else failed += 1;
      }
      deps.prices.write({
        instrument: m.instrument,
        market: m.id,
        period,
        price: outcome.price,
        ccy: m.ccy,
        provenance: { kind: 'traded', qty: settledVolume, trades: trades.length - failed },
      });
      deps.journal.record(
        period,
        cycle,
        'print',
        [m.id, m.instrument],
        {
          price: outcome.price,
          volume: outcome.volume,
          settledVolume,
          failedTrades: failed,
          rationed: outcome.rationed,
        },
        true,
      );
      const auction = offer.some
        ? some(
            auctionResult(offer.value, orders, outcome.fills, some(outcome.price), allotted),
          )
        : none<AuctionResult>();
      if (auction.some) journalAuction(m, auction.value, period, cycle, deps);
      return {
        market: m.id,
        outcome: 'cleared',
        price: some(outcome.price),
        settledVolume,
        failedTrades: failed,
        auction,
      };
    }
    case 'noDemand':
    case 'noSupply':
    case 'noOverlap': {
      const auction = offer.some
        ? some(auctionResult(offer.value, orders, [], none<number>(), 0))
        : none<AuctionResult>();
      if (auction.some) journalAuction(m, auction.value, period, cycle, deps);
      const last = deps.prices.latest(m.instrument, period);
      if (!last.some) {
        // Nothing traded and nothing to carry: the line has no price at all, which is what a
        // reader is told when it asks (XI-6). A print is never invented to fill the gap.
        deps.journal.record(
          period,
          cycle,
          'print',
          [m.id, m.instrument],
          { printed: false, reason: outcome.kind },
          true,
        );
        return {
          market: m.id,
          outcome: outcome.kind,
          price: none(),
          settledVolume: 0,
          failedTrades: 0,
          auction,
        };
      }
      const from = struckIn(last.value);
      deps.prices.write({
        instrument: m.instrument,
        market: m.id,
        period,
        price: last.value.price,
        ccy: m.ccy,
        provenance: { kind: 'stale', from, reason: outcome.kind },
      });
      deps.journal.record(
        period,
        cycle,
        'print',
        [m.id, m.instrument],
        {
          stale: true,
          reason: outcome.kind,
          carriedFrom: from,
          price: last.value.price,
        },
        true,
      );
      return {
        market: m.id,
        outcome: outcome.kind,
        price: some(last.value.price),
        settledVolume: 0,
        failedTrades: 0,
        auction,
      };
    }
    default:
      return assertNever(outcome, 'Outcome');
  }
}

/** Pair buy fills with sell fills, walking both lists; every trade has two named sides (D2). */
function pairFills(fills: readonly Fill[]): Trade[] {
  const buys = fills.filter((f) => f.side === 'buy').map((f) => ({ ...f }));
  const sells = fills.filter((f) => f.side === 'sell').map((f) => ({ ...f }));
  const out: Trade[] = [];
  let b = 0;
  let s = 0;
  let bLeft = zeroIfNone(buys[0]?.qty);
  let sLeft = zeroIfNone(sells[0]?.qty);
  while (b < buys.length && s < sells.length) {
    const buyer = buys[b];
    const seller = sells[s];
    if (buyer === undefined || seller === undefined) break;
    const q = bLeft < sLeft ? bLeft : sLeft;
    if (q > 0) out.push({ buyer: buyer.party, seller: seller.party, qty: q });
    bLeft = finite(bLeft - q, 'buy left');
    sLeft = finite(sLeft - q, 'sell left');
    if (bLeft <= 0) {
      b += 1;
      bLeft = zeroIfNone(buys[b]?.qty);
    }
    if (sLeft <= 0) {
      s += 1;
      sLeft = zeroIfNone(sells[s]?.qty);
    }
  }
  return out;
}

/**
 * Bond N9.b: the paper changes hands at the clean price and the buyer pays the seller what has
 * accrued since the last coupon on top. The lot's basis is the clean price, so the coupon that
 * arrives next is not a windfall to whoever holds it on the date: the buyer's equity falls by the
 * accrued now and rises by the coupon then, and the seller's income is what it earned.
 */
function tradeInstruction(
  m: MarketDecl,
  t: Trade,
  price: number,
  accruedPerUnit: number,
  deps: MarketRunDeps,
): InstructionDraft {
  const buyer = deps.parties.get(t.buyer);
  const seller = deps.parties.get(t.seller);
  const cash = mul(t.qty, add(price, accruedPerUnit, 'dirty price'), 'trade cash');
  const buyerCell = cellSide(buyer, t.qty / (buyer.representation === 'cell' ? buyer.weight : 1));
  const sellerCell = cellSide(
    seller,
    t.qty / (seller.representation === 'cell' ? seller.weight : 1),
  );
  const buyerCashCell = cellSide(
    buyer,
    cash / (buyer.representation === 'cell' ? buyer.weight : 1),
  );
  const sellerCashCell = cellSide(
    seller,
    cash / (seller.representation === 'cell' ? seller.weight : 1),
  );
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: t.seller,
      to: t.buyer,
      instrument: m.instrument,
      qty: t.qty,
      pricePerUnit: some(price),
      accruedPerUnit: accruedPerUnit === 0 ? none() : some(accruedPerUnit),
      fromCell: sellerCell === undefined ? none() : some(sellerCell),
      toCell: buyerCell === undefined ? none() : some(buyerCell),
    },
    {
      kind: 'money',
      from: deps.accountOf(t.buyer, m.ccy),
      to: deps.accountOf(t.seller, m.ccy),
      ccy: m.ccy,
      amount: cash,
      fromCell: buyerCashCell === undefined ? none() : some(buyerCashCell),
      toCell: sellerCashCell === undefined ? none() : some(sellerCashCell),
    },
  ];
  const issuer = deps.instrumentIssuer(m.instrument);
  const cause = issuer.some && t.seller === issuer.value ? 'issuance' : 'trade';
  return { legs, cause, reason: `${m.name}: ${t.qty} @ ${price}` };
}

/** Sovereign C4: the cover ratio and the tail are read off the book, not stated. */
function auctionResult(
  offer: PrimaryOffer,
  orders: readonly Order[],
  fills: readonly Fill[],
  stopOut: Option<number>,
  allotted: number,
): AuctionResult {
  const bids = orders.filter((o) => o.side === 'buy' && o.party !== offer.issuer);
  const demand = sum(bids.map((o) => o.qty)).value;
  const won = fills.filter((f) => f.side === 'buy');
  const wonQty = sum(won.map((f) => f.qty)).value;
  const wonValue = sum(won.map((f) => mul(f.qty, f.at, 'bid value'))).value;
  const tail =
    stopOut.some && wonQty > 0
      ? some(sub(div(wonValue, wonQty, 'average bid'), stopOut.value, 'tail'))
      : none<number>();
  return {
    issuer: offer.issuer,
    size: offer.size,
    allotted,
    cover: div(demand, offer.size, 'cover ratio'),
    tail,
    stopOut,
  };
}

function journalAuction(
  m: MarketDecl,
  a: AuctionResult,
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
): void {
  deps.journal.record(
    period,
    cycle,
    'auction.result',
    [m.id, m.instrument, a.issuer],
    {
      line: m.instrument,
      size: a.size,
      allotted: a.allotted,
      withdrawn: sub(a.size, a.allotted, 'withdrawn'),
      cover: a.cover,
      stopOut: a.stopOut.some ? a.stopOut.value : null,
      tail: a.tail.some ? a.tail.value : null,
    },
    true,
  );
}
