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

/** A3, B1: coarse and ordinal, best first. */
export const GRADES = ['aaa', 'aa', 'a', 'bbb', 'bb', 'b', 'c'] as const;
export type Grade = (typeof GRADES)[number];

/** The worst grade: where an issuer that has missed a payment goes, whatever else is true of it. */
export const WORST: Grade = 'c';

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
