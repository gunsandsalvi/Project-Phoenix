/**
 * FREIGHT: moving a thing costs money and takes time, and somebody with a finite number of hulls
 * decides whether to move it.
 *
 * @spec Freight A1 Freight A2 Freight A3 Freight A3.a Freight A4 Freight B1 Freight B2 Freight B3 Freight B4 Freight C1 Freight C3 Freight D1 Freight D2 Freight D3 Freight D6 Freight E1 Freight E2 Freight E3 Clearing A2 Clearing A3 Law 3 Law 5 Law 6 Law 19
 *
 * A world where a thing moves for nothing has ONE PRICE EVERYWHERE by construction: nothing is
 * dearer where it is short, there is no basis between two places, and there is nothing for anybody
 * to close. So room on a route is a real thing with a real limit — a named carrier's hulls, which
 * are plant with a life (A4, Capital Programme A4) — and what it costs to move a tonne is CLEARED
 * from shippers wanting room meeting carriers with room to sell (D1). No freight rate is declared
 * anywhere.
 *
 * AND IT TAKES TIME (A3). What is shipped leaves the origin now and arrives at the destination
 * `transitPeriods` later. In between it is ON THE SHIPPER'S BOOK, at a place of its own, which is
 * A3.a's working capital: a shipper that has paid for a cargo and not yet got it is short of both.
 * The kernel's corporate actions deliver it; nothing teleports and nothing is in two places.
 *
 * WHAT THIS MODULE DOES NOT DO is decide for anybody. Which shipper wants room and what it will pay
 * is read from that party's own view (Clearing A3), and a carrier's reservation is what the voyage
 * costs IT — below which sailing is worse than staying in port.
 */
import type { CurrencyCode, InstrumentId, PartyId, RegionId } from '../../core/ids.js';
import { instrumentId, marketId, partyId } from '../../core/ids.js';
import { FIRM } from '../../registry/profiles.js';
import { add, atMost, div, finite, material, mul, sub, sum, zeroIfNone } from '../../core/num.js';
import { asQty, downTick } from '../../core/tick.js';
import { none } from '../../core/option.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { conditionsFor, WIND } from '../../registry/environment.js';
import {
  goodId,
  isGoodTerms,
  plantHeld,
  vintagesHeld,
  wearPerPlantUnit,
} from '../../registry/physical.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { routeParam, VESSEL, type CarrierDecl, type RouteDecl } from './data.js';

export { VESSEL, VESSEL_KIND, drawCarriers, freightMarket, freightVenue } from './data.js';
export type { CarrierDecl, RouteDecl } from './data.js';

/** A3: where a cargo IS while it is neither here nor there. A place, with a name, that it is at. */
export const inTransit = (subUnit: string, from: RegionId, to: RegionId): InstrumentId =>
  instrumentId(`good.${subUnit}.transit.${from}.${to}`);

export const FREIGHT_SESSION = 'freight.session';
export const FREIGHT_REFUSED = 'freight.refused';

/**
 * B2, B4: HOW MUCH A CARRIER CAN MOVE ON THIS ROUTE THIS PERIOD — its hulls times what a hull
 * carries, less what the weather takes off it. A storm does not sink the fleet to move the price:
 * it takes the hulls that were in it (`capital.weathered`, one event several consequences), and
 * what is left is what is left. This reads the standing state for the same reason a crop does:
 * a route nobody can sail is a route with no capacity, continuously and with no threshold.
 */
function capacityOf(
  ctx: MechanismContext,
  view: ParticipantView,
  route: RouteDecl,
  perVessel: number,
): number {
  const hulls = plantHeld(vintagesHeld(view, ctx.calendar.startOf(ctx.period)), VESSEL);
  if (hulls <= 0) return 0;
  // B4: what can sail this week. `exp(-(wind / standsWind) ^ hardness)` again, and the same
  // argument: positive at every wind, never one, nothing happens AT any level (Law 6).
  const wind = conditionsFor(ctx, route.from, [WIND]);
  const sails = finite(
    Math.exp(
      -Math.pow(
        wind / ctx.params.ratio(routeParam(route.from, route.to, 'sailsIn')),
        ctx.params.ratio(routeParam(route.from, route.to, 'sailsHardness')),
      ),
    ),
    'what can sail',
  );
  return downTick(mul(mul(hulls, perVessel, 'what its hulls carry'), sails, 'what the weather lets'));
}

/**
 * C1, C3, D1: THE SESSION. Carriers post the room they have at what the voyage costs them; shippers
 * post what they want moved at what moving it is worth to them, which is the gap between the two
 * places' prints less what they will pay to close it. One solver, one print, like every market.
 */
function sail(
  ctx: MechanismContext,
  route: RouteDecl,
  carriers: readonly CarrierDecl[],
  ccy: CurrencyCode,
  said: Map<string, Record<string, unknown>>,
): void {
  const orders: Order[] = [];
  for (const c of carriers) {
    if (c.region !== route.from) continue;
    const party = ctx.parties.get(c.carrier as PartyId);
    if (!party.status.alive) continue;
    const view = ctx.participant(party.id);
    const perVessel = ctx.params.ratio(routeParam(route.from, route.to, 'perVessel'));
    const room = capacityOf(ctx, view, route, perVessel);
    if (room <= 0) continue;
    // B3: what the voyage costs IT — the wear on the hulls it uses, over what they carry, times
    // its own crews' scale. Two carriers on one route do not cost the same, which is why one is
    // marginal and the other is not (A3.a, Seed B4).
    const wear = wearPerPlantUnit(vintagesHeld(view, ctx.calendar.startOf(ctx.period)), VESSEL);
    if (!wear.some) continue;
    const perUnit = mul(
      div(wear.value, perVessel, 'the hull a unit uses for the voyage'),
      c.crewScale,
      'what this carrier costs to run',
    );
    orders.push({ party: party.id, side: 'sell', price: perUnit, qty: asQty(room) });
  }
  orders.push(...shippers(ctx, route));
  if (orders.length === 0) return;
  const outcome = clear(orders, 'proRata', 'sellersCompete');
  said.set(`${route.from}|${route.to}`, {
    from: route.from,
    to: route.to,
    outcome: outcome.kind,
    shippers: orders.filter((o) => o.side === 'buy').length,
    carriers: orders.filter((o) => o.side === 'sell').length,
    ...(isCleared(outcome) ? { rate: outcome.price, moved: outcome.volume } : {}),
  });
  if (!isCleared(outcome)) return;
  load(ctx, route, outcome.fills, outcome.price, ccy);
}

/**
 * C1.a, C3: WHY A SHIPPER WANTS ROOM, and it is never a series. It is the gap between what the
 * thing fetches where it is going and what it fetches where it is — read off two prints, both
 * public and both already made (Law 19, Law 3) — and what it will pay is that gap, because above it
 * the voyage is worse than selling at home.
 */
function shippers(ctx: MechanismContext, route: RouteDecl): readonly Order[] {
  const out: Order[] = [];
  for (const p of ctx.parties.alive()) {
    if (p.region !== route.from) continue;
    const view = ctx.participant(p.id);
    for (const h of view.holdings()) {
      const i = ctx.instruments.get(h.instrument);
      if (!i.status.live || !isGoodTerms(i.terms) || i.terms.region !== route.from) continue;
      const there = goodId(i.terms.subUnit, route.to);
      if (!ctx.instruments.has(there)) continue;
      const here = view.print(i.id);
      const away = view.print(there);
      if (!here.some || !away.some) continue;
      const gap = sub(away.value.price, here.value.price, 'what the voyage is worth a unit');
      if (gap <= 0) continue;
      const units = view.free(i.id);
      if (units <= 0) continue;
      out.push({ party: p.id, side: 'buy', price: gap, qty: asQty(units) });
    }
  }
  return out;
}

/**
 * A3, A3.a, D6: WHAT WAS BOUGHT IS LOADED. The freight is paid to the named carrier and the cargo
 * leaves the origin for a place of its own — in transit, on the shipper's own book, which is what
 * ties its working capital up for the voyage (A3.a). Capacity rations quantity and nothing else
 * does: a shipper turned away is journaled with what it wanted (D6), and its units stay where they
 * are, which is what being unable to ship means.
 */
function load(
  ctx: MechanismContext,
  route: RouteDecl,
  fills: readonly { readonly party: PartyId; readonly side: string; readonly qty: number }[],
  rate: number,
  ccy: CurrencyCode,
): void {
  const room = fills.filter((f) => f.side === 'sell' && f.qty > 0).map((f) => ({ ...f }));
  let at = 0;
  for (const shipper of fills) {
    if (shipper.side !== 'buy' || shipper.qty <= 0) continue;
    let want = shipper.qty;
    while (want > 0 && at < room.length) {
      const carrier = room[at];
      if (carrier === undefined) break;
      // Law 6: a carrier cannot take more than it has room for and a shipper cannot ship more
      // than it wanted. Arithmetic impossibility on both sides, named at the site.
      const moved = atMost(carrier.qty, want, 'a carrier has only the room it has');
      const paid = ctx.registry.payable(ccy, mul(moved, rate, 'the freight on what it shipped'));
      if (paid > 0 && cargo(ctx, route, shipper.party, carrier.party, moved, paid, ccy)) {
        want = sub(want, moved, 'what it still wants moved');
        room[at] = { ...carrier, qty: sub(carrier.qty, moved, 'the room it has left') };
      } else {
        at += 1;
      }
      if (room[at] !== undefined && zeroIfNone(room[at]?.qty) <= 0) at += 1;
    }
  }
}

/** D2, Law 5: the cargo leaves and the freight is paid, in ONE instruction with both sides named. */
function cargo(
  ctx: MechanismContext,
  route: RouteDecl,
  shipper: PartyId,
  carrier: PartyId,
  units: number,
  paid: number,
  ccy: CurrencyCode,
): boolean {
  const here = ctx.parties.get(shipper).region;
  if (here !== route.from) return false;
  let shipped = 0;
  for (const h of ctx.register.holdingsOf(shipper)) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isGoodTerms(i.terms) || i.terms.region !== route.from) continue;
    const transit = inTransit(i.terms.subUnit, route.from, route.to);
    if (!ctx.instruments.has(transit)) continue;
    const free = ctx.register.free(shipper, i.id);
    // Law 6: it cannot put aboard what it does not hold, nor more than it bought room for.
    const left = sub(units, shipped, 'what is still to go aboard');
    const take = ctx.registry.deliverable(i.unit, atMost(free, left, 'it ships what it holds'));
    if (take <= 0) continue;
    const r = ctx.settle({
      legs: [
        { kind: 'destroy', party: shipper, instrument: i.id, qty: asQty(take), why: 'consumed', fromCell: none() },
        {
          kind: 'create',
          party: shipper,
          instrument: transit,
          qty: asQty(take),
          costPerUnit: div(add(paid, 0, 'the freight'), units, 'what the voyage added to a unit'),
          toCell: none(),
        },
        {
          kind: 'money',
          from: ctx.accountOf(shipper, ccy),
          to: ctx.accountOf(carrier, ccy),
          ccy,
          amount: ctx.registry.payable(ccy, mul(paid, div(take, units, 'its share of the cargo'), 'its share of the freight')),
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'trade',
      reason: `${shipper} ships ${take} of ${i.terms.subUnit} to ${route.to} with ${carrier}`,
    });
    if (r.outcome !== 'settled') continue;
    shipped = add(shipped, take, 'what has gone aboard');
    ctx.record(
      'freight.loaded',
      [String(shipper), String(carrier)],
      { shipper, carrier, from: route.from, to: route.to, good: i.terms.subUnit, units: take, paid },
      true,
    );
    if (shipped >= units) break;
  }
  return shipped > 0;
}

/**
 * A3, E3: WHAT ARRIVES. After the transit, the cargo is at the destination and the units that were
 * in transit are not. It is a reseat of where the thing is and never a second creation: what left
 * the origin is what arrives, at what it cost including the voyage (D2), and every unit in transit
 * has an owner the whole time.
 */
function arrive(ctx: MechanismContext, routes: readonly RouteDecl[]): void {
  for (const route of routes) {
    for (const h of ctx.register.allHoldings()) {
      const i = ctx.instruments.get(h.instrument);
      if (!i.status.live || !isGoodTerms(i.terms)) continue;
      const subUnit = i.terms.subUnit;
      if (i.id !== inTransit(subUnit, route.from, route.to)) continue;
      const there = goodId(subUnit, route.to);
      if (!ctx.instruments.has(there)) continue;
      const transitPeriods = ctx.params.periods(routeParam(route.from, route.to, 'transit'));
      const due = ctx.register
        .holdingsOf(h.holder)
        .find((x) => x.instrument === i.id)
        ?.lots.filter((lot) => ctx.period - lot.acquired >= transitPeriods);
      if (due === undefined || due.length === 0) continue;
      const units = sum(due.map((lot) => lot.qty)).value;
      if (!material(units, due.length + 1, units) || units <= 0) continue;
      const cost = sum(due.map((lot) => mul(lot.qty, lot.basisPerUnit, 'what it cost'))).value;
      const r = ctx.settle({
        legs: [
          { kind: 'destroy', party: h.holder, instrument: i.id, qty: asQty(units), why: 'consumed', fromCell: none() },
          {
            kind: 'create',
            party: h.holder,
            instrument: there,
            qty: asQty(units),
            costPerUnit: div(cost, units, 'what a unit cost delivered'),
            toCell: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${units} of ${subUnit} arrives in ${route.to}`,
      });
      if (r.outcome !== 'settled') continue;
      ctx.record(
        'freight.arrived',
        [String(h.holder)],
        { holder: h.holder, from: route.from, to: route.to, good: subUnit, units },
        true,
      );
    }
  }
}

export function freight(
  carriers: readonly CarrierDecl[],
  routes: readonly RouteDecl[],
): SystemModule {
  return {
    id: 'freight',
    spec: 'Freight A, B, C, D, E',
    // A4.b: hulls are plant, so the kind has to be registered; and what they carry is a good.
    requires: ['goods', 'capital-programme'],
    instrumentKinds: [],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: routes.flatMap((r) => [
      {
        id: routeParam(r.from, r.to, 'transit'),
        value: r.transitPeriods,
        unit: 'periods',
        dimension: 'periods' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: `Freight A3, A3.a: periods a cargo spends between ${r.from} and ${r.to}. It is on the shipper's own book the whole time, which is what ties its working capital up for the voyage.`,
      },
      {
        id: routeParam(r.from, r.to, 'perVessel'),
        value: r.unitsPerVesselPerPeriod,
        unit: 'units of a good per vessel per period',
        dimension: 'ratio' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: `Freight A4, B2: what one hull moves on this leg in a period. Capacity on one route is not capacity on another, which is why a blocked leg stays blocked. ${r.why}`,
      },
      {
        id: routeParam(r.from, r.to, 'sailsIn'),
        value: r.sailsIn,
        unit: "multiples of an ordinary period's wind",
        dimension: 'ratio' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: `Freight B4: what this PASSAGE is sailable in. It is about the strait and not the hull. Not a threshold — what sails is exp(-(wind / this) ^ hardness), positive at every wind and never one (Law 6).`,
      },
      {
        id: routeParam(r.from, r.to, 'sailsHardness'),
        value: r.sailsHardness,
        unit: 'exponent',
        dimension: 'ratio' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: 'Freight B4: how sharply the leg closes above what it is sailable in. A blocked route is a real loss of units moved and never a rate anybody set.',
      },
    ]),
    participants: [],
    families: [],
    seed(ctx: SeedContext): void {
      // B1, Small-Business Pools A6.b: A CARRIER IS A FIRM. It is a named party that owns plant,
      // employs people, banks somewhere and can fail — which is what a firm is — and giving it a
      // party kind of its own would be a second kind that behaves identically, so that every rule
      // written for one would have to be written again for the other (Law 15, Law 4). What makes
      // it a carrier is its BUSINESS: hulls, and a route to sail them on.
      for (const c of carriers) {
        const id = partyId(c.carrier);
        if (ctx.parties.has(id) || !ctx.registry.regions.has(c.region)) continue;
        ctx.parties.add({
          id,
          kind: FIRM,
          region: c.region,
          name: c.name,
          bank: partyId(c.bank),
          status: { alive: true },
          representation: 'named',
        });
      }
    },
    phases: [
      {
        name: 'freight.arrive',
        spec: 'Freight A3 Freight A3.a Freight E3',
        // At the top of the period: a cargo that has finished its voyage is at the destination
        // before anybody decides what to do with what is there (Clearing F1).
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          arrive(ctx, routes);
        },
      },
      {
        name: 'freight.session',
        spec: 'Freight C1 Freight C3 Freight D1 Freight D6',
        // BEFORE the goods markets it feeds: what is shipped this period is decided against the
        // two prints already made, and a shipper that got no room knows before it posts (D6).
        cycle: 1,
        anchor: { before: 'markets' },
        run: (ctx: MechanismContext): void => {
          const said = new Map<string, Record<string, unknown>>();
          for (const route of routes) {
            const region = ctx.registry.regions.get(route.from);
            if (region === undefined) continue;
            sail(ctx, route, carriers, region.ccy, said);
          }
          ctx.record(FREIGHT_SESSION, [...said.keys()], { byRoute: Object.fromEntries(said) }, true);
        },
      },
    ],
  };
}

export const transitMarket = (subUnit: string, from: RegionId, to: RegionId): string =>
  String(marketId(`mkt.transit.${subUnit}.${from}.${to}`));
