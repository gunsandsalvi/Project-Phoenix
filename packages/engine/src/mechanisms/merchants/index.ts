/**
 * What a merchant does: buy here, carry it, sell there.
 *
 * @spec Freight C1 Freight C1.a Freight C2 Freight D3 Commodities Spot A1.a Commodities Spot D1 Goods C1 Clearing A2 Clearing B2 Law 2 Law 3 Law 15 Law 19
 *
 * ITS BID IS TWO PRINTS AND NOTHING ELSE (Law 3, Law 19). What a thing is worth to a merchant
 * standing here is what it fetches THERE — a print the market made, not a number this module works
 * out — less the margin its own management wants for putting money into a cargo it gets back only
 * when the cargo lands. It never computes a freight cost to subtract: what the voyage costs is
 * cleared in the freight session against carriers who have hulls, and a merchant that bid too much
 * for the cargo finds it cannot pay for the passage and the cargo stays where it is. That is the
 * loss, and it is on a named book.
 *
 * ITS ASK IS ITS OWN COST (Law 19). It offers what it holds at what those LOTS cost it plus its
 * margin, read off the register where the basis of every lot already lives. A merchant selling
 * below its basis is a merchant taking a loss, and it does that when the gap closed the wrong way —
 * which it can, because the price it sells into is the market's and not its own.
 *
 * NOTHING HERE IS A SPREAD TABLE. There is no mark-up per line, no fee, no schedule of margins by
 * distance: one preference per merchant, drawn, and two prints.
 */
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { MarketId, PartyId, RegionId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { asQty, downTick, NO_QTY } from '../../core/tick.js';
import { FIRM } from '../../registry/profiles.js';
import { goodId, goodMarketId, isGoodTerms } from '../../registry/physical.js';
import type { ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { merchantParam, type MerchantDecl } from './data.js';

export * from './data.js';

/** Law 18: by name. A merchant is asked about every goods market there is, and it is asked often. */
function indexOf(rows: readonly MerchantDecl[]): ReadonlyMap<string, MerchantDecl> {
  return new Map(rows.map((r) => [r.merchant, r]));
}

/**
 * §46 B3: what this party thinks a thing fetches in a place — its own outlook where it has formed
 * one, the last print where it has not, and nothing at all where the place has never printed. A
 * merchant with no opinion about the other end is a merchant with no reason to buy.
 */
function expected(view: ParticipantView, subUnit: string, region: RegionId): number | undefined {
  const id = goodId(subUnit, region);
  if (!view.instruments.has(id)) return undefined;
  const outlook = view.outlook(`price.${id}`);
  if (outlook.some) return outlook.value.expected;
  const print = view.print(id);
  if (print.some) return print.value.price;
  return undefined;
}

/** D3: the best another place would pay for this line, and which place that is. */
function dearest(
  view: ParticipantView,
  subUnit: string,
  notHere: RegionId,
): number | undefined {
  let best: number | undefined;
  for (const region of view.registry.regions.keys()) {
    if (region === notHere) continue;
    const there = expected(view, subUnit, region);
    if (there === undefined) continue;
    if (best === undefined || there > best) best = there;
  }
  return best;
}

/** Law 19: what its own lots of this cost it, per unit. The register is where a basis lives. */
function basisPerUnit(view: ParticipantView, market: MarketDecl): number | undefined {
  for (const h of view.holdings()) {
    if (h.instrument !== market.instrument) continue;
    const units = sum(h.lots.map((l) => l.qty));
    if (units.value <= 0) continue;
    const cost = sum(h.lots.map((l) => mul(l.qty, l.basisPerUnit, 'what this lot cost it')));
    return div(cost.value, units.value, 'what a unit of it cost it');
  }
  return undefined;
}

function ordersOf(view: ParticipantView, market: MarketDecl, m: MerchantDecl): readonly Order[] {
  if (!view.instruments.has(market.instrument)) return [];
  const i = view.instruments.get(market.instrument);
  if (!i.status.live || !isGoodTerms(i.terms)) return [];
  // C2: a merchant deals in things that can be carried. Nothing else has two places to be in.
  if (!i.terms.portable) return [];
  const margin = view.params.ratio(merchantParam(m.merchant, 'margin'));
  const here = i.terms.region;
  if (here === m.region) {
    // D3, C1.a: it will pay what the thing fetches where it is dear, less what it wants for its
    // money and its wait. Above that the cargo is worse than not buying it, which is the substitution
    // that caps the basis (C2) seen from the buying end.
    const away = dearest(view, i.terms.subUnit, here);
    if (away === undefined) return [];
    const bid = mul(away, sub(1, margin, 'less what this management wants for the wait'), 'what it will pay');
    if (bid <= 0) return [];
    const appetite = view.params.ratio(merchantParam(m.merchant, 'appetite'));
    const behind = mul(view.cash(i.ccy), appetite, 'what it will put behind this line');
    const qty = downTick(div(behind, bid, 'units it can pay for'));
    if (qty <= 0) return [];
    return [{ party: view.self.id, side: 'buy', price: bid, qty: asQty(qty) }];
  }
  // What it is holding somewhere else is for sale there, at what those lots cost it plus its
  // margin. It is not a mark-up: it is the reservation of somebody who can wait, and the price it
  // gets is the market's.
  const free = view.free(market.instrument);
  if (free <= NO_QTY) return [];
  const basis = basisPerUnit(view, market);
  if (basis === undefined || basis <= 0) return [];
  const ask = mul(basis, add(1, margin, 'and what it wants for having carried it'), 'what it will take');
  return [{ party: view.self.id, side: 'sell', price: ask, qty: free }];
}

/**
 * Law 18, Clearing B2: the books a merchant could be in this cycle — every portable line where it
 * is standing, because that is where it buys, and every line it is holding, because that is where
 * it sells. It is a filter and not a claim: a market it names that does not exist costs nothing.
 */
function marketsOf(view: ParticipantView, m: MerchantDecl): readonly MarketId[] {
  const out: MarketId[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || !isGoodTerms(i.terms) || !i.terms.portable) continue;
    if (i.terms.region === m.region) out.push(goodMarketId(i.terms.subUnit, i.terms.region));
  }
  for (const h of view.holdings()) {
    if (!view.instruments.has(h.instrument)) continue;
    const i = view.instruments.get(h.instrument);
    if (!isGoodTerms(i.terms) || i.terms.region === m.region) continue;
    out.push(goodMarketId(i.terms.subUnit, i.terms.region));
  }
  return out;
}

export function merchants(rows: readonly MerchantDecl[]): SystemModule {
  const index = indexOf(rows);
  const mine = (who: PartyId): MerchantDecl | undefined => index.get(String(who));
  return {
    id: 'merchants',
    spec: 'Freight C1 Freight D3 Commodities Spot A1.a',
    // Law 15: it needs the lines to exist and the places to be places, and nothing else. It adds no
    // party, no instrument and no market: the firms are already here and so are the books.
    requires: ['goods', 'firms'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: rows.flatMap((m) => [
      {
        id: merchantParam(m.merchant, 'margin'),
        value: m.margin,
        kind: 'preference' as const,
        dimension: 'ratio' as const,
        unit: 'ratio',
        owner: 'model',
        spec: 'Freight D3',
        why: `Freight D3: ${MARGIN_WHY}`,
      },
      {
        id: merchantParam(m.merchant, 'appetite'),
        value: m.appetite,
        kind: 'preference' as const,
        dimension: 'ratio' as const,
        unit: 'ratio',
        owner: 'model',
        spec: 'Firm A3',
        why: `Firm A3: ${APPETITE_WHY}`,
      },
    ]),
    phases: [],
    participants: [
      {
        partyKind: FIRM,
        // XI-13: it is in the book because it has a view — that a thing is worth more somewhere
        // else — and it puts its own money behind that view and loses it when it is wrong.
        speculative: true,
        markets: (view) => {
          const m = mine(view.self.id);
          if (m === undefined) return [];
          return marketsOf(view, m);
        },
        orders: (view, market) => {
          const m = mine(view.self.id);
          if (m === undefined) return [];
          return ordersOf(view, market, m);
        },
      },
    ],
    families: [],
  };
}

const MARGIN_WHY =
  'what this management wants on the gap before it commits money to a cargo it gets back only when the cargo lands. It is what stands between two prints and what a merchant will pay, so it is also what a basis cannot close below — and nobody is charged it, which is what makes it a preference rather than a fee.';
const APPETITE_WHY =
  'how much of its money goes behind one line. It is the concentration risk in merchanting, and which house is long when a gap closes the wrong way is what makes a failure somebody in particular.';
