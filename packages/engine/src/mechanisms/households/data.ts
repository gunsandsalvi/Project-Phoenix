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
    subUnit: 'retailBread',
    neededPerMember: 0.0012,
    wantedPerMember: 0.0006,
    why: 'About a kilo and a quarter of bread a week, and half as much again when the money reaches. Bought in a shop, because that is where bread is bought.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailBread',
    neededPerMember: 0.001,
    wantedPerMember: 0.0004,
    why: 'The same line for a household past working age: about a kilo and a quarter of bread a week, and half as much again when the money reaches. Bought in a shop, because that is where bread is bought.',
  },
  {
    cohort: 'working',
    subUnit: 'retailMeat',
    neededPerMember: 0.0005,
    wantedPerMember: 0.0004,
    why: 'Meat and fish, about a kilo a week between what it must have and what it would like.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailMeat',
    neededPerMember: 0.0004,
    wantedPerMember: 0.0003,
    why: 'The same line for a household past working age: meat and fish, about a kilo a week between what it must have and what it would like.',
  },
  {
    cohort: 'working',
    subUnit: 'retailClothing',
    neededPerMember: 0.05,
    wantedPerMember: 0.25,
    why: 'Two or three garments a month at the top of the range and almost none at the bottom: this is the line that moves most with what a household has.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailClothing',
    neededPerMember: 0.03,
    wantedPerMember: 0.1,
    why: 'The same line for a household past working age: two or three garments a month at the top of the range and almost none at the bottom: this is the line that moves most with what a household has.',
  },
  {
    cohort: 'working',
    subUnit: 'retailMedicine',
    neededPerMember: 2e-06,
    wantedPerMember: 1e-06,
    why: 'A dose a day for somebody, and three times as much for somebody older. It is the one line where the retired cohort needs more, and it is why who the people are changes what the economy makes.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailMedicine',
    neededPerMember: 6e-06,
    wantedPerMember: 2e-06,
    why: 'The same line for a household past working age: a dose a day for somebody, and three times as much for somebody older. It is the one line where the retired cohort needs more, and it is why who the people are changes what the economy makes.',
  },
  {
    cohort: 'working',
    subUnit: 'retailFurniture',
    neededPerMember: 0.005,
    wantedPerMember: 0.02,
    why: 'A chair a year that it must have, three or four more when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailFurniture',
    neededPerMember: 0.002,
    wantedPerMember: 0.006,
    why: 'The same line for a household past working age: a chair a year that it must have, three or four more when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'retailAppliance',
    neededPerMember: 0.002,
    wantedPerMember: 0.006,
    why: 'Something breaks and is replaced; something else is wanted.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailAppliance',
    neededPerMember: 0.0015,
    wantedPerMember: 0.003,
    why: 'The same line for a household past working age: something breaks and is replaced; something else is wanted.',
  },
  {
    cohort: 'working',
    subUnit: 'retailElectronics',
    neededPerMember: 0.01,
    wantedPerMember: 0.05,
    why: 'The most discretionary thing in the basket and the youngest cohort’s favourite.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailElectronics',
    neededPerMember: 0.004,
    wantedPerMember: 0.012,
    why: 'The same line for a household past working age: the most discretionary thing in the basket and the youngest cohort’s favourite.',
  },
  {
    cohort: 'working',
    subUnit: 'retailFuel',
    neededPerMember: 8,
    wantedPerMember: 12,
    why: 'Litres a week: the tank, and the heating. It is why a crude shock reaches a household at all.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailFuel',
    neededPerMember: 4,
    wantedPerMember: 6,
    why: 'The same line for a household past working age: litres a week: the tank, and the heating. It is why a crude shock reaches a household at all.',
  },
  {
    cohort: 'working',
    subUnit: 'retailVehicle',
    neededPerMember: 0.0005,
    wantedPerMember: 0.0015,
    why: 'A car every ten years that it must have, and one every five when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'retailVehicle',
    neededPerMember: 0.0002,
    wantedPerMember: 0.0006,
    why: 'The same line for a household past working age: a car every ten years that it must have, and one every five when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'power',
    neededPerMember: 0.03,
    wantedPerMember: 0.02,
    why: 'Megawatt-hours a week. Bought from the generator through no shop, which is what a utility is — and the retired cohort is at home more, which is the whole of why its need is higher.',
  },
  {
    cohort: 'retired',
    subUnit: 'power',
    neededPerMember: 0.032,
    wantedPerMember: 0.015,
    why: 'The same line for a household past working age: megawatt-hours a week. Bought from the generator through no shop, which is what a utility is — and the retired cohort is at home more, which is the whole of why its need is higher.',
  },
  {
    cohort: 'working',
    subUnit: 'care',
    neededPerMember: 0.02,
    wantedPerMember: 0.01,
    why: 'Courses of care a week. Three times as much for the retired cohort, which is the single largest way a population’s age changes what its economy makes.',
  },
  {
    cohort: 'retired',
    subUnit: 'care',
    neededPerMember: 0.06,
    wantedPerMember: 0.02,
    why: 'The same line for a household past working age: courses of care a week. Three times as much for the retired cohort, which is the single largest way a population’s age changes what its economy makes.',
  },
  {
    cohort: 'working',
    subUnit: 'teaching',
    neededPerMember: 0.25,
    wantedPerMember: 0.05,
    why: 'Pupil weeks. A quarter of a working household is at school; a retired one is not, and the line goes to nearly nothing.',
  },
  {
    cohort: 'retired',
    subUnit: 'teaching',
    neededPerMember: 0.0,
    wantedPerMember: 0.01,
    why: 'The same line for a household past working age: pupil weeks. A quarter of a working household is at school; a retired one is not, and the line goes to nearly nothing.',
  },
  {
    cohort: 'working',
    subUnit: 'hospitality',
    neededPerMember: 0.5,
    wantedPerMember: 2.5,
    why: 'Covers: meals out and nights away. Almost all of it is want rather than need, which is why it is the first thing to go when a household is squeezed and the loudest thing in a recession.',
  },
  {
    cohort: 'retired',
    subUnit: 'hospitality',
    neededPerMember: 0.3,
    wantedPerMember: 1.0,
    why: 'The same line for a household past working age: covers: meals out and nights away. Almost all of it is want rather than need, which is why it is the first thing to go when a household is squeezed and the loudest thing in a recession.',
  },
  {
    cohort: 'working',
    subUnit: 'telecoms',
    neededPerMember: 1,
    wantedPerMember: 0.5,
    why: 'Connection weeks. One is not optional any more and the rest is.',
  },
  {
    cohort: 'retired',
    subUnit: 'telecoms',
    neededPerMember: 1,
    wantedPerMember: 0.2,
    why: 'The same line for a household past working age: connection weeks. One is not optional any more and the rest is.',
  },
  {
    cohort: 'working',
    subUnit: 'personalCare',
    neededPerMember: 0.05,
    wantedPerMember: 0.15,
    why: 'Appointments: ten a year that it must have and thirty more when it can.',
  },
  {
    cohort: 'retired',
    subUnit: 'personalCare',
    neededPerMember: 0.05,
    wantedPerMember: 0.1,
    why: 'The same line for a household past working age: appointments: ten a year that it must have and thirty more when it can.',
  },
  {
    cohort: 'working',
    subUnit: 'entertainment',
    neededPerMember: 0.05,
    wantedPerMember: 0.6,
    why: 'Admissions. Nearly all want, and the retired cohort has the time for it.',
  },
  {
    cohort: 'retired',
    subUnit: 'entertainment',
    neededPerMember: 0.05,
    wantedPerMember: 0.5,
    why: 'The same line for a household past working age: admissions. Nearly all want, and the retired cohort has the time for it.',
  },
  {
    cohort: 'working',
    subUnit: 'transport',
    neededPerMember: 3,
    wantedPerMember: 4,
    why: 'Journeys a week. Getting to work is the need; the rest is not, and a retired household makes a third of them.',
  },
  {
    cohort: 'retired',
    subUnit: 'transport',
    neededPerMember: 1,
    wantedPerMember: 2,
    why: 'The same line for a household past working age: journeys a week. Getting to work is the need; the rest is not, and a retired household makes a third of them.',
  },
  {
    cohort: 'working',
    subUnit: 'repair',
    neededPerMember: 0.01,
    wantedPerMember: 0.02,
    why: 'Jobs a week: something in the house, something on the car.',
  },
  {
    cohort: 'retired',
    subUnit: 'repair',
    neededPerMember: 0.012,
    wantedPerMember: 0.015,
    why: 'The same line for a household past working age: jobs a week: something in the house, something on the car.',
  },
];

/**
 * Households F1.b: what a cohort's members die at, per period. Declared per cohort, never per
 * world: the difference between the two rows is the whole reason an ageing population changes what
 * an economy owns and who owns it.
 */
export interface MortalityDecl {
  readonly cohort: string;
  readonly perPeriod: number;
  readonly why: string;
}

/** A period is a week, so these are weekly. Two facts about people, and nothing else. */
export const MORTALITY: readonly MortalityDecl[] = [
  {
    cohort: 'working',
    perPeriod: 0.0000385,
    why: 'Households F1.b: about two in a thousand a year between eighteen and sixty-five. It is a fact about people — a real-world primitive imported as one (Law 2) — and it is why an estate happens to somebody who was not failing.',
  },
  {
    cohort: 'retired',
    perPeriod: 0.000769,
    why: 'Households F1.b: about four in a hundred a year past sixty-five, which is some twenty more years of life. The difference between these two rows is what makes an ageing population change what an economy owns and who owns it.',
  },
];
