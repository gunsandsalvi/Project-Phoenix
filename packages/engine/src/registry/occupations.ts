/**
 * THE OCCUPATIONS THIS WORLD HAS, and which trade each line employs — data, in the registry.
 *
 * @spec Labour A3 Labour A3.a Firm A2 Law 15
 *
 * Two tables, one subject. `OCCUPATIONS` lived in the labour module and `OCCUPATION_OF` in the
 * firms module, and the OBSERVER imported both — the one place in the engine that may import no
 * module, reaching into two of them for a list (0e′.5, OB5). Law 15 says all DATA is in a registry,
 * and a trade is data: nothing anywhere branches on which one a line employs. The modules read the
 * same rows from here; what moved is where the rows live, not what they say.
 */

export interface OccupationDecl {
  readonly id: string;
  readonly name: string;
  /** A3: what a person must be able to do; it is why a match in one is not a match in another. */
  readonly skill: string;
  /** A3: where the work is, in the production chain. */
  readonly sector: string;
}

/**
 * A3: labour is heterogeneous by skill and by sector, and a job in one is not a job in another —
 * which is why unemployment and vacancies can be high at once (A3.a). One trade per line this world
 * makes, plus the state's, and nothing anywhere branches on which (Law 15).
 */
export const OCCUPATIONS: readonly OccupationDecl[] = [
  { id: 'field', name: 'field work', skill: 'manual', sector: 'agriculture' },
  { id: 'herding', name: 'stock keeping', skill: 'manual', sector: 'agriculture' },
  { id: 'forestry', name: 'forestry', skill: 'manual', sector: 'agriculture' },
  { id: 'mining', name: 'mining', skill: 'manual', sector: 'extraction' },
  { id: 'drilling', name: 'drilling', skill: 'machine operation', sector: 'extraction' },
  { id: 'quarrying', name: 'quarrying', skill: 'manual', sector: 'extraction' },
  { id: 'generation', name: 'power generation', skill: 'machine operation', sector: 'utilities' },
  { id: 'refining', name: 'refining', skill: 'process', sector: 'energy' },
  { id: 'smelting', name: 'smelting', skill: 'process', sector: 'metals' },
  { id: 'sawing', name: 'sawmilling', skill: 'machine operation', sector: 'processing' },
  { id: 'kiln', name: 'kiln work', skill: 'process', sector: 'minerals' },
  { id: 'chemistry', name: 'process chemistry', skill: 'process', sector: 'chemicals' },
  { id: 'pharmacy', name: 'pharmaceutical work', skill: 'laboratory', sector: 'pharmaceuticals' },
  { id: 'mill', name: 'milling', skill: 'machine operation', sector: 'processing' },
  { id: 'butchery', name: 'meat processing', skill: 'craft', sector: 'food' },
  { id: 'textile', name: 'textile work', skill: 'craft', sector: 'textiles' },
  { id: 'garment', name: 'garment making', skill: 'craft', sector: 'textiles' },
  { id: 'glassmaking', name: 'glassmaking', skill: 'craft', sector: 'minerals' },
  { id: 'moulding', name: 'moulding', skill: 'machine operation', sector: 'chemicals' },
  { id: 'papermaking', name: 'papermaking', skill: 'machine operation', sector: 'paper' },
  { id: 'bakery', name: 'baking', skill: 'craft', sector: 'food' },
  { id: 'concreting', name: 'concreting', skill: 'manual', sector: 'construction' },
  { id: 'works', name: 'machine building', skill: 'engineering', sector: 'capital goods' },
  { id: 'shipyard', name: 'shipbuilding', skill: 'engineering', sector: 'capital goods' },
  { id: 'assembly', name: 'vehicle assembly', skill: 'engineering', sector: 'transport equipment' },
  { id: 'electronics', name: 'electronics assembly', skill: 'precision', sector: 'electronics' },
  { id: 'joinery', name: 'joinery', skill: 'craft', sector: 'furniture' },
  { id: 'converting', name: 'packaging converting', skill: 'machine operation', sector: 'packaging' },
  { id: 'building', name: 'building trades', skill: 'manual', sector: 'construction' },
  // ----------------------------------------------------------------------------------------------
  // 13c.2: the trades of the lines that cannot be put in a box. A3.a is the whole reason they are
  // here as sixteen rows rather than one: a nurse out of work is not a bricklayer's vacancy filled,
  // and a world where every service was one trade would answer a shortage of clinicians by sending
  // it a security guard.
  // ----------------------------------------------------------------------------------------------
  { id: 'clinical', name: 'clinical work', skill: 'professional', sector: 'health' },
  { id: 'teaching', name: 'teaching', skill: 'professional', sector: 'education' },
  { id: 'catering', name: 'catering and hospitality', skill: 'craft', sector: 'hospitality' },
  { id: 'telecoms', name: 'network engineering', skill: 'engineering', sector: 'communications' },
  { id: 'software', name: 'software and support', skill: 'technical', sector: 'information' },
  { id: 'professional', name: 'professional practice', skill: 'professional', sector: 'business services' },
  { id: 'design', name: 'design and engineering', skill: 'professional', sector: 'business services' },
  { id: 'media', name: 'media production', skill: 'creative', sector: 'media' },
  { id: 'driving', name: 'driving', skill: 'machine operation', sector: 'transport' },
  { id: 'maintenance', name: 'repair and maintenance', skill: 'craft', sector: 'repair' },
  { id: 'facilities', name: 'facilities work', skill: 'manual', sector: 'support services' },
  { id: 'grooming', name: 'personal care', skill: 'craft', sector: 'personal services' },
  { id: 'entertainment', name: 'performance and recreation', skill: 'creative', sector: 'recreation' },
  { id: 'security', name: 'security work', skill: 'manual', sector: 'support services' },
  { id: 'sanitation', name: 'waste collection', skill: 'manual', sector: 'utilities' },
  { id: 'warehousing', name: 'warehouse handling', skill: 'manual', sector: 'logistics' },
  { id: 'retail', name: 'shop work', skill: 'service', sector: 'retail' },
  { id: 'wholesale', name: 'merchant trading', skill: 'service', sector: 'wholesale' },
  // ----------------------------------------------------------------------------------------------
  // 13d: THE TRADES A BANK EMPLOYS. A bank's costs were a number added to every borrower's rate and
  // paid to nobody (`loan.operatingCost`), which is margin wearing the clothes of a cost. These are
  // the people that number stood in for, and now somebody is paid it.
  // ----------------------------------------------------------------------------------------------
  { id: 'banking', name: 'lending and branch work', skill: 'administration', sector: 'finance' },
  { id: 'dealing', name: 'market making', skill: 'professional', sector: 'finance' },
  { id: 'analysis', name: 'research and credit analysis', skill: 'professional', sector: 'finance' },
  // 10f.4: THE PEOPLE WHO RUN A SALE. *"M&A processes lead by IBD departments"* — a corporate-finance
  // banker is not a lending officer and not a market maker (A3), and what a bank can run at once is
  // how many of them it employs. A capacity stated directly would be a count of people wearing a
  // policy's clothes; this is the people.
  { id: 'advisory', name: 'corporate finance advisory', skill: 'professional', sector: 'finance' },
  { id: 'public', name: 'public service', skill: 'administration', sector: 'government' },
];

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
  dwelling: 'building',
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
  wholesale: 'wholesale',
  // 13c.2: the shop. One trade for all nine lines of it — shop work is shop work, and a world
  // that made a fishmonger a different trade from a greengrocer would be inventing a labour market.
  retailAppliance: 'retail',
  retailBread: 'retail',
  retailClothing: 'retail',
  retailElectronics: 'retail',
  retailFuel: 'retail',
  retailFurniture: 'retail',
  retailMeat: 'retail',
  retailMedicine: 'retail',
  retailVehicle: 'retail',
};
