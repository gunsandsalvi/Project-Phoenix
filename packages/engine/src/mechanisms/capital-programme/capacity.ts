/**
 * What a firm's plant lets it make, what that plant costs it to use, and what it is short of.
 *
 * @spec Capital Programme A2 Capital Programme A4 Capital Programme A6 Capital Programme B3 Capital Programme D1 Capital Programme D2 Capital Programme D4 Goods B1.a Goods B1.d Goods B5 Goods G4 Law 19
 *
 * A2: capacity is a FUNCTION OF THE STOCK, and output is limited by it. A4: the stock is specific in
 * kind, so a use that needs several kinds is limited by the SCARCEST of them — which is why capital
 * bought for one purpose is worth less to another and why misallocation costs something.
 *
 * Everything here is a read over the vintages a party actually holds (Law 19). Nothing is stored:
 * there is no capacity field, no accumulated-depreciation total and no utilisation number anybody
 * writes down. Utilisation in particular is a read of the OUTCOME against capacity (Goods B1.d) and
 * never an input to the decision that produced the outcome.
 */
import type { Civil } from '../../calendar/civil.js';
import { div, mul, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import { isPlant, plantTerms, serviceLeft, wearPerUnit } from './plant.js';

/** What one unit of an output per period takes, in plant of one kind (A2, Goods A2.c). */
export interface PlantNeed {
  readonly capitalKind: string;
  /** Units of plant of that kind that make one unit of output per period. */
  readonly unitsPerUnitPerPeriod: number;
}

/** One vintage a party holds, as the register has it (A6). */
export interface HeldVintage {
  readonly instrument: string;
  readonly capitalKind: string;
  readonly units: number;
  readonly basisPerUnit: number;
  /** A6: periods of service it has left, from the two dates on it. */
  readonly periodsLeft: number;
  /** A3: what a unit of it wears out by this period. */
  readonly wearPerUnit: number;
}

/** A6, D2: every vintage of plant this party holds in its own region, with what is left of each. */
export function vintagesHeld(view: ParticipantView, on: Civil): HeldVintage[] {
  const out: HeldVintage[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !isPlant(i)) continue;
    const terms = plantTerms(i);
    if (terms.region !== view.self.region) continue;
    for (const lot of h.lots) {
      if (lot.qty <= 0) continue;
      out.push({
        instrument: i.id,
        capitalKind: terms.capitalKind,
        units: lot.qty,
        basisPerUnit: lot.basisPerUnit,
        periodsLeft: serviceLeft(terms, on, view.calendar),
        wearPerUnit: wearPerUnit(terms, lot.basisPerUnit, on, view.calendar),
      });
    }
  }
  return out;
}

/** A2, A4: the units of one kind of plant this party has in service. */
export function plantHeld(vintages: readonly HeldVintage[], capitalKind: string): number {
  return sum(vintages.filter((v) => v.capitalKind === capitalKind).map((v) => v.units)).value;
}

/**
 * A3, Goods B5: what a unit of one kind of plant costs its holder in wear, per period, averaged
 * over what it holds. It is the capital charge that belongs in unit cost, and it is the SAME number
 * the write-down takes off the stock (A3: one schedule, charged in both places).
 */
export function wearPerPlantUnit(
  vintages: readonly HeldVintage[],
  capitalKind: string,
): Option<number> {
  const mine = vintages.filter((v) => v.capitalKind === capitalKind);
  const units = sum(mine.map((v) => v.units)).value;
  if (units <= 0) return none<number>();
  const charge = sum(mine.map((v) => mul(v.units, v.wearPerUnit, 'what this vintage wears out by')));
  return some(div(charge.value, units, 'what a unit of plant costs it per period'));
}

/** A2, A4: what the stock lets it make per period, and which kind of plant is the scarcest. */
export interface Capacity {
  readonly perPeriod: number;
  /** A4: the kind that binds. It is what an extra unit of output has to be bought in. */
  readonly binding: string;
}

/**
 * A2, A4, Goods B1.a: capacity per period, which is the scarcest of the kinds the recipe names.
 *
 * A line whose recipe names NO plant has no capital constraint, and it says so rather than
 * returning a large number: a constraint that does not exist is not a constraint with a big value
 * in it (Law 6).
 */
export function capacityFrom(
  needs: readonly PlantNeed[],
  vintages: readonly HeldVintage[],
): Option<Capacity> {
  let scarcest: Capacity | undefined;
  for (const need of needs) {
    if (need.unitsPerUnitPerPeriod <= 0) continue;
    const held = plantHeld(vintages, need.capitalKind);
    const perPeriod = div(held, need.unitsPerUnitPerPeriod, 'what this kind of plant lets it make');
    if (scarcest === undefined || perPeriod < scarcest.perPeriod) {
      scarcest = { perPeriod, binding: need.capitalKind };
    }
  }
  return scarcest === undefined ? none<Capacity>() : some(scarcest);
}

/**
 * Goods B1.d, D4, Goods G4: utilisation is a READ of the outcome against capacity. It is here so
 * that whoever reports it reports the one derivation; nothing decides anything with it.
 */
export function utilisation(output: number, capacity: number): Option<number> {
  if (capacity <= 0) return none<number>();
  return some(div(output, capacity, 'how much of its capacity it used'));
}

/**
 * Goods B5, A3: the capital charge in the cost of one unit of output — the plant a unit takes,
 * times what a unit of that plant wears out by in a period. A firm with none of a kind it needs has
 * no charge to state, because it has no plant to wear out and is making nothing.
 */
export function capitalChargePerUnit(
  needs: readonly PlantNeed[],
  vintages: readonly HeldVintage[],
): Option<number> {
  if (needs.length === 0) return some(0);
  const terms: number[] = [];
  for (const need of needs) {
    const per = wearPerPlantUnit(vintages, need.capitalKind);
    if (!per.some) return none<number>();
    terms.push(mul(need.unitsPerUnitPerPeriod, per.value, 'what the plant a unit takes costs'));
  }
  return some(sum(terms).value);
}
