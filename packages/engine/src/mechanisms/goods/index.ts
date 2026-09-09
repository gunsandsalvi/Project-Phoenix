/**
 * Goods: things that are made from other things, held as stock at what they cost, and sold in a
 * market of their own.
 *
 * @spec Goods A1 Goods A2 Goods A2.a Goods A2.b Goods A2.c Goods A3 Goods A4 Goods C1 Goods C2 Goods C3 Goods C4 Goods C5 Goods C6 Goods E1 Goods E2 Goods E2.a Goods E2.c Goods E3 Goods E4 Goods E4.a Goods E5 Commodities Spot D5 Commodities Spot F1 Part XII Law 2 Law 15 XI-6
 *
 * One instrument per (region, sub-unit) and one market for it, because a good is homogeneous within
 * its sub-unit (A4) and its price is in the money of where it is sold (C6). The module owns:
 *
 *   - the kinds: physical, carried at cost, written down to the print and never up (E1, E2, E2.c);
 *   - the recipes, as the goods' own public terms, in physical quantities only (A2.a, A2.b);
 *   - what perishes in store, as units leaving the world at their own cost (E4);
 *   - the units identity per good and region, contributed to the audit's units family (Part XII).
 *
 * It owns no production and no demand: who decides to make a thing is the firm (Firm B1, worklist
 * 4.5) and who bids for it is the buyer (C3, worklist 4.5 and 4.6). Until they exist the markets
 * open, clear nothing, and print nothing — which is what a market with no two sides honestly is.
 */
import type { Family, Violation } from '../../audit/audit.js';
import { forbid } from '../../core/assert.js';
import type { Period } from '../../calendar/calendar.js';
import { Missing } from '../../core/errors.js';
import type { InstrumentId, InstrumentKindId } from '../../core/ids.js';
import { combineDust, sum, withinDust } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { isCreateLeg, isDestroyLeg } from '../../ledger/instruction.js';
import { displayName } from '../../registry/naming.js';
import type { ParamDecl } from '../../registry/params.js';
import type { UnitDecl } from '../../registry/registry.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { GOODS, type GoodDecl } from './data.js';
import { goodProfile, perish } from './inventory.js';
import {
  goodId,
  goodKindId,
  goodMarketId,
  goodTermsOf,
  goodUnitId,
  labourParam,
  leadTimeParam,
  recipeParam,
  recipeUnit,
  spoilageParam,
} from './recipes.js';

export * from './data.js';
export * from './recipes.js';
export { goodProfile } from './inventory.js';

/** The declaration of an input this recipe names, or a defect: a recipe cannot name a non-good. */
function inputsOf(d: GoodDecl, rows: readonly GoodDecl[]): GoodDecl[] {
  return d.inputs.map((i) => {
    const found = rows.find((r) => r.subUnit === i.subUnit);
    if (found === undefined) {
      throw new Missing('Goods A2', `${d.subUnit} is made from ${i.subUnit}, which is not a good`, {
        output: d.subUnit,
        input: i.subUnit,
      });
    }
    return found;
  });
}

/**
 * Law 2, XI-14: every number the goods make behaviour out of, declared with its unit and its kind.
 * They are all TECHNOLOGY: how much of a thing it takes to make another thing, how long it takes,
 * and what the weather does to it in store. None of them is a claim about an answer.
 */
function paramsOf(rows: readonly GoodDecl[]): ParamDecl[] {
  const out: ParamDecl[] = [];
  for (const d of rows) {
    out.push({
      id: spoilageParam(d.subUnit),
      value: d.spoilagePerPeriod,
      unit: `fraction of units in store per period`,
      kind: 'technology',
      owner: 'model',
      why: `Goods E4: ${d.spoilageWhy} It is units that leave, never a fee: a storage charge is cash to whoever stores the goods and is a different thing (E4.a).`,
    });
    out.push({
      id: labourParam(d.subUnit),
      value: d.labourHoursPerUnit,
      unit: `hours per ${d.unit} of ${d.subUnit}`,
      kind: 'technology',
      owner: 'model',
      why: `Goods A2.c: ${d.labourWhy}`,
    });
    out.push({
      id: leadTimeParam(d.subUnit),
      value: d.leadTimePeriods,
      unit: 'periods',
      kind: 'technology',
      owner: 'model',
      why: `Goods B3: ${d.leadTimeWhy}`,
    });
    for (const input of inputsOf(d, rows)) {
      const decl = d.inputs.find((i) => i.subUnit === input.subUnit);
      if (decl === undefined) throw new Missing('Goods A2', `${d.subUnit}: input vanished`);
      out.push({
        id: recipeParam(d.subUnit, input.subUnit),
        value: decl.qtyPerUnit,
        unit: recipeUnit(input, d),
        kind: 'technology',
        owner: 'model',
        why: `Goods A2.a: ${decl.why} It is a fixed physical quantity, so a price that doubles does not halve the draw (A2.b).`,
      });
    }
  }
  return out;
}

/** A1: the physical units the goods are measured in, one declaration each however many use it. */
function unitsOf(rows: readonly GoodDecl[]): UnitDecl[] {
  const byUnit = new Map<string, UnitDecl>();
  for (const d of rows) {
    // A tonne is divisible; a good counted in whole things (a machine, a dwelling) is its own row.
    byUnit.set(d.unit, { id: goodUnitId(d.unit), name: d.unit, countable: false });
  }
  return [...byUnit.values()];
}

/**
 * A2.b, enforced: every recipe quantity is denominated in the physical units of its own two goods.
 * A number denominated in money — cost per unit of revenue, from which a physical draw would be
 * got by dividing by a price — is the strongest substitution assumption there is, and it is
 * refused here by name rather than left to be noticed.
 */
function refuseValueRecipes(ctx: SeedContext, rows: readonly GoodDecl[]): void {
  for (const d of rows) {
    for (const input of inputsOf(d, rows)) {
      const decl = ctx.params.decl(recipeParam(d.subUnit, input.subUnit));
      forbid(
        decl.unit === recipeUnit(input, d),
        'Goods A2.b',
        `${decl.id} is declared in "${decl.unit}"; a recipe is ${recipeUnit(input, d)}`,
      );
      for (const ccy of ctx.registry.currencies.keys()) {
        forbid(
          !decl.unit.includes(ccy),
          'Goods A2.b',
          `${decl.id} is denominated in ${ccy}: a recipe is a physical quantity, not a value share`,
        );
      }
    }
  }
}

/**
 * Part XII, Commodities Spot D5: what exists now is what existed, plus what was made, less what was
 * used up — per good and region, every period. The two records are independent: the register's own walk over every
 * holding, and the create and destroy legs that said why units appeared or left. The kernel checks
 * the same identity against the instrument's issued total; this one checks it against the holdings,
 * so a stock that moved without a leg has nowhere to hide.
 */
function unitsIdentity(kinds: ReadonlySet<InstrumentKindId>): Family {
  const seen: { period: Period | undefined; held: Map<InstrumentId, number> } = {
    period: undefined,
    held: new Map(),
  };
  return {
    name: 'units',
    contributor: 'goods',
    spec: 'Commodities Spot D5 Commodities Spot F1 Goods E4',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const moved = new Map<InstrumentId, number[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        for (const leg of r.instruction.legs) {
          if (!isCreateLeg(leg) && !isDestroyLeg(leg)) continue;
          const list = moved.get(leg.instrument) ?? [];
          list.push(isCreateLeg(leg) ? leg.qty : -leg.qty);
          moved.set(leg.instrument, list);
        }
      }
      // A weight event moves stock between books without an instruction (a cell splits, a member
      // dies): the holders' totals are re-struck, and this period's identity is not about them.
      const weights = view.journal.ofKind('weight').filter((e) => e.period === view.period).length;
      const consecutive = seen.period !== undefined && view.period === seen.period + 1;
      const held = new Map<InstrumentId, number>();
      for (const i of view.instruments.all()) {
        if (!kinds.has(i.kind)) continue;
        const now = view.register.heldTotal(i.id);
        held.set(i.id, now.value);
        const before = seen.held.get(i.id);
        if (!consecutive || before === undefined || weights > 0) continue;
        const legs = sum(moved.get(i.id) ?? []);
        const change = sum([now.value, -before]);
        if (withinDust(change.value, legs.value, combineDust(legs, change))) continue;
        out.push({
          family: 'units',
          spec: 'Commodities Spot D5',
          owner: i.id,
          size: change.value - legs.value,
          unit: i.unit,
          period: view.period,
          message: `${i.id}: the stock held moved by ${change.value} and ${legs.value} was made or used up`,
        });
      }
      seen.period = view.period;
      seen.held = held;
      return out;
    },
  };
}

/**
 * The module. It is built per world, because the units identity it contributes remembers the stock
 * it saw last period, and that memory is one world's (Law 4).
 */
export function goods(rows: readonly GoodDecl[] = GOODS): SystemModule {
  const kinds = new Set(rows.map((d) => goodKindId(d.subUnit)));
  return {
    id: 'goods',
    spec: 'Goods',
    requires: [],
    instrumentKinds: rows.map(goodProfile),
    partyKinds: [],
    curveFamilies: [],
    units: unitsOf(rows),
    params: paramsOf(rows),
    phases: [
      {
        name: 'goods.spoilage',
        spec: 'Goods E4 Goods E4.a',
        // What perishes is what is still in store at the end of the period: after this period's
        // trades and production, and before the write-down looks at what survived (E2).
        cycle: 'anchor',
        anchor: { before: 'revaluation' },
        run: (ctx: MechanismContext) => {
          perish(ctx, kinds);
        },
      },
    ],
    participants: [],
    families: [unitsIdentity(kinds)],
    seed(ctx: SeedContext): void {
      refuseValueRecipes(ctx, rows);
      for (const region of ctx.registry.regions.values()) {
        for (const d of rows) {
          const id = goodId(d.subUnit, region.id);
          ctx.instruments.add({
            id,
            kind: goodKindId(d.subUnit),
            // A1: nobody issued a tonne of grain, and naming a party that had would be a fiction.
            issuer: none(),
            // C6: the price is in the seller's money, which is the money of the region it is in.
            ccy: region.ccy,
            terms: goodTermsOf(d, region.id, inputsOf(d, rows)),
            market: some(goodMarketId(d.subUnit, region.id)),
          });
          ctx.openMarket({
            id: goodMarketId(d.subUnit, region.id),
            name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
            instrument: id,
            ccy: region.ccy,
            // C4: the rationing rule, stated once for every goods market: pro rata, so a shortage
            // is shared in the proportion each buyer asked for and nobody is privileged.
            rationing: 'proRata',
          });
        }
      }
    },
  };
}
