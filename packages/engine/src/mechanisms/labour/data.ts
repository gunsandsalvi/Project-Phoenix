/**
 * The labour module's numbers. Its occupations are `registry/occupations.ts` (Law 15: data in a
 * registry; 0e′.5), re-exported here so the module's own readers keep one door.
 *
 * @spec Labour A3 Labour A3.a Law 15
 *
 * Data only (Law 15). Labour is heterogeneous by skill, sector and region, and that is exactly why
 * unemployment and vacancies can be high at the same time (A3.a): a baker out of work is not a
 * miller's vacancy filled. Region is not in this table because it is the world's own dimension —
 * there is one venue per (region, occupation), built from the regions the registry declares.
 */

/** Hours one person supplies in a period (a week), and the numbers around the relationship. */
export { OCCUPATIONS, type OccupationDecl } from '../../registry/occupations.js';

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
