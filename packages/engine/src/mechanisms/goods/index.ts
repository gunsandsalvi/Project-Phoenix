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
 *   - the batch on the line (B3), a kind of its own, carried at what it has cost so far;
 *   - what perishes in store, as units leaving the world at their own cost (E4);
 *   - the units identity per good and region, and the check that what a batch consumed IS its
 *     recipe (B2, A2.a), both contributed to the audit's units family (Part XII).
 *
 * It owns no production DECISION and no demand: who decides to make a thing is the firm (Firm B1)
 * and who bids for it is the buyer (C3, worklist 4.6). What the firm draws when it makes something
 * is not its choice, though — the recipe is the good's own technology, and the audit says so.
 */
import type { Family, Violation } from '../../audit/audit.js';
import { forbid } from '../../core/assert.js';
import type { Period } from '../../calendar/calendar.js';
import { Missing } from '../../core/errors.js';
import type { AuditView } from '../../audit/view.js';
import type { InstrumentId, InstrumentKindId } from '../../core/ids.js';
import { addTo, combineDust, dustOf, mul, sub, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { isCreateLeg, isDestroyLeg } from '../../ledger/instruction.js';
import { displayName } from '../../registry/naming.js';
import type { ParamDecl } from '../../registry/params.js';
import { TONNE_PIECES, WHOLE_PIECES } from '../../registry/grid.js';
import type { UnitDecl } from '../../registry/registry.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { GOODS, type GoodDecl } from './data.js';
import { goodProfile, perish, wipProfile } from './inventory.js';
import {
  goodId,
  goodKindId,
  goodMarketId,
  goodTermsOf,
  goodUnitId,
  isGoodTerms,
  isWipTerms,
  labourParam,
  leadTimeParam,
  plantParam,
  recipeParam,
  recipeUnit,
  spoilageParam,
  wipId,
  wipKindId,
  wipTermsOf,
  yieldParam,
} from './recipes.js';

export * from './data.js';
export * from './recipes.js';
export { costOfDraw, dueFromLine, goodProfile, wipProfile } from './inventory.js';

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
    for (const plant of d.plant) {
      out.push({
        id: plantParam(d.subUnit, plant.capitalKind),
        value: plant.unitsPerUnitPerPeriod,
        unit: `units of ${plant.capitalKind} in service per ${d.unit} of ${d.subUnit} started per period`,
        kind: 'technology',
        owner: 'model',
        why: `Goods A2.c, Capital Programme A2: ${plant.why} It is the stock that lets the line run at a rate, so what it can make is a function of what it holds and not of what it wants.`,
      });
    }
    out.push({
      id: yieldParam(d.subUnit),
      value: d.yieldRate,
      unit: `${d.unit} finished per ${d.unit} started`,
      kind: 'technology',
      owner: 'model',
      why: `Goods B4: ${d.yieldWhy} What survives is dearer than what was started, because the cost of the units that did not make it is carried by the ones that did.`,
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
    // Goods A1, Law 8: the smallest piece of this good that exists, and it is set by what the
    // SMALLEST holder deals in rather than the largest: a household member buys a kilo or two of
    // bread a week, so a piece at the kilo would round a person's whole week's shopping up or down
    // and the sector's demand with it — a gram does not. A good counted in whole things (a machine,
    // a dwelling) has no piece below one of itself at all. The invariance test says the world's
    // path does not turn on the choice (test/tick.test.ts).
    byUnit.set(d.unit, {
      id: goodUnitId(d.unit),
      name: d.unit,
      perUnit: d.unit === 'tonnes' ? TONNE_PIECES : WHOLE_PIECES,
    });
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
 * Goods B2, A2.a, B4, Commodities Spot F1: what a batch consumed IS the recipe. Production is the
 * only way units enter this world, and the recipe is the good's own public technology, so the
 * quantities drawn are not the maker's to choose: what is made draws `qty x qtyPerUnit` of every
 * input the recipe names. A good may come off the line out of its own batch instead (B3), and then
 * what it takes is that batch: at least its own units of it, because units started and not finished
 * are the yield (B4) and a loss of units is never a gain.
 *
 * The kernel cannot check this: it never looks inside an instrument's terms (Law 15), so all it
 * knows is that units came from a production event. This is where the recipe is enforceable, and it
 * is measured rather than refused because a batch that drew the wrong thing is a defect in a
 * mechanism, not a violation at the wire (docs/ARCHITECTURE.md 5).
 */
function recipeIdentity(): Family {
  return {
    name: 'units',
    contributor: 'goods.production',
    spec: 'Goods B2 Goods A2.a Goods B4 Commodities Spot F1',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled' || r.instruction.cause !== 'production') continue;
        const made = r.instruction.legs.filter(isCreateLeg);
        if (made.length === 0) continue;
        const used = new Map<InstrumentId, number>();
        for (const leg of r.instruction.legs.filter(isDestroyLeg)) {
          addTo(used, leg.instrument, leg.qty);
        }
        for (const leg of made) {
          const ways = waysToMake(view, leg.instrument, leg.qty);
          if (ways.length === 0 || ways.some((w) => satisfied(view, w, used))) continue;
          const canonical = ways[0];
          if (canonical === undefined) continue;
          for (const [instrument, need] of canonical.needs) {
            const drawn = zeroIfNone(used.get(instrument));
            if (satisfiedBy(canonical.atLeast, need, drawn)) continue;
            out.push({
              family: 'units',
              spec: canonical.spec,
              owner: leg.instrument,
              size: sub(need, drawn, 'draw against the recipe'),
              unit: view.instruments.get(instrument).unit,
              period: view.period,
              message: `${leg.qty} of ${leg.instrument} was made from ${drawn} of ${instrument}, and it takes ${need}`,
            });
          }
        }
      }
      return out;
    },
  };
}

/** One admissible way of making something: what it draws, and whether that is exact or a floor. */
interface Way {
  readonly spec: string;
  readonly needs: ReadonlyMap<InstrumentId, number>;
  readonly atLeast: boolean;
}

/** B2, B3: a batch is made from the recipe; a good is made from its batch, or from the recipe. */
function waysToMake(view: AuditView, made: InstrumentId, qty: number): Way[] {
  const terms = view.instruments.get(made).terms;
  if (isWipTerms(terms)) {
    return [{ spec: 'Goods B2', needs: drawFor(view, terms.output, qty), atLeast: false }];
  }
  if (!isGoodTerms(terms)) return [];
  return [
    {
      spec: 'Goods B4',
      needs: new Map([[wipId(terms.subUnit, terms.region), qty]]),
      atLeast: true,
    },
    { spec: 'Goods B2', needs: drawFor(view, made, qty), atLeast: false },
  ];
}

/**
 * A2.c: a thing drawn from labour and land alone draws no units, so a recipe that names no input
 * has nothing to check and is satisfied by anything. That is not a hole in the identity — it is
 * where labour enters the chain, and what it cost is on the batch (B5).
 */
function satisfied(
  view: AuditView,
  way: Way,
  used: ReadonlyMap<InstrumentId, number>,
): boolean {
  for (const [instrument, need] of way.needs) {
    if (!satisfiedBy(way.atLeast, need, zeroIfNone(used.get(instrument)))) return false;
  }
  return true;
}

/**
 * Law 7 and Law 8 together: what a draw is entitled to differ from the recipe by. The arithmetic's
 * own dust, and one piece of the input — because the input is drawn in WHOLE PIECES of itself
 * (core/tick.ts) and a recipe met with the piece below is a recipe not met, so a batch draws the
 * piece above. It is a derived allowance from a real granularity, not a band around a defect.
 */
function satisfiedBy(atLeast: boolean, need: number, drawn: number): boolean {
  // Law 7, Law 8: the arithmetic's own dust, plus the one piece the draw was rounded up by.
  const dust = dustOf(2, need + drawn) + 1;
  return atLeast ? drawn - need >= -dust : withinDust(drawn, need, dust);
}

/** A2.a: what a batch of `qty` of one good draws, input by input, at the declared quantities. */
function drawFor(view: AuditView, output: InstrumentId, qty: number): Map<InstrumentId, number> {
  const out = new Map<InstrumentId, number>();
  const terms = view.instruments.get(output).terms;
  if (!isGoodTerms(terms)) return out;
  for (const input of terms.recipe.inputs) {
    const id = goodId(input.subUnit, terms.region);
    out.set(id, mul(qty, view.params.get(input.qtyPerUnit), 'units the recipe draws'));
  }
  return out;
}

/**
 * The module. It is built per world, because the units identity it contributes remembers the stock
 * it saw last period, and that memory is one world's (Law 4).
 */
export function goods(rows: readonly GoodDecl[] = GOODS): SystemModule {
  // What perishes in store is the good; the batch on the line is counted with it and perishes with
  // nothing (B3), so the two sets are different questions and are kept apart.
  const kinds = new Set(rows.map((d) => goodKindId(d.subUnit)));
  const physical = new Set([...kinds, ...rows.map((d) => wipKindId(d.subUnit))]);
  return {
    id: 'goods',
    spec: 'Goods',
    requires: [],
    instrumentKinds: rows.flatMap((d) => [goodProfile(d), wipProfile(d)]),
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
    families: [unitsIdentity(physical), recipeIdentity()],
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
          // B3: the batch this good is made in. It is held, it is carried at what it has cost, and
          // nobody trades it: there is no market for a half-made thing.
          ctx.instruments.add({
            id: wipId(d.subUnit, region.id),
            kind: wipKindId(d.subUnit),
            issuer: none(),
            ccy: region.ccy,
            terms: wipTermsOf(d, region.id),
            market: none(),
          });
        }
      }
    },
  };
}
