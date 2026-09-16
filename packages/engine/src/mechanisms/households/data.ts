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

import { WARMTH } from '../../registry/environment.js';

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
  /**
   * Goods B4, Commodities Spot E2 (12d.3): the physical conditions a member takes MORE of this to
   * stand against — what it burns to stay warm in a cold week. The quantities above are a normal
   * week's; the period's are those over how the conditions stand (`registry/environment.ts`), so a
   * week at four fifths of its warmth burns a quarter more. Empty for almost everything: a loaf is
   * a loaf whatever the weather.
   */
  readonly standsAgainst: readonly string[];
  readonly why: string;
}

/**
 * A period is a week (docs/ARCHITECTURE.md 4.7), so every quantity below is per person per week in
 * the good's own unit. Bread is in tonnes, so a loaf is about a thousandth of one.
 */
export const CONSUMPTION: readonly ConsumptionDecl[] = [
  {
    cohort: 'working',
    subUnit: 'retailBread',
    neededPerMember: 0.0012,
    wantedPerMember: 0.0006,
    standsAgainst: [],
    why: 'About a kilo and a quarter of bread a week, and half as much again when the money reaches. Bought in a shop, because that is where bread is bought.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailBread',
    neededPerMember: 0.001,
    wantedPerMember: 0.0004,
    standsAgainst: [],
    why: 'The same line for a household past working age: about a kilo and a quarter of bread a week, and half as much again when the money reaches. Bought in a shop, because that is where bread is bought.',
  },
  {
    cohort: 'working',
    subUnit: 'retailMeat',
    neededPerMember: 0.0005,
    wantedPerMember: 0.0004,
    standsAgainst: [],
    why: 'Meat and fish, about a kilo a week between what it must have and what it would like.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailMeat',
    neededPerMember: 0.0004,
    wantedPerMember: 0.0003,
    standsAgainst: [],
    why: 'The same line for a household past working age: meat and fish, about a kilo a week between what it must have and what it would like.',
  },
  {
    cohort: 'working',
    subUnit: 'retailClothing',
    neededPerMember: 0.05,
    wantedPerMember: 0.25,
    standsAgainst: [],
    why: 'Two or three garments a month at the top of the range and almost none at the bottom: this is the line that moves most with what a household has.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailClothing',
    neededPerMember: 0.03,
    wantedPerMember: 0.1,
    standsAgainst: [],
    why: 'The same line for a household past working age: two or three garments a month at the top of the range and almost none at the bottom: this is the line that moves most with what a household has.',
  },
  {
    cohort: 'working',
    subUnit: 'retailMedicine',
    neededPerMember: 2e-06,
    wantedPerMember: 1e-06,
    standsAgainst: [],
    why: 'A dose a day for somebody, and three times as much for somebody older. It is the one line where the retired cohort needs more, and it is why who the people are changes what the economy makes.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailMedicine',
    neededPerMember: 6e-06,
    wantedPerMember: 2e-06,
    standsAgainst: [],
    why: 'The same line for a household past working age: a dose a day for somebody, and three times as much for somebody older. It is the one line where the retired cohort needs more, and it is why who the people are changes what the economy makes.',
  },
  {
    cohort: 'working',
    subUnit: 'retailFurniture',
    neededPerMember: 0.005,
    wantedPerMember: 0.02,
    standsAgainst: [],
    why: 'A chair a year that it must have, three or four more when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailFurniture',
    neededPerMember: 0.002,
    wantedPerMember: 0.006,
    standsAgainst: [],
    why: 'The same line for a household past working age: a chair a year that it must have, three or four more when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'retailAppliance',
    neededPerMember: 0.002,
    wantedPerMember: 0.006,
    standsAgainst: [],
    why: 'Something breaks and is replaced; something else is wanted.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailAppliance',
    neededPerMember: 0.0015,
    wantedPerMember: 0.003,
    standsAgainst: [],
    why: 'The same line for a household past working age: something breaks and is replaced; something else is wanted.',
  },
  {
    cohort: 'working',
    subUnit: 'retailElectronics',
    neededPerMember: 0.01,
    wantedPerMember: 0.05,
    standsAgainst: [],
    why: 'The most discretionary thing in the basket and the youngest cohort’s favourite.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailElectronics',
    neededPerMember: 0.004,
    wantedPerMember: 0.012,
    standsAgainst: [],
    why: 'The same line for a household past working age: the most discretionary thing in the basket and the youngest cohort’s favourite.',
  },
  {
    cohort: 'working',
    subUnit: 'retailFuel',
    neededPerMember: 8,
    wantedPerMember: 12,
    standsAgainst: [WARMTH],
    why: 'Litres a week: the tank, and the heating. It is why a crude shock reaches a household at all. 12d.3: and the heating is what a cold week takes more of — the litres are a normal week\u2019s, over how warm the week was.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailFuel',
    neededPerMember: 4,
    wantedPerMember: 6,
    standsAgainst: [WARMTH],
    why: 'The same line for a household past working age: litres a week: the tank, and the heating. It is why a crude shock reaches a household at all. 12d.3: and the heating is what a cold week takes more of — the litres are a normal week\u2019s, over how warm the week was.',
  },
  {
    cohort: 'working',
    subUnit: 'retailVehicle',
    neededPerMember: 0.0005,
    wantedPerMember: 0.0015,
    standsAgainst: [],
    why: 'A car every ten years that it must have, and one every five when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailVehicle',
    neededPerMember: 0.0002,
    wantedPerMember: 0.0006,
    standsAgainst: [],
    why: 'The same line for a household past working age: a car every ten years that it must have, and one every five when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'power',
    neededPerMember: 0.03,
    wantedPerMember: 0.02,
    standsAgainst: [],
    why: 'Megawatt-hours a week. Bought from the generator through no shop, which is what a utility is — and the retired cohort is at home more, which is the whole of why its need is higher.',
  },
  {
    cohort: 'retired',
    subUnit: 'power',
    neededPerMember: 0.032,
    wantedPerMember: 0.015,
    standsAgainst: [],
    why: 'The same line for a household past working age: megawatt-hours a week. Bought from the generator through no shop, which is what a utility is — and the retired cohort is at home more, which is the whole of why its need is higher.',
  },
  {
    cohort: 'working',
    subUnit: 'care',
    neededPerMember: 0.02,
    wantedPerMember: 0.01,
    standsAgainst: [],
    why: 'Courses of care a week. Three times as much for the retired cohort, which is the single largest way a population’s age changes what its economy makes.',
  },
  {
    cohort: 'retired',
    subUnit: 'care',
    neededPerMember: 0.06,
    wantedPerMember: 0.02,
    standsAgainst: [],
    why: 'The same line for a household past working age: courses of care a week. Three times as much for the retired cohort, which is the single largest way a population’s age changes what its economy makes.',
  },
  {
    cohort: 'working',
    subUnit: 'teaching',
    neededPerMember: 0.25,
    wantedPerMember: 0.05,
    standsAgainst: [],
    why: 'Pupil weeks. A quarter of a working household is at school; a retired one is not, and the line goes to nearly nothing.',
  },
  {
    cohort: 'retired',
    subUnit: 'teaching',
    neededPerMember: 0.0,
    wantedPerMember: 0.01,
    standsAgainst: [],
    why: 'The same line for a household past working age: pupil weeks. A quarter of a working household is at school; a retired one is not, and the line goes to nearly nothing.',
  },
  {
    cohort: 'working',
    subUnit: 'hospitality',
    neededPerMember: 0.5,
    wantedPerMember: 2.5,
    standsAgainst: [],
    why: 'Covers: meals out and nights away. Almost all of it is want rather than need, which is why it is the first thing to go when a household is squeezed and the loudest thing in a recession.',
  },
  {
    cohort: 'retired',
    subUnit: 'hospitality',
    neededPerMember: 0.3,
    wantedPerMember: 1.0,
    standsAgainst: [],
    why: 'The same line for a household past working age: covers: meals out and nights away. Almost all of it is want rather than need, which is why it is the first thing to go when a household is squeezed and the loudest thing in a recession.',
  },
  {
    cohort: 'working',
    subUnit: 'telecoms',
    neededPerMember: 1,
    wantedPerMember: 0.5,
    standsAgainst: [],
    why: 'Connection weeks. One is not optional any more and the rest is.',
  },
  {
    cohort: 'retired',
    subUnit: 'telecoms',
    neededPerMember: 1,
    wantedPerMember: 0.2,
    standsAgainst: [],
    why: 'The same line for a household past working age: connection weeks. One is not optional any more and the rest is.',
  },
  {
    cohort: 'working',
    subUnit: 'personalCare',
    neededPerMember: 0.05,
    wantedPerMember: 0.15,
    standsAgainst: [],
    why: 'Appointments: ten a year that it must have and thirty more when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'personalCare',
    neededPerMember: 0.05,
    wantedPerMember: 0.1,
    standsAgainst: [],
    why: 'The same line for a household past working age: appointments: ten a year that it must have and thirty more when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'entertainment',
    neededPerMember: 0.05,
    wantedPerMember: 0.6,
    standsAgainst: [],
    why: 'Admissions. Nearly all want, and the retired cohort has the time for it.',
  },
  {
    cohort: 'retired',
    subUnit: 'entertainment',
    neededPerMember: 0.05,
    wantedPerMember: 0.5,
    standsAgainst: [],
    why: 'The same line for a household past working age: admissions. Nearly all want, and the retired cohort has the time for it.',
  },
  {
    cohort: 'working',
    subUnit: 'transport',
    neededPerMember: 3,
    wantedPerMember: 4,
    standsAgainst: [],
    why: 'Journeys a week. Getting to work is the need; the rest is not, and a retired household makes a third of them.',
  },
  {
    cohort: 'retired',
    subUnit: 'transport',
    neededPerMember: 1,
    wantedPerMember: 2,
    standsAgainst: [],
    why: 'The same line for a household past working age: journeys a week. Getting to work is the need; the rest is not, and a retired household makes a third of them.',
  },
  {
    cohort: 'working',
    subUnit: 'repair',
    neededPerMember: 0.01,
    wantedPerMember: 0.02,
    standsAgainst: [],
    why: 'Jobs a week: something in the house, something on the car.',
  },
  {
    cohort: 'retired',
    subUnit: 'repair',
    neededPerMember: 0.012,
    wantedPerMember: 0.015,
    standsAgainst: [],
    why: 'The same line for a household past working age: jobs a week: something in the house, something on the car.',
  },
];

/**
 * Households F1.b: what a cohort's members die at, per period. Declared per cohort, never per
 * world: the difference between the two rows is the whole reason an ageing population changes what
 * an economy owns and who owns it.
 */
/**
 * Households F1.b (12.3): MORTALITY IS TECHNOLOGY, PER FIVE-YEAR BAND OF AGE — an imported real-world
 * primitive (Law 2), declared with its source. What a COHORT dies at is derived: the cohorts are the
 * lattice's bands and a cohort spans several of these, so its rate is the mean over the years it
 * spans, by the same uniform-age geometry that ages a band out (`crossingShare`). Nothing here is
 * per cohort, and a world that cuts its cohorts differently reads the same table.
 */
export interface MortalityDecl {
  readonly fromAge: number;
  /** Exclusive; the last band's end is the end of life this table describes. */
  readonly toAge: number;
  /** The probability of dying within a year, for somebody in the band. */
  readonly perAnnum: number;
  readonly why: string;
}

/** A period is a week, so these are weekly. Two facts about people, and nothing else. */
export const MORTALITY: readonly MortalityDecl[] = [
  { fromAge: 18, toAge: 25, perAnnum: 0.001, why: 'Households F1.b: the chance of dying within a year between 18 and 25. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 25, toAge: 30, perAnnum: 0.0013, why: 'Households F1.b: the chance of dying within a year between 25 and 30. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 30, toAge: 35, perAnnum: 0.0016, why: 'Households F1.b: the chance of dying within a year between 30 and 35. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 35, toAge: 40, perAnnum: 0.002, why: 'Households F1.b: the chance of dying within a year between 35 and 40. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 40, toAge: 45, perAnnum: 0.0027, why: 'Households F1.b: the chance of dying within a year between 40 and 45. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 45, toAge: 50, perAnnum: 0.004, why: 'Households F1.b: the chance of dying within a year between 45 and 50. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 50, toAge: 55, perAnnum: 0.006, why: 'Households F1.b: the chance of dying within a year between 50 and 55. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 55, toAge: 60, perAnnum: 0.009, why: 'Households F1.b: the chance of dying within a year between 55 and 60. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 60, toAge: 65, perAnnum: 0.013, why: 'Households F1.b: the chance of dying within a year between 60 and 65. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 65, toAge: 70, perAnnum: 0.019, why: 'Households F1.b: the chance of dying within a year between 65 and 70. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 70, toAge: 75, perAnnum: 0.029, why: 'Households F1.b: the chance of dying within a year between 70 and 75. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 75, toAge: 80, perAnnum: 0.045, why: 'Households F1.b: the chance of dying within a year between 75 and 80. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 80, toAge: 85, perAnnum: 0.072, why: 'Households F1.b: the chance of dying within a year between 80 and 85. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 85, toAge: 90, perAnnum: 0.12, why: 'Households F1.b: the chance of dying within a year between 85 and 90. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 90, toAge: 95, perAnnum: 0.19, why: 'Households F1.b: the chance of dying within a year between 90 and 95. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
  { fromAge: 95, toAge: 100, perAnnum: 0.3, why: 'Households F1.b: the chance of dying within a year between 95 and 100. Source: United States Social Security Administration, Period Life Table 2020 (Actuarial Study No. 128), the two sexes averaged and rounded to two significant figures — a real-world primitive imported as one (Law 2).' },
];
