/**
 * FREIGHT: moving a thing costs money and takes time, and somebody with a finite number of hulls
 * decides whether to move it.
 *
 * @spec Freight A1 Freight A2 Freight A3 Freight A3.a Freight A4 Freight B1 Freight B2 Freight B2.a Freight B3 Freight B4 Freight C1 Freight C1.a Freight C3 Freight D1 Freight D2 Freight D3 Freight D4 Freight D6 Freight E1 Freight E2 Freight E3 Clearing A2 Clearing A3 Law 3 Law 5 Law 6 Law 19
 *
 * A world where a thing moves for nothing has ONE PRICE EVERYWHERE by construction: nothing is
 * dearer where it is short, there is no basis between two places, and there is nothing for anybody
 * to close. So room is a real thing with a real limit — a named carrier's hulls, which are plant
 * with a life (A4, Capital Programme A4) — and what it costs to move a tonne is CLEARED from
 * shippers wanting room meeting carriers with room to sell (D1). No freight rate is declared.
 *
 * NOTHING ABOUT A LEG IS DECLARED EITHER (13c.1). Every ordered pair of places a hull can get
 * between is read off the map, with the kilometres it covers and the days it takes; `RouteDecl`
 * and its four numbers are gone and each names its read (Law 19):
 *
 *   transitPeriods          -> the voyage's own progress against the weather it met
 *   unitsPerVesselPerPeriod -> what a hull holds, times the hulls that are free
 *   sailsIn, sailsHardness  -> the ground the voyage is actually crossing, tile by tile
 *
 * AND IT TAKES TIME (A3). A cargo leaves the origin now and is ON THE SHIPPER'S BOOK the whole way,
 * at a place of its own — A3.a's working capital: a shipper that has paid for a cargo and not yet
 * got it is short of both. How long that is, is not a number: each period the voyage comes as far
 * as the weather AT THE PLACE IT IS IN let it, so a gale holds up the ships in that sea area and no
 * others and leaves the next session short of hulls. That is half of B2.a — freight violent and
 * inelastic in the short run — as a mechanism and not a claim.
 *
 * THE OTHER HALF IS NOT BUILT, and this said it was (item 0c): *"takes a share of the hulls caught
 * in it WITH THE CARGO ABOARD on two named books"*. Nothing here emits `act: 'lose'` — the leg
 * exists, settlement handles it, and no module in this world has ever written one — so a storm
 * delays a voyage and has never sunk one. E3 is what that clause is, and it is item 21's.
 *
 * CAPACITY IS NOT A NUMBER ANYWHERE. What a carrier can offer is the hulls it has FREE: a hull on a
 * voyage is liened, and the register already refuses to move encumbered units, so "a hull cannot be
 * in two trades at once" is enforced by a store rather than remembered by a rule (E2, Law 12).
 *
 * WHAT THIS MODULE DOES NOT DO is decide for anybody. Which shipper wants room and what it will pay
 * is read from that party's own view (Clearing A3), and a carrier's reservation is what the voyage
 * costs IT — below which sailing is worse than staying in port.
 */
import type { CurrencyCode, InstrumentId, PartyId, RegionId } from '../../core/ids.js';
import { costOfDraw } from '../../register/register.js';
import { instrumentId, partyId } from '../../core/ids.js';
import { FIRM } from '../../registry/profiles.js';
import { atMost, div, finite, mul, sum, zeroIfNone } from '../../core/num.js';
import { NO_QTY, type Qty, addQty, asQty, downTick, subQty } from '../../core/tick.js';
import {
  type PerPiece,
  asCash,
  asPerPiece,
  asRatio,
  heldAsMoney,
  minus,
  over,
  plus,
  pricedAt,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import { none } from '../../core/option.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { WIND, conditionsFor } from '../../registry/environment.js';
import {
  type Path,
  daysAcross,
  legKey,
  legsBetween,
  placeAt,
  standsIn,
} from '../../registry/geography.js';
import {
  goodId,
  isGoodTerms,
  type GoodTerms,
  vintagesHeld,
  wearPerPlantUnit,
  windHardnessParam,
} from '../../registry/physical.js';
import { tileReached, type Voyage } from '../../register/voyages.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  HOLD_UNITS,
  VESSEL,
  VESSEL_HOLD,
  VESSEL_WEAR_PER_UNIT_KM,
  WEAR_PER_UNIT_KM,
  type CarrierDecl,
} from './data.js';

export { VESSEL, VESSEL_KIND, drawCarriers, freightMarket, freightVenue } from './data.js';
export type { CarrierDecl } from './data.js';

/** A3: where a cargo IS while it is neither here nor there. A place, with a name, that it is at. */
export const inTransit = (subUnit: string, from: RegionId, to: RegionId): InstrumentId =>
  instrumentId(`good.${subUnit}.transit.${from}.${to}`);

export const FREIGHT_SESSION = 'freight.session';
export const FREIGHT_REFUSED = 'freight.refused';
export const FREIGHT_SAIL = 'freight.sail';

/** The legs this world has for hulls: every pair of places one can get between, off the map. */
function legs(ctx: MechanismContext): ReadonlyMap<string, Path> {
  return legsBetween(ctx.registry.geography, ctx.params, VESSEL, [...ctx.registry.regions.keys()]);
}

/**
 * B2, E2: WHAT A CARRIER CAN OFFER — the hulls it has FREE, times what a hull holds. A hull on a
 * voyage is liened, so it is not here; that is the whole of "capacity on one route is not capacity
 * on another" and of "no capacity without a carrier that owns it", and there is no number anywhere
 * that says how much a route can take (A4).
 */
function roomOf(ctx: MechanismContext, view: ParticipantView): Qty {
  let free = NO_QTY;
  for (const v of vintagesHeld(view, ctx.calendar.startOf(ctx.period))) {
    if (v.capitalKind !== VESSEL) continue;
    free = addQty(free, view.free(instrumentId(v.instrument)), 'the hulls it has free');
  }
  if (free <= 0) return NO_QTY;
  return downTick(
    scale(free, asRatio(ctx.params.count(HOLD_UNITS), 'what one hull holds'), 'what its free hulls hold'),
  );
}

/**
 * B3: WHAT THE VOYAGE COSTS IT, per unit carried. The hull is used up per unit-KILOMETRE and the
 * crew is paid per DAY, so a long leg is dearer twice over and a slow one dearer again — all of it
 * out of the leg the map gave, and none of it a rate (Law 3).
 */
function costOf(ctx: MechanismContext, view: ParticipantView, c: CarrierDecl, leg: Path): PerPiece {
  const wear = wearPerPlantUnit(vintagesHeld(view, ctx.calendar.startOf(ctx.period)), VESSEL);
  if (!wear.some) return asPerPiece(0, 'a carrier with no hull wears none of one');
  const hold = asRatio(ctx.params.count(HOLD_UNITS), 'what one hull holds');
  // Both halves are a LEVEL — what a unit carried costs — so they add as levels and the carrier's
  // own scale multiplies through as the pure number it is.
  const perKm = scale(
    asPerPiece(ctx.params.ratio(WEAR_PER_UNIT_KM), 'the hull a unit uses over a kilometre'),
    asRatio(leg.km, 'the kilometres of it'),
    'the hull a unit uses on the way',
  );
  const crew = scale(
    over(wear.value, hold, 'the hull a unit uses standing'),
    asRatio(leg.days, 'the days it takes'),
    'the days it takes',
  );
  return scale(
    plus(perKm, crew, 'what the voyage costs a unit'),
    asRatio(c.crewScale, 'what this carrier runs at'),
    'this carrier',
  );
}

/**
 * C1.a, C3: WHY A SHIPPER WANTS ROOM, and it is never a series. It is the gap between what the
 * thing fetches where it is going and what it fetches where it is — read off two prints, both
 * public and both already made (Law 19, Law 3) — and what it will pay is that gap, because above it
 * the voyage is worse than selling at home.
 *
 * WHAT A PLACE HAS TO SHIP is read ONCE per origin and offered on every leg out of it (Law 18:
 * the traversal is free, the mechanism is not). Walking every party for every destination cost
 * thirty seconds a period in a world with seven places and forty-two legs, and it was the same walk
 * forty-two times: the holdings and the price at home do not depend on where the thing is going.
 */
interface Shippable {
  readonly party: PartyId;
  readonly instrument: InstrumentId;
  readonly subUnit: string;
  readonly units: number;
  readonly here: PerPiece;
}

function toShip(ctx: MechanismContext, from: RegionId): readonly Shippable[] {
  const out: Shippable[] = [];
  for (const p of ctx.parties.alive()) {
    if (p.region !== from) continue;
    const view = ctx.participant(p.id);
    for (const h of view.holdings()) {
      const i = ctx.instruments.get(h.instrument);
      if (!i.status.live || !isGoodTerms(i.terms) || i.terms.region !== from) continue;
      // 13c.2, A3: a hold takes what can be loaded. A service is made where it is bought and there
      // is nothing to put aboard, which is why its price is local and no voyage can close a gap in
      // it — the technology of the thing says so, and nothing here decides it.
      if (!i.terms.portable) continue;
      const here = view.print(i.id);
      if (!here.some) continue;
      const units = view.free(i.id);
      if (units <= 0) continue;
      out.push({
        party: p.id,
        instrument: i.id,
        subUnit: i.terms.subUnit,
        units,
        here: here.value.price,
      });
    }
  }
  return out;
}

/** What that becomes on one leg: the gap to where it is going, which is what a shipper will pay. */
function shippers(
  ctx: MechanismContext,
  have: readonly Shippable[],
  to: RegionId,
): readonly Order[] {
  const out: Order[] = [];
  for (const s of have) {
    const there = goodId(s.subUnit, to);
    if (!ctx.instruments.has(there)) continue;
    const away = ctx.participant(s.party).print(there);
    if (!away.some) continue;
    const gap = minus(away.value.price, s.here, 'what the voyage is worth a unit');
    if (gap <= 0) continue;
    out.push({ party: s.party, side: 'buy', price: gap, qty: asQty(s.units) });
  }
  return out;
}

/**
 * A3, C6: THE PLACE A CARGO IS WHILE IT IS NEITHER HERE NOR THERE, opened once per good and leg.
 *
 * `cargo()` moves a shipment out of the origin line and into one of these; `arrive()` moves it out
 * again into the destination line, carrying its own basis. Without them the destroy-and-create that
 * loading IS had nowhere to put the cargo, so every voyage in this world stopped at the `has`
 * check and nothing was ever loaded — which is why `freight.loaded` had never fired.
 *
 * IT IS THE GOOD'S OWN TERMS, read off the line it came from (Law 19) rather than rebuilt here,
 * with ONE fact changed: it cannot be loaded. A cargo at sea is not cargo a hull can take on, and
 * `portable: false` is that fact in the one place every reader already asks it — the shipper's own
 * offer, a merchant's book, a futures line's deliverable. The region it carries is the one it is
 * CONSIGNED to, because that is what it becomes when it lands, and its money is the SHIPPER'S: what
 * a lot of it cost is what the cargo cost plus the freight, and both were paid at the origin.
 *
 * Nobody trades it (`market: none()`), so it is carried at cost like every other unpriced line, and
 * it spoils at sea because it is the same instrument KIND as the good and `perish` walks holdings.
 */
function openTransit(ctx: SeedContext): void {
  const places = [...ctx.registry.regions.keys()];
  const paths = legsBetween(ctx.registry.geography, ctx.params, VESSEL, places);
  for (const from of places) {
    for (const to of places) {
      if (from === to || !paths.has(legKey(from, to))) continue;
      for (const i of ctx.instruments.all()) {
        const here = i.terms;
        if (!isGoodTerms(here) || here.region !== from || !here.portable) continue;
        const id = inTransit(here.subUnit, from, to);
        if (ctx.instruments.has(id)) continue;
        const terms: GoodTerms = { ...here, region: to, portable: false };
        ctx.instruments.add({
          id,
          kind: i.kind,
          // A1: nobody issued a tonne of grain at sea either.
          issuer: none(),
          ccy: ctx.registry.currencyOf(from),
          terms,
          market: none(),
        });
      }
    }
  }
}

/** The keenest price on one side of a session, or nothing because nobody stood on it. */
function best(orders: readonly Order[], side: 'buy' | 'sell'): number | undefined {
  let out: number | undefined;
  for (const o of orders) {
    if (o.side !== side || typeof o.price !== 'number') continue;
    if (out === undefined || (side === 'buy' ? o.price > out : o.price < out)) out = o.price;
  }
  return out;
}

/** C1, D1: the session on one leg. One solver, one print, like every other market. */
function session(
  ctx: MechanismContext,
  carriers: readonly CarrierDecl[],
  from: RegionId,
  to: RegionId,
  leg: Path,
  have: readonly Shippable[],
  ccy: CurrencyCode,
  said: Map<string, Record<string, unknown>>,
): void {
  const orders: Order[] = [];
  const room = new Map<PartyId, { readonly hull: InstrumentId; readonly hulls: number }>();
  for (const c of carriers) {
    if (c.region !== from) continue;
    const party = ctx.parties.get(c.carrier as PartyId);
    if (!party.status.alive) continue;
    const view = ctx.participant(party.id);
    const offered = roomOf(ctx, view);
    if (offered <= 0) continue;
    const perUnit = costOf(ctx, view, c, leg);
    if (perUnit <= 0) continue;
    const held = vintagesHeld(view, ctx.calendar.startOf(ctx.period)).find(
      (v) => v.capitalKind === VESSEL && view.free(instrumentId(v.instrument)) > 0,
    );
    if (held === undefined) continue;
    room.set(party.id, {
      hull: instrumentId(held.instrument),
      hulls: view.free(instrumentId(held.instrument)),
    });
    orders.push({ party: party.id, side: 'sell', price: perUnit, qty: asQty(offered) });
  }
  orders.push(...shippers(ctx, have, to));
  if (orders.length === 0) return;
  const outcome = clear(orders, 'proRata', 'sellersCompete');
  // D1, Part II: WHAT THE TWO SIDES SAID, not only how many said it. A leg that does not clear
  // because the gap between two places is under what the voyage costs is a different world from one
  // where nobody offered, and `noOverlap` alone cannot tell them apart — which is the first thing
  // anybody reading a world with no freight in it asks.
  const bid = best(orders, 'buy');
  const ask = best(orders, 'sell');
  said.set(legKey(from, to), {
    from,
    to,
    km: leg.km,
    days: leg.days,
    outcome: outcome.kind,
    shippers: orders.filter((o) => o.side === 'buy').length,
    carriers: orders.filter((o) => o.side === 'sell').length,
    ...(bid === undefined ? {} : { bid }),
    ...(ask === undefined ? {} : { ask }),
    ...(isCleared(outcome) ? { rate: outcome.price, moved: outcome.volume } : {}),
  });
  if (!isCleared(outcome)) return;
  load(ctx, from, to, leg, room, outcome.fills, outcome.price, ccy);
}

/**
 * A3, A3.a, D6: WHAT WAS BOUGHT IS LOADED. The freight is paid to the named carrier, the cargo
 * leaves the origin for a place of its own, and the hulls carrying it are bound to the voyage —
 * all in one instruction with both sides of every leg named (Law 5).
 */
function load(
  ctx: MechanismContext,
  from: RegionId,
  to: RegionId,
  leg: Path,
  room: ReadonlyMap<PartyId, { readonly hull: InstrumentId; readonly hulls: number }>,
  fills: readonly { readonly party: PartyId; readonly side: string; readonly qty: Qty }[],
  rate: PerPiece,
  ccy: CurrencyCode,
): void {
  const carriers = fills.filter((f) => f.side === 'sell' && f.qty > 0).map((f) => ({ ...f }));
  let at = 0;
  for (const shipper of fills) {
    if (shipper.side !== 'buy' || shipper.qty <= 0) continue;
    let want: Qty = shipper.qty;
    while (want > 0 && at < carriers.length) {
      const carrier = carriers[at];
      if (carrier === undefined) break;
      // Law 6: a carrier cannot take more than it has room for and a shipper cannot ship more
      // than it wanted. Arithmetic impossibility on both sides, named at the site.
      const moved = atMost(carrier.qty, want, 'a carrier has only the room it has');
      const paid = ctx.registry.payable(valueAt(rate, moved, 'the freight on what it shipped'));
      if (paid > 0 && cargo(ctx, from, to, leg, room, shipper.party, carrier.party, moved, paid, ccy)) {
        want = subQty(want, moved, 'what it still wants moved');
        carriers[at] = { ...carrier, qty: subQty(carrier.qty, moved, 'the room it has left') };
      } else {
        at += 1;
      }
      if (carriers[at] !== undefined && zeroIfNone(carriers[at]?.qty) <= 0) at += 1;
    }
  }
}

/** D2, Law 5: the cargo leaves, the freight is paid, and the voyage opens, in ONE instruction. */
function cargo(
  ctx: MechanismContext,
  from: RegionId,
  to: RegionId,
  leg: Path,
  room: ReadonlyMap<PartyId, { readonly hull: InstrumentId; readonly hulls: number }>,
  shipper: PartyId,
  carrier: PartyId,
  units: Qty,
  paid: Qty,
  ccy: CurrencyCode,
): boolean {
  if (ctx.parties.get(shipper).region !== from) return false;
  const hulls = room.get(carrier);
  if (hulls === undefined) return false;
  let shipped = NO_QTY;
  for (const h of ctx.register.holdingsOf(shipper)) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isGoodTerms(i.terms) || i.terms.region !== from) continue;
    const transit = inTransit(i.terms.subUnit, from, to);
    if (!ctx.instruments.has(transit)) continue;
    const free = ctx.register.free(shipper, i.id);
    const left = minus(units, shipped, 'what is still to go aboard');
    const take = ctx.registry.deliverable(atMost(free, left, 'it ships what it holds'));
    if (take <= 0) continue;
    // E2: the hulls this cargo needs, and never more than the carrier has free. One hull's worth
    // of cargo takes one hull, which is what a hold IS.
    // Register D5.a, 11.0a: THE HULLS IT HAS FREE NOW, read off the register at the pledge and not
    // off the count taken when the session opened — a carrier loading two cargoes in one session
    // pledged the same hull twice, and the second pledge was refused. Nothing shipped abroad in
    // an opens run until the small firms bid for flour across the water, so it never showed.
    const freeHulls = ctx.register.free(carrier, hulls.hull);
    const need = asQty(
      atMost(
        Math.ceil(take / ctx.params.count(HOLD_UNITS)),
        freeHulls,
        'a carrier commits the hulls it has',
      ),
    );
    if (need <= 0) continue;
    const share = ctx.registry.payable(scale(
        heldAsMoney(paid, 'the freight on the whole cargo'),
        ratioOf(take, units, 'its share'),
        'its freight',
      ),
    );
    /**
     * A-68, D2: WHAT THE CARGO COST, PLUS THE FREIGHT — and it used to be the freight alone.
     *
     * The destroy leg takes the cargo off the shipper's book at its whole carrying value; the
     * create leg puts it back on at `costPerUnit x take`. With only the freight in that second
     * number the shipper's equity fell by everything the cargo had cost to buy or to make, at the
     * moment it was loaded. The tell was `add(share, 0, 'the freight')` — a sum of one term and a
     * zero, where the second term is what the cargo cost.
     *
     * The total over a completed voyage was right (`-C` at loading, `+P - F` at sale), which is why
     * no conservation family caught it. What was wrong is the module's own subject: A3.a's working
     * capital. A shipper mid-voyage showed an equity hole the size of its cargo, which `failedWhy`'s
     * solvency trigger reads and which could kill a merchant on the water, and the arrival booked a
     * profit equal to the cargo's cost that no trade had produced. `arrive()` was always right —
     * it carries the transit lot's own basis forward — so the error was entirely here, under a
     * docstring that said the opposite: *"the destination at what it cost INCLUDING the voyage"*.
     *
     * `costOfDraw` is exported from the register for exactly this read (Law 19).
     */
    const held = ctx.register.holding(shipper, i.id);
    const cost = held.some
      ? costOfDraw(held.value.lots, asQty(take))
      : asCash(0, 'a shipper with no lots of it has paid nothing for it');
    const r = ctx.settle({
      legs: [
        { kind: 'destroy', party: shipper, instrument: i.id, qty: asQty(take), why: 'consumed'},
        {
          kind: 'create',
          party: shipper,
          instrument: transit,
          qty: asQty(take),
          costPerUnit: pricedAt(
            plus(cost, heldAsMoney(share, 'the freight'), 'what it cost and what the voyage cost'),
            take,
            'what a unit cost delivered',
          ),
        },
        {
          kind: 'money',
          from: ctx.accountOf(shipper, ccy),
          to: ctx.accountOf(carrier, ccy),
          ccy,
          amount: share,
        },
        {
          kind: 'voyage',
          act: 'sail',
          carrier,
          shipper,
          by: VESSEL,
          hullInstrument: hulls.hull,
          hulls: need,
          tiles: leg.tiles,
          km: leg.km,
          cargo: transit,
          qty: asQty(take),
          freight: share,
        },
      ],
      // Commodities Spot F1, 11.0a: a good becomes a good IN TRANSIT — units are made, not traded — and
      // settlement admits a `create` only under the cause that makes units. No cargo had ever been
      // loaded in an opens run until the small firms bid for flour abroad.
      cause: 'production',
      reason: `${shipper} ships ${take} of ${i.terms.subUnit} to ${to} with ${carrier}`,
    });
    if (r.outcome !== 'settled') continue;
    shipped = plus(shipped, take, 'what has gone aboard');
    ctx.record(
      'freight.loaded',
      [String(shipper), String(carrier)],
      { shipper, carrier, from, to, good: i.terms.subUnit, units: take, paid: share, km: leg.km },
      true,
    );
    if (shipped >= units) break;
  }
  return shipped > 0;
}

/**
 * B4, D4: WHAT THE WEATHER DID TO EVERY VOYAGE THIS PERIOD, at the place each one is actually in.
 *
 * The advance is `kmPerDay of the ground under it × the days in a period × what gets through`,
 * where what gets through is `exp(-(wind / what this ground stands) ^ hardness)` — the same
 * arithmetic a storm on land uses, positive at every wind and never one, so a voyage in a gale
 * crawls and never stops dead and nothing anywhere is clamped (Law 6).
 */
function sail(ctx: MechanismContext, hardness: number): void {
  const g = ctx.registry.geography;
  const said: Record<string, unknown>[] = [];
  for (const v of ctx.voyages.underWay()) {
    const tile = tileReached(v);
    const where = placeAt(g, tile);
    const wind = conditionsFor(ctx, where, [WIND]);
    const stands = standsIn(g, ctx.params, tile);
    const through = finite(Math.exp(-Math.pow(wind / stands, hardness)), 'what gets through');
    // How fast the ground under it lets a hull go, share-weighted over the tile's own mix —
    // you cross all of a tile, so a part-shallow one is slow (Law 15: no branch on terrain).
    const days = daysAcross(g, ctx.params, tile, VESSEL, 1);
    if (days === undefined) continue;
    const km = mul(
      mul(div(1, days, 'the km a day over this ground'), ctx.calendar.periodDays, 'a period of it'),
      through,
      'what the weather let it do',
    );
    if (km <= 0) continue;
    ctx.settle({
      legs: [{ kind: 'voyage', act: 'advance', voyage: v.id, km }],
      cause: 'corporateAction',
      reason: `voyage ${v.id} comes ${km} km in ${where}`,
    });
    said.push({ voyage: v.id, place: where, wind, through, km, of: v.km });
  }
  if (said.length > 0) ctx.record(FREIGHT_SAIL, [], { voyages: said }, true);
}



/**
 * A3, E3: WHAT ARRIVES. A voyage that has come as far as its path is long lands: the cargo is at
 * the destination at what it cost INCLUDING the voyage (D2), and the hulls are free again. It is a
 * reseat of where the thing is and never a second creation — what left the origin is what arrives,
 * and every unit of it had an owner the whole time.
 */
function arrive(ctx: MechanismContext): void {
  for (const v of ctx.voyages.underWay()) {
    if (v.kmTravelled < v.km || v.aboard <= 0) continue;
    const i = ctx.instruments.get(v.cargo);
    if (!isGoodTerms(i.terms)) continue;
    const to = destinationOf(v, ctx);
    const there = goodId(i.terms.subUnit, to);
    if (!ctx.instruments.has(there)) continue;
    const held = ctx.register.holdingsOf(v.shipper).find((x) => x.instrument === v.cargo);
    if (held === undefined) continue;
    const units = ctx.registry.deliverable(atMost(v.aboard, ctx.register.free(v.shipper, v.cargo), 'what it still has aboard'));
    if (units <= 0) continue;
    const cost = sum(held.lots.map((lot) => valueAt(lot.basisPerUnit, lot.qty, 'what it cost'))).value;
    const total = sum(held.lots.map((lot) => lot.qty)).value;
    const r = ctx.settle({
      legs: [
        { kind: 'destroy', party: v.shipper, instrument: v.cargo, qty: asQty(units), why: 'consumed'},
        {
          kind: 'create',
          party: v.shipper,
          instrument: there,
          qty: asQty(units),
          costPerUnit: pricedAt(cost, total, 'what a unit cost delivered'),
        },
        { kind: 'voyage', act: 'land', voyage: v.id },
      ],
      // Commodities Spot F1: and back the other way at the quay — the same transformation, landed.
      cause: 'production',
      reason: `${units} of ${i.terms.subUnit} arrives in ${to}`,
    });
    if (r.outcome !== 'settled') continue;
    ctx.record(
      'freight.arrived',
      [String(v.shipper)],
      { holder: v.shipper, to, good: i.terms.subUnit, units, km: v.km, took: ctx.period - v.departed },
      true,
    );
  }
}

/** Law 19: where a voyage is going is read off the last tile of its own path, never restated. */
function destinationOf(v: Voyage, ctx: MechanismContext): RegionId {
  const last = v.tiles[v.tiles.length - 1];
  if (last === undefined) throw new Error(`voyage ${v.id} has no path`);
  return placeAt(ctx.registry.geography, last) as RegionId;
}

export function freight(carriers: readonly CarrierDecl[]): SystemModule {
  return {
    id: 'freight',
    spec: 'Freight A, B, C, D, E',
    requires: ['goods', 'capital-programme'],
    instrumentKinds: [],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: HOLD_UNITS,
        value: VESSEL_HOLD,
        unit: 'units of a good one hull carries',
        dimension: 'count' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: 'Freight B2: what one hull holds. It is a fact about the ship and not about any leg, which is why it replaced a capacity declared per route (13c.1, Law 19).',
      },
      {
        id: WEAR_PER_UNIT_KM,
        value: VESSEL_WEAR_PER_UNIT_KM,
        unit: 'hulls used up per unit carried per kilometre',
        dimension: 'ratio' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: 'Freight B3: what carrying a unit a kilometre uses a hull up by. It is per KILOMETRE, so a long leg costs more because it IS longer and not because anybody said so.',
      },
    ],
    participants: [],
    families: [],
    seed(ctx: SeedContext): void {
      // B1, Small-Business Pools A6.b: A CARRIER IS A FIRM. It is a named party that owns plant,
      // employs people, banks somewhere and can fail — which is what a firm is — and giving it a
      // party kind of its own would be a second kind that behaves identically (Law 15, Law 4).
      for (const c of carriers) {
        const id = partyId(c.carrier);
        if (ctx.parties.has(id) || !ctx.registry.regions.has(c.region)) continue;
        ctx.parties.add({
          id,
          kind: FIRM,
          region: c.region,
          name: c.name,
          bank: partyId(c.bank),
          status: { alive: true, standing: 'good' },
          representation: 'named',
        });
      }
      openTransit(ctx);
    },
    phases: [
      {
        name: FREIGHT_SAIL,
        spec: 'Freight A3 Freight B4 Freight D4',
        anchor: { before: 'corporateActions' },
        // B4: a voyage stands in the weather, which the environment published (Clearing F1.a: a
        // phase says what it reads). No voyage had ever sailed in an opens run before 11.0a.
        reads: [{ kind: 'event', name: 'environment.state', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: FREIGHT_SAIL }],
        run: (ctx: MechanismContext): void => {
          sail(ctx, ctx.params.ratio(windHardnessParam(VESSEL)));
        },
      },
      {
        name: 'freight.arrive',
        spec: 'Freight A3 Freight A3.a Freight E3',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext): void => {
          arrive(ctx);
        },
      },
      {
        name: FREIGHT_SESSION,
        spec: 'Freight C1 Freight C3 Freight D1 Freight D6',
        anchor: { before: 'markets' },
        reads: [],
        writes: [{ kind: 'event', name: 'freight.session' }],
        run: (ctx: MechanismContext): void => {
          const said = new Map<string, Record<string, unknown>>();
          // Law 18: one walk of each origin, offered on every leg out of it.
          const have = new Map<RegionId, readonly Shippable[]>();
          for (const [key, leg] of legs(ctx)) {
            const [from, to] = key.split('|') as [RegionId, RegionId];
            if (!ctx.registry.regions.has(from) || !ctx.registry.regions.has(to)) continue;
            const mine = have.get(from) ?? toShip(ctx, from);
            have.set(from, mine);
            if (mine.length === 0) continue;
            session(ctx, carriers, from, to, leg, mine, ctx.registry.currencyOf(from), said);
          }
          ctx.record(FREIGHT_SESSION, [...said.keys()], { byLeg: Object.fromEntries(said) }, true);
        },
      },
    ],
  };
}


