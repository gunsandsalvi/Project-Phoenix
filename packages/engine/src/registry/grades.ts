/**
 * The credit scale: an ordered vocabulary more than one system reads.
 *
 * @spec Ratings A3 Ratings B1 CDS A5.a Indices A1.a Law 4 Law 15
 *
 * It lives in the registry because it is DATA that two systems must order the same way, and a
 * second copy of an ordering is how two systems come to disagree about which way is better (Law 4).
 * The assessors publish opinions on this scale (Ratings A3); a default index divides its series on
 * it (CDS A5.a); a mandate boundary and a risk weight are steps on it. Every one of them has to
 * agree that `bbb` is better than `bb`, and there is one place that says so.
 *
 * What the LABELS mean is nobody's business but the reader's: the scale is ordinal, best first, and
 * what orders it is the list.
 */

import { paramId, type ParamId } from '../core/ids.js';

/** A3, B1: coarse and ordinal, best first. */
export const GRADES = ['aaa', 'aa', 'a', 'bbb', 'bb', 'b', 'c'] as const;
export type Grade = (typeof GRADES)[number];

/**
 * Ratings C2: the weight a supervisor makes a bank hold capital against, per unit of an exposure of
 * a grade. The standard-setter DECLARES it (the ratings module carries the declaration) and every
 * bank READS it by this one name, because a bank and an assessor that spelt the parameter apart
 * would be two rules about one grade (Law 4).
 */
export const riskWeightParam = (grade: Grade): ParamId => paramId(`regulation.riskWeight.${grade}`);

/** The worst grade: where an issuer that has missed a payment goes, whatever else is true of it. */
export const WORST: Grade = 'c';

/**
 * Indices A1, C2, Ratings C2 (17.10): THE LINE THE MARKET DRAWS ACROSS ITS OWN SCALE.
 *
 * Investment grade and high yield are not two scales: they are one scale with a line across it, and
 * everything that matters about a credit market happens at that line. A mandate says which side it
 * may buy, an index is built on one side of it, and a name that crosses it is sold by every holder
 * that may not hold the other side — which is why a downgrade moves a price and a rating means
 * anything at all (C2: an assessor's opinion has consequences because other people's rules refer to
 * it).
 *
 * It is DATA and not a number: the boundary is a grade on the published scale, stated here beside
 * the scale itself, and nothing derives it. Where a market draws it is a convention of that market
 * — this one draws it where every real one does.
 */
export const INVESTMENT_GRADE: Grade = 'bbb';

/** C2: whether a published grade is on the investment side of that line. */
export function isInvestmentGrade(grade: Grade): boolean {
  return rankOf(grade) <= rankOf(INVESTMENT_GRADE);
}

/** Where a grade sits on the scale, best first. A name that is not on it is not a grade. */
export function rankOf(grade: string): number {
  return (GRADES as readonly string[]).indexOf(grade);
}

export const isGrade = (g: string): g is Grade => rankOf(g) >= 0;

/**
 * CDS A5.a, Indices A1.a: THE RULE FOR COMBINING OPINIONS, stated once, publicly and in advance.
 *
 * This world has assessors that disagree and no reason to privilege one of them, so what counts is
 * the MIDDLE opinion: a name moves across a boundary when a majority of those looking at it say it
 * has. That is what makes a downgrade contestable rather than arithmetic — one assessor moving
 * changes nothing, and which two agree is a real question with a real answer.
 *
 * None when nobody has published an opinion: a name nobody has assessed has no grade, which is a
 * different answer from the worst one.
 */
export function middleGrade(grades: readonly string[]): Grade | undefined {
  const ranks = grades
    .filter(isGrade)
    .map(rankOf)
    .sort((a, b) => a - b);
  if (ranks.length === 0) return undefined;
  const middle = ranks[(ranks.length - 1) >> 1];
  return middle === undefined ? undefined : GRADES[middle];
}
