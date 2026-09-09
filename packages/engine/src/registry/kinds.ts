/**
 * The closed sets of kinds whose behaviour lives in profiles (Law 15).
 *
 * @spec Law 15 Granularity XI-15
 *
 * Adding a kind is one entry here plus one profile per dispatch table; the compiler refuses a table
 * that misses one. Mechanics never branch on these values directly (lint: phoenix/no-kind-branch).
 */

/** Who can exist in the world. Named parties and cells (XI-15). */
export type PartyKind =
  'centralBank' | 'treasury' | 'bank' | 'firm' | 'household' | 'smallBusiness';

export const PARTY_KINDS: readonly PartyKind[] = [
  'centralBank',
  'treasury',
  'bank',
  'firm',
  'household',
  'smallBusiness',
];

/** Named individually or represented as cells with a weight (XI-15). */
export type Representation = 'named' | 'cell';

/**
 * What can be held. Money is an instrument (Money D2). A sovereign bond and a sovereign bill are two
 * instruments, not one with a flag (Sovereign B1).
 */
export type InstrumentKind = 'money' | 'sovereign.bond' | 'sovereign.bill';

export const INSTRUMENT_KINDS: readonly InstrumentKind[] = [
  'money',
  'sovereign.bond',
  'sovereign.bill',
];

/** How an instrument gets its value (XI-6). */
export type Pricing = 'money' | 'cleared' | 'carriedAtCost';

/**
 * The stated, consistently applied lot-flow assumption (Register D4; Goods E5 forbids LIFO).
 * Weighted average arrives with inventory (Goods E5); until then FIFO is the only flow.
 */
export type LotFlow = 'FIFO';
