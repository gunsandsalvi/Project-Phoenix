/**
 * What households consume, and how much of it.
 *
 * @spec Households A2.a Households A2.b Households C3 Goods A2.a Law 2 Law 15
 *
 * Data only (Law 15). What a cohort takes is a PREFERENCE, declared per cohort because life stage
 * is one of the ways households differ (A2.b) and the same aggregate income therefore produces
 * different demand depending on who has it (A2.a).
 *
 * IT IS A QUANTITY, NOT A SHARE (13c.2). It used to be the share of its spending that went on a
 * good, which is the strongest substitution assumption there is: spending on a thing never
 * responds to that thing's price, so a household facing bread at twice the price buys half as much
 * and spends the same, for ever, whatever else is on the shelf. Inert while there was one final
 * good; a lie the moment there were two. It is exactly what `phoenix/no-value-recipe` refuses on
 * the production side (Goods A2.a: a recipe is tonnes per tonne and never a share of cost), and the
 * reason is the same on both sides — a share of spending is an OUTCOME of prices meeting a
 * preference, and writing it down is writing down the answer.
 *
 * So a cohort declares two physical quantities per good, per member, per period:
 *
 *   NEEDED   what it will have before it has anything else. A person eats.
 *   WANTED   what it takes on top of that when the money reaches, in the order it can pay for.
 *
 * Both are TECHNOLOGY-shaped facts about people rather than claims about an answer: how much bread
 * a person eats in a week is a real-world primitive and may be imported (Law 2). What a household
 * SPENDS, what share of its income goes on food, and whether the share falls as income rises are
 * all outcomes of these two numbers meeting prices — and the last of those is the oldest measured
 * regularity in economics, which this world now produces instead of assuming.
 *
 * Only FINAL goods appear here: a household never buys grain, which is why an intermediate price is
 * one no household pays (Goods G1.a, G1.b) and why the two indices can diverge at all.
 */

export interface ConsumptionDecl {
  /** A2.b: whose preference this is. A cohort is a key dimension of the cell (XI-15). */
  readonly cohort: string;
  /** C3: the good, by its sub-unit. */
  readonly subUnit: string;
  /**
   * C3: units of it ONE MEMBER takes in a period before it takes anything else, in the good's own
   * physical unit. Zero is a real answer and a common one: most of what a household buys is not a
   * thing it must have this week.
   */
  readonly neededPerMember: number;
  /**
   * C3: units of it one member takes ON TOP of what it needed, when the money reaches that far.
   * Zero is a real answer: a thing nobody takes more of than they need.
   */
  readonly wantedPerMember: number;
  readonly why: string;
}

/**
 * A period is a week (docs/ARCHITECTURE.md 4.7), so every quantity below is per person per week in
 * the good's own unit. Bread is in tonnes, so a loaf is about a thousandth of one.
 */
export const CONSUMPTION: readonly ConsumptionDecl[] = [
  {
    cohort: 'working',
    subUnit: 'bread',
    neededPerMember: 0.0012,
    wantedPerMember: 0.0006,
    why: 'About one and a quarter kilos of bread a week, which is what a person eats, and half as much again when the money reaches. A real-world primitive imported as one (Law 2): it is a fact about people, not about this world’s prices.',
  },
  {
    cohort: 'retired',
    subUnit: 'bread',
    neededPerMember: 0.0010,
    wantedPerMember: 0.0004,
    why: 'A little less, and less again on top: a life stage that eats differently is a different row, which is what makes A2.a produce different demand from the same income.',
  },
];
