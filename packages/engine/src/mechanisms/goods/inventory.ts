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
import type { InstrumentKindId } from '../../core/ids.js';
import { material, mul, sub, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import type { MechanismContext } from '../../world/context.js';
import type { GoodDecl } from './data.js';
import type { Lot } from '../../register/register.js';
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
    carry: 'cost',
    // A1: a tonne of grain is a real thing, not a promise; nobody owes it to anybody.
    liabilityOfIssuer: false,
    physical: true,
    unit: () => unit,
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
    revalue: (_i, lot, marked) =>
      marked < lot.basisPerUnit
        ? mul(lot.qty, sub(marked, lot.basisPerUnit, 'below cost'), 'write-down')
        : 0,
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
 * E5: what giving up `qty` units costs the holder, under the one lot flow this registry states —
 * first in, first out. It is a read of the lots themselves, which are where the cost lives (E1);
 * nothing here stores or re-derives a value beside them (Law 19).
 */
export function costOfDraw(lots: readonly Lot[], qty: number): number {
  const terms: number[] = [];
  let left = qty;
  for (const lot of lots) {
    if (left <= 0) break;
    const take = lot.qty < left ? lot.qty : left;
    terms.push(mul(take, lot.basisPerUnit, 'cost of the units drawn'));
    left = sub(left, take, 'units left to draw');
  }
  return sum(terms).value;
}

/**
 * B3, B4: the units of a batch that are due off the line — started at or before the period the
 * lead time names. The lots are the batch book, so this is a read of when each was started; with
 * one lead time per good the oldest lots are exactly the ones due, which is the order they are
 * drawn in anyway (E5).
 */
export function dueFromLine(lots: readonly Lot[], startedBy: number): number {
  return sum(lots.filter((l) => l.acquired <= startedBy).map((l) => l.qty)).value;
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
    const perMember = mul(held.value, rate, `${inst.id} perished`);
    // Law 7: a fraction of a dust-sized stock is dust, and destroying dust would book a leg for
    // arithmetic that never happened.
    if (!material(perMember, h.lots.length + 1, held.value)) continue;
    const party = ctx.parties.get(h.holder);
    const side = cellSide(party, perMember);
    const record = ctx.settle({
      legs: [
        {
          kind: 'destroy',
          party: h.holder,
          instrument: inst.id,
          qty: totalFor(party, perMember),
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
