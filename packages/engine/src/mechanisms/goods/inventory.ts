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
import { CENT_TICK } from '../../registry/grid.js';
import type { InstrumentKindId } from '../../core/ids.js';
import { material, mul, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { cellSide, shareFor } from '../../ledger/settlement.js';
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
      marked.some && marked.value < lot.basisPerUnit ? some(marked.value) : none<number>(),
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
    const rate = ctx.params.get(goodTerms(inst).spoilage);
    if (rate === 0) continue;
    const held = sum(h.lots.map((l) => l.qty));
    const party = ctx.parties.get(h.holder);
    // Law 8, E4: what perishes is whole pieces of the good, and for a cell whole pieces on each
    // member's own shelf. A fraction of a piece has not spoiled; it is still there, and it spoils
    // when enough of it has gone the same way.
    const share = shareFor(ctx.registry, party, inst.unit, mul(held.value, rate, `${inst.id} perished`));
    const perMember = share.perMember;
    if (!material(perMember, h.lots.length + 1, held.value) || perMember <= 0) continue;
    const side = cellSide(party, perMember);
    const record = ctx.settle({
      legs: [
        {
          kind: 'destroy',
          party: h.holder,
          instrument: inst.id,
          qty: share.total,
          why: 'perished',
          fromCell: side === undefined ? none() : some(side),
        },
      ],
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
        chargePerMember: charge === undefined ? 0 : charge.delta,
      },
      false,
    );
  }
}
