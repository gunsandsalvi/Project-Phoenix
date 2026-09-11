/**
 * The primary form of clearing: the issuer's own supply, cleared by the one solver at one uniform
 * price, with a walk-away and no buyer of last resort.
 *
 * @spec Sovereign C2 Sovereign C4 Sovereign C5 Sovereign C7 Central Bank C3 Clearing C4.c Treasury D5.a Treasury D2.a
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  GOV_LINE,
  GOV_MARKET,
  USD,
  SOVEREIGN_BILL,
  TREASURY_US,
  assemble,
  civil,
  clear,
  instrumentId,
  marketId,
  partyId,
  period,
  resolveMarketOrders,
  some,
  type SovereignBillTerms,
  type MechanismContext,
  type Order,
  type MarketResult,
  type SystemModule,
  type World,
} from '../src/index.js';
import { dustOf, withinDust } from '../src/core/num.js';
import { rigSpec, withDependencies, mergeModules } from './rig.js';
import { paidTheSame } from './expected.js';
import { notDealing } from './no-dealing.js';
import { asQty, type Qty } from '../src/core/tick.js';

/** Law 8: a size in this test is a count of the unit's own pieces, like every size anywhere. */
const q = asQty;


/** The one market this test drives, with its optional readings taken. */
const BILL = instrumentId('ust.bill.2027-12-15');
const BILL_MARKET = marketId('mkt.bill');

function result(markets: readonly MarketResult[], id: string): MarketResult {
  const m = markets.find((x) => x.market === id);
  if (m === undefined) throw new Error(`no result for ${id}`);
  return m;
}

function priceOf(m: MarketResult): number {
  if (!m.price.some) throw new Error('no price');
  return m.price.value;
}

function auctionOf(m: MarketResult): {
  allotted: number;
  cover: number;
  tail: number;
  stopOut: number;
} {
  if (!m.auction.some) throw new Error('no auction');
  const a = m.auction.value;
  return {
    allotted: a.allotted,
    cover: a.cover,
    tail: a.tail.some ? a.tail.value : Number.NaN,
    stopOut: a.stopOut.some ? a.stopOut.value : Number.NaN,
  };
}

/** A module that posts the treasury's offer and the bidders' schedules for one period. */
function auctioneer(
  size: Qty,
  reservation: number,
  bids: readonly { party: string; price: number | 'market'; qty: Qty }[],
): SystemModule {
  return {
    id: 'test.auctioneer',
    spec: 'Sovereign C',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.offer',
        spec: 'Sovereign C1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 1) return;
          ctx.offer({
            market: GOV_MARKET,
            issuer: TREASURY_US,
            size,
            reservation,
            allotment: 'uniformPrice',
          });
        },
      },
    ],
    participants: [
      {
        partyKind: FIRM,
        orders: (view): readonly Order[] =>
          view.period === 1
            ? bids
                .filter((b) => b.party === view.self.id)
                .map((b) => ({ party: view.self.id, side: 'buy' as const, price: b.price, qty: b.qty }))
            : [],
      },
    ],
    families: [],
  };
}

/** The kernel and the opening state: the auction under test is the only one in the world. */
function world(...extra: SystemModule[]): World {
  const spec = rigSpec('seed-auction');
  const kernelOnly = withDependencies(spec.modules, (m) =>
m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map(notDealing);
  return assemble({ ...spec, modules: mergeModules(kernelOnly, extra) });
}

describe('the primary market (Sovereign C)', () => {
  it('clears at the stop-out, every winner pays it, and the issuance reaches the issuer', () => {
    const w = world(
      // Sized to what these three bidders actually hold: a dealer bids out of the cash it has
      // (C3.a), and a test whose bids exceed it would be testing that rule and not this one.
      auctioneer(q(60), 0.9, [
        { party: 'firm.1', price: 0.99, qty: q(40) },
        { party: 'firm.2', price: 0.97, qty: q(40) },
        { party: 'firm.3', price: 0.93, qty: q(40) },
      ]),
    );
    const before = w.instruments.get(GOV_LINE).issued;
    const cashBefore = w.cash(TREASURY_US, USD);
    const accrued = w.accruedPerUnit(GOV_LINE, period(1));
    const r = w.step();
    const m = result(r.markets, GOV_MARKET);
    expect(m.outcome).toBe('cleared');
    // C2: one level, the lowest accepted bid; the top bidder does not pay its own bid.
    //
    // Law 7, Law 8: TO WITHIN THE DUST OF THE GRID THE LEVEL IS ON. A limit is posted on the
    // market's own tick (12b.1), and a tick of a ten-thousandth of face is not binary-exact — 0.97
    // is 9700 ticks and 9700 ticks is 0.9700000000000001. An exact comparison against the decimal
    // somebody typed is comparing a level to a number that is not on the grid at all.
    expect(withinDust(priceOf(m), 0.97, dustOf(2, priceOf(m) + 0.97))).toBe(true);
    expect(auctionOf(m).allotted).toBe(60);
    // C4: cover is what was bid over what was offered.
    expect(auctionOf(m).cover).toBeCloseTo(2, 12);
    // C4: the tail is the average winning bid against the stop-out, and it is positive here.
    expect(auctionOf(m).tail).toBeCloseTo((40 * 0.99 + 20 * 0.97) / 60 - 0.97, 12);
    expect(w.instruments.get(GOV_LINE).issued).toBeCloseTo(before + 60, 9);
    // C6: the proceeds reach the treasury's account — the clean price and the interest that had
    // accrued on the paper it just sold (N9.b).
    paidTheSame(w.cash(TREASURY_US, USD), cashBefore + 60 * (0.97 + accrued));
    const ev = w.journal.ofKind('auction.result');
    expect(ev).toHaveLength(1);
    expect(ev[0]?.public).toBe(true);
  });

  it('withdraws the paper nobody bid for and the withdrawal is an event (C7, D5.a)', () => {
    const w = world(auctioneer(q(300), 0.99, [{ party: 'firm.1', price: 0.5, qty: q(500) }]));
    const before = w.instruments.get(GOV_LINE).issued;
    const r = w.step();
    const m = result(r.markets, GOV_MARKET);
    expect(m.outcome).toBe('noOverlap');
    expect(auctionOf(m).allotted).toBe(0);
    // Nobody absorbs the remainder: the line's issued amount did not move.
    expect(w.instruments.get(GOV_LINE).issued).toBeCloseTo(before, 9);
    const ev = w.journal.ofKind('auction.result')[0];
    expect(ev?.data['withdrawn']).toBe(300);
  });

  it('cuts the size when the book is thin: weak demand is a lower price or less paper (C5)', () => {
    const w = world(auctioneer(q(60), 0.9, [{ party: 'firm.1', price: 0.92, qty: q(20) }]));
    const r = w.step();
    const m = result(r.markets, GOV_MARKET);
    expect(priceOf(m)).toBe(0.92);
    expect(auctionOf(m).allotted).toBe(20);
    expect(auctionOf(m).cover).toBeCloseTo(1 / 3, 12);
  });
});

describe('a quantity with no level (Central Bank C3, Clearing C4.c)', () => {
  it('takes the worst level the other side posted and never invents one', () => {
    const buys: Order[] = [{ party: partyId('a'), side: 'buy', price: 'market', qty: q(10) }];
    const sells: Order[] = [
      { party: partyId('s1'), side: 'sell', price: 0.95, qty: q(5) },
      { party: partyId('s2'), side: 'sell', price: 0.98, qty: q(5) },
    ];
    const { resolved, unpriced } = resolveMarketOrders([...buys, ...sells]);
    expect(unpriced).toHaveLength(0);
    expect(resolved.find((o) => o.party === 'a')?.price).toBe(0.98);
    const outcome = clear([...buys, ...sells], 'proRata');
    expect(outcome.kind).toBe('cleared');
    if (outcome.kind === 'cleared') expect(outcome.price).toBe(0.98);
  });

  it('is not in the book at all when the other side posted nothing', () => {
    const orders: Order[] = [{ party: partyId('a'), side: 'buy', price: 'market', qty: q(10) }];
    const { resolved, unpriced } = resolveMarketOrders(orders);
    expect(resolved).toHaveLength(0);
    expect(unpriced).toHaveLength(1);
    expect(clear(orders, 'proRata').kind).toBe('noDemand');
  });
});

describe('a line with no price (XI-6)', () => {
  it('writes no print when nothing cleared and there is nothing to carry', () => {
    const newLine: SystemModule = {
      id: 'test.newline',
      spec: 'Sovereign C',
      requires: ['seed.foundation'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.newline',
          spec: 'Sovereign C1',
          cycle: 0,
          anchor: { after: 'corporateActions' },
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 1) return;
            const terms: SovereignBillTerms = {
              kind: SOVEREIGN_BILL,
              issueDate: civil(2026, 1, 5),
              maturity: civil(2027, 12, 15),
            };
            ctx.issue({
              id: BILL,
              kind: SOVEREIGN_BILL,
              issuer: some(TREASURY_US),
              ccy: USD,
              terms,
              market: some(BILL_MARKET),
            });
            ctx.openMarket({
              id: BILL_MARKET,
              name: 'North bill 2027-12-15',
              instrument: BILL,
              ccy: USD,
              rationing: 'proRata',
            });
            ctx.offer({
              market: BILL_MARKET,
              issuer: TREASURY_US,
              size: q(100),
              reservation: 0.99,
              allotment: 'uniformPrice',
            });
          },
        },
      ],
      participants: [],
      families: [],
    };
    const w = world(newLine);
    const r = w.step();
    const m = result(r.markets, BILL_MARKET);
    expect(m.outcome).toBe('noDemand');
    expect(m.price.some).toBe(false);
    expect(w.prices.latest(BILL, w.period).some).toBe(false);
    expect(r.audit.total).toBe(0);
  });
});
