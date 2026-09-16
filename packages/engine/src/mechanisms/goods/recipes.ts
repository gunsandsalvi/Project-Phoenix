/**
 * What a good's DECLARATION becomes: the terms it carries and the numbers named in them.
 *
 * @spec Goods A2 Goods A2.b Goods B3 Law 15
 *
 * The SHAPE of a physical line — its terms, its recipe, the grammar that names it and the reads
 * that recognise it — is kernel data (`registry/physical.ts`), because four other modules have to
 * name the same line. What is here is what only this module can do: turn a `GoodDecl` into those
 * terms. Everything else is re-exported, so a good's shape has one definition and one spelling.
 */
export {
  goodId,
  goodKindId,
  goodMarketId,
  goodTerms,
  goodUnitId,
  inputPerUnit,
  isGoodTerms,
  isWipTerms,
  labourParam,
  learningParam,
  leadTimeParam,
  plantParam,
  recipeParam,
  spoilageParam,
  storageParam,
  wipId,
  wipKindId,
  wipTerms,
  yieldParam,
  type GoodTerms,
  type Recipe,
  type RecipeInput,
  type RecipePlant,
  type WipTerms,
} from '../../registry/physical.js';

import { OVERHEADS } from './data.js';
import type { RegionId } from '../../core/ids.js';
import {
  goodId,
  goodKindId,
  labourParam,
  learningParam,
  leadTimeParam,
  plantParam,
  recipeParam,
  spoilageParam,
  storageParam,
  wipKindId,
  yieldParam,
  type GoodTerms,
  type WipTerms,
  overheadParam,
  type RecipeOverhead,
} from '../../registry/physical.js';
import type { GoodDecl } from './data.js';

/** A2.b: what a recipe quantity is measured in. Physical on both sides, and nothing else. */
export const recipeUnit = (input: GoodDecl, output: GoodDecl): string =>
  `${input.unit} of ${input.subUnit} per ${output.unit} of ${output.subUnit}`;

/** The terms of a good, built from its declaration (Law 15: the data says, the code reads). */
export function goodTermsOf(d: GoodDecl, region: RegionId, inputs: readonly GoodDecl[]): GoodTerms {
  return {
    kind: goodKindId(d.subUnit),
    subUnit: d.subUnit,
    region,
    spoilage: spoilageParam(d.subUnit),
    // A3, D3: what a unit of it takes up while it waits, or nothing because nobody stores it.
    storagePerUnit: d.storagePerUnit === null ? null : storageParam(d.subUnit),
    // 13c.2: whether it can be loaded at all. A service is made where it is bought.
    portable: d.portable,
    recipe: {
      inputs: inputs.map((i) => ({
        subUnit: i.subUnit,
        qtyPerUnit: recipeParam(d.subUnit, i.subUnit),
      })),
      // A2.a, item 7b: what having the plant costs per period, whatever it makes. Derived from the
      // plant this line already declares, so a line that needs no premises buys no cleaning.
      overheads: overheadsFor(d),
      labourHoursPerUnit: labourParam(d.subUnit),
      learningRate: learningParam(d.subUnit),
      plant: d.plant.map((r) => ({
        capitalKind: r.capitalKind,
        unitsPerUnitPerPeriod: plantParam(d.subUnit, r.capitalKind),
        ...(r.leased === true ? { leased: true } : {}),
      })),
      yieldRate: yieldParam(d.subUnit),
      // B4: the facts this line's yield stands in, carried as the names the environment publishes.
      exposedTo: d.exposedTo,
      // 13c.1: and the ground it stands on, which is where the region stops being a label.
      standsOn: d.standsOn,
      leadTimePeriods: leadTimeParam(d.subUnit),
    },
  };
}

/**
 * A2.a, item 7b: the services this line buys because it HAS the plant — one per (service, kind of
 * plant) the line declares, and none for the service lines themselves, which would otherwise clean
 * and support each other into a loop with nobody outside it.
 */
export function overheadsFor(d: GoodDecl): RecipeOverhead[] {
  const out: RecipeOverhead[] = [];
  for (const o of OVERHEADS) {
    if (d.subUnit === o.subUnit) continue;
    if (!d.plant.some((r) => r.capitalKind === o.capitalKind)) continue;
    out.push({
      subUnit: o.subUnit,
      capitalKind: o.capitalKind,
      qtyPerPlantUnitPerPeriod: overheadParam(d.subUnit, o.subUnit, o.capitalKind),
    });
  }
  return out;
}

/** B3: the terms of a batch of one good, in one region, on its way to being that good. */
export function wipTermsOf(d: GoodDecl, region: RegionId): WipTerms {
  return {
    kind: wipKindId(d.subUnit),
    subUnit: d.subUnit,
    region,
    output: goodId(d.subUnit, region),
  };
}

