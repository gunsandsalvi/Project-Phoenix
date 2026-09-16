/**
 * The capital programme: plant that produces, wears out and is replaced, bought from a named
 * producer and commissioned after it is built.
 *
 * @spec Capital Programme A1 Capital Programme A2 Capital Programme A3 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme A6.b Capital Programme C1 Capital Programme C2 Capital Programme C3 Capital Programme C4 Capital Programme D1 Capital Programme D2 Capital Programme D3 Capital Programme E1 Capital Programme E2 Capital Programme E3 Goods A2 Goods B1.a Goods B5 Firm Birth A2.a Commodities Spot F1 XI-4 Law 2 Law 15 Law 19
 *
 * WHAT THIS MODULE OWNS is the STOCK: the kinds of capital, the vintages, the schedule they wear
 * out on, the moment a good that was bought becomes plant that works, and the identity that says
 * neither can move without a leg saying why. What it does NOT own is the DECISION to invest: that
 * is the firm's, taken from its own view against its own cost of capital (B1, XI-4 joint two), and
 * it lives with the firm's other decisions because it is made of the same things they are.
 *
 * WHY A PURCHASE BECOMES PLANT AND NOT AN INPUT (A4.c). Whether a thing is plant is the BUYER's
 * question, and the answer here is read off the wire: what a firm BOUGHT of a capital good is
 * commissioned into plant when the build lag is up, and what it MADE of one is its own stock to
 * sell. A capital-goods producer holding its own output is therefore not investing in itself, and
 * nothing has to ask what industry anybody is in to know that (Law 15).
 *
 * WHY PLANT IS NEVER BORN FROM NOTHING (Firm Birth A2.a, Commodities Spot F1). Commissioning is one
 * instruction: the good is destroyed and the plant created in the same pass, at the same cost. So
 * every unit of plant in this world was made by somebody, sold by somebody and paid for in cash —
 * which is also why investment is demand now and capacity later (C1.a), employs the people who
 * built it (E2), and shows up in the producer's revenue rather than in a firm's own balance sheet
 * out of nowhere.
 */
import {
  asRatio,
  minus,
  pricedAt,
  scale,
  noCash,
  plus,
  valueAt,
  type Cash,
  type PerPiece,
} from '../../core/measure.js';
import type { Family, Violation } from '../../audit/audit.js';
import { downTick, negQty, asQty, subQty, type Qty } from '../../core/tick.js';
import { authorityIdFor, hectaresOf, hectaresUnder, landId } from '../../registry/land.js';
import type { Lien } from '../../register/register.js';
import { addDays, type Civil } from '../../calendar/civil.js';
import { period, type Period } from '../../calendar/calendar.js';
import {
  addTo,
  atMost,
  dustOf,
  material,
  sub,
  sum,
  withinDust,
  zeroIfNone,
} from '../../core/num.js';
import type { CurrencyCode, InstrumentId, PartyId, RegionId } from '../../core/ids.js';
import { none, some } from '../../core/option.js';
import { isAssetLeg, isCreateLeg, isDestroyLeg, type Leg } from '../../ledger/instruction.js';
import { displayName } from '../../registry/naming.js';
import { WHOLE_PIECES } from '../../registry/grid.js';
import type { ParamDecl } from '../../registry/params.js';
import type { UnitDecl } from '../../registry/registry.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
// Law 15, docs/PLAN.md 3.2: a typed accessor for another kind's TERMS, from the module that owns
// the kind. What a good is and what a lot of it cost are the goods module's to say; this module
// asks it rather than keeping a second copy of the answer (Law 4, Law 19).
import {
  goodId,
  groundUnderPlant,
  isGoodTerms,
  survivesWind,
  vintagesHeld,
} from '../../registry/physical.js';
import { costOfDraw } from '../../register/register.js';
import { CAPITAL_KINDS, type CapitalKindDecl } from './data.js';
import { conditionsFor, WIND } from '../../registry/environment.js';
import {
  capitalKindOf,
  failedForWant,
  landPerUnitParam,
  serviceLeft,
  standsWindParam,
  upkeepFor,
  upkeepParam,
  wentWithout,
  windHardnessParam,
} from '../../registry/physical.js';
import {
  isPlant,
  plantKindId,
  plantMarketId,
  plantProfile,
  plantTerms,
  plantUnitId,
  plantVintageId,
  wornOut,
  type PlantTerms,
} from './plant.js';

export * from './data.js';
export * from './plant.js';
import { buildLagParam, lifeParam } from '../../registry/physical.js';

/** A4, Law 2: the numbers a kind of capital states about itself, declared with their units. */
function paramsOf(rows: readonly CapitalKindDecl[]): ParamDecl[] {
  return rows.flatMap((d): ParamDecl[] => [
    ...(d.landPerUnit === null
      ? []
      : [
          {
            id: landPerUnitParam(d.id),
            value: d.landPerUnit,
            unit: 'square kilometres a unit stands on',
            dimension: 'ratio' as const,
            kind: 'technology' as const,
            owner: 'model' as const,
            why: `Goods B4 (13c.1): the ground a unit of ${d.name} stands on. It is what makes a place fill up — the more plant a region carries, the poorer the ground the next unit of it stands on — and it is why a rent emerges instead of a cap being needed (Law 6).`,
          },
        ]),
    {
      id: upkeepParam(d.id),
      value: d.upkeepPerUnitPerPeriod,
      unit: `${d.madeFrom}s a unit takes a period`,
      dimension: 'ratio',
      kind: 'technology',
      owner: 'model',
      why: `Capital Programme A6, Housing A5 (17e.2): what a unit of ${d.name} eats of what it is made of each period to stay in service. A thing that wears can be KEPT, and keeping it is a purchase from whoever makes the parts — so a holder that stops buying is a holder whose plant starts failing, which is the outlay a depreciating world was missing.`,
    },
    {
      id: lifeParam(d.id),
      value: d.usefulLifePeriods,
      unit: 'periods of service',
      dimension: 'periods',
      kind: 'technology',
      owner: 'model',
      why: `Capital Programme A4.b, A6: how long a vintage of ${d.name} works before it is worn out and leaves the register. ${d.why} The presence of a life is what makes the good it is built from a capital good.`,
    },
    {
      id: buildLagParam(d.id),
      value: d.buildLagPeriods,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'model',
      why: `Capital Programme C3, C1.a: periods between the ${d.madeFrom} arriving and the plant working. It is why investment is demand now and capacity later, and why a firm cannot answer this week's demand by spending this week.`,
    },
    {
      id: standsWindParam(d.id),
      value: d.standsWind,
      unit: "multiples of an ordinary period's wind",
      dimension: 'ratio',
      kind: 'technology',
      owner: 'model',
      why: `Commodities Spot B3, Freight B4 (13c): what a structure of ${d.name} is built for. It is NOT a threshold — nothing happens at it — and what survives a period is exp(-(wind / this) ^ hardness), which is positive at every wind and never one, so an ordinary week takes a little and a storm takes most (Law 6).`,
    },
    {
      id: windHardnessParam(d.id),
      value: d.windHardness,
      unit: 'exponent',
      dimension: 'ratio',
      kind: 'technology',
      owner: 'model',
      why: `Commodities Spot B3: how sharply ${d.name} fails above what it was built for. Wind damage is not linear in wind — doubling it is far more than twice the loss, because what fails is what the load exceeded — and this is the shape of that.`,
    },
  ]);
}

/** A1, A4: each kind of capital is counted in its own unit and never added to another's. */
function unitsOf(rows: readonly CapitalKindDecl[]): UnitDecl[] {
  // A4, Law 8: plant in service is the SAME PHYSICAL THING as the good it was built from, so its
  // smallest piece is that good's. A coarser grid here would strand a fraction of every machine at
  // the moment it went into service — a thing that had been bought, paid for and delivered, and
  // then did not fit into the unit it was about to be counted in.
  // Law 8: plant is counted in whole machines. A machine does not wear away into two thirds of
  // one: it wears in VALUE (A3, the depreciation schedule) and leaves in whole units when it goes.
  return rows.map((d) => ({ id: plantUnitId(d.id), name: d.unit, perUnit: WHOLE_PIECES }));
}

/**
 * A6, A6.a: the vintage a unit commissioned on this date belongs to — registered the first time
 * anybody commissions one, with its market, and shared by everybody who commissions the same week.
 * A vintage is a DATE and a kind, not a firm: two firms whose machines went into service the same
 * week hold the same thing, and one of them selling to the other moves units and not identities.
 */
export function vintage(
  ctx: MechanismContext,
  d: CapitalKindDecl,
  region: RegionId,
  serviceDate: Civil,
): InstrumentId {
  const id = plantVintageId(d.id, region, serviceDate);
  if (ctx.instruments.has(id)) return id;
  const market = plantMarketId(d.id, region, serviceDate);
  const terms: PlantTerms = {
    kind: plantKindId(d.id),
    capitalKind: d.id,
    region,
    serviceDate,
    // A4.b, A6: the date it is worn out, placed by the calendar from its own life (Money G3.a).
    retires: addDays(serviceDate, ctx.params.periods(lifeParam(d.id)) * ctx.calendar.periodDays),
  };
  ctx.issue({
    id,
    kind: plantKindId(d.id),
    // A1: nobody issued a machine. It is a thing its holder owns, like a tonne of grain.
    issuer: none(),
    ccy: ctx.registry.currencyOf(region),
    terms,
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy: ctx.registry.currencyOf(region),
    // D3, Clearing C4: a dead firm's plant is sold to whoever will have it, and a shortage of it is
    // shared in the proportion each bidder asked for. It is the rule every market here states once.
    rationing: 'proRata',
  });
  return id;
}

/**
 * A6, A6.a: a vintage that is worn out leaves the register, so the charge stops because the plant
 * is gone rather than because a schedule ran out of rows. Its carrying value is already nothing —
 * the last period of its life took it there — so what leaves is units and no value moves.
 */
function retire(ctx: MechanismContext): void {
  const on = ctx.calendar.startOf(ctx.period);
  for (const h of ctx.register.allHoldings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isPlant(i)) continue;
    const terms = plantTerms(i);
    if (!wornOut(terms, on)) continue;
    const units = ctx.register.free(h.holder, i.id);
    if (!material(units, h.lots.length + 1, units)) continue;
    // 0f.1: `free` is the cell's TOTAL; the leg moves it and the side is derived from it.
    const legs: Leg[] = [
      {
        kind: 'destroy',
        party: h.holder,
        instrument: i.id,
        qty: units,
        why: 'scrapped',
      },
    ];
    // 15.1: the ground it stood on is free again — every lien the vintage held on it goes with it.
    for (const lien of groundLiensFor(ctx, h.holder, terms.region, String(i.id))) {
      legs.push({
        kind: 'release',
        pledgor: h.holder,
        beneficiary: lien.beneficiary,
        instrument: landId(terms.region),
        lien: lien.id,
      });
    }
    const record = ctx.settle({
      legs,
      cause: 'corporateAction',
      reason: `${h.holder} retires ${i.id}: it is worn out`,
    });
    if (record.outcome !== 'settled') continue;
    ctx.record(
      'capital.retired',
      [h.holder, i.id],
      { holder: h.holder, vintage: i.id, capitalKind: terms.capitalKind, units, why: 'retired' },
      false,
    );
    // A6, Clearing C3 (18.4): AND ITS BOOK CLOSES WITH IT. A vintage that is worn out is scrap;
    // the second-hand market in it held a session every period for the rest of the run, printing
    // `noDemand` at nobody. What may still be sold is a vintage that still has life in it.
    if (i.market.some && ctx.register.heldTotal(i.id).value <= 0) {
      ctx.closeMarket(i.market.value, 'the vintage is worn out and there is none of it left');
    }
  }
}

/**
 * Capital Programme A6, Housing A5 (17e.2): KEEPING WHAT IT HAS, and what it costs not to.
 *
 * Wear was already an event and nobody could answer it: a holder's only reply to a machine getting
 * older was to hold fewer machines. What was missing is the OUTLAY. A unit in service eats a little
 * of what it is made of every period — parts for a machine, a roof for a building — and the holder
 * either bought that or it did not. What it did buy is CONSUMED here, off its own shelf, at what
 * the lots cost it; what it did not buy is a share of this period's upkeep the plant went without,
 * and what goes with it is the service that went with it (`failedForWant`): a machine that nobody
 * maintains ages at twice the calendar, and one maintained by halves at half again.
 *
 * NOTHING IS WRITTEN DOWN AND NOTHING IS RESCHEDULED. The vintage keeps the two dates it was made
 * with (Law 4: one representation of how worn a thing is, and 17e.1 reads it); what neglect takes
 * is UNITS — machines that broke and were not put right. So the carrying value, the wear charge,
 * the capacity read and the condition read all follow with no second writer, and the loss lands on
 * the holder's equity because settlement debits the lots at what they carried (Goods E3).
 *
 * It is one mechanism over every kind of plant this world has — machinery, premises, silos, hulls,
 * fleet — and it never asks what a kind is FOR (Law 15).
 */
function maintain(ctx: MechanismContext, rows: readonly CapitalKindDecl[]): void {
  const on = ctx.calendar.startOf(ctx.period);
  for (const h of ctx.register.allHoldings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isPlant(i)) continue;
    const terms = plantTerms(i);
    const kind = capitalKindOf(rows, terms.capitalKind);
    if (kind === undefined) continue;
    const left = serviceLeft(terms, on, ctx.calendar);
    if (left <= 0) continue;
    const units = ctx.register.free(h.holder, i.id);
    if (units <= 0) continue;
    const needed = upkeepFor(units, ctx.params.ratio(upkeepParam(kind.id)));
    if (needed <= 0) continue;
    const part = goodId(kind.madeFrom, terms.region);
    // What it holds of what its plant is made of, and never more of it than keeping the plant takes:
    // a shelf with one part on it does not repair two machines (arithmetic impossibility, Law 6).
    const have = ctx.instruments.has(part)
      ? ctx.register.free(h.holder, part)
      : asQty(0, 'it holds none of what its plant is made of');
    const used = atMost(have, needed, 'it cannot use more parts than it holds');
    const without = wentWithout(needed, used);
    const failed = ctx.registry.deliverable(failedForWant(units, left, without));
    if (used <= 0 && failed <= 0) continue;
    const legs: Leg[] = [];
    if (used > 0) {
      // The parts went INTO the plant: consumed, at what the lots cost it (Goods E3).
      legs.push({ kind: 'destroy', party: h.holder, instrument: part, qty: used, why: 'consumed' });
    }
    if (failed > 0) {
      legs.push({
        kind: 'destroy',
        party: h.holder,
        instrument: i.id,
        qty: failed,
        // A machine that broke and was not put right is scrap, and it is scrap for a reason the
        // journal carries: `capital.kept` says what it needed, what it bought and what it lost.
        why: 'scrapped',
      });
    }
    const record = ctx.settle({
      legs,
      // The physical world acting on units a party holds, as `perish` does; each leg's own `why`
      // says which way (Goods E4).
      cause: 'production',
      reason: `${h.holder} keeps ${i.id}`,
    });
    if (record.outcome !== 'settled') continue;
    ctx.record(
      'capital.kept',
      [h.holder, i.id],
      {
        holder: h.holder,
        vintage: i.id,
        capitalKind: terms.capitalKind,
        units,
        needed,
        used,
        without,
        failed,
      },
      false,
    );
  }
}

/**
 * Commodities Spot B3, Freight B4, Law 6: WHAT THE WEATHER TAKES DOWN, where it stood.
 *
 * A storm is one of the physical facts the environment publishes (13c), and this is one of the
 * several consequences of that one event: a real destruction of units of plant, on the named party
 * that owned them, at the site they were at. Nothing here multiplies a price and nothing writes
 * anything down — the loss is that the machines are gone, and what that costs their owner is what
 * settlement charged its equity (A3, Goods E3).
 *
 * WHAT SURVIVES IS `exp(-(wind / standsWind) ^ hardness)`, and there is no threshold in it. The
 * quantity is positive at every wind and never reaches one, so an ordinary week takes a little and
 * a storm takes most — continuously, with nothing happening AT any level and nothing clamped
 * (Law 6). Two technologies per kind say it: what the structure is built for, and how sharply what
 * it was not built for fails.
 */
function weather(ctx: MechanismContext, rows: readonly CapitalKindDecl[]): void {
  for (const h of ctx.register.allHoldings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isPlant(i)) continue;
    const terms = plantTerms(i);
    if (capitalKindOf(rows, terms.capitalKind) === undefined) continue;
    const wind = conditionsFor(ctx, terms.region, [WIND]);
    const standard = ctx.params.ratio(standsWindParam(terms.capitalKind));
    const hardness = ctx.params.ratio(windHardnessParam(terms.capitalKind));
    if (standard <= 0) continue;
    // 14.2, Law 4: the one relation, in the registry, read here and by the firm that insures against it.
    const survived = survivesWind(wind, standard, hardness);
    const units = ctx.register.free(h.holder, i.id);
    const lost = ctx.registry.deliverable(
      scale(units, asRatio(1 - survived, 'what the weather took'), 'what the wind took'),
    );
    if (!material(lost, 2, units) || lost <= 0) continue;
    // Insurers A4 (14.3): WHAT THE LOST UNITS WERE ON THE BOOKS AT, read off the lots the destroy
    // will draw (first in, first out, as the register draws) — the amount a claim on them is for.
    const atCost = costOfDrawing(h.lots, lost, ctx.instruments.get(h.instrument).ccy);
    const record = ctx.settle({
      legs: [
        {
          kind: 'destroy',
          party: h.holder,
          instrument: i.id,
          qty: lost,
          why: 'scrapped',
        },
      ],
      cause: 'corporateAction',
      reason: `${h.holder} lost ${lost} of ${i.id} to the weather`,
    });
    if (record.outcome !== 'settled') continue;
    ctx.record(
      'capital.weathered',
      [h.holder, i.id],
      {
        holder: h.holder,
        vintage: i.id,
        capitalKind: terms.capitalKind,
        units: lost,
        wind,
        survived,
        atCost: atCost.pieces,
        ccy: atCost.ccy,
      },
      true,
    );
  }
}

/** Register D4, Law 19: what drawing `units` first-in-first-out off these lots costs — the register's own order. */
function costOfDrawing(
  lots: readonly { readonly qty: Qty; readonly basisPerUnit: PerPiece }[],
  units: Qty,
  ccy: CurrencyCode,
): Cash {
  let left: number = units;
  let cost = noCash(ccy);
  for (const lot of lots) {
    if (left <= 0) break;
    const take = atMost(
      lot.qty,
      asQty(left, 'what is left to draw'),
      'a lot gives no more than it has',
    );
    cost = plus(
      cost,
      valueAt(
        lot.basisPerUnit,
        asQty(take, 'the pieces this lot gives'),
        ccy,
        'what these pieces cost',
      ),
      'what the drawing costs',
    );
    left -= take;
  }
  return cost;
}

/**
 * C1, C2, C3, C4, A4.c: what a firm BOUGHT of a capital good, the build lag ago, becomes plant now.
 *
 * The purchase is read off the wire (Law 19) rather than inferred from a stock, and that is what
 * makes A4.c answerable without asking anybody's industry: a producer's own output is not something
 * it bought. What it drew is destroyed and the plant created in the same instruction at the same
 * cost, so the spend is irreversible (C4) — the money went to the producer and what the firm has
 * now is a machine, not a claim on anybody.
 */
function commission(ctx: MechanismContext, rows: readonly CapitalKindDecl[]): void {
  for (const d of rows) {
    const lag = ctx.params.periods(buildLagParam(d.id));
    if (ctx.period < lag) continue;
    const bought = purchases(ctx, d, period(sub(ctx.period, lag, 'the period it was bought in')));
    for (const [buyer, byRegion] of bought) {
      for (const [region, qty] of byRegion) {
        commissionOne(ctx, d, buyer, region as RegionId, qty);
      }
    }
  }
}

/** C1: who bought units of this capital good in a period, and where, read off the settled trades. */
function purchases(
  ctx: MechanismContext,
  d: CapitalKindDecl,
  when: Period,
): Map<PartyId, Map<string, number>> {
  const out = new Map<PartyId, Map<string, number>>();
  for (const r of ctx.ledger.inPeriod(when)) {
    if (r.outcome !== 'settled' || r.instruction.cause !== 'trade') continue;
    for (const leg of r.instruction.legs) {
      if (!isAssetLeg(leg)) continue;
      const i = ctx.instruments.get(leg.instrument);
      if (!isGoodTerms(i.terms) || i.terms.subUnit !== d.madeFrom) continue;
      const byRegion = out.get(leg.to) ?? new Map<string, number>();
      // 0f.2: a leg moves the party's total, for a cell as for a named buyer.
      addTo(byRegion, i.terms.region, leg.qty);
      out.set(leg.to, byRegion);
    }
  }
  return out;
}

/** C3, A6.a: one buyer's machines going into service, as a transformation on its own book. */
/** 15.1: the liens a vintage holds on its holder's ground, by the vintage's name on each. */
function groundLiensFor(
  ctx: MechanismContext,
  holder: PartyId,
  region: RegionId,
  vintage: string,
): readonly Lien[] {
  const land = landId(region);
  if (!ctx.instruments.has(land)) return [];
  const holding = ctx.register.holding(holder, land);
  if (!holding.some) return [];
  return holding.value.liens.filter((l) => l.reason === vintage);
}

/**
 * C1 (15.1): HOW MUCH OF WHAT IT BOUGHT ITS GROUND CARRIES. The hectares it holds beyond what its
 * standing plant is on, against what a piece of this kind stands on — whole pieces, DOWN, because
 * plant on a fraction of a hectare it does not hold is on ground it does not hold. Nothing is
 * refused where the world has no ground line or the kind takes no ground.
 */
function groundToCarry(
  ctx: MechanismContext,
  d: CapitalKindDecl,
  buyer: PartyId,
  region: RegionId,
  bought: Qty,
): {
  readonly carries: Qty;
  readonly refused: Qty;
  readonly short: Qty;
  readonly pledge?: { readonly land: InstrumentId; readonly to: PartyId; readonly hectares: Qty };
} {
  const land = landId(region);
  if (!ctx.instruments.has(land) || d.landPerUnit === null || bought <= 0)
    return {
      carries: bought,
      refused: asQty(0, 'nothing refused'),
      short: asQty(0, 'no ground short'),
    };
  const reads = { registry: ctx.registry, params: ctx.params };
  const view = ctx.participant(buyer);
  const standing = hectaresOf(
    groundUnderPlant(reads, vintagesHeld(view, ctx.calendar.startOf(ctx.period))),
  );
  const held = ctx.register.quantity(buyer, land);
  const free = held > standing ? held - standing : 0;
  const needed = hectaresUnder(reads, d.id, bought);
  if (needed <= free) {
    const authority = authorityIdFor(region);
    const pledge =
      ctx.parties.has(authority) && needed > 0
        ? { land, to: authority, hectares: needed }
        : undefined;
    return pledge === undefined
      ? {
          carries: bought,
          refused: asQty(0, 'nothing refused'),
          short: asQty(0, 'no ground short'),
        }
      : {
          carries: bought,
          refused: asQty(0, 'nothing refused'),
          short: asQty(0, 'no ground short'),
          pledge,
        };
  }
  // Law 8: the pieces the free ground carries, down — one hectare carries so many pieces of it.
  const carries = needed > 0 ? downTick((bought * free) / needed) : bought;
  const fits = carries > 0 ? hectaresUnder(reads, d.id, carries) : asQty(0, 'nothing fits');
  const authority = authorityIdFor(region);
  const pledge =
    ctx.parties.has(authority) && fits > 0 ? { land, to: authority, hectares: fits } : undefined;
  const out = {
    carries,
    refused: subQty(bought, carries, 'the pieces its ground does not carry'),
    short: subQty(needed, asQty(free, 'the hectares it holds free'), 'the hectares it is short of'),
  };
  return pledge === undefined ? out : { ...out, pledge };
}

function commissionOne(
  ctx: MechanismContext,
  d: CapitalKindDecl,
  buyer: PartyId,
  region: RegionId,
  bought: number,
): void {
  if (!ctx.parties.has(buyer) || !ctx.parties.get(buyer).status.alive) return;
  const good = goodId(d.madeFrom, region);
  if (!ctx.instruments.has(good)) return;
  const holding = ctx.register.holding(buyer, good);
  if (!holding.some) return;
  const free = ctx.register.free(buyer, good);
  // It commissions what it bought, and it cannot commission what it no longer has: a firm that
  // sold the machine on before it was installed installed nothing.
  const bought_ = atMost(
    free,
    ctx.registry.deliverable(bought),
    'only what is unencumbered can be built into plant',
  );
  /**
   * C1, Law 8 (15.1): AND ONLY ON GROUND IT HOLDS. Plant stands on hectares, and a firm that has
   * not bought them has nowhere to put it: what is commissioned is what its free ground carries —
   * the hectares it holds beyond what its standing plant is on — and the rest is refused and said
   * (`capital.refused`), the machines staying what they were, goods it holds. The ground under the
   * new vintage is PLEDGED to it for as long as it stands — to the authority of the place, whose
   * consent it is — so it cannot be sold from under the plant; the lien is released when the
   * vintage retires. A world with no ground line has no ground to be short of and refuses nothing.
   */
  const ground = groundToCarry(ctx, d, buyer, region, bought_);
  const qty = ground.carries;
  if (ground.refused > 0) {
    ctx.record(
      'capital.refused',
      [buyer],
      {
        firm: buyer,
        capitalKind: d.id,
        units: ground.refused,
        hectaresShort: ground.short,
        why: 'no ground to stand it on',
      },
      true,
    );
  }
  if (qty <= 0 || !material(qty, holding.value.lots.length + 1, bought)) return;
  const cost = costOfDraw(holding.value.lots, qty, ctx.instruments.get(good).ccy);
  const serviceDate = ctx.calendar.startOf(ctx.period);
  const id = vintage(ctx, d, region, serviceDate);
  // 0f.1: `free` is the cell's TOTAL; what is built is that, and the side is derived from it.
  const total = qty;
  const legs: Leg[] = [
    {
      kind: 'destroy',
      party: buyer,
      instrument: good,
      qty: total,
      why: 'consumed',
    },
    {
      kind: 'create',
      party: buyer,
      instrument: id,
      qty: total,
      // C4, A6: what the plant cost is what the machines cost. Nothing is added at the door.
      costPerUnit: pricedAt(cost, qty, 'what a unit of plant cost'),
    },
  ];
  if (ground.pledge !== undefined && ground.pledge.hectares > 0) {
    legs.push({
      kind: 'pledge',
      pledgor: buyer,
      beneficiary: ground.pledge.to,
      instrument: ground.pledge.land,
      qty: ground.pledge.hectares,
      secures: String(id),
    });
  }
  const record = ctx.settle({
    legs,
    cause: 'production',
    reason: `${buyer} commissions ${qty} of ${d.name}`,
  });
  if (record.outcome !== 'settled') return;
  ctx.record(
    'capital.commissioned',
    [buyer, id],
    {
      firm: buyer,
      vintage: id,
      capitalKind: d.id,
      units: qty,
      cost: cost.pieces,
      ccy: cost.ccy,
      serviceDate,
    },
    false,
  );
}

/**
 * A6.b: the change in a firm's plant equals what it commissioned, less what it retired, scrapped
 * or abandoned, plus what came in and out by transfer — per firm, per kind, every period. And the
 * same for capital that has ARRIVED and is not yet in service, which is the stock of the capital
 * good itself: what it bought, less what it commissioned, less what it sold on.
 *
 * Two independent records are compared: the register's own walk over the holdings, and the legs
 * that said why anything moved. A stock that moved without a leg has nowhere to hide, which is what
 * makes commissioning, wearing out and a sale from an estate the only ways plant can appear or go.
 */
function plantMoves(rows: readonly CapitalKindDecl[]): Family {
  const seen: { period: Period | undefined; held: Map<string, Qty> } = {
    period: undefined,
    held: new Map(),
  };
  const goods = new Set(rows.map((d) => d.madeFrom));
  const key = (party: string, instrument: string): string => `${party}|${instrument}`;
  return {
    name: 'units',
    contributor: 'capital-programme',
    spec: 'Capital Programme A6.b Capital Programme D1',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const moved = new Map<string, Qty[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        for (const leg of r.instruction.legs) {
          if (isCreateLeg(leg) || isDestroyLeg(leg)) {
            const list = moved.get(key(leg.party, leg.instrument)) ?? [];
            list.push(isCreateLeg(leg) ? leg.qty : negQty(leg.qty, 'what left the world'));
            moved.set(key(leg.party, leg.instrument), list);
          } else if (isAssetLeg(leg)) {
            const into = moved.get(key(leg.to, leg.instrument)) ?? [];
            into.push(leg.qty);
            moved.set(key(leg.to, leg.instrument), into);
            const from = moved.get(key(leg.from, leg.instrument)) ?? [];
            from.push(negQty(leg.qty, 'what left'));
            moved.set(key(leg.from, leg.instrument), from);
          }
        }
      }
      // A-42: no weight exemption, for the reason `goods/index.ts:unitsIdentity` gives at length —
      // this counted every weight event in the world and a household ages every period, so the
      // family had been switched off since the first ageing. This one compares PER HOLDER and per
      // member, where a split changes the set of keys and not the number on either side of it.
      const consecutive = seen.period !== undefined && view.period === seen.period + 1;
      const held = new Map<string, Qty>();
      for (const i of view.instruments.all()) {
        const capital = isPlant(i) || (isGoodTerms(i.terms) && goods.has(i.terms.subUnit));
        if (!capital) continue;
        for (const holder of view.register.holdersOf(i.id)) {
          const k = key(holder, i.id);
          const now = view.register.quantity(holder, i.id);
          held.set(k, now);
        }
      }
      const comparable = consecutive;
      for (const [k, before] of comparable ? seen.held : new Map<string, Qty>()) {
        const now = zeroIfNone(held.get(k));
        const legs = sum(moved.get(k) ?? []);
        const change = minus(now, before, 'what the stock moved by');
        const dust =
          legs.dust + dustOf(legs.terms + 2, Math.abs(before) + Math.abs(now) + legs.magnitude);
        if (withinDust(change, legs.value, dust)) continue;
        const [party, instrument] = k.split('|');
        out.push({
          family: 'units',
          spec: 'Capital Programme A6.b',
          owner: party ?? k,
          size: minus(change, legs.value, 'plant that moved with no leg behind it'),
          unit:
            instrument === undefined
              ? 'units'
              : view.instruments.get(instrument as InstrumentId).unit,
          period: view.period,
          message: `${party ?? k}: ${instrument ?? 'plant'} moved by ${change} and its legs account for ${legs.value}`,
        });
      }
      // A holding that appeared this period was nothing before it, and the legs must say so too.
      if (comparable) {
        for (const [k, now] of held) {
          if (seen.held.has(k)) continue;
          const legs = sum(moved.get(k) ?? []);
          const dust = legs.dust + dustOf(legs.terms + 2, Math.abs(now) + legs.magnitude);
          if (withinDust(now, legs.value, dust)) continue;
          const [party, instrument] = k.split('|');
          out.push({
            family: 'units',
            spec: 'Capital Programme A6.b',
            owner: party ?? k,
            size: minus(now, legs.value, 'plant that appeared with no leg behind it'),
            unit:
              instrument === undefined
                ? 'units'
                : view.instruments.get(instrument as InstrumentId).unit,
            period: view.period,
            message: `${party ?? k}: ${instrument ?? 'plant'} appeared at ${now} and its legs account for ${legs.value}`,
          });
        }
      }
      seen.period = view.period;
      seen.held = held;
      return out;
    },
  };
}

/**
 * The module. It is built per world, because the identity it contributes remembers the stock it saw
 * last period and that memory is one world's (Law 4).
 */
export function capitalProgramme(rows: readonly CapitalKindDecl[] = CAPITAL_KINDS): SystemModule {
  return {
    id: 'capital-programme',
    spec: 'Capital Programme, XI-4',
    // A4.b, C1: a kind of capital is made from a good, so the goods have to exist for it to be
    // made of anything. Nothing else: the decision to invest belongs to whoever is deciding.
    requires: ['goods'],
    instrumentKinds: rows.map(plantProfile),
    partyKinds: [],
    curveFamilies: [],
    units: unitsOf(rows),
    params: paramsOf(rows),
    phases: [
      {
        name: 'capital.retire',
        spec: 'Capital Programme A6 Capital Programme A6.a Capital Programme A6.b',
        // At the top of the period, so what a firm decides and what it can make this period are
        // decided against the plant it actually still has.
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'event', name: 'environment.state', of: 'anyPeriod' }],
        writes: [
          { kind: 'event', name: 'capital.retired' },
          { kind: 'event', name: 'capital.weathered' },
        ],
        run: (ctx: MechanismContext) => {
          retire(ctx);
          // B3: and what the weather took, before anybody decides what it can make with what is
          // left. A storm is not a surprise a firm hears about later: it is standing in it.
          weather(ctx, rows);
        },
      },
      {
        name: 'capital.upkeep',
        spec: 'Capital Programme A6 Housing A5',
        // After the session the parts were bought in and after the lines have run, so what is left
        // on the shelf is what keeping the plant can draw on, and before the write-down looks at
        // what survived (the same place `goods.spoilage` sits).
        anchor: { before: 'revaluation' },
        reads: [],
        writes: [{ kind: 'event', name: 'capital.kept' }],
        run: (ctx: MechanismContext) => {
          maintain(ctx, rows);
        },
      },
      {
        name: 'capital.commission',
        spec: 'Capital Programme C1 Capital Programme C3 Capital Programme C4 Capital Programme A4.c',
        // After the session it was bought in and after the lines have run, so a machine delivered
        // this period is installed at the end of the period the build lag names, and the vintage
        // it joins is dated by when it went into service rather than when it was ordered.
        anchor: { before: 'revaluation' },
        reads: [],
        writes: [
          { kind: 'event', name: 'capital.commissioned' },
          { kind: 'event', name: 'capital.refused' },
        ],
        run: (ctx: MechanismContext) => {
          commission(ctx, rows);
        },
      },
    ],
    participants: [],
    families: [plantMoves(rows)],
  };
}
