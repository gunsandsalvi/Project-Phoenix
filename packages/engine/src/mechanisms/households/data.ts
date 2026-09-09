/**
 * What households consume, and in what proportion.
 *
 * @spec Households A2.b Households C3 Law 15
 *
 * Data only (Law 15). What a cohort spends its money on is a PREFERENCE, declared per cohort
 * because life stage is one of the ways households differ (A2.b) and the same aggregate income
 * therefore produces different demand depending on who has it (A2.a).
 *
 * The weights are shares of what a household decides to spend; how much of a thing that buys is the
 * price's business, so a household facing a dearer loaf buys fewer of them and spends the same on
 * bread. Only FINAL goods appear here: a household never buys grain, which is why an intermediate
 * price is one no household pays (Goods G1.a, G1.b) and why the two indices can diverge at all.
 */

export interface ConsumptionDecl {
  /** A2.b: whose preference this is. A cohort is a key dimension of the cell (XI-15). */
  readonly cohort: string;
  /** C3: the good, by its sub-unit. */
  readonly subUnit: string;
  /** C3: the share of what it spends that goes on this good. The shares of a cohort sum to one. */
  readonly share: number;
  readonly why: string;
}

export const CONSUMPTION: readonly ConsumptionDecl[] = [
  {
    cohort: 'working',
    subUnit: 'bread',
    share: 1,
    why: 'Bread is the only thing this world makes that a household eats. When the chain is longer, this is where the rest of the basket goes, and the shares are what a cohort spends its money on.',
  },
  {
    cohort: 'retired',
    subUnit: 'bread',
    share: 1,
    why: 'The same basket for now; a life stage that spends differently is a different row, which is what makes A2.a produce different demand from the same income.',
  },
];
