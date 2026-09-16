/**
 * Inventory: a stock is lots, each carrying what it cost, and what happens to it in store.
 *
 * @spec Goods A1 Goods A2 Goods A3 Goods A4 Goods C2 Goods E1 Goods E2 Goods E2.a Goods E2.c Goods E3 Goods E4 Goods E4.a Goods E5 Law 9 XI-6
 *
 * A good CLEARS in a market and is CARRIED at what it cost: two different questions about one
 * thing (C2, E1). The carrying value only ever falls — to the market's own print when that print is
 * below cost (E2), never above it (E2.c), and the fall is a charge to income in the period it
 * happens (E3). Cost flows out first-in-first-out, the registry's one stated lot flow (E5).
 *
 * SPOILAGE is units leaving the world at the lot's own cost (E4): a destroy leg, one side, nobody
 * on the other end of a batch that went stale. It is not a fee and must never be summed with one
 * (E4.a): a storage charge is cash paid to whoever stores the goods, and until somebody does
 * (Goods D, worklist 13c) there is nobody to pay, so no such charge exists here at all.
 */
import { InvalidRegistry } from '../../core/errors.js';
import { asRatio, scale } from '../../core/measure.js';
import { asQty } from '../../core/tick.js';
import { goodId, upkeepFor, wentWithout } from '../../registry/physical.js';
import type { Leg } from '../../ledger/instruction.js';
import { CENT_TICK } from '../../registry/grid.js';
import type { InstrumentKindId } from '../../core/ids.js';
import { atMost, material, sum } from '../../core/num.js';
import type { PerPiece } from '../../core/measure.js';
import { none, some } from '../../core/option.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import type { MechanismContext } from '../../world/context.js';
import type { GoodDecl } from './data.js';
import {
  goodKindId,
  goodTerms,
  goodUnitId,
  isGoodTerms,
  isWipTerms,
  wipKindId,
} from './recipes.js';

/** The kind profile of one good: everything that varies by good, in one place (Law 15). */
export function goodProfile(d: GoodDecl): InstrumentKindProfile {
  const unit = goodUnitId(d.unit);
  return {
    id: goodKindId(d.subUnit),
    // C2: its price is what its market cleared at. E1: what a holder carries it at is what it cost.
    pricing: 'cleared',
    // Law 8: a commodity is quoted in cents the tonne, which is about a tenth of what a real grain
    // market ticks in — finer than the convention, never coarser, so nothing real is unquotable.
    priceTick: CENT_TICK,
    carry: 'cost',
    // A1: a tonne of grain is a real thing, not a promise; nobody owes it to anybody.
    liabilityOfIssuer: false,
    // A tonne is nobody's promise, so there is no issuer for a price move to reach.
    owes: 'face',
    physical: true,
    unit: () => unit,
    // Bond N13, N13.a: stated because the clause says to state it even when the answer is nothing.
    // A tonne is a thing its holder owns, not a promise anybody made, so there is no issuer to fail
    // and no claim to rank — which is a different answer from "unsecured", not a missing one.
    ranking: () => ({
      seniority: 0,
      secured: [],
      claim: 'nothing: it is owned outright, and nobody promised it',
    }),
    validateTerms: (t) => {
      if (!isGoodTerms(t)) {
        throw new InvalidRegistry('Goods A1', `${d.subUnit}: these are not the terms of a good`);
      }
      if (t.subUnit !== d.subUnit) {
        throw new InvalidRegistry('Goods A4', `${d.subUnit} terms carry sub-unit ${t.subUnit}`);
      }
      if (t.recipe.inputs.some((i) => i.subUnit === d.subUnit)) {
        throw new InvalidRegistry('Goods A2', `${d.subUnit} would be made from itself`);
      }
    },
    // Law 9: a market names it by what it is and where it trades, never by an identifier.
    displayName: (i, namer) =>
      isGoodTerms(i.terms) ? `${d.name}, ${namer.region(i.terms.region)}` : d.name,
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
    // E2, E2.a: down to the print when the print is below cost, and never back up. There is no
    // fairValueThroughIncome here: a holder of ordinary inventory is not a broker-dealer (E2.b).
    // A stock nobody has priced has nothing to write down to, which is not the same as nothing to
    // write down: it is carried at what it cost until a market says otherwise.
    carriedAt: (_i, lot, marked) =>
      marked.some && marked.value < lot.basisPerUnit ? some(marked.value) : none<PerPiece>(),
  };
}

/**
 * B3: a batch on its way to being a good. It is physical and it is carried at what it has cost so
 * far, and at nothing else: it has no market, so there is no net realisable value to write it down
 * to (E2), and no mark to write it up to. It never perishes in store, because it is not in store —
 * it is on the line, and what happens to it there is the yield (B4).
 */
export function wipProfile(d: GoodDecl): InstrumentKindProfile {
  const unit = goodUnitId(d.unit);
  return {
    id: wipKindId(d.subUnit),
    pricing: 'carriedAtCost',
    carry: 'cost',
    liabilityOfIssuer: false,
    // A tonne is nobody's promise, so there is no issuer for a price move to reach.
    owes: 'face',
    physical: true,
    unit: () => unit,
    // Bond N13, N13.a: stated because the clause says to state it even when the answer is nothing.
    // A tonne is a thing its holder owns, not a promise anybody made, so there is no issuer to fail
    // and no claim to rank — which is a different answer from "unsecured", not a missing one.
    ranking: () => ({
      seniority: 0,
      secured: [],
      claim: 'nothing: it is owned outright, and nobody promised it',
    }),
    validateTerms: (t) => {
      if (!isWipTerms(t)) {
        throw new InvalidRegistry('Goods B3', `${d.subUnit}: these are not the terms of a batch`);
      }
      if (t.subUnit !== d.subUnit) {
        throw new InvalidRegistry('Goods B3', `${d.subUnit} batch carries sub-unit ${t.subUnit}`);
      }
    },
    displayName: (i, namer) =>
      isWipTerms(i.terms)
        ? `${d.name} in progress, ${namer.region(i.terms.region)}`
        : `${d.name} in progress`,
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
  };
}


/**
 * E4: what perished this period leaves the world at the lot's own cost per unit, on the book of
 * whoever was holding it. The units identity is what keeps it honest (D5), and the charge lands on
 * the holder's equity because settlement debits the lots at what they carried.
 */
export function perish(ctx: MechanismContext, mine: ReadonlySet<InstrumentKindId>): void {
  for (const h of ctx.register.allHoldings()) {
    const inst = ctx.instruments.get(h.instrument);
    if (!mine.has(inst.kind) || !inst.status.live) continue;
    const terms = goodTerms(inst);
    const rate = ctx.params.ratio(terms.spoilage);
    if (rate === 0) continue;
    const held = sum(h.lots.map((l) => l.qty));
    /**
     * Housing A5, Capital Programme A6 (17e.2b): AND WHAT THE HOLDER SPENT ON KEEPING IT.
     *
     * Wear was a fact nobody could answer: a dwelling fell out of the stock every period and its
     * owner's only reply was to hold fewer dwellings. Where a good states what keeping a unit of it
     * takes, what the holder bought of that is CONSUMED here off its own shelf, and what perishes is
     * this good's own spoilage times the share it went without — the same read the plant side
     * answers its own wear with (Law 4: one fact, one writer, read twice). A good that states none —
     * grain in a silo — goes the whole of its spoilage, exactly as it did before.
     */
    const legs: Leg[] = [];
    let without = asRatio(1, 'a thing nothing keeps up goes without all of it');
    if (terms.upkeep !== null) {
      const part = goodId(terms.upkeep.subUnit, terms.region);
      const needed = upkeepFor(
        asQty(held.value, 'what it holds of it'),
        ctx.params.ratio(terms.upkeep.qtyPerUnitPerPeriod),
      );
      const have = ctx.instruments.has(part)
        ? ctx.register.free(h.holder, part)
        : asQty(0, 'it holds none of what would keep them');
      // Arithmetic impossibility and not a bound: a shelf with one plank on it mends one roof.
      const used = atMost(have, needed, 'it cannot use more than it holds');
      without = wentWithout(needed, used);
      if (used > 0) {
        legs.push({ kind: 'destroy', party: h.holder, instrument: part, qty: used, why: 'consumed' });
      }
    }
    // Law 8, E4: what perishes is whole pieces of the good, and for a cell whole pieces on each
    // member's own shelf. A fraction of a piece has not spoiled; it is still there, and it spoils
    // when enough of it has gone the same way.
    // 0f.1: `held` is the cell's TOTAL; what perishes is whole pieces of that, and the side is
    // derived from it.
    const gone = ctx.registry.deliverable(
      scale(scale(held.value, rate, `${inst.id} perished`), without, 'what its upkeep did not save'),
    );
    if (!material(gone, h.lots.length + 1, held.value) || gone <= 0) {
      // It kept them, and keeping them still cost it what the materials cost: the purchase settles
      // even in the period nothing is lost, which is the whole point of making it.
      if (legs.length > 0) {
        ctx.settle({ legs, cause: 'production', reason: `${h.holder} keeps ${inst.id}` });
      }
      continue;
    }
    const perMember = gone;
    legs.push({
      kind: 'destroy',
      party: h.holder,
      instrument: inst.id,
      qty: gone,
      why: 'perished',
    });
    const record = ctx.settle({
      legs,
      // The physical world acting on units it holds; the leg's own `why` says which way (E4).
      cause: 'production',
      reason: `${inst.id} perished in store`,
    });
    if (record.outcome !== 'settled') continue;
    // E3: the loss has a date, a size and an income line — it is what settlement charged the
    // holder's equity, read from the record rather than recomputed (Law 19).
    const charge = record.equity.find((e) => e.party === h.holder);
    ctx.record(
      'goods.perished',
      [h.holder, inst.id],
      {
        unitsPerMember: perMember,
        rate,
        // 17e.2b: and the share of its upkeep the holder went without, which is what let it go.
        without,
        // 0f.5, App A: no charge is no charge, not a charge of nothing.
        chargePerMember: charge === undefined ? null : charge.delta,
      },
      false,
    );
  }
}
