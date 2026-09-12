/**
 * The goods registry: what is made in this world, out of what, and what happens to it in store.
 *
 * @spec Goods A1 Goods A2 Goods A2.a Goods A2.b Goods A2.c Goods A3 Goods A4 Goods E4 Law 15
 *
 * Data only (Law 15). Every number here is declared as a TECHNOLOGY parameter by the module, with
 * its own physical unit, and every recipe quantity is PHYSICAL — so many tonnes of the input per
 * tonne of the output (A2.a), never a share of cost or of revenue (A2.b). That is the whole point:
 * a miller facing dearer grain draws the same tonnage and takes the margin, which is what makes a
 * supply shock transmit instead of being smoothed away by a substitution nobody can observe.
 *
 * THE CHAIN IS THE POINT. Extraction stands on land and labour alone — grain, timber, iron, coal,
 * crude, limestone, bauxite — so nothing upstream of it can be short and there is no cycle in the
 * input-output graph. Everything above it is made of what is below, up to five deep
 * (petroleum -> crude -> chemicals -> plastic -> vehicle), and several industries meet in one
 * thing: a vehicle takes steel, aluminium, plastic, glass and rubber; a building takes concrete,
 * steel, lumber and glass. Fertiliser closes the one loop worth having — chemicals and power go
 * back into the field — so an energy shock reaches the price of bread by two paths.
 *
 * WHAT IS NOT HERE, and deliberately: services. A consultancy has no bill of materials and nobody
 * buys an "engagement"; what it sells is LABOUR, with plant, to a named buyer, and a good with a
 * recipe is the wrong shape for it entirely. Retail and distribution is not a good either — it is a
 * TRADER'S MARGIN on somebody else's goods, which is what the location basis and inventory already
 * produce. Both are named to the items that build them rather than faked as manufactures.
 */

/**
 * A2.c: the capital services one unit of output takes — units of plant of a named kind that let a
 * line start one unit per period. It is technology like the recipe's tonnages and its hours, and it
 * is what makes capacity a function of the stock (Capital Programme A2) rather than a number.
 */
export interface RecipePlantDecl {
  /** Capital Programme A4: the kind of plant. Plant of one kind is not plant of another. */
  readonly capitalKind: string;
  readonly unitsPerUnitPerPeriod: number;
  readonly why: string;
}

export interface RecipeInputDecl {
  readonly subUnit: string;
  /** A2.a: physical units of the input consumed per unit of output. Fixed; no substitution. */
  readonly qtyPerUnit: number;
  readonly why: string;
}

export interface GoodDecl {
  /** A1, A4: the sub-unit — the thing itself, homogeneous within it. */
  readonly subUnit: string;
  /** A1: its physical unit of measure. */
  readonly unit: string;
  /** The name a market would use for it. */
  readonly name: string;
  /** A3, E4: the fraction of what is in store that perishes each period. */
  readonly spoilagePerPeriod: number;
  readonly spoilageWhy: string;
  /**
   * A1, Freight A3, 13c.2: WHETHER A UNIT OF THIS CAN BE SOMEWHERE OTHER THAN WHERE IT WAS MADE.
   *
   * Technology, and the one fact that divides a manufacture from a service. A tonne can be loaded;
   * a diagnosis, a lesson, a night's lodging and a haircut cannot, at any price. It is declared on
   * the GOOD and never on the holder, because whether a thing can be moved is a fact about the
   * thing, and freight reads it to decide what it can carry.
   *
   * There is no `portableWhy` beside it, deliberately (Law 16): for a physical good the value is
   * the whole of the statement and thirty-six copies of "it is a thing and things can be loaded"
   * would say nothing. Why a SERVICE is not portable is stated once, where the services are.
   */
  readonly portable: boolean;
  /** A2: what one unit is made from. Empty means it is drawn from labour and nature alone. */
  readonly inputs: readonly RecipeInputDecl[];
  /** A2.c: hours of labour per unit of output. */
  readonly labourHoursPerUnit: number;
  readonly labourWhy: string;
  /**
   * A2.c: the capital services a unit takes, per kind of plant. Empty is a real answer and not a
   * missing one: a line that needs no plant is limited by its people and by its inputs, and saying
   * so is different from saying its capacity is large (Law 6).
   */
  readonly plant: readonly RecipePlantDecl[];
  /**
   * B4: the fraction of what is started that is finished IN AN ORDINARY PERIOD. The rest is scrap,
   * at the point it would have been made. What this period actually yields is this times the
   * conditions the line stood in (`exposedTo`), so a bad season is a real loss of units and never
   * a number anybody wrote down for it.
   */
  readonly yieldRate: number;
  readonly yieldWhy: string;
  /**
   * B4, Commodities Spot B3: the physical facts this line's yield stands in, by the name the
   * environment publishes them under. Empty is a line made indoors, which is most of them.
   */
  readonly exposedTo: readonly string[];
  /** 13c.1: the resource in the ground this line stands on, or null for a line made indoors. */
  readonly standsOn: string | null;
  readonly standsOnWhy: string;
  /**
   * Commodities Spot A3, D3: how much covered space one unit takes for a period. Null is a line
   * nobody stores in bulk, and it is a real answer: a loaf does not wait for a silo, which is what
   * its spoilage already says.
   */
  readonly storagePerUnit: number | null;
  readonly storageWhy: string | null;
  /** B3, B4: periods a batch spends in work in progress before it yields. */
  readonly leadTimePeriods: number;
  readonly leadTimeWhy: string;
}

/** A period is a week (docs/ARCHITECTURE.md 4.7), so every rate below is per week. */
export const GOODS: readonly GoodDecl[] = [
  {
    subUnit: 'grain',
    unit: 'tonnes',
    name: 'grain',
    portable: true,
    spoilagePerPeriod: 0.004,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Grown. What a bad season takes, it takes here first.',
    inputs: [
      { subUnit: 'fertiliser', qtyPerUnit: 0.02, why: 'A2.a: 0.02 of fertiliser per unit of grain — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 9,
    labourWhy: 'A2.c: hours of work per unit. Grown. What a bad season takes, it takes here first.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of grain takes.' },
    ],
    yieldRate: 0.92,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: ['growing'],
    standsOn: 'arable',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on arable, so where it is run is part of what it yields.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'livestock',
    unit: 'head',
    name: 'livestock',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Grazed and finished on grain.',
    inputs: [
      { subUnit: 'grain', qtyPerUnit: 0.5, why: 'A2.a: 0.5 of grain per unit of livestock — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 14,
    labourWhy: 'A2.c: hours of work per unit. Grazed and finished on grain.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of livestock takes.' },
    ],
    yieldRate: 0.9,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: ['growing', 'warmth'],
    standsOn: 'pasture',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on pasture, so where it is run is part of what it yields.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 4,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'timberLog',
    unit: 'cubic metres',
    name: 'timber logs',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Felled. Slow to grow and slow to cut.',
    inputs: [
    ],
    labourHoursPerUnit: 6,
    labourWhy: 'A2.c: hours of work per unit. Felled. Slow to grow and slow to cut.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of timberLog takes.' },
    ],
    yieldRate: 0.95,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: ['growing'],
    standsOn: 'timber',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on timber, so where it is run is part of what it yields.',
    storagePerUnit: 0.0005,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 3,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'ironOre',
    unit: 'tonnes',
    name: 'iron ore',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Dug. Drawn from the ground and labour alone, so nothing upstream of it can be short.',
    inputs: [
    ],
    labourHoursPerUnit: 5,
    labourWhy: 'A2.c: hours of work per unit. Dug. Drawn from the ground and labour alone, so nothing upstream of it can be short.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of ironOre takes.' },
    ],
    yieldRate: 0.97,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: 'ore',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on ore, so where it is run is part of what it yields.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'coalRaw',
    unit: 'tonnes',
    name: 'coal',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Dug. What this world burns for power and smelts with.',
    inputs: [
    ],
    labourHoursPerUnit: 4,
    labourWhy: 'A2.c: hours of work per unit. Dug. What this world burns for power and smelts with.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of coalRaw takes.' },
    ],
    yieldRate: 0.97,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: 'coal',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on coal, so where it is run is part of what it yields.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'crude',
    unit: 'barrels',
    name: 'crude oil',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Pumped. Fuel and chemicals both start here.',
    inputs: [
    ],
    labourHoursPerUnit: 3,
    labourWhy: 'A2.c: hours of work per unit. Pumped. Fuel and chemicals both start here.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of crude takes.' },
    ],
    yieldRate: 0.98,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: 'petroleum',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on petroleum, so where it is run is part of what it yields.',
    storagePerUnit: 0.0001,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'limestoneRaw',
    unit: 'tonnes',
    name: 'limestone',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Quarried. Cement and glass both start here.',
    inputs: [
    ],
    labourHoursPerUnit: 4,
    labourWhy: 'A2.c: hours of work per unit. Quarried. Cement and glass both start here.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of limestoneRaw takes.' },
    ],
    yieldRate: 0.98,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: 'limestone',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on limestone, so where it is run is part of what it yields.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'bauxite',
    unit: 'tonnes',
    name: 'bauxite',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Dug. The light metal begins here.',
    inputs: [
    ],
    labourHoursPerUnit: 5,
    labourWhy: 'A2.c: hours of work per unit. Dug. The light metal begins here.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of bauxite takes.' },
    ],
    yieldRate: 0.97,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: 'bauxite',
    standsOnWhy:
      'Goods B4, 13c.1: it stands on bauxite, so where it is run is part of what it yields.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'power',
    unit: 'MWh',
    name: 'electric power',
    portable: true,
    spoilagePerPeriod: 1,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Generated and consumed in the same period: nobody stores a megawatt-hour, which is why an outage is a real shortage rather than a dearer price.',
    inputs: [
      { subUnit: 'coalRaw', qtyPerUnit: 0.4, why: 'A2.a: 0.4 of coalRaw per unit of power — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 0.4,
    labourWhy: 'A2.c: hours of work per unit. Generated and consumed in the same period: nobody stores a megawatt-hour, which is why an outage is a real shortage rather than a dearer price.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of power takes.' },
    ],
    yieldRate: 0.94,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 0,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'fuel',
    unit: 'litres',
    name: 'refined fuel',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Refined. What moves what is not moved by hand.',
    inputs: [
      { subUnit: 'crude', qtyPerUnit: 0.007, why: 'A2.a: 0.007 of crude per unit of fuel — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 0.002,
    labourWhy: 'A2.c: hours of work per unit. Refined. What moves what is not moved by hand.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of fuel takes.' },
    ],
    yieldRate: 0.93,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 1e-06,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'steel',
    unit: 'tonnes',
    name: 'steel',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Smelted. Most made things are built on it or out of it.',
    inputs: [
      { subUnit: 'ironOre', qtyPerUnit: 1.6, why: 'A2.a: 1.6 of ironOre per unit of steel — physical, never a share of cost.' },
      { subUnit: 'coalRaw', qtyPerUnit: 0.7, why: 'A2.a: 0.7 of coalRaw per unit of steel — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.5, why: 'A2.a: 0.5 of power per unit of steel — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 7,
    labourWhy: 'A2.c: hours of work per unit. Smelted. Most made things are built on it or out of it.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of steel takes.' },
    ],
    yieldRate: 0.9,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'aluminium',
    unit: 'tonnes',
    name: 'aluminium',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Smelted, and it is mostly electricity: a power shortage is an aluminium shortage a period later.',
    inputs: [
      { subUnit: 'bauxite', qtyPerUnit: 4, why: 'A2.a: 4 of bauxite per unit of aluminium — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 14, why: 'A2.a: 14 of power per unit of aluminium — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 6,
    labourWhy: 'A2.c: hours of work per unit. Smelted, and it is mostly electricity: a power shortage is an aluminium shortage a period later.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of aluminium takes.' },
    ],
    yieldRate: 0.91,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'lumber',
    unit: 'cubic metres',
    name: 'sawn lumber',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Sawn.',
    inputs: [
      { subUnit: 'timberLog', qtyPerUnit: 1.4, why: 'A2.a: 1.4 of timberLog per unit of lumber — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.05, why: 'A2.a: 0.05 of power per unit of lumber — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 3,
    labourWhy: 'A2.c: hours of work per unit. Sawn.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of lumber takes.' },
    ],
    yieldRate: 0.88,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0004,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'cement',
    unit: 'tonnes',
    name: 'cement',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Burned. Dear to move, so it is made near where it is poured.',
    inputs: [
      { subUnit: 'limestoneRaw', qtyPerUnit: 1.3, why: 'A2.a: 1.3 of limestoneRaw per unit of cement — physical, never a share of cost.' },
      { subUnit: 'coalRaw', qtyPerUnit: 0.1, why: 'A2.a: 0.1 of coalRaw per unit of cement — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.1, why: 'A2.a: 0.1 of power per unit of cement — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 3,
    labourWhy: 'A2.c: hours of work per unit. Burned. Dear to move, so it is made near where it is poured.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of cement takes.' },
    ],
    yieldRate: 0.95,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'chemicals',
    unit: 'tonnes',
    name: 'industrial chemicals',
    portable: true,
    spoilagePerPeriod: 0.003,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Cracked from crude.',
    inputs: [
      { subUnit: 'crude', qtyPerUnit: 7, why: 'A2.a: 7 of crude per unit of chemicals — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.6, why: 'A2.a: 0.6 of power per unit of chemicals — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 6,
    labourWhy: 'A2.c: hours of work per unit. Cracked from crude.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of chemicals takes.' },
    ],
    yieldRate: 0.9,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'fertiliser',
    unit: 'tonnes',
    name: 'fertiliser',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Made of chemicals and power, and it goes back into the field — so an energy shock reaches the price of bread by two paths.',
    inputs: [
      { subUnit: 'chemicals', qtyPerUnit: 0.6, why: 'A2.a: 0.6 of chemicals per unit of fertiliser — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.9, why: 'A2.a: 0.9 of power per unit of fertiliser — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 2,
    labourWhy: 'A2.c: hours of work per unit. Made of chemicals and power, and it goes back into the field — so an energy shock reaches the price of bread by two paths.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of fertiliser takes.' },
    ],
    yieldRate: 0.93,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'medicine',
    unit: 'tonnes',
    name: 'medicines',
    portable: true,
    spoilagePerPeriod: 0.01,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Synthesised. Low yield, because most of a batch fails its test.',
    inputs: [
      { subUnit: 'chemicals', qtyPerUnit: 0.4, why: 'A2.a: 0.4 of chemicals per unit of medicine — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.3, why: 'A2.a: 0.3 of power per unit of medicine — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 40,
    labourWhy: 'A2.c: hours of work per unit. Synthesised. Low yield, because most of a batch fails its test.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of medicine takes.' },
    ],
    yieldRate: 0.82,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 3,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'flour',
    unit: 'tonnes',
    name: 'flour',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Milled. The intermediate nobody eats.',
    inputs: [
      { subUnit: 'grain', qtyPerUnit: 1.25, why: 'A2.a: 1.25 of grain per unit of flour — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.08, why: 'A2.a: 0.08 of power per unit of flour — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 5,
    labourWhy: 'A2.c: hours of work per unit. Milled. The intermediate nobody eats.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of flour takes.' },
    ],
    yieldRate: 0.96,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0002,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'meat',
    unit: 'tonnes',
    name: 'meat',
    portable: true,
    spoilagePerPeriod: 0.04,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Dressed. It keeps badly, so it moves fast or not at all.',
    inputs: [
      { subUnit: 'livestock', qtyPerUnit: 4, why: 'A2.a: 4 of livestock per unit of meat — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.15, why: 'A2.a: 0.15 of power per unit of meat — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 11,
    labourWhy: 'A2.c: hours of work per unit. Dressed. It keeps badly, so it moves fast or not at all.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of meat takes.' },
    ],
    yieldRate: 0.93,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'wool',
    unit: 'tonnes',
    name: 'wool',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Clipped. The other thing a herd gives.',
    inputs: [
      { subUnit: 'livestock', qtyPerUnit: 0.2, why: 'A2.a: 0.2 of livestock per unit of wool — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 5,
    labourWhy: 'A2.c: hours of work per unit. Clipped. The other thing a herd gives.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of wool takes.' },
    ],
    yieldRate: 0.94,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'cloth',
    unit: 'tonnes',
    name: 'cloth',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Spun and woven.',
    inputs: [
      { subUnit: 'wool', qtyPerUnit: 1.1, why: 'A2.a: 1.1 of wool per unit of cloth — physical, never a share of cost.' },
      { subUnit: 'chemicals', qtyPerUnit: 0.05, why: 'A2.a: 0.05 of chemicals per unit of cloth — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.4, why: 'A2.a: 0.4 of power per unit of cloth — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 9,
    labourWhy: 'A2.c: hours of work per unit. Spun and woven.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of cloth takes.' },
    ],
    yieldRate: 0.9,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'clothing',
    unit: 'units',
    name: 'clothing',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Cut and sewn.',
    inputs: [
      { subUnit: 'cloth', qtyPerUnit: 0.0006, why: 'A2.a: 0.0006 of cloth per unit of clothing — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.004, why: 'A2.a: 0.004 of power per unit of clothing — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 0.6,
    labourWhy: 'A2.c: hours of work per unit. Cut and sewn.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of clothing takes.' },
    ],
    yieldRate: 0.92,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'glass',
    unit: 'tonnes',
    name: 'glass',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Melted. Fragile and heavy, and wanted everywhere.',
    inputs: [
      { subUnit: 'limestoneRaw', qtyPerUnit: 0.3, why: 'A2.a: 0.3 of limestoneRaw per unit of glass — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.7, why: 'A2.a: 0.7 of power per unit of glass — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 6,
    labourWhy: 'A2.c: hours of work per unit. Melted. Fragile and heavy, and wanted everywhere.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of glass takes.' },
    ],
    yieldRate: 0.9,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'plastic',
    unit: 'tonnes',
    name: 'plastics',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Moulded. Light, which is why it travels.',
    inputs: [
      { subUnit: 'chemicals', qtyPerUnit: 1.1, why: 'A2.a: 1.1 of chemicals per unit of plastic — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.4, why: 'A2.a: 0.4 of power per unit of plastic — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 4,
    labourWhy: 'A2.c: hours of work per unit. Moulded. Light, which is why it travels.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of plastic takes.' },
    ],
    yieldRate: 0.94,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'rubber',
    unit: 'tonnes',
    name: 'rubber',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Compounded. Nothing rolls without it.',
    inputs: [
      { subUnit: 'chemicals', qtyPerUnit: 0.9, why: 'A2.a: 0.9 of chemicals per unit of rubber — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.3, why: 'A2.a: 0.3 of power per unit of rubber — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 4,
    labourWhy: 'A2.c: hours of work per unit. Compounded. Nothing rolls without it.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of rubber takes.' },
    ],
    yieldRate: 0.92,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'paper',
    unit: 'tonnes',
    name: 'paper',
    portable: true,
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Pulped and pressed.',
    inputs: [
      { subUnit: 'lumber', qtyPerUnit: 2.2, why: 'A2.a: 2.2 of lumber per unit of paper — physical, never a share of cost.' },
      { subUnit: 'chemicals', qtyPerUnit: 0.1, why: 'A2.a: 0.1 of chemicals per unit of paper — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.5, why: 'A2.a: 0.5 of power per unit of paper — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 4,
    labourWhy: 'A2.c: hours of work per unit. Pulped and pressed.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of paper takes.' },
    ],
    yieldRate: 0.91,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: 0.0003,
    storageWhy:
      'Commodities Spot A3, D3: the covered space one unit takes for a period.',
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'bread',
    unit: 'tonnes',
    name: 'bread',
    portable: true,
    spoilagePerPeriod: 0.25,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Baked. It goes stale in days, so it is made near where it is eaten.',
    inputs: [
      { subUnit: 'flour', qtyPerUnit: 0.78, why: 'A2.a: 0.78 of flour per unit of bread — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.06, why: 'A2.a: 0.06 of power per unit of bread — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 12,
    labourWhy: 'A2.c: hours of work per unit. Baked. It goes stale in days, so it is made near where it is eaten.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 1, why: 'Capital Programme A2: the plant one unit a period of bread takes.' },
    ],
    yieldRate: 0.97,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'concrete',
    unit: 'cubic metres',
    name: 'concrete',
    portable: true,
    spoilagePerPeriod: 0.3,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Mixed and poured the same period: nobody stores it, which is what its spoilage says.',
    inputs: [
      { subUnit: 'cement', qtyPerUnit: 0.35, why: 'A2.a: 0.35 of cement per unit of concrete — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.03, why: 'A2.a: 0.03 of power per unit of concrete — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 1.5,
    labourWhy: 'A2.c: hours of work per unit. Mixed and poured the same period: nobody stores it, which is what its spoilage says.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of concrete takes.' },
    ],
    yieldRate: 0.96,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 0,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'machine',
    unit: 'machines',
    name: 'machinery',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. The capital good: what every line\'s capacity is made of, including its own.',
    inputs: [
      { subUnit: 'steel', qtyPerUnit: 2.5, why: 'A2.a: 2.5 of steel per unit of machine — physical, never a share of cost.' },
      { subUnit: 'chemicals', qtyPerUnit: 0.2, why: 'A2.a: 0.2 of chemicals per unit of machine — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 1.2, why: 'A2.a: 1.2 of power per unit of machine — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 120,
    labourWhy: 'A2.c: hours of work per unit. The capital good: what every line\'s capacity is made of, including its own.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 6, why: 'Capital Programme A2: the plant one unit a period of machine takes.' },
    ],
    yieldRate: 0.95,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 3,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'vessel',
    unit: 'vessels',
    name: 'vessels',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Built in a yard over half a year, which is why freight capacity answers a shortage slowly.',
    inputs: [
      { subUnit: 'steel', qtyPerUnit: 900, why: 'A2.a: 900 of steel per unit of vessel — physical, never a share of cost.' },
      { subUnit: 'machine', qtyPerUnit: 14, why: 'A2.a: 14 of machine per unit of vessel — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 600, why: 'A2.a: 600 of power per unit of vessel — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 9000,
    labourWhy: 'A2.c: hours of work per unit. Built in a yard over half a year, which is why freight capacity answers a shortage slowly.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 10, why: 'Capital Programme A2: the plant one unit a period of vessel takes.' },
    ],
    yieldRate: 0.96,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 26,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'vehicle',
    unit: 'vehicles',
    name: 'vehicles',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Assembled out of six industries at once.',
    inputs: [
      { subUnit: 'steel', qtyPerUnit: 1.1, why: 'A2.a: 1.1 of steel per unit of vehicle — physical, never a share of cost.' },
      { subUnit: 'aluminium', qtyPerUnit: 0.15, why: 'A2.a: 0.15 of aluminium per unit of vehicle — physical, never a share of cost.' },
      { subUnit: 'plastic', qtyPerUnit: 0.25, why: 'A2.a: 0.25 of plastic per unit of vehicle — physical, never a share of cost.' },
      { subUnit: 'glass', qtyPerUnit: 0.05, why: 'A2.a: 0.05 of glass per unit of vehicle — physical, never a share of cost.' },
      { subUnit: 'rubber', qtyPerUnit: 0.04, why: 'A2.a: 0.04 of rubber per unit of vehicle — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 2, why: 'A2.a: 2 of power per unit of vehicle — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 90,
    labourWhy: 'A2.c: hours of work per unit. Assembled out of six industries at once.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 5, why: 'Capital Programme A2: the plant one unit a period of vehicle takes.' },
    ],
    yieldRate: 0.94,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 3,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'electronics',
    unit: 'units',
    name: 'electronics',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Assembled from very little of a great many things. Most of what is started fails a test.',
    inputs: [
      { subUnit: 'plastic', qtyPerUnit: 0.004, why: 'A2.a: 0.004 of plastic per unit of electronics — physical, never a share of cost.' },
      { subUnit: 'steel', qtyPerUnit: 0.003, why: 'A2.a: 0.003 of steel per unit of electronics — physical, never a share of cost.' },
      { subUnit: 'chemicals', qtyPerUnit: 0.002, why: 'A2.a: 0.002 of chemicals per unit of electronics — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.05, why: 'A2.a: 0.05 of power per unit of electronics — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 1.2,
    labourWhy: 'A2.c: hours of work per unit. Assembled from very little of a great many things. Most of what is started fails a test.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 5, why: 'Capital Programme A2: the plant one unit a period of electronics takes.' },
    ],
    yieldRate: 0.86,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'appliance',
    unit: 'units',
    name: 'appliances',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Assembled. What a household replaces rather than repairs.',
    inputs: [
      { subUnit: 'steel', qtyPerUnit: 0.03, why: 'A2.a: 0.03 of steel per unit of appliance — physical, never a share of cost.' },
      { subUnit: 'plastic', qtyPerUnit: 0.01, why: 'A2.a: 0.01 of plastic per unit of appliance — physical, never a share of cost.' },
      { subUnit: 'electronics', qtyPerUnit: 4, why: 'A2.a: 4 of electronics per unit of appliance — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.08, why: 'A2.a: 0.08 of power per unit of appliance — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 3,
    labourWhy: 'A2.c: hours of work per unit. Assembled. What a household replaces rather than repairs.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 4, why: 'Capital Programme A2: the plant one unit a period of appliance takes.' },
    ],
    yieldRate: 0.93,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 2,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'furniture',
    unit: 'units',
    name: 'furniture',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Made from lumber. Bulky, so it is made near its market.',
    inputs: [
      { subUnit: 'lumber', qtyPerUnit: 0.12, why: 'A2.a: 0.12 of lumber per unit of furniture — physical, never a share of cost.' },
      { subUnit: 'plastic', qtyPerUnit: 0.01, why: 'A2.a: 0.01 of plastic per unit of furniture — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.03, why: 'A2.a: 0.03 of power per unit of furniture — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 7,
    labourWhy: 'A2.c: hours of work per unit. Made from lumber. Bulky, so it is made near its market.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 2, why: 'Capital Programme A2: the plant one unit a period of furniture takes.' },
    ],
    yieldRate: 0.93,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 1,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'packaging',
    unit: 'units',
    name: 'packaging',
    portable: true,
    spoilagePerPeriod: 0.001,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. What everything else moves in. Nobody buys it for itself.',
    inputs: [
      { subUnit: 'paper', qtyPerUnit: 0.002, why: 'A2.a: 0.002 of paper per unit of packaging — physical, never a share of cost.' },
      { subUnit: 'plastic', qtyPerUnit: 0.001, why: 'A2.a: 0.001 of plastic per unit of packaging — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 0.005, why: 'A2.a: 0.005 of power per unit of packaging — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 0.15,
    labourWhy: 'A2.c: hours of work per unit. What everything else moves in. Nobody buys it for itself.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 3, why: 'Capital Programme A2: the plant one unit a period of packaging takes.' },
    ],
    yieldRate: 0.96,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 0,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'building',
    unit: 'buildings',
    name: 'buildings',
    portable: true,
    spoilagePerPeriod: 0,
    spoilageWhy: 'Goods A3, E4: what is lost in store each period. Built on site over six periods: the longest lead time in this world.',
    inputs: [
      { subUnit: 'concrete', qtyPerUnit: 380, why: 'A2.a: 380 of concrete per unit of building — physical, never a share of cost.' },
      { subUnit: 'steel', qtyPerUnit: 55, why: 'A2.a: 55 of steel per unit of building — physical, never a share of cost.' },
      { subUnit: 'lumber', qtyPerUnit: 90, why: 'A2.a: 90 of lumber per unit of building — physical, never a share of cost.' },
      { subUnit: 'glass', qtyPerUnit: 8, why: 'A2.a: 8 of glass per unit of building — physical, never a share of cost.' },
      { subUnit: 'power', qtyPerUnit: 40, why: 'A2.a: 40 of power per unit of building — physical, never a share of cost.' },
    ],
    labourHoursPerUnit: 2600,
    labourWhy: 'A2.c: hours of work per unit. Built on site over six periods: the longest lead time in this world.',
    plant: [
      { capitalKind: 'machinery', unitsPerUnitPerPeriod: 8, why: 'Capital Programme A2: the plant one unit a period of building takes.' },
    ],
    yieldRate: 0.97,
    yieldWhy:
      'B4: what an ordinary period finishes of what it starts. The weather and the ground are NOT in this number (13c, 13c.1) — they multiply it where the line is actually run.',
    exposedTo: [],
    standsOn: null,
    standsOnWhy:
      'A line made indoors stands on no particular ground.',
    storagePerUnit: null,
    storageWhy:
      null,
    leadTimePeriods: 6,
    leadTimeWhy:
      'B3: periods between committing the inputs and having the thing, so a decision taken on a stale view of demand cannot be unwound.',
  },
];
