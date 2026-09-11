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
import { prng } from '../../rng/prng.js';
import { between, betweenWhole, drawSize, type Spread, type Tail } from '../../rng/spread.js';
import { asQty, splitOnTick } from '../../core/tick.js';
import { zeroIfNone } from '../../core/num.js';

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
  /**
   * Capital Programme B1.d: the margin over its cost of capital this MANAGEMENT insists on before
   * it commits money it cannot get back, per annum. It is its risk aversion, and it is a preference:
   * two managements facing the same price do not take the same project, which is what makes the
   * line-up of who expands an outcome rather than a rule.
   */
  readonly hurdle: number;
  /**
   * Capital Programme B1.d: how many periods of service this management counts. It is its patience,
   * and a short one is what a growth-against-margin orientation actually is: a management that will
   * not look past two years values a machine at what two years of it are worth.
   */
  readonly horizonPeriods: number;
  /** Law 9: the name a market would use for it. */
  readonly name: string;
  /**
   * Seed B4: how big it is beside the others in its own line. A weight and not a share, so the same
   * spread describes a line of three firms and one of three hundred; what it opens holding, what it
   * can make and what it employs all follow from it and from the good's own recipe, so no endowment
   * is stated one firm at a time.
   */
  readonly size: number;
  readonly why: string;
}

/** The parameter carrying a named firm's own productivity (XI-14: declared, never a literal). */
export const labourScaleId = (firm: string): ParamId => paramId(`firms.labourScale.${firm}`);

/** A management's own numbers, one per firm (XI-14: declared, never a literal). */
export const firmParam = (firm: string, what: string): ParamId => paramId(`firm.${what}.${firm}`);

/**
 * Firm A3, Seed B1, B1.a, B4: A FIRM'S OWN NUMBERS ARE DRAWN, ONCE, AT THE SEED.
 *
 * It used to be a table of twelve, three to a line, written out one firm at a time. Twelve is not a
 * sector and a table is not a distribution: Seed B1 wants enough of a type that it IS a
 * distribution, B1.a wants the count to be a property of the world rather than of the file, and
 * Law 2 wants the fewest primitives — so what a firm varies over is stated once, with a reason, and
 * how many of them there are is a count. Ask for three thousand and there are three thousand, each
 * with its own productivity and its own management, and no row is written anywhere.
 *
 * Every drawn value is declared in the parameter register under that firm's own name
 * (`firms/index.ts`), so the register prints what each one is like and a reader can see why it did
 * what it did. What is stated here is the WIDTH, which is what a dispersion is.
 */
export interface FirmDispersion {
  readonly labourScale: Spread;
  readonly hurdle: Spread;
  readonly horizonPeriods: Spread;
  /** Seed B4: how unequal a line is. A few large firms in it and a long tail of small ones. */
  readonly size: Tail;
}

export const FIRM_SPREAD: FirmDispersion = {
  labourScale: {
    low: 0.8,
    high: 1.25,
    why: 'Firm A3: the hours a tonne takes THIS firm, as a ratio of the hours the recipe names. Below one does more with an hour; above one does less. It is the whole of the cost dispersion, deliberately (Law 2): what makes one firm\'s wage bid differ from its neighbour\'s is what a venue needs two of, and a second dispersion over the inputs would be doing the same work twice. It is technology and not a claim about an answer — firms really do differ in how many hours a tonne takes them.',
  },
  hurdle: {
    low: 0.02,
    high: 0.06,
    why: 'Capital Programme B1.d: the margin over its cost of capital this management insists on before it commits money it cannot get back. Two managements facing the same price do not take the same project, which is what makes the line-up of who expands an outcome rather than a rule.',
  },
  horizonPeriods: {
    low: 52,
    high: 156,
    why: 'Capital Programme B1.d: how many periods of service this management counts. Its patience — one that will not look past a year values a machine at what a year of it is worth, and a growth-against-margin orientation is that and nothing else.',
  },
  size: {
    concentration: 1.2,
    why: 'Firm A3, Seed B4: how big it is beside the others in its own line. A line is not a set of firms of similar size — it is a few that supply most of it and a long tail that supply almost none — so the weight is drawn from a distribution with a tail and never from a width. It is a SHAPE with a scheduled death: what a firm is worth is an OUTCOME of entry, investment and failure, and this world has the last two; firm birth (worklist 13g) adds the first and the full recipe (worklist 15) decides what a line can support, and after both the size distribution is what the mechanisms produced.',
  },
};

/**
 * Seed B1, B1.a: HOW MANY NAMED FIRMS THIS WORLD HAS. A few thousand, because that is what a real
 * economy's named tier is: the firms big enough to have a name in a market, above the small-business
 * tier that is represented as cells (XI-15, worklist 13e). It is a count and nothing follows from
 * changing it but the world having that many.
 */
export const FIRM_COUNT = 3000;

/**
 * A2, Labour A3: which trade each line employs, which is the venue its firms post their openings
 * in. One row per good this world makes, and nothing branches on which (Law 15).
 */
export const OCCUPATION_OF: Readonly<Record<string, string>> = {
  grain: 'field',
  flour: 'mill',
  bread: 'bakery',
  machine: 'works',
};

/**
 * Law 9: the name a market would use. A generated world has generated names, and they are built the
 * way a market builds them — what the firm does and where, numbered so no two are the same party.
 */
const STEMS: Readonly<Record<string, readonly string[]>> = {
  grain: ['Broadacre', 'Middlefield', 'Hollow', 'Longmeadow', 'Stonebridge', 'Fairview'],
  flour: ['Riverside', 'Town', 'Old', 'Kingsmill', 'Waterwheel', 'Northgate'],
  bread: ['City', 'High Street', 'Corner', 'Market', 'Bridgeside', 'Crown'],
  machine: ['North', 'Town', 'Lane', 'Foundry', 'Ironside', 'Anvil'],
};

const TRADE: Readonly<Record<string, string>> = {
  grain: 'Farm',
  flour: 'Mill',
  bread: 'Bakery',
  machine: 'Works',
};

/** Where it is. A market says the trade and the place, and so does this. */
const PLACES: readonly string[] = [
  'Ashby', 'Barrow', 'Calder', 'Denby', 'Elswick', 'Fenton', 'Garsdale', 'Halstead',
  'Ingleton', 'Jarrow', 'Kelsall', 'Linton', 'Marsden', 'Netherby', 'Oakworth', 'Penrith',
  'Quarrend', 'Ravensby', 'Sandwith', 'Thornby', 'Ulverton', 'Ventnor', 'Wenlock', 'Yarrow',
];

/**
 * Seed B1.a, B4: the firms of a world, drawn from the spreads above and deterministic in the
 * world's own seed value (Seed A5, Audit D3). How many of them are in each line is stated as a
 * share of the count, because a world has more bakeries than machine works — bread is made near
 * where it is eaten and a machine is not.
 */
export const LINE_SHARE: Readonly<Record<string, number>> = {
  grain: 0.3,
  flour: 0.15,
  bread: 0.45,
  machine: 0.1,
};

/** A2: how much of the count this line gets. A line this world does not make gets none of it. */
function shareOfLine(subUnit: string): number {
  return zeroIfNone(LINE_SHARE[subUnit]);
}

export function drawFirms(count: number, seed: string): readonly FirmDecl[] {
  const rng = prng(seed, 'firms');
  const out: FirmDecl[] = [];
  const lines = Object.keys(LINE_SHARE).sort();
  // Law 8, Clearing C3: a firm is a whole firm, so the count is split into whole parts that sum to
  // exactly what was asked for, the odd one going to the largest remainder. That rule has ONE
  // writer (`splitOnTick`) and this used to be a second copy of it — the same sort, the same
  // floors, the same tie-break, written out again because a count of parties is not money. It is
  // not money and it does not need to be: what the rule is about is indivisible things.
  const perLine = splitOnTick(asQty(count), lines.map((l) => shareOfLine(l)));
  let n = 0;
  lines.forEach((subUnit, at) => {
    const stems = STEMS[subUnit] ?? [subUnit];
    const trade = TRADE[subUnit] ?? 'Works';
    const occupation = OCCUPATION_OF[subUnit];
    if (occupation === undefined) return;
    const inLine = perLine[at];
    if (inLine === undefined) return;
    for (let k = 0; k < inLine; k += 1) {
      n += 1;
      const place = PLACES[Math.floor(k / stems.length) % PLACES.length] ?? subUnit;
      const round = Math.floor(k / (stems.length * PLACES.length));
      const stem = stems[k % stems.length] ?? subUnit;
      out.push({
        firm: `firm.${n}`,
        name: round === 0 ? `${stem} ${trade}, ${place}` : `${stem} ${trade}, ${place} ${round + 1}`,
        subUnit,
        occupation,
        labourScale: between(rng, FIRM_SPREAD.labourScale),
        hurdle: between(rng, FIRM_SPREAD.hurdle),
        horizonPeriods: betweenWhole(rng, FIRM_SPREAD.horizonPeriods),
        // Seed B4: how big it is beside the others in its line. A sector of equals never produces
        // a market, and a sector of equals is what a uniform draw would give.
        size: drawSize(rng, FIRM_SPREAD.size),
        why: `Seed B4: drawn at the seed from the stated spreads, like every other firm in this world. Nothing about it is stated one firm at a time.`,
      });
    }
  });
  return out;
}

