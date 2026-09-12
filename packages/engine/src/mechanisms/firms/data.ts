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
export const FIRM_COUNT = 9000;

/**
 * A2, Labour A3: which trade each line employs, which is the venue its firms post their openings
 * in. One row per good this world makes, and nothing branches on which (Law 15).
 */
export const OCCUPATION_OF: Readonly<Record<string, string>> = {
  grain: 'field',
  livestock: 'herding',
  timberLog: 'forestry',
  ironOre: 'mining',
  coalRaw: 'mining',
  crude: 'drilling',
  limestoneRaw: 'quarrying',
  bauxite: 'mining',
  power: 'generation',
  fuel: 'refining',
  steel: 'smelting',
  aluminium: 'smelting',
  lumber: 'sawing',
  cement: 'kiln',
  chemicals: 'chemistry',
  fertiliser: 'chemistry',
  medicine: 'pharmacy',
  flour: 'mill',
  meat: 'butchery',
  wool: 'textile',
  cloth: 'textile',
  clothing: 'garment',
  glass: 'glassmaking',
  plastic: 'moulding',
  rubber: 'moulding',
  paper: 'papermaking',
  bread: 'bakery',
  concrete: 'concreting',
  machine: 'works',
  vessel: 'shipyard',
  vehicle: 'assembly',
  electronics: 'electronics',
  appliance: 'electronics',
  furniture: 'joinery',
  packaging: 'converting',
  building: 'building',
  // 13c.2: the trades of the lines that cannot be put in a box.
  care: 'clinical',
  teaching: 'teaching',
  hospitality: 'catering',
  telecoms: 'telecoms',
  itServices: 'software',
  professional: 'professional',
  design: 'design',
  media: 'media',
  transport: 'driving',
  repair: 'maintenance',
  facilities: 'facilities',
  personalCare: 'grooming',
  entertainment: 'entertainment',
  security: 'security',
  waste: 'sanitation',
  logistics: 'warehousing',
};

/**
 * Law 9: the name a market would use. A generated world has generated names, and they are built the
 * way a market builds them — what the firm does and where, numbered so no two are the same party.
 */
const STEMS: Readonly<Record<string, readonly string[]>> = {
  grain: ['Broadacre', 'Middlefield', 'Hollow', 'Longmeadow', 'Stonebridge', 'Fairview'],
  livestock: ['Highfold', 'Westpasture', 'Drovers', 'Clover', 'Ridgeway', 'Meadowbank'],
  timberLog: ['Blackwood', 'Tallpine', 'Coppice', 'Northwood', 'Elmgrove', 'Birchfell'],
  ironOre: ['Deepshaft', 'Redseam', 'Ironhill', 'Crossgate', 'Blackrock', 'Old Lode'],
  coalRaw: ['Seaton', 'Blackvein', 'Pithead', 'Longwall', 'Greystone', 'Eastmoor'],
  crude: ['Wellhead', 'Deepwater', 'Saltmarsh', 'Derrick', 'Farpoint', 'Brent'],
  limestoneRaw: ['Whitecliff', 'Chalkpit', 'Hardstone', 'Marlgate', 'Limehurst', 'Craghead'],
  bauxite: ['Redearth', 'Clayhill', 'Ochre', 'Highridge', 'Terrace', 'Bauxhill'],
  power: ['Riverside', 'Coastgate', 'Northfield', 'Baseload', 'Steamside', 'Gridpoint'],
  fuel: ['Tidewater', 'Longreach', 'Portside', 'Crackerton', 'Fairhaven', 'Oilgate'],
  steel: ['Ironside', 'Blastgate', 'Furnace', 'Rollingmill', 'Blackforge', 'Kingsteel'],
  aluminium: ['Lightmetal', 'Potline', 'Whitehall', 'Alloygate', 'Reduction', 'Skyfell'],
  lumber: ['Sawbridge', 'Planks', 'Timberside', 'Millrace', 'Woodgate', 'Deal'],
  cement: ['Kilnhurst', 'Greymoor', 'Rotary', 'Clinker', 'Highkiln', 'Marlgate'],
  chemicals: ['Cracker', 'Reactorside', 'Tanksfield', 'Solvent', 'Distill', 'Purity'],
  fertiliser: ['Greenacre', 'Nitrate', 'Fieldgate', 'Yieldmore', 'Bloomfield', 'Cropworks'],
  medicine: ['Aldergate', 'Meridian', 'Purebeck', 'Salix', 'Thornfield', 'Vitalis'],
  flour: ['Riverside', 'Town', 'Old', 'Kingsmill', 'Waterwheel', 'Northgate'],
  meat: ['Shambles', 'Smithfield', 'Drovers', 'Eastgate', 'Butchers Row', 'Longhorn'],
  wool: ['Fleecebank', 'Woolhouse', 'Cardmill', 'Shearside', 'Tuftgate', 'Westfleece'],
  cloth: ['Spinners', 'Weaverside', 'Loomgate', 'Broadcloth', 'Dyehouse', 'Warpend'],
  clothing: ['Tailors', 'Cutgate', 'Seamside', 'Everyday', 'Thimble', 'Highstreet'],
  glass: ['Clearview', 'Furnace', 'Crystalgate', 'Flatglass', 'Brightside', 'Panewick'],
  plastic: ['Mouldgate', 'Polyworks', 'Extrusion', 'Formside', 'Resinhill', 'Lightform'],
  rubber: ['Treadgate', 'Vulcan', 'Blackrubber', 'Compound', 'Rollside', 'Elastic'],
  paper: ['Pulpside', 'Riverpaper', 'Whitefold', 'Reamgate', 'Foldmill', 'Sheetwick'],
  bread: ['City', 'High Street', 'Corner', 'Market', 'Bridgeside', 'Crown'],
  concrete: ['Readymix', 'Batchgate', 'Poursite', 'Aggregate', 'Setstone', 'Cureworks'],
  machine: ['North', 'Town', 'Lane', 'Foundry', 'Ironside', 'Anvil'],
  vessel: ['Drydock', 'Slipway', 'Keelgate', 'Harbourside', 'Longyard', 'Sternfield'],
  vehicle: ['Coachgate', 'Assembly', 'Roadside', 'Axletree', 'Longbody', 'Wheelhouse'],
  electronics: ['Circuitgate', 'Boardworks', 'Silicon', 'Microside', 'Chipfield', 'Signal'],
  appliance: ['Homeworks', 'Whitegoods', 'Hearthside', 'Applegate', 'Domestic', 'Everyhome'],
  furniture: ['Joiners', 'Chairgate', 'Oakside', 'Cabinet', 'Benchwood', 'Homestead'],
  packaging: ['Cartongate', 'Foldside', 'Wrapworks', 'Boxfield', 'Sealgate', 'Palletwick'],
  building: ['Masons', 'Scaffold', 'Groundworks', 'Kingsbuild', 'Cornerstone', 'Newbuild'],
  care: ['St Aldric', 'Meadowbrook', 'Northgate', 'Trinity', 'Lakeside', 'Elmfield'],
  teaching: ['Grammarhill', 'Chantry', 'Kingsmead', 'Bellhouse', 'Oakhill', 'Priory'],
  hospitality: ['The Crown', 'The Anchor', 'Bellrose', 'Saltmarsh', 'The Fox', 'Harbour'],
  telecoms: ['Longline', 'Beacon', 'Exchange', 'Farcall', 'Signal', 'Hillmast'],
  itServices: ['Keystone', 'Sandbox', 'Loopworks', 'Bitfield', 'Runtime', 'Clearcode'],
  professional: ['Aldergate', 'Hartwell', 'Crowe & Vane', 'Marlow', 'Kestrel', 'Thorne & Bell'],
  design: ['Drawingboard', 'Vaultworks', 'Trussfield', 'Cantilever', 'Blueprint', 'Keystone'],
  media: ['Broadsheet', 'Lantern', 'Airwave', 'Pressgate', 'Rollcall', 'Foreground'],
  transport: ['Longhaul', 'Roadside', 'Carrick', 'Wheelgate', 'Nightrun', 'Fairway'],
  repair: ['Fitters', 'Ironhand', 'Spannerworks', 'Callout', 'Overhaul', 'Toolgate'],
  facilities: ['Brightwork', 'Sitecare', 'Keeper', 'Cleanline', 'Groundsman', 'Whitehall'],
  personalCare: ['Chairside', 'Trimmers', 'Ivory', 'Combgate', 'The Parlour', 'Willow'],
  entertainment: ['Palace', 'The Gallery', 'Stands', 'Roundhouse', 'Playhouse', 'Fairground'],
  security: ['Keepgate', 'Nightwatch', 'Sentinel', 'Bulwark', 'Wardens', 'Lockfast'],
  waste: ['Roundsmen', 'Tipgate', 'Kerbside', 'Binfield', 'Sorters', 'Cleanhaul'],
  logistics: ['Dockside', 'Palletgate', 'Crossdock', 'Shedworks', 'Handlers', 'Forkgate'],
};

const TRADE: Readonly<Record<string, string>> = {
  grain: 'Farm',
  livestock: 'Herd',
  timberLog: 'Forestry',
  ironOre: 'Mine',
  coalRaw: 'Colliery',
  crude: 'Field',
  limestoneRaw: 'Quarry',
  bauxite: 'Mine',
  power: 'Power',
  fuel: 'Refinery',
  steel: 'Steelworks',
  aluminium: 'Smelter',
  lumber: 'Sawmill',
  cement: 'Cement Works',
  chemicals: 'Chemicals',
  fertiliser: 'Fertiliser',
  medicine: 'Laboratories',
  flour: 'Mill',
  meat: 'Meatworks',
  wool: 'Woolworks',
  cloth: 'Mills',
  clothing: 'Clothing',
  glass: 'Glassworks',
  plastic: 'Plastics',
  rubber: 'Rubber',
  paper: 'Paper Mill',
  bread: 'Bakery',
  concrete: 'Concrete',
  machine: 'Works',
  vessel: 'Shipyard',
  vehicle: 'Motor Works',
  electronics: 'Electronics',
  appliance: 'Appliances',
  furniture: 'Furniture',
  packaging: 'Packaging',
  building: 'Construction',
  care: 'Infirmary',
  teaching: 'School',
  hospitality: 'Inn',
  telecoms: 'Telephone Company',
  itServices: 'Systems',
  professional: 'Partners',
  design: 'Design',
  media: 'Media',
  transport: 'Haulage',
  repair: 'Engineering Services',
  facilities: 'Services',
  personalCare: 'Salon',
  entertainment: 'Entertainments',
  security: 'Security',
  waste: 'Waste',
  logistics: 'Logistics',
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
  aluminium: 0.004536,
  appliance: 0.009978,
  bauxite: 0.003628,
  bread: 0.040821,
  building: 0.026307,
  care: 0.047541,
  cement: 0.007257,
  chemicals: 0.008164,
  cloth: 0.009978,
  clothing: 0.022678,
  coalRaw: 0.008164,
  concrete: 0.013607,
  crude: 0.003628,
  design: 0.028525,
  electronics: 0.013607,
  entertainment: 0.028525,
  facilities: 0.038033,
  fertiliser: 0.006350,
  flour: 0.013607,
  fuel: 0.005443,
  furniture: 0.013607,
  glass: 0.007257,
  grain: 0.038553,
  hospitality: 0.104590,
  ironOre: 0.005443,
  itServices: 0.023770,
  limestoneRaw: 0.006350,
  livestock: 0.022678,
  logistics: 0.023770,
  lumber: 0.012700,
  machine: 0.012700,
  meat: 0.012700,
  media: 0.019016,
  medicine: 0.005443,
  packaging: 0.010886,
  paper: 0.009071,
  personalCare: 0.052295,
  plastic: 0.009071,
  power: 0.009071,
  professional: 0.057049,
  repair: 0.047541,
  rubber: 0.006350,
  security: 0.019016,
  steel: 0.009071,
  teaching: 0.028525,
  telecoms: 0.004754,
  timberLog: 0.011339,
  transport: 0.047541,
  vehicle: 0.010886,
  vessel: 0.002721,
  waste: 0.009508,
  wool: 0.006351,
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

