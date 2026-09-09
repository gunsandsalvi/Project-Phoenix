/**
 * A market: a venue where one instrument clears at a stated point in the period, and the phase that
 * runs every market in order.
 *
 * @spec Clearing A1 Clearing B1 Clearing B2 Clearing C4.a Clearing C4.b Clearing D2 Clearing D3 Clearing D4 Clearing E1 Clearing E4 Clearing F1 Clearing F2 Register C3 Register C3.a XI-5 Goods C2
 *
 * Trades become instructions with the paper one way and the cash the other, settled together (D3,
 * XI-5). The print is written once per period (F2): traded if anything cleared, otherwise the last
 * print carried and marked stale with the reason it did not clear (C4.b, E4).
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { assertNever, forbid } from '../core/assert.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../core/ids.js';
import { finite, mul, zeroIfNone } from '../core/num.js';
import { none, some } from '../core/option.js';
import type { Journal } from '../journal/journal.js';
import type { AccountRef, InstructionDraft, Leg } from '../ledger/instruction.js';
import { cellSide, type Settlement } from '../ledger/settlement.js';
import type { Parties } from '../parties/party.js';
import type { PriceStore, StaleReason } from '../prices/price-store.js';
import { clear, type Fill, type Order, type Rationing } from './solver.js';

export interface MarketDecl {
  readonly id: MarketId;
  readonly name: string;
  readonly instrument: InstrumentId;
  readonly ccy: CurrencyCode;
  readonly rationing: Rationing;
}

/** Cash on each side settles through the party's own account (Money B1). */
export type AccountResolver = (party: PartyId, ccy: CurrencyCode) => AccountRef;

export interface MarketRunDeps {
  readonly parties: Parties;
  readonly prices: PriceStore;
  readonly settlement: Settlement;
  readonly journal: Journal;
  readonly accountOf: AccountResolver;
}

export interface MarketResult {
  readonly market: MarketId;
  readonly outcome: 'cleared' | StaleReason;
  readonly price: number;
  readonly settledVolume: number;
  readonly failedTrades: number;
}

/** A matched trade: two named sides and a quantity (Clearing D2). */
interface Trade {
  readonly buyer: PartyId;
  readonly seller: PartyId;
  readonly qty: number;
}

export function runMarket(
  m: MarketDecl,
  orders: readonly Order[],
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
): MarketResult {
  const outcome = clear(orders, m.rationing);
  switch (outcome.kind) {
    case 'cleared': {
      const trades = match(outcome.fills);
      let settledVolume = 0;
      let failed = 0;
      for (const t of trades) {
        const record = deps.settlement.settle(
          tradeInstruction(m, t, outcome.price, deps),
          period,
          cycle,
        );
        if (record.outcome === 'settled')
          settledVolume = finite(settledVolume + t.qty, 'settled volume');
        else failed += 1;
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
      return {
        market: m.id,
        outcome: 'cleared',
        price: outcome.price,
        settledVolume,
        failedTrades: failed,
      };
    }
    case 'noDemand':
    case 'noSupply':
    case 'noOverlap': {
      const last = deps.prices.latest(m.instrument, period);
      forbid(
        last.some,
        'Clearing E4',
        `${m.id} has no print to carry; the seed must state an opening price`,
      );
      const from =
        last.value.provenance.kind === 'stale' ? last.value.provenance.from : last.value.period;
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
        price: last.value.price,
        settledVolume: 0,
        failedTrades: 0,
      };
    }
    default:
      return assertNever(outcome, 'Outcome');
  }
}

/** Pair buy fills with sell fills, walking both lists; every trade has two named sides (D2). */
function match(fills: readonly Fill[]): Trade[] {
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

function tradeInstruction(
  m: MarketDecl,
  t: Trade,
  price: number,
  deps: MarketRunDeps,
): InstructionDraft {
  const buyer = deps.parties.get(t.buyer);
  const seller = deps.parties.get(t.seller);
  const cash = mul(t.qty, price, 'trade cash');
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
  return { legs, cause: 'trade', reason: `${m.name}: ${t.qty} @ ${price}` };
}
