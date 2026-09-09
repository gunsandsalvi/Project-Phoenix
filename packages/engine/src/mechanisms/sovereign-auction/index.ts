/**
 * Who bids at a sovereign auction, and why.
 *
 * @spec Sovereign C2 Sovereign C3 Sovereign C3.a Sovereign C3.b Sovereign C4 Sovereign E2 Sovereign E2.a Sovereign E5 Treasury D5 Treasury D5.a Clearing A3 Clearing B2 Clearing B4 Dealer Desks D4 Dealer Desks D4.a
 *
 * Every bank in this world is a primary dealer: it carries an OBLIGATION to bid a share of what is
 * offered, and it holds the privilege of the liquidity buffer sovereign paper counts towards (E2.a,
 * E5). The obligation is what makes an auction hard to fail (C3.a) — and it does not make failure
 * impossible, which is the whole point: a bank bids out of the cash it actually has, so a bank at
 * its limit bids nothing (Dealer Desks D4, D4.a) and the auction fails when the dealers step back.
 * Nobody absorbs the remainder by construction (Treasury D5.a, Clearing B4).
 *
 * The bid is a price, not a quantity at any price: the bank's own reservation yield over the curve,
 * turned into what that yield is worth on this line's own cash flows. It can bid badly and wear it
 * (C3.b).
 */
import { issuerOf } from '../../register/instruments.js';
import { paramId } from '../../core/ids.js';
import { div, material, mul, sub } from '../../core/num.js';
import type { Order } from '../../clearing/solver.js';
import { BANK } from '../../registry/profiles.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  P_REQUIRED_YIELD,
  bufferTarget,
  demandSteps,
  priceAtYield,
  sovereignValue,
} from '../sovereign-curve/index.js';

export const P_MIN_BID_SHARE = paramId('sovereign.primaryDealers.minBidShare');

export const sovereignAuction: SystemModule = {
  id: 'sovereign-auction',
  spec: 'Sovereign C',
  requires: ['sovereign-instruments', 'sovereign-curve'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: P_MIN_BID_SHARE,
      value: 0.5,
      unit: 'ratio of the size offered',
      kind: 'policy',
      owner: 'model',
      why: 'Sovereign C3, C3.a: primary dealers bid because they are obliged to, in exchange for privileges, and that obligation is what makes an auction hard to fail. The issuer states a share such that its dealers between them cover what it brings — with two dealers in this world, half each. It does not make failure impossible: a dealer bids out of the cash it has, so a dealer at its limit still bids nothing.',
    },
  ],
  phases: [],
  participants: [
    {
      partyKind: BANK,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
        const offer = view.offer(m.id);
        if (!offer.some) return [];
        const i = view.instruments.get(m.instrument);
        const price = priceAtYield(view, m, view.params.get(P_REQUIRED_YIELD));
        if (price === undefined || price <= 0) return [];
        // What it must bid for (C3), and what its own buffer is short of (E2.a, E5): the larger of
        // the two is what it wants at the yield it requires, and anything beyond that costs more.
        const obliged = mul(offer.value.size, view.params.get(P_MIN_BID_SHARE), 'obligation');
        const target = bufferTarget(view, i.ccy);
        const value = sovereignValue(view, issuerOf(i), i.ccy);
        const gap = sub(target, value, 'gap');
        const wantedForBuffer =
          gap > 0 && material(gap, 2, Math.abs(target) + Math.abs(value))
            ? div(gap, price, 'units wanted')
            : 0;
        const needed = obliged > wantedForBuffer ? obliged : wantedForBuffer;
        // C3.a: it bids out of the cash it has. A bank with none bids nothing, and that is how an
        // auction fails: not because a rule allowed it to, but because nobody could pay.
        const affordable = div(view.cash(i.ccy), price, 'affordable');
        return demandSteps(view, m, needed, affordable);
      },
    },
  ],
  families: [],
};
