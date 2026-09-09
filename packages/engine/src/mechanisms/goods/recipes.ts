/**
 * What a good is and what it is made from: the terms a good carries and the declared numbers in it.
 *
 * @spec Goods A1 Goods A2 Goods A2.a Goods A2.b Goods A2.c Goods A3 Goods A4 Goods B3 Goods E4 Law 2 Law 15 XI-14
 *
 * The RECIPE is public technology and it lives in the good's own terms, so anybody who can see the
 * instrument can see what it takes to make — a firm deciding what to produce, a buyer working out
 * what a shortage upstream means for it. The terms carry the STRUCTURE (which inputs, in what
 * units); each quantity in them is a declared TECHNOLOGY parameter named by the terms, so the
 * number itself lives once, in the parameter register, with its unit and its owner (XI-14).
 *
 * That unit is what makes A2.b enforceable: a recipe quantity is `tonnes per tonne of flour`, and a
 * number denominated in money — cost per unit of revenue, from which a physical draw would be
 * computed by dividing by a price — is refused at assembly, by name, in the module's seed.
 */
import { Missing } from '../../core/errors.js';
import {
  instrumentId,
  instrumentKindId,
  marketId,
  paramId,
  unitId,
  type InstrumentId,
  type InstrumentKindId,
  type MarketId,
  type ParamId,
  type RegionId,
  type UnitId,
} from '../../core/ids.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import type { ParamRegister } from '../../registry/params.js';
import type { GoodDecl } from './data.js';

/** A2.a: one input, in physical units per unit of output, at the number the register declares. */
export interface RecipeInput {
  readonly kind: InstrumentKindId;
  readonly qtyPerUnit: ParamId;
}

/** A2: the fixed way one unit of a good is made. Leontief: no substitution, no value share. */
export interface Recipe {
  readonly inputs: readonly RecipeInput[];
  /** A2.c: hours of labour per unit of output. */
  readonly labourHoursPerUnit: ParamId;
  /** B3: periods a batch is work in progress before it yields. */
  readonly leadTimePeriods: ParamId;
}

export interface GoodTerms extends Terms {
  /** A1, A4: the sub-unit this instrument is a quantity of. */
  readonly subUnit: string;
  /** C6: where it is made and sold, which is also whose money it is priced in. */
  readonly region: RegionId;
  /** A3, E4: the declared fraction of the stock that perishes each period. */
  readonly spoilage: ParamId;
  readonly recipe: Recipe;
}

export const goodKindId = (subUnit: string): InstrumentKindId =>
  instrumentKindId(`good.${subUnit}`);
export const goodId = (subUnit: string, region: RegionId): InstrumentId =>
  instrumentId(`good.${subUnit}.${region}`);
export const goodMarketId = (subUnit: string, region: RegionId): MarketId =>
  marketId(`mkt.good.${subUnit}.${region}`);
export const goodUnitId = (unit: string): UnitId => unitId(unit);

export const spoilageParam = (subUnit: string): ParamId => paramId(`goods.${subUnit}.spoilage`);
export const recipeParam = (output: string, input: string): ParamId =>
  paramId(`goods.${output}.recipe.${input}`);
export const labourParam = (subUnit: string): ParamId => paramId(`goods.${subUnit}.labourHours`);
export const leadTimeParam = (subUnit: string): ParamId => paramId(`goods.${subUnit}.leadTime`);

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
    recipe: {
      inputs: inputs.map((i) => ({
        kind: goodKindId(i.subUnit),
        qtyPerUnit: recipeParam(d.subUnit, i.subUnit),
      })),
      labourHoursPerUnit: labourParam(d.subUnit),
      leadTimePeriods: leadTimeParam(d.subUnit),
    },
  };
}

/**
 * Whether these terms are a good's. Structural, not a kind comparison: what makes a good a good is
 * that it says what it is made from and what it is measured in (Law 15).
 */
export function isGoodTerms(t: Terms): t is GoodTerms {
  return 'subUnit' in t && 'recipe' in t && 'spoilage' in t;
}

/** The terms of a good; asking any other instrument for them is a defect in the caller. */
export function goodTerms(i: Instrument): GoodTerms {
  if (!isGoodTerms(i.terms)) {
    throw new Missing('Goods A1', `${i.id} is not a good and has no recipe`, { instrument: i.id });
  }
  return i.terms;
}

/** A2.a: how much of one input a unit of output takes, read from the register at the moment asked. */
export function inputPerUnit(params: Pick<ParamRegister, 'get'>, input: RecipeInput): number {
  return params.get(input.qtyPerUnit);
}
