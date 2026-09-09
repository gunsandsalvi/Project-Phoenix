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

export const OCCUPATIONS: readonly OccupationDecl[] = [
  { id: 'field', name: 'field work', skill: 'manual', sector: 'agriculture' },
  { id: 'mill', name: 'milling', skill: 'machine operation', sector: 'processing' },
  { id: 'bakery', name: 'baking', skill: 'craft', sector: 'food' },
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
} as const;
