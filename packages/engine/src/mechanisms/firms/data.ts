/**
 * The firms this world has, what each one makes, and how good it is at making it.
 *
 * @spec Firm A1 Firm A2 Firm A3 Firm F4 Seed B1 Seed B1.a Seed B4 Law 2 Law 15
 *
 * Data only (Law 15). A firm exists in a region and a sector, and both are load-bearing (A2): the
 * region fixes its money, and the line it is in fixes what it buys, what it sells and who it
 * employs. Every firm decides the same way from its own state (F4), and nothing in the mechanism
 * ever asks which industry it is in.
 *
 * THREE FIRMS TO A LINE, AND NO TWO ALIKE. There was one, and one is not a sector: Seed B1 wants
 * enough of a type that it is a distribution rather than a single instance, and Seed B4 says a
 * sector of equals never produces a market. Both are load-bearing here rather than decorative. A
 * venue with one employer in it has one bid, so whichever end of the book the clearing rule takes
 * the level from, that one bid gets the whole surplus: at one end the employer pays what the last
 * seeker will accept, at the other it pays what an hour is worth to it and earns exactly nothing.
 * Neither is a market. The second thing a market needs is that the bids DIFFER, which is Firm A3 —
 * firms are heterogeneous in size, cost and leverage, "and the dispersion is the reason markets
 * exist among them". Copies of one firm bid the same number and are no better than one.
 *
 * So a firm has its OWN LABOUR PRODUCTIVITY: the hours the recipe names are what the work takes at
 * a firm that is neither good nor bad at it, and each firm scales them by a number of its own. That
 * is the whole of the cost dispersion, deliberately (Law 2, fewest primitives): what makes one
 * firm's wage bid differ from its neighbour's is what the venue needs two of, and a second
 * dispersion over the inputs would be doing the same work twice. It is technology, not a claim
 * about an answer: firms really do differ in how many hours a tonne takes them.
 */
import { paramId, type ParamId } from '../../core/ids.js';

export interface FirmDecl {
  /** The named party (Firm A1). It banks somewhere, and the wage and the invoices leave that account. */
  readonly firm: string;
  /** A2: the good it makes. Its recipe, its lead time and its yield are the good's own technology. */
  readonly subUnit: string;
  /** A2, Labour A3: the occupation it employs, which is the venue it posts its openings in. */
  readonly occupation: string;
  /**
   * A3: the hours a tonne takes THIS firm, as a ratio of the hours the recipe names. Below one is a
   * firm that does more with an hour; above one is one that does less. It is why two firms in a line
   * bid different wages, and therefore why one of them is marginal and the other is not.
   */
  readonly labourScale: number;
  readonly why: string;
}

/** The parameter carrying a named firm's own productivity (XI-14: declared, never a literal). */
export const labourScaleId = (firm: string): ParamId => paramId(`firms.labourScale.${firm}`);

export const FIRMS: readonly FirmDecl[] = [
  {
    firm: 'firm.4',
    subUnit: 'grain',
    occupation: 'field',
    labourScale: 0.9,
    why: 'The best land in the region and the machinery to work it: fewer hours to the tonne than the trade takes on average.',
  },
  {
    firm: 'firm.1',
    subUnit: 'grain',
    occupation: 'field',
    labourScale: 1,
    why: 'An ordinary farm. It is the trade average by construction, which is what the recipe states.',
  },
  {
    firm: 'firm.7',
    subUnit: 'grain',
    occupation: 'field',
    labourScale: 1.15,
    why: 'Poorer ground and older equipment: the same tonne takes it longer, so it is the one that stops first when the wage rises.',
  },
  {
    firm: 'firm.5',
    subUnit: 'flour',
    occupation: 'mill',
    labourScale: 0.92,
    why: 'A modern mill: more of the grinding is done by the machine and less by the miller.',
  },
  {
    firm: 'firm.2',
    subUnit: 'flour',
    occupation: 'mill',
    labourScale: 1,
    why: 'An ordinary mill.',
  },
  {
    firm: 'firm.8',
    subUnit: 'flour',
    occupation: 'mill',
    labourScale: 1.12,
    why: 'An old mill kept running: it works, and it takes more hands to do it.',
  },
  {
    firm: 'firm.6',
    subUnit: 'bread',
    occupation: 'bakery',
    labourScale: 0.88,
    why: 'A plant bakery. Baking is the labour-intensive step, so this is where doing it better is worth the most.',
  },
  {
    firm: 'firm.3',
    subUnit: 'bread',
    occupation: 'bakery',
    labourScale: 1,
    why: 'An ordinary bakery.',
  },
  {
    firm: 'firm.9',
    subUnit: 'bread',
    occupation: 'bakery',
    labourScale: 1.18,
    why: 'A craft bakery: the most hours to the tonne of anybody in this world, and the first to be priced out of the labour it needs.',
  },
];
