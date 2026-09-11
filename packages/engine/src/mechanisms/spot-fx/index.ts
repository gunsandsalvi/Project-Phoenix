/**
 * The currency market: every pair a market on its own flow, and no vehicle anywhere.
 *
 * @spec Spot FX A1 Spot FX A2 Spot FX A3 Spot FX B1 Spot FX B2 Spot FX B5 Spot FX B6 Spot FX C1 Spot FX C2 Spot FX C2.a Spot FX C3 Spot FX C3.b Spot FX C4 Spot FX C5 Spot FX C6 Spot FX D1 Spot FX D2 Spot FX D3 Spot FX D4 Spot FX D5 Spot FX E1 Spot FX E3 Currency A3 Currency B1 Currency C1 Currency C2 Currency C3 Currency C3.a Currency C3.b XI-12 XI-13 Law 3 Law 6 Law 15
 *
 * A RATE IS A PRICE, and this module does nothing to it but bring the two sides that make it. Every
 * pair among this world's currencies opens as a market (A3); a spot trade is two money legs settling
 * atomically (A1, C6, the kernel's `fx` market kind); and the print is what those two sides met at
 * (C1). Nothing here converts anything for anybody and nothing computes a cross: the pairs clear on
 * their OWN flows and triangular consistency is an outcome a bounded arbitrageur produces (C2.a,
 * C3), which is exactly why the gap is measurable and sometimes open.
 *
 * THE REASONS, and there are four of them, because a market with one reason in it is a fixed point
 * (XI-13):
 *   B1 somebody who OWES a money it has not got — read off its own liabilities (`view.owedIn`);
 *   B2 somebody holding a money it does not want — a balance in a money it has no use for;
 *   B5 a DEALER, quoting both ways from its own inventory and its own limit (D1-D5);
 *   E3 an ARBITRAGEUR, when the cross and the direct route disagree by more than its own cost.
 *
 * NO VEHICLE CURRENCY (C3.b, XI-12). The ledger never triangulates: every conversion is a trade in
 * the pair the payer chose, and if the cheapest route it can see is two trades then it posts two
 * orders. That is its choice and never the kernel's, which is why a cross rate here is a fact about
 * what people did rather than an arithmetic identity.
 */
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { fxPairId, marketId, type CurrencyCode, type MarketId } from '../../core/ids.js';
import { BANK, FIRM, HOUSEHOLD, TREASURY } from '../../registry/profiles.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { BankDecl } from '../banks/data.js';
import { FX_SPREAD, fxParam } from './data.js';
import { arbitrage } from './arbitrage.js';
import { arbitrageOrders, dealerOrders, needOrders } from './participants.js';
import { triangularConsistency } from './family.js';

export * from './data.js';
export { triangles, type Triangle } from './arbitrage.js';

/** Law 9: a pair's market is named after the pair, which is how a market names one. */
export const fxMarketOf = (base: CurrencyCode, quote: CurrencyCode): MarketId =>
  marketId(`mkt.fx.${base}/${quote}`);

/**
 * A3: THE PAIRS THIS WORLD HAS, which is every ordered pair of its currencies taken once. `A/B` and
 * `B/A` are the same market read two ways (C3), not two markets: one book, one print, and the
 * inverse is one over it. Which way round it is stated is the registry's order, so it is the same
 * in every world built from the same registry.
 */
export function pairsOf(currencies: readonly CurrencyCode[]): readonly { base: CurrencyCode; quote: CurrencyCode }[] {
  const out: { base: CurrencyCode; quote: CurrencyCode }[] = [];
  for (let i = 0; i < currencies.length; i += 1) {
    for (let j = i + 1; j < currencies.length; j += 1) {
      const base = currencies[i];
      const quote = currencies[j];
      if (base !== undefined && quote !== undefined) out.push({ base, quote });
    }
  }
  return out;
}

/**
 * Spot FX F1.a: the pairs clear FIRST. A payer short of a money buys it here, with its own
 * counterparty at a rate that cleared, before the session that needs it — which is what "never
 * inside the trade" means. Nothing else in the period is ordered, so everything else runs after.
 */
export const FX_MARKET_ORDER = 0;

export function spotFx(rows: readonly BankDecl[]): SystemModule {
  const byName = new Map(rows.map((r) => [r.bank, r]));
  return {
    id: 'spot-fx',
    spec: 'Spot FX, Currency',
    // Its dealers are banks with their own capital and their own limits, and a party's reason to be
    // here is what it owes — which the money market and the banks module put into the world.
    requires: ['banks', 'money-market', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: rows.flatMap((r): ParamDecl[] => [
      {
        id: fxParam(r.bank, 'inventoryLimit'),
        value: r.fxInventoryLimit,
        unit: 'share of its own capital behind a position in one currency',
        kind: 'preference',
        owner: 'model',
        why: `Spot FX D1, D2: ${FX_SPREAD.inventoryLimit.why}`,
      },
      {
        id: fxParam(r.bank, 'edge'),
        value: r.fxEdge,
        unit: 'share of the rate, per period',
        kind: 'preference',
        owner: 'model',
        why: `Spot FX D3: ${FX_SPREAD.edge.why}`,
      },
      {
        id: fxParam(r.bank, 'arbitrageEdge'),
        value: r.fxArbitrageEdge,
        unit: 'share of the rate',
        kind: 'preference',
        owner: 'model',
        why: `Spot FX C2.a, E3: ${FX_SPREAD.arbitrageEdge.why}`,
      },
    ]),
    phases: [
      {
        name: 'fx.arbitrage',
        spec: 'Spot FX C2.a C3 E3',
        cycle: 'anchor',
        // Before the session it acts in: what it sees is what the market last said (Clearing F1),
        // and what it does about it is an order like anybody else's.
        anchor: { before: 'markets' },
        run: (ctx: MechanismContext) => {
          arbitrage(ctx, byName);
        },
      },
    ],
    participants: [
      {
        // B5, E3: the desks. A bank that will put capital behind a currency position quotes both
        // ways — and takes the round trip it decided on this morning, in whichever of the three
        // books is asking (Clearing F1).
        partyKind: BANK,
        in: 'fx' as const,
        // B5, D4, Law 4: a bank's position in a pair is ITS DESK'S BOOK, and the quote already has
        // that position in it (`dealerOrders` skews both sides by it). Asking it for a need order
        // as well would be the same balance offered twice by the same party — and at crossing
        // prices, because the desk's own bid is where its unwanted money goes (Clearing A2). What a
        // bank does with a money it has too much of is quote it cheaper, which is D4.
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => [
          ...dealerOrders(view, m, byName.get(String(view.self.id))),
          ...arbitrageOrders(view, m),
        ],
      },
      // B1, B2: and everybody else who owes a money it has not got or holds one it does not want.
      // The reason is the same reason whoever has it, so it is one function asked of every kind
      // that can have a foreign obligation — never a rule about what sort of party this is (Law 15).
      //
      // The KERNEL's kinds, because a module may not name another module's (`no-cross-module-import`
      // is the lint, and the boundary is the point): a fund or an estate with a foreign obligation
      // is its own module's participant, calling the same `needOrders` this one exports.
      ...([TREASURY, FIRM, HOUSEHOLD] as const).map((partyKind) => ({
        partyKind,
        in: 'fx' as const,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => needOrders(view, m),
      })),
    ],
    families: [triangularConsistency(rows)],
    seed(ctx) {
      // A3: every pair is a market, opened at the seed because the pairs are the currencies and the
      // currencies are the registry's. Nothing states which pairs trade: a world with two moneys in
      // it has one market between them whether or not anybody has used it yet.
      for (const { base, quote } of pairsOf([...ctx.registry.currencies.keys()])) {
        ctx.openMarket({
          id: fxMarketOf(base, quote),
          name: `${base}/${quote}`,
          instrument: fxPairId(base, quote),
          ccy: quote,
          rationing: 'proRata',
          kind: 'fx',
          fx: { base, quote },
          order: FX_MARKET_ORDER,
        });
      }
    },
  };
}

export { needOrders, dealerOrders, arbitrageOrders } from './participants.js';
