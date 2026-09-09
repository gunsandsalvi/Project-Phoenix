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
    leadTimePeriods: 0,
    leadTimeWhy: 'Baked and sold inside the week.',
  },
];
