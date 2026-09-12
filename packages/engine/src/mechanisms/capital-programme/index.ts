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
import type { Family, Violation } from '../../audit/audit.js';
import { negQty } from '../../core/tick.js';
import { addDays, type Civil } from '../../calendar/civil.js';
import { period, type Period } from '../../calendar/calendar.js';
import {
  addTo,
  atMost,
  div,
  dustOf,
  material,
  sub,
  sum,
  withinDust,
  zeroIfNone,
} from '../../core/num.js';
import type { InstrumentId, PartyId, RegionId } from '../../core/ids.js';
import { none, some } from '../../core/option.js';
import { isAssetLeg, isCreateLeg, isDestroyLeg, type Leg } from '../../ledger/instruction.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import { displayName } from '../../registry/naming.js';
import { WHOLE_PIECES } from '../../registry/grid.js';
import type { ParamDecl } from '../../registry/params.js';
import type { UnitDecl } from '../../registry/registry.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
// Law 15, docs/PLAN.md 3.2: a typed accessor for another kind's TERMS, from the module that owns
// the kind. What a good is and what a lot of it cost are the goods module's to say; this module
// asks it rather than keeping a second copy of the answer (Law 4, Law 19).
import { goodId, isGoodTerms } from '../../registry/physical.js';
import { costOfDraw } from '../../register/register.js';
import { CAPITAL_KINDS, type CapitalKindDecl } from './data.js';
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


/** A4, Law 2: the two numbers a kind of capital states about itself, declared with their units. */
function paramsOf(rows: readonly CapitalKindDecl[]): ParamDecl[] {
  return rows.flatMap((d): ParamDecl[] => [
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
    ccy: ctx.registry.region(region).ccy,
    terms,
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy: ctx.registry.region(region).ccy,
    // D3, Clearing C4: a dead firm's plant is sold to whoever will have it, and a shortage of it is
    // shared in the proportion each bidder asked for. It is the rule every market here states once.
    rationing: 'proRata',
  });
  return id;
}

/** The same, at the seed: the opening world's plant is registered before anything can be endowed. */
export function seedVintage(
  ctx: SeedContext,
  d: CapitalKindDecl,
  region: RegionId,
  serviceDate: Civil,
): InstrumentId {
  const id = plantVintageId(d.id, region, serviceDate);
  if (ctx.instruments.has(id)) return id;
  const market = plantMarketId(d.id, region, serviceDate);
  const ccy = ctx.registry.region(region).ccy;
  const terms: PlantTerms = {
    kind: plantKindId(d.id),
    capitalKind: d.id,
    region,
    serviceDate,
    retires: addDays(serviceDate, ctx.params.periods(lifeParam(d.id)) * ctx.calendar.periodDays),
  };
  ctx.instruments.add({
    id,
    kind: plantKindId(d.id),
    issuer: none(),
    ccy,
    terms,
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy,
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
    const party = ctx.parties.get(h.holder);
    const side = cellSide(party, units);
    const record = ctx.settle({
      legs: [
        {
          kind: 'destroy',
          party: h.holder,
          instrument: i.id,
          qty: totalFor(party, units),
          why: 'scrapped',
          fromCell: side === undefined ? none() : some(side),
        },
      ],
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
  }
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
      // XI-15: a cell's leg is per member; a named buyer's is the whole of it.
      addTo(byRegion, i.terms.region, leg.toCell.some ? leg.toCell.value.perMember : leg.qty);
      out.set(leg.to, byRegion);
    }
  }
  return out;
}

/** C3, A6.a: one buyer's machines going into service, as a transformation on its own book. */
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
  const qty = atMost(free, ctx.registry.deliverable(ctx.instruments.get(good).unit, bought), 'only what is unencumbered can be built into plant');
  if (!material(qty, holding.value.lots.length + 1, bought)) return;
  const cost = costOfDraw(holding.value.lots, qty);
  const serviceDate = ctx.calendar.startOf(ctx.period);
  const id = vintage(ctx, d, region, serviceDate);
  const party = ctx.parties.get(buyer);
  const side = cellSide(party, qty);
  const total = totalFor(party, qty);
  const legs: Leg[] = [
    {
      kind: 'destroy',
      party: buyer,
      instrument: good,
      qty: total,
      why: 'consumed',
      fromCell: side === undefined ? none() : some(side),
    },
    {
      kind: 'create',
      party: buyer,
      instrument: id,
      qty: total,
      // C4, A6: what the plant cost is what the machines cost. Nothing is added at the door.
      costPerUnit: div(cost, qty, 'what a unit of plant cost'),
      toCell: side === undefined ? none() : some(side),
    },
  ];
  const record = ctx.settle({
    legs,
    cause: 'production',
    reason: `${buyer} commissions ${qty} of ${d.name}`,
  });
  if (record.outcome !== 'settled') return;
  ctx.record(
    'capital.commissioned',
    [buyer, id],
    { firm: buyer, vintage: id, capitalKind: d.id, units: qty, cost, serviceDate },
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
  const seen: { period: Period | undefined; held: Map<string, number> } = {
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
      const moved = new Map<string, number[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        for (const leg of r.instruction.legs) {
          if (isCreateLeg(leg) || isDestroyLeg(leg)) {
            const list = moved.get(key(leg.party, leg.instrument)) ?? [];
            list.push(isCreateLeg(leg) ? leg.qty : negQty(leg.qty, 'what left the world'));
            moved.set(key(leg.party, leg.instrument), list);
          } else if (isAssetLeg(leg)) {
            const into = moved.get(key(leg.to, leg.instrument)) ?? [];
            into.push(leg.toCell.some ? leg.toCell.value.perMember : leg.qty);
            moved.set(key(leg.to, leg.instrument), into);
            const from = moved.get(key(leg.from, leg.instrument)) ?? [];
            from.push(negQty(leg.fromCell.some ? leg.fromCell.value.perMember : leg.qty, 'what left'));
            moved.set(key(leg.from, leg.instrument), from);
          }
        }
      }
      // A weight event restates every holding of a cell without an instruction; this period's
      // identity is not about that, exactly as the goods module's own units check has it.
      const weights = view.journal.ofKind('weight').filter((e) => e.period === view.period).length;
      const consecutive = seen.period !== undefined && view.period === seen.period + 1;
      const held = new Map<string, number>();
      for (const i of view.instruments.all()) {
        const capital =
          isPlant(i) || (isGoodTerms(i.terms) && goods.has(i.terms.subUnit));
        if (!capital) continue;
        for (const holder of view.register.holdersOf(i.id)) {
          const k = key(holder, i.id);
          const now = view.register.quantity(holder, i.id);
          held.set(k, now);
        }
      }
      const comparable = consecutive && weights === 0;
      for (const [k, before] of comparable ? seen.held : new Map<string, number>()) {
        const now = zeroIfNone(held.get(k));
        const legs = sum(moved.get(k) ?? []);
        const change = sub(now, before, 'what the stock moved by');
        const dust = legs.dust + dustOf(legs.terms + 2, Math.abs(before) + Math.abs(now) + legs.magnitude);
        if (withinDust(change, legs.value, dust)) continue;
        const [party, instrument] = k.split('|');
        out.push({
          family: 'units',
          spec: 'Capital Programme A6.b',
          owner: party ?? k,
          size: sub(change, legs.value, 'plant that moved with no leg behind it'),
          unit: instrument === undefined ? 'units' : view.instruments.get(instrument as InstrumentId).unit,
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
            size: sub(now, legs.value, 'plant that appeared with no leg behind it'),
            unit: instrument === undefined ? 'units' : view.instruments.get(instrument as InstrumentId).unit,
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
        cycle: 0,
        // At the top of the period, so what a firm decides and what it can make this period are
        // decided against the plant it actually still has.
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          retire(ctx);
        },
      },
      {
        name: 'capital.commission',
        spec: 'Capital Programme C1 Capital Programme C3 Capital Programme C4 Capital Programme A4.c',
        cycle: 'anchor',
        // After the session it was bought in and after the lines have run, so a machine delivered
        // this period is installed at the end of the period the build lag names, and the vintage
        // it joins is dated by when it went into service rather than when it was ordered.
        anchor: { before: 'revaluation' },
        run: (ctx: MechanismContext) => {
          commission(ctx, rows);
        },
      },
    ],
    participants: [],
    families: [plantMoves(rows)],
  };
}
