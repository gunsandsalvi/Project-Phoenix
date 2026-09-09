/**
 * The kinds of capital this world has: what plant is made of, how long it works, and how long it
 * takes to build.
 *
 * @spec Capital Programme A1 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme C3 Goods A2 Law 2 Law 15
 *
 * Data only (Law 15). A4 is what this table exists for: capital is SPECIFIC, and specific IN KIND —
 * capital of one kind is not capital of another, so it is counted in its own unit, made from its
 * own good, and worn out on its own schedule. A use that needs several kinds is limited by the
 * scarcest of them, which is why a row here is a row and not a column in one basket.
 *
 * A4.b is why the table has a life in it at all: **the presence of a life is what makes a good a
 * capital good.** A world that declares a capital kind made from a good has said that good is a
 * capital good, and nothing else has to say so — there is no flag on the good, and the buyer's own
 * recipe decides whether what it bought is an input or plant (A4.c).
 */

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
  readonly why: string;
}

/**
 * A period is a week (docs/ARCHITECTURE.md 4.7), so a life below is a count of weeks.
 *
 * ONE KIND, and it is stated rather than hidden: this world makes one thing that has a life, and
 * every line that needs plant needs that one. The mechanism takes the SCARCEST of the kinds a
 * recipe names (A4) and is tested with two; a second kind arrives with the dwellings of Housing
 * (worklist 13d) and the storage of Commodities (13c), each of which is a stock of productive
 * assets with a life of its own.
 */
export const CAPITAL_KINDS: readonly CapitalKindDecl[] = [
  {
    id: 'machinery',
    name: 'machinery',
    unit: 'machines in service',
    madeFrom: 'machine',
    usefulLifePeriods: 156,
    buildLagPeriods: 2,
    why: 'A machine works for three years and then it is scrap. Three years is short enough that a firm which stops investing loses its capacity inside a run, and long enough that the spend and the capacity it buys are separated by more than a cycle — which is what makes investment a commitment rather than a purchase.',
  },
];

/** The declaration of a capital kind by id; asking for one this world does not have is a defect. */
export function capitalKindOf(
  rows: readonly CapitalKindDecl[],
  id: string,
): CapitalKindDecl | undefined {
  return rows.find((r) => r.id === id);
}
