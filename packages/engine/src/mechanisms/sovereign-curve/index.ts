/**
 * The sovereign curve and the reasons a bank holds sovereign paper.
 *
 * @spec Treasury E4 Sovereign E1.a Sovereign D1 Sovereign D2 Sovereign D3 Sovereign D3.a Sovereign D3.b Sovereign D3.c Sovereign D4 Sovereign D5 Sovereign E1 Sovereign E2 Sovereign E2.a Sovereign E3 Sovereign E4 Sovereign E5 Sovereign B3.a Bond N7.b Clearing A3 Clearing B2 XI-14
 *
 * This module owns one curve family (D3.a: one owner) and states its convention once (D3.c). The
 * curve itself is never stored: `ctx.curve` builds it from prints already produced when somebody
 * asks (see prices/curve.ts), so the fit's own output can never become an observation (D3.b).
 *
 * It also carries the secondary market's reason to exist: a bank holds sovereign paper because the
 * regulatory liquidity buffer makes it hold some (E2.a, E5), and its holding drifts away from that
 * buffer every period as its deposits move. Its orders are the difference, at its own reservation.
 * A bank whose buffer is already met posts nothing, and a market with nobody on one side does not
 * clear — which is D5 read from the other side: a seller with no buyer keeps its paper.
 */
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { add, div, material, mul, sub, sum } from '../../core/num.js';
import { curveFamilyOf, priceAt, type CurveFamilyDecl } from '../../prices/curve.js';
import { tradedIn } from '../../prices/price-store.js';
import { paramId } from '../../core/ids.js';
import { BANK } from '../../registry/profiles.js';
import type { Order } from '../../clearing/solver.js';
import { issuedBy } from '../../register/instruments.js';
import type { SystemModule } from '../../world/module.js';
import type { ParticipantView } from '../../world/context.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { MarketDecl } from '../../clearing/market.js';

export const P_BUFFER = paramId('bank.liquidityBuffer.perDeposit');
export const P_REQUIRED_YIELD = paramId('sovereign.holders.requiredYield');
export const P_SURPLUS_PREMIUM = paramId('sovereign.holders.surplusPremium');

/** D3.c: one compounding convention and one day count, stated by the owner and used by everyone. */
export const CURVE_COMPOUNDING = 'annual';
export const CURVE_DAY_COUNT = 'ACT/ACT';

/**
 * What a line is worth to a party at a yield it has decided for itself: its own cash flows,
 * discounted. The yield is an INPUT to the schedule the participant posts and the price is what
 * clears; the curve is then read back out of the prints (D2, N7.b). Nothing here reads the print it
 * is about to help set, which is the fixed point XI-13 exists to prevent.
 */
export function priceAtYield(view: ParticipantView, m: MarketDecl, y: number): number | undefined {
  const i = view.instruments.get(m.instrument);
  const on = view.calendar.startOf(view.period);
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  if (flows.length === 0) return undefined;
  return priceAt(flows, y, on, CURVE_DAY_COUNT, `value of ${i.id}`);
}

/**
 * A holder's two-step demand for sovereign paper (E2.a, E5, Clearing A2): it takes what its buffer
 * is short of at the yield it requires, and more only if it is paid a premium for paper it does not
 * need. Two steps make the schedule a schedule — which is what lets a bigger auction clear lower
 * (Treasury E4) instead of at a level somebody wrote down.
 */
export function demandSteps(
  view: ParticipantView,
  m: MarketDecl,
  neededUnits: number,
  affordableUnits: number,
): readonly Order[] {
  const required = view.params.get(P_REQUIRED_YIELD);
  const premium = view.params.get(P_SURPLUS_PREMIUM);
  const atRequired = priceAtYield(view, m, required);
  const atPremium = priceAtYield(view, m, add(required, premium, 'surplus yield'));
  if (atRequired === undefined || atPremium === undefined) return [];
  const out: Order[] = [];
  const first = neededUnits < affordableUnits ? neededUnits : affordableUnits;
  if (first > 0 && atRequired > 0) {
    out.push({ party: view.self.id, side: 'buy', price: atRequired, qty: first });
  }
  const rest = sub(affordableUnits, first, 'surplus capacity');
  if (rest > 0 && atPremium > 0) {
    out.push({ party: view.self.id, side: 'buy', price: atPremium, qty: rest });
  }
  return out;
}

/** How long a line has to run, on the curve's own day count. */
export function tenorOf(view: ParticipantView, m: MarketDecl): number | undefined {
  const i = view.instruments.get(m.instrument);
  const on = view.calendar.startOf(view.period);
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  const last = flows[flows.length - 1];
  if (last === undefined) return undefined;
  return yearFraction(CURVE_DAY_COUNT, on, last.date);
}

/** The value of every sovereign line a party holds, at the prints in force (E3). */
export function sovereignValue(view: ParticipantView, issuer: PartyId, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!issuedBy(i, issuer) || i.ccy !== ccy || !i.status.live) continue;
    const print = view.print(i.id);
    if (!print.some) continue;
    terms.push(mul(view.quantity(i.id), print.value.price, 'sovereign value'));
  }
  return sum(terms).value;
}

/** The buffer a bank is holding sovereign paper against: a share of the money it has issued (E5). */
export function bufferTarget(view: ParticipantView, ccy: CurrencyCode): number {
  const money = view.instruments
    .all()
    .find(
      (i) =>
        issuedBy(i, view.self.id) &&
        i.ccy === ccy &&
        view.registry.instrumentKind(i.kind).pricing === 'money',
    );
  if (money === undefined) return 0;
  return mul(money.issued, view.params.get(P_BUFFER), 'buffer target');
}

export function sovereignCurve(issuer: PartyId, ccy: CurrencyCode): SystemModule {
  const family: CurveFamilyDecl = {
    id: curveFamilyOf(issuer, ccy),
    name: `${issuer} curve in ${ccy}`,
    issuer,
    ccy,
    compounding: CURVE_COMPOUNDING,
    dayCount: CURVE_DAY_COUNT,
  };
  return {
    id: 'sovereign-curve',
    spec: 'Sovereign D, E',
    requires: ['sovereign-instruments'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [family],
    units: [],
    params: [
      {
        id: P_BUFFER,
        value: 0.2,
        unit: 'ratio of deposits issued',
        kind: 'placeholder',
        owner: 'model',
        why: 'Sovereign E2.a, E5: a bank holds sovereign paper against a liquidity buffer. Money Market A2.a says the size of that buffer is the bank own preference, derived from how sticky its liabilities are; until deposit classes exist a stated share of what it has issued stands in.',
        standsInFor: { mechanism: 'Money Market A2, A2.a (the buffer a bank chooses)', worklistItem: '11' },
      },
      {
        id: P_REQUIRED_YIELD,
        value: 0.02,
        unit: 'per annum',
        kind: 'placeholder',
        owner: 'model',
        why: 'Corporate Credit E5: a holder reservation is built from its own cost of funds, its expected loss and the capital the position consumes. None of the three exists yet, so one stated yield stands in for all three. It is deliberately NOT a spread over the curve: a reservation formed from the print is the fixed point XI-13 forbids, and the market could not then disagree with itself.',
        standsInFor: {
          mechanism: 'XI-4 and Corporate Credit E5 (cost of funds, expected loss, capital)',
          worklistItem: '10',
        },
      },
      {
        id: P_SURPLUS_PREMIUM,
        value: 0.01,
        unit: 'per annum over the required yield',
        kind: 'preference',
        owner: 'model',
        why: 'Clearing A2, A2.a: a schedule answers "and if it were cheaper?". A holder takes paper it does not need only if it is paid more for it, and that step is what makes a heavier auction clear lower (Treasury E4) rather than at a level nobody chose.',
      },
    ],
    phases: [],
    participants: [
      {
        partyKind: BANK,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const i = view.instruments.get(m.instrument);
          if (!issuedBy(i, issuer) || i.ccy !== ccy) return [];
          // A session carrying the issuer own offer is an auction: the bank bids there through its
          // obligation (Sovereign C3), and posting twice would be two reasons for one demand.
          if (view.offer(m.id).some) return [];
          const price = priceAtYield(view, m, view.params.get(P_REQUIRED_YIELD));
          if (price === undefined || price <= 0) return [];
          const target = bufferTarget(view, ccy);
          const value = sovereignValue(view, issuer, ccy);
          const gap = sub(target, value, 'buffer gap');
          if (!material(gap, 2, Math.abs(target) + Math.abs(value))) return [];
          const lines = view.markets.filter((x) => {
            const inst = view.instruments.get(x.instrument);
            return issuedBy(inst, issuer) && inst.ccy === ccy && inst.status.live;
          }).length;
          if (lines === 0) return [];
          const share = div(gap, lines, 'gap per line');
          const qty = div(share, price, 'units');
          if (qty > 0) {
            const affordable = div(view.cash(ccy), price, 'affordable');
            return demandSteps(view, m, qty, affordable);
          }
          // Above its buffer it is a seller, and what it will let go for is the same yield it
          // requires to hold the paper at all (D5: an offer no bid reaches simply does not clear).
          const held = view.free(m.instrument);
          const want = -qty;
          const size = want < held ? want : held;
          return size > 0 ? [{ party: view.self.id, side: 'sell', price, qty: size }] : [];
        },
      },
    ],
    families: [pointsMatchPrints(family)],
  };
}

/**
 * Sovereign D3.b: a point that says it traded must correspond to a print that says the same. The
 * curve is built at the read, so this is not a check against itself: it compares the label the read
 * produced with the provenance the price store recorded when the market printed.
 */
function pointsMatchPrints(family: CurveFamilyDecl): Family {
  return {
    name: 'prices',
    contributor: 'sovereign-curve',
    spec: 'Sovereign D3.b',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!issuedBy(i, family.issuer) || i.ccy !== family.ccy || !i.status.live) continue;
        const print = view.prices.read(i.id, view.period);
        const traded = print.some && tradedIn(print.value, view.period);
        const carried = view.prices.latest(i.id, view.period);
        if (!carried.some) continue;
        const labelled = tradedIn(carried.value, view.period);
        if (labelled !== traded) {
          out.push({
            family: 'prices',
            spec: 'Sovereign D3.b',
            owner: i.id,
            size: 1,
            unit: 'point',
            period: view.period,
            message: `${i.id}: the curve would call this point ${labelled ? 'traded' : 'stale'} and the price store says otherwise`,
          });
        }
      }
      return out;
    },
  };
}
