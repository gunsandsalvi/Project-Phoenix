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
 * The chain here is deliberately short and complete — a good grown from labour alone, an
 * intermediate nobody eats, and a consumer good that goes stale — so that an input shortage, a
 * perishing stock and an intermediate price that no household ever pays all exist from the start.
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
    spoilagePerPeriod: 0.004,
    spoilageWhy:
      'Grain in store is lost to moisture, pests and handling. It keeps well, so the loss is small, but it is not zero and it is units, never a fee.',
    inputs: [],
    labourHoursPerUnit: 9,
    labourWhy:
      'Hours of field labour per tonne harvested. It is drawn from land and labour, so it starts the chain and nothing upstream of it can be short.',
    plant: [
      {
        capitalKind: 'machinery',
        unitsPerUnitPerPeriod: 1,
        why: 'A tonne a week of grain takes a machine to work the ground with. Farming is the most machine-intensive step in this chain, which is why a farm that stops replacing its plant loses tonnage before anybody else does.',
      },
    ],
    yieldRate: 0.92,
    yieldWhy:
      'Pests and handling take part of every crop between the sowing and the barn, in a season that behaves. It is the largest yield loss in the chain, which is why a farmer commits labour for a tonnage it does not get. THE WEATHER IS NOT IN THIS NUMBER (13c): a crop stands in the growing conditions the environment publishes, so a bad season is a real loss of tonnes and this is what an ordinary one leaves.',
    exposedTo: ['growing'],
    leadTimePeriods: 2,
    leadTimeWhy:
      'Two periods between committing the labour and having the tonne, so a decision taken on a stale view of demand cannot be unwound.',
  },
  {
    subUnit: 'flour',
    unit: 'tonnes',
    name: 'flour',
    spoilagePerPeriod: 0.002,
    spoilageWhy: 'Milled flour keeps nearly as well as the grain it came from.',
    inputs: [
      {
        subUnit: 'grain',
        qtyPerUnit: 1.25,
        why: 'A tonne of flour takes a tonne and a quarter of grain: the bran and the milling loss are the difference, and they are units that leave.',
      },
    ],
    labourHoursPerUnit: 3,
    labourWhy: 'Hours at the mill per tonne of flour.',
    plant: [
      {
        capitalKind: 'machinery',
        unitsPerUnitPerPeriod: 0.8,
        why: 'A mill is machinery with a miller beside it: fewer hours to the tonne than the field takes, and nearly as much plant.',
      },
    ],
    yieldRate: 0.98,
    yieldWhy: 'A little of every batch is lost to the machinery and to sweeping up.',
    exposedTo: [],
    leadTimePeriods: 0,
    leadTimeWhy: 'Milling is within the period: grain in at the start is flour by the end.',
  },
  {
    subUnit: 'bread',
    unit: 'tonnes',
    name: 'bread',
    spoilagePerPeriod: 0.25,
    spoilageWhy:
      'Bread goes stale. A quarter of what is still in store at the end of a week is not sold as bread, and that is why a baker who misreads demand loses the loaves and not just the margin.',
    inputs: [
      {
        subUnit: 'flour',
        qtyPerUnit: 0.7,
        why: 'Seven tenths of a tonne of flour per tonne of bread; the rest of the weight is water and it costs nothing to draw.',
      },
    ],
    labourHoursPerUnit: 14,
    labourWhy: 'Hours at the bakery per tonne of bread: it is the labour-intensive step.',
    plant: [
      {
        capitalKind: 'machinery',
        unitsPerUnitPerPeriod: 0.5,
        why: 'An oven and a mixer, and hands doing most of the rest. It is the least machine-intensive step, which is why a bakery answers demand with hours where a farm has to answer it with plant.',
      },
    ],
    yieldRate: 0.97,
    yieldWhy: 'Loaves come out of the oven wrong, and the ones that do are a loss of units, not of margin.',
    exposedTo: [],
    leadTimePeriods: 0,
    leadTimeWhy: 'Baked and sold inside the week.',
  },
  {
    subUnit: 'machine',
    unit: 'machines',
    name: 'machines',
    spoilagePerPeriod: 0,
    spoilageWhy:
      'A machine in a crate does not go off. What happens to one is that it wears out once it is working, and that is its life rather than its store (Capital Programme A4.b).',
    inputs: [],
    labourHoursPerUnit: 60,
    labourWhy:
      'Hours in the workshop per machine. It is the longest single thing anybody in this world makes, which is why the people who build capital are employed by investment and not by consumption (Capital Programme E2).',
    // Capital Programme A4.a: what a workshop is made of is its own industry's business, and in
    // this world it is people and a bench. A line that needs no plant is limited by its people.
    plant: [],
    yieldRate: 0.98,
    yieldWhy: 'A machine that comes off the bench wrong is scrapped; most do not.',
    exposedTo: [],
    leadTimePeriods: 1,
    leadTimeWhy:
      'A machine takes longer to build than a loaf takes to bake, and the gap between ordering one and having it working is the other half of Capital Programme C3.',
  },
];
