/**
 * The occupations this world has: what makes one job a different job from another.
 *
 * @spec Labour A3 Labour A3.a Law 15
 *
 * Data only (Law 15). Labour is heterogeneous by skill, sector and region, and that is exactly why
 * unemployment and vacancies can be high at the same time (A3.a): a baker out of work is not a
 * miller's vacancy filled. Region is not in this table because it is the world's own dimension —
 * there is one venue per (region, occupation), built from the regions the registry declares.
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
  { id: 'public', name: 'public service', skill: 'administration', sector: 'government' },
];

/** Hours one person supplies in a period (a week), and the numbers around the relationship. */
export const LABOUR_NUMBERS = {
  /** A1, B2: hours of a person's time in a week. The workforce is people, and this is their time. */
  hoursPerMember: 35,
  /** B3: the age at which a cohort is out of the workforce; a cohort at or above it is inactive. */
  retirementAge: 65,
  /** C2: periods between the match and the day the person is productive. Finding takes time. */
  hiringLagPeriods: 1,
  /** C3: periods of pay a firing costs the employer, paid to the worker it separates. */
  severancePeriods: 4,
  /**
   * A3.b, XI-10 (13d): periods a person who changes trade takes to become productive in the new
   * one, on TOP of the hiring lag. A quarter, which is what learning a trade takes, and it is why
   * an employer fills from its own trade first and why a mover is a real cost to somebody.
   */
  retrainingPeriods: 13,
} as const;
