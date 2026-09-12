/**
 * A PHYSICAL LINE: a thing nobody promised, named by what it is and where it is, and made to a
 * stated recipe. The shape of it, and the grammar that names it.
 *
 * @spec Goods A1 Goods A2 Goods A2.a Goods A2.b Goods A2.c Goods A3 Goods A4 Goods B3 Goods B4 Goods E4 Law 4 Law 9 Law 15 XI-14
 *
 * WHY THIS IS KERNEL DATA AND NOT THE GOODS MODULE'S. Four other modules have to name the same
 * line: the firm that makes it, the household that eats it, the treasury that procures it and the
 * capital programme that builds out of it. Naming it from the module that owns the mechanism would
 * be four modules importing a fifth, which is the crossing `phoenix/no-cross-module-import` exists
 * to forbid — and it is the same decision ARCHITECTURE §4.9b already took for party kind ids, for
 * the same reason: an id is a NAME, and a module that does not own a kind still has to say it.
 *
 * The goods module keeps the MECHANISM — what spoils, what a batch costs, what comes off the line —
 * and it keeps the one function that reads a `GoodDecl`, because a declaration is that module's.
 *
 * The RECIPE is public technology and lives in the good's own terms, so anybody who can see the
 * instrument can see what it takes to make: a firm deciding what to produce, a buyer working out
 * what a shortage upstream means for it. The terms carry the STRUCTURE (which inputs, in what
 * units); each quantity in them is a declared TECHNOLOGY parameter named by the terms, so the
 * number itself lives once, in the parameter register, with its unit and its owner (XI-14). That
 * unit is what makes A2.b enforceable: a recipe quantity is `tonnes per tonne of flour`, and a
 * number denominated in money is refused at assembly, by name.
 */
import { Missing } from '../core/errors.js';
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
} from '../core/ids.js';
import type { Calendar } from '../calendar/calendar.js';
import type { Event } from '../journal/journal.js';
import { compareCivil, dayNumber, formatCivil, type Civil } from '../calendar/civil.js';
import { div, mul, sub, sum } from '../core/num.js';
import { none, some, type Option } from '../core/option.js';
import type { Instrument, InstrumentsReads, Terms } from '../register/instruments.js';
import type { Holding } from '../register/register.js';
import type { ParamRegister } from './params.js';
import type { Registry } from './registry.js';

/**
 * What reading somebody's plant needs OF them: where they are, what they hold, and the calendar
 * the dates on it are read against. Stated here as a shape rather than taken as a
 * `ParticipantView`, so that `registry/` imports no `world/` — a registry row is data and a read
 * over data is arithmetic (ARCHITECTURE 4.9b). Every `ParticipantView` satisfies it.
 */
export interface PlantHolder {
  readonly self: { readonly region: RegionId };
  readonly calendar: Calendar;
  readonly instruments: InstrumentsReads;
  holdings(): readonly Holding[];
}

/** A2.a: one input, in physical units per unit of output, at the number the register declares. */
export interface RecipeInput {
  /** A1, A4: the thing itself. Its kind and its instrument in a region are both read off this. */
  readonly subUnit: string;
  readonly qtyPerUnit: ParamId;
}

/**
 * A2.c, Capital Programme A2, A4: the capital services one unit takes, per kind of plant. It is in
 * the recipe because it is technology about making the thing, and it is a LIST because a use that
 * needs several kinds is limited by the scarcest of them (Capital Programme A4).
 */
export interface RecipePlant {
  readonly capitalKind: string;
  readonly unitsPerUnitPerPeriod: ParamId;
}

/** A2: the fixed way one unit of a good is made. Leontief: no substitution, no value share. */
export interface Recipe {
  readonly inputs: readonly RecipeInput[];
  /** A2.c: hours of labour per unit of output. */
  readonly labourHoursPerUnit: ParamId;
  /** A2.c: the plant a unit of it takes, per kind. Empty is a line that needs none. */
  readonly plant: readonly RecipePlant[];
  /** B4: the fraction of what is started that is finished; the rest is scrap. */
  readonly yieldRate: ParamId;
  /** B3: periods a batch is work in progress before it yields. */
  readonly leadTimePeriods: ParamId;
  /**
   * Goods B4, Commodities Spot B3: THE PHYSICAL FACTS THIS LINE'S YIELD STANDS IN, by the name the
   * environment publishes them under (`registry/environment.ts`). A crop stands in the weather; a
   * mill and an oven do not, and saying so is different from saying their yield is high. Empty is a
   * real answer — a line made indoors — and it is the answer for most of them.
   *
   * It is a list of NAMES and not an import: the module that owns the weather is not the module
   * that owns the recipe, and a name is how a module says a thing it does not own (4.9b).
   */
  readonly exposedTo: readonly string[];
  /**
   * 13c.1, Goods B4: THE GROUND THIS LINE STANDS ON, by the name the map declares it under, or null
   * for a line made indoors — a mill and an oven stand on nothing, and that is a real answer rather
   * than a quality of one (Law 16).
   *
   * It is a NAME and not an import, which is how a module says a thing it does not own (4.9b).
   */
  readonly standsOn: string | null;
}

/**
 * B3: a batch between the input and the output. It is a physical thing on its maker's book, carried
 * at what it has cost so far, in the unit of what it will become; it has no market, because nobody
 * buys half a loaf of bread (E2's net realisable value has nothing to read, so it is carried at
 * cost and never written down). Its lots ARE the batch book: each carries what it cost and the
 * period it was started, so what is due to come off the line is a read of the register (Law 19).
 */
export interface WipTerms extends Terms {
  readonly subUnit: string;
  readonly region: RegionId;
  /** What this batch becomes: the good whose recipe made it and whose units it will be. */
  readonly output: InstrumentId;
}

export interface GoodTerms extends Terms {
  /** A1, A4: the sub-unit this instrument is a quantity of. */
  readonly subUnit: string;
  /** C6: where it is made and sold, which is also whose money it is priced in. */
  readonly region: RegionId;
  /** A3, E4: the declared fraction of the stock that perishes each period. */
  readonly spoilage: ParamId;
  /**
   * Commodities Spot A3, D3: HOW MUCH COVERED SPACE ONE UNIT OF THIS TAKES UP FOR A PERIOD, as the
   * parameter that says it. Null is a line nobody stores in bulk — one that turns over inside the
   * period it is made in, which its own spoilage already says — and that is a real answer rather
   * than a capacity somebody set to a large number (Law 6).
   *
   * It is on the GOOD and not on the holder, because how much room a tonne takes is a fact about
   * the tonne. What a holder has room for is a fact about the holder, and it is plant.
   */
  readonly storagePerUnit: ParamId | null;
  /**
   * A1, Freight A3, 13c.2: whether a unit of this can be somewhere other than where it was made.
   * Freight reads it: what cannot be loaded is never offered a hold and never reaches a transit
   * instrument, so a service price is LOCAL by the technology of the thing rather than by a rule.
   */
  readonly portable: boolean;
  readonly recipe: Recipe;
}

export const goodKindId = (subUnit: string): InstrumentKindId =>
  instrumentKindId(`good.${subUnit}`);
export const wipKindId = (subUnit: string): InstrumentKindId => instrumentKindId(`wip.${subUnit}`);
export const wipId = (subUnit: string, region: RegionId): InstrumentId =>
  instrumentId(`wip.${subUnit}.${region}`);
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
export const plantParam = (subUnit: string, capitalKind: string): ParamId =>
  paramId(`goods.${subUnit}.plant.${capitalKind}`);
export const yieldParam = (subUnit: string): ParamId => paramId(`goods.${subUnit}.yield`);
export const storageParam = (subUnit: string): ParamId =>
  paramId(`goods.${subUnit}.storagePerUnit`);

/** Whether these terms are a batch's: it says what it will become, and a good never does. */
export function isWipTerms(t: Terms): t is WipTerms {
  return 'output' in t && 'subUnit' in t && !('recipe' in t);
}

/** The terms of a batch; asking anything else for them is a defect in the caller. */
export function wipTerms(i: Instrument): WipTerms {
  if (!isWipTerms(i.terms)) {
    throw new Missing('Goods B3', `${i.id} is not work in progress`, { instrument: i.id });
  }
  return i.terms;
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
export function inputPerUnit(params: Pick<ParamRegister, 'ratio'>, input: RecipeInput): number {
  return params.ratio(input.qtyPerUnit);
}

/* ------------------------------------------------------------------------------------------------
 * PLANT: the other physical line, and the same argument.
 *
 * @spec Capital Programme A1 Capital Programme A2 Capital Programme A3 Capital Programme A4 Capital Programme A4.b Capital Programme A4.c Capital Programme A6 Capital Programme C1 Capital Programme C3 Capital Programme D2 Capital Programme D3 Goods A2.c Law 4 Law 9 Law 15
 *
 * A vintage of plant is a thing nobody promised, named by what it is, where it is and when it went
 * into service. Three modules have to name and read it: the capital programme that builds and wears
 * it, the firm that produces with it, and the goods recipe that says how much of it a unit takes.
 * So the SHAPE, the NAMING and the READS are here with the goods' — the capital programme keeps the
 * mechanism (the profile, the vintage it opens, the wear it books, the phases).
 * ---------------------------------------------------------------------------------------------- */

/** A6: what a vintage is. Its own kind, its own place, its own service date and its own life. */
export interface PlantTerms extends Terms {
  readonly capitalKind: string;
  readonly region: RegionId;
  /** A6: the date this vintage went into service and started producing. */
  readonly serviceDate: Civil;
  /** A4.b, A6: the date it is worn out; the charge stops because the plant is gone. */
  readonly retires: Civil;
}

export const plantKindId = (capitalKind: string): InstrumentKindId =>
  instrumentKindId(`plant.${capitalKind}`);
export const plantUnitId = (capitalKind: string): UnitId => unitId(`plant.${capitalKind}`);

/**
 * Commodities Spot A3, A4: THE KIND OF PLANT COVERED SPACE IS. The id is a NAME and the kernel owns
 * it, for the same reason it owns a good's: the commodities module owns the MECHANISM — what a silo
 * costs, how long it stands, the market in what it lets — and the firm that has to know whether its
 * line needs room still has to spell it (ARCHITECTURE 4.9b).
 */
export const STORAGE = 'storage';

/**
 * Law 8: THE CONVERSION IS AT THE BOUNDARY AND IT IS STATED ONCE. `storagePerUnit` is declared in
 * NAMED units on both sides — units of covered space per tonne of grain — and the register counts
 * both in PIECES, a thousand to the tonne and one to the space unit. So a count of pieces of the
 * thing is taken back to the tonnes it is, multiplied by the declared ratio, and put back into
 * pieces of space. Any other spelling is off by whatever the two subdivisions differ by, silently —
 * and two spellings of it would differ by whatever their rounding differed by, which is a holder
 * short of room through arithmetic alone (Law 4).
 */
export interface SpaceReads {
  readonly registry: Pick<Registry, 'subdivision' | 'pieces'>;
}

export function spaceFor(reads: SpaceReads, unit: UnitId, pieces: number, perUnit: number): number {
  const named = div(pieces, reads.registry.subdivision(unit), 'what it holds, in its own named unit');
  return reads.registry.pieces(plantUnitId(STORAGE), mul(named, perUnit, 'the space that takes'));
}

/**
 * The same ratio, UNROUNDED, for the arithmetic that divides rather than totals: how many pieces of
 * space one piece of the thing takes. It is a ratio and not a quantity, so it is not on any grid —
 * a kilo of grain takes a thousandth of a unit of space, and rounding that to a whole piece of
 * space makes it nothing and makes room bind nothing (Law 8: what is a COUNT lands on the grid;
 * what is a RATIO between two counts does not).
 */
export function spacePerPiece(reads: SpaceReads, unit: UnitId, perUnit: number): number {
  return div(
    mul(perUnit, reads.registry.subdivision(plantUnitId(STORAGE)), 'in pieces of space'),
    reads.registry.subdivision(unit),
    'per piece of the thing',
  );
}
export const plantVintageId = (
  capitalKind: string,
  region: RegionId,
  serviceDate: Civil,
): InstrumentId => instrumentId(`plant.${capitalKind}.${region}.${formatCivil(serviceDate)}`);
export const plantMarketId = (
  capitalKind: string,
  region: RegionId,
  serviceDate: Civil,
): MarketId => marketId(`mkt.plant.${capitalKind}.${region}.${formatCivil(serviceDate)}`);

export function isPlantTerms(t: Terms): t is PlantTerms {
  return 'capitalKind' in t && 'serviceDate' in t && 'retires' in t;
}

/** The terms of a vintage; asking anything else for them is a defect in the caller. */
export function plantTerms(i: Instrument): PlantTerms {
  if (!isPlantTerms(i.terms)) {
    throw new Missing('Capital Programme A6', `${i.id} is not a vintage of plant`, {
      instrument: i.id,
    });
  }
  return i.terms;
}

/** Whether this instrument is a vintage of plant at all, without asking it for terms it may not have. */
export function isPlant(i: Instrument): boolean {
  return isPlantTerms(i.terms);
}

/**
 * A6, A4.b: how many periods of service this vintage has left on a date, on the one calendar. It is
 * a count of periods derived from two DATES (Money G3.a), never a counter anybody decrements.
 */
export function serviceLeft(terms: PlantTerms, on: Civil, calendar: Calendar): number {
  const days = sub(dayNumber(terms.retires), dayNumber(on), 'days of service left');
  return div(days, calendar.periodDays, 'periods of service left');
}

/**
 * A3, A5: what a lot of a vintage is carried at after this period's wear.
 *
 * Straight line over what is LEFT: a lot with n periods of service ahead of it gives up one of them
 * this period, so it keeps (n-1)/n of what it was carried at. A lot bought second-hand starts from
 * what its buyer paid and runs out at the same date as the rest of the vintage, which is what makes
 * this per-lot rather than per-line (A6: its own cost, one service date). The last period of a
 * vintage's life takes it to nothing, so the charge stops when the plant is gone.
 */
export function carriedAfterWear(
  terms: PlantTerms,
  basisPerUnit: number,
  on: Civil,
  calendar: Calendar,
): Option<number> {
  if (basisPerUnit === 0) return none<number>();
  const left = serviceLeft(terms, on, calendar);
  if (left <= 1) return some(0);
  return some(mul(basisPerUnit, div(sub(left, 1, 'periods after this one'), left, 'what is left of its life'), 'what it is carried at now'));
}

/**
 * A3, A6: what one unit of a vintage costs its holder in wear this period — the charge that follows
 * from the same schedule the carrying value falls on. It is a read of the two, never a third number.
 */
export function wearPerUnit(
  terms: PlantTerms,
  basisPerUnit: number,
  on: Civil,
  calendar: Calendar,
): number {
  const after = carriedAfterWear(terms, basisPerUnit, on, calendar);
  return after.some ? sub(basisPerUnit, after.value, 'what a unit of it wears out by') : 0;
}

/** A6: whether this vintage is worn out on a date, and so leaves the register. */
export function wornOut(terms: PlantTerms, on: Civil): boolean {
  return compareCivil(on, terms.retires) >= 0;
}


/* ------------------------------------------------------------------------------------------------
 * WHAT A FIRM'S PLANT LETS IT MAKE, what that plant costs it to use, and what it is short of.
 *
 * A2: capacity is a FUNCTION OF THE STOCK, and output is limited by it. A4: the stock is specific in
 * kind, so a use that needs several kinds is limited by the SCARCEST of them — which is why capital
 * bought for one purpose is worth less to another and why misallocation costs something.
 *
 * Everything here is a read over the vintages a party actually holds (Law 19). Nothing is stored:
 * there is no capacity field, no accumulated-depreciation total and no utilisation number anybody
 * writes down. Utilisation in particular is a read of the OUTCOME against capacity (Goods B1.d) and
 * never an input to the decision that produced the outcome.
 * ---------------------------------------------------------------------------------------------- */

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
export function vintagesHeld(view: PlantHolder, on: Civil): HeldVintage[] {
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

export interface CapitalKindDecl {
  /** A4: the kind. Plant of one kind is not plant of another, at any price. */
  readonly id: string;
  readonly name: string;
  /** A1, A4: the unit its stock is counted in. Never added to the stock of another kind. */
  readonly unit: string;
  /**
   * A4.b, A4.c, C1: the good a unit of it is made from. Investment is a purchase from a named
   * capital-goods producer, and this is the line that produces it.
   */
  readonly madeFrom: string;
  /**
   * A4.b, A6: how many periods a vintage of it works before it is worn out and leaves the register.
   * It is the life that makes the good a capital good, and it is technology.
   */
  readonly usefulLifePeriods: number;
  /** C3: periods between the good arriving and the plant working. The asset is built, then it works. */
  readonly buildLagPeriods: number;
  /**
   * Commodities Spot B3, Freight B4: WHAT THIS KIND OF STRUCTURE IS BUILT FOR, as a multiple of the
   * wind an ordinary period brings. A shed and a silo are not built to the same standard, and what
   * a storm takes down is the difference between what stood over it and what it was built for.
   *
   * It is TECHNOLOGY — a fact about how the thing is made — and it is not a threshold: nothing
   * happens AT it. What survives is `exp(-(wind / standard) ^ hardness)`, which is positive at
   * every wind and never one, so an ordinary period takes a little and a storm takes most (Law 6).
   */
  readonly standsWind: number;
  /**
   * How sharply the loss grows with the wind. Wind damage is not linear in wind: doubling it is far
   * more than twice the loss, because what fails is what the load exceeded. Technology.
   */
  readonly windHardness: number;
  /**
   * 13c.1, Goods B4: THE GROUND A UNIT OF IT STANDS ON, in square kilometres. It is what makes a
   * place fill up — the more plant a region carries, the poorer the ground the next unit stands on —
   * and null is plant that takes no ground worth counting, which is what a hull at sea is.
   */
  readonly landPerUnit: number | null;
  readonly why: string;
}

/**
 * A period is a week (docs/ARCHITECTURE.md 4.7), so a life below is a count of weeks.
 *
 * THE KINDS EVERY LINE MIGHT NEED. Storage (13c) and hulls (13c step 5) are declared by the modules
 * whose behaviour they are; these three are not any module's — a works, a surgery, a classroom and a
 * shop are one kind of structure to the arithmetic that wears it out, and declaring one per trade
 * would be one fact written out many times (Law 2). The mechanism takes the SCARCEST of the kinds a
 * recipe names (A4) and never asks what a kind is for.
 */
export const CAPITAL_KINDS: readonly CapitalKindDecl[] = [
  {
    id: 'machinery',
    name: 'machinery',
    unit: 'machines in service',
    madeFrom: 'machine',
    usefulLifePeriods: 156,
    buildLagPeriods: 2,
    // Machinery lives in a shed and moves about: built for about three times an ordinary week's
    // wind, and what fails above that fails quickly (13c, Commodities Spot B3).
    standsWind: 3,
    windHardness: 6,
    // 13c.1: a machine and the yard around it. Small, but it is what makes a place fill up.
    landPerUnit: 0.02,
    why: 'A machine works for three years and then it is scrap. Three years is short enough that a firm which stops investing loses its capacity inside a run, and long enough that the spend and the capacity it buys are separated by more than a cycle — which is what makes investment a commitment rather than a purchase.',
  },
  {
    id: 'premises',
    name: 'premises',
    unit: 'premises in use',
    // 13c.2: built, not manufactured, which is why a shop cannot be ordered from a machine works
    // and why a town that wants more of them has to wait for a building to go up.
    madeFrom: 'building',
    // Forty years. It is the longest life this world has, and it is why the cost of being in a place
    // is so much slower to answer a shortage than the cost of making anything in it.
    usefulLifePeriods: 2080,
    buildLagPeriods: 12,
    // A building stands up to far more than a machine shed, and what takes one down is rare.
    standsWind: 8,
    windHardness: 7,
    // 13c.2, 13c.1: the ground a shop, a surgery or a restaurant stands on. It is small per unit and
    // there are a great many of them, which is what makes a town fill up before a coalfield does.
    landPerUnit: 0.004,
    why: 'Freight A4 for a building rather than a hull: a service is made where it is bought, so being somewhere is the capital a service business has. It is the same arithmetic for a shop, a surgery, a classroom and an office, and five kinds would be five copies of one fact (Law 2, Law 15).',
  },
  {
    id: 'fleet',
    name: 'road fleet',
    unit: 'vehicles in service',
    madeFrom: 'vehicle',
    // Ten years, and a lorry is worked hard for every one of them.
    usefulLifePeriods: 520,
    buildLagPeriods: 2,
    // Out in the weather the whole time, and light: a gale takes more of these than of a building.
    standsWind: 4,
    windHardness: 6,
    // 13c.2: a yard and a parking space. Small, and there are many.
    landPerUnit: 0.0004,
    why: 'What carries a thing the last few miles and what carries a person to work. It is the plant of the lines that move things WITHIN a place, and it is not a hull: a voyage is freight’s, it is built (13c.1), and a second writer of the same fact would be a Law 4 defect.',
  },
];

/** The declaration of a capital kind by id; asking for one this world does not have is a defect. */
export function capitalKindOf(
  rows: readonly CapitalKindDecl[],
  id: string,
): CapitalKindDecl | undefined {
  return rows.find((r) => r.id === id);
}

/**
 * A4, A4.b, C3: the two numbers a kind of capital states about itself, named. A firm weighing a
 * project reads the life of what it would buy and the capital programme reads the same number to
 * wear it out — one parameter, one spelling, and neither module learns the other's (Law 4).
 */
/**
 * Commodities Spot D3, B4: THE SESSION THE ROOM CLEARED IN, and what it cleared at.
 *
 * The name and the read are the kernel's for the same reason every other crossing name is: a firm
 * deciding whether to hold a tonne or sell it needs what holding costs, and the module that runs
 * the storage session is not the module that owns the firm (4.9b). The commodities module owns the
 * session; this is how anybody else reads what it said.
 */
export const STORAGE_SESSION = 'commodities.storage';

/**
 * What reading a session needs: this period, and the public events the reader may see. A party
 * reads it through its own view's public door (Observer A3) and a module through the journal, and
 * both satisfy this — which is what keeps the read one read (Law 4).
 */
export interface SessionReads {
  lastPublic(kind: string): Option<Event>;
}

/**
 * What a piece of room cost for a period in this region, or none because no session cleared — which
 * is a real answer and not a zero: a world where nobody let any room is not a world where room is
 * free, it is one where a holder who needed room did not get it (Law 6, Appendix A).
 */
export function storageRateIn(reads: SessionReads, region: RegionId): number | undefined {
  const said = reads.lastPublic(STORAGE_SESSION);
  if (!said.some) return undefined;
  const byRegion = said.value.data['byRegion'];
  if (typeof byRegion !== 'object' || byRegion === null) return undefined;
  const here = (byRegion as Record<string, unknown>)[String(region)];
  if (typeof here !== 'object' || here === null) return undefined;
  const rate = (here as Record<string, unknown>)['rate'];
  return typeof rate === 'number' ? rate : undefined;
}

export const standsWindParam = (capitalKind: string): ParamId =>
  paramId(`plant.standsWind.${capitalKind}`);
/** 13c.1: the ground a unit of this kind stands on, under its own name (XI-14). */
export const landPerUnitParam = (capitalKind: string): ParamId =>
  paramId(`capital.${capitalKind}.landPerUnit`);
export const windHardnessParam = (capitalKind: string): ParamId =>
  paramId(`plant.windHardness.${capitalKind}`);
export const lifeParam = (capitalKind: string): ParamId =>
  paramId(`plant.usefulLife.${capitalKind}`);
export const buildLagParam = (capitalKind: string): ParamId =>
  paramId(`plant.buildLag.${capitalKind}`);
