/**
 * The assessment itself: a grade made from the books the issuer showed, by a party that cannot see
 * a price.
 *
 * @spec Ratings A2 Ratings A2.a Ratings A3 Ratings A5 Ratings B1 Ratings B2 Ratings B2.a Ratings B3 Ratings E3 Corporate Credit A3.b Observer A4 Law 6 Law 19 XI-13
 *
 * A2.a: NOT FROM THE PRICE, and not by choosing not to look: what the assessor is handed is a
 * STATEMENT — income, balance sheet, debt service — and there is no price in one. A grade made
 * from a spread would be the market's own opinion handed back to it with a letter on it — the fixed
 * point XI-13 is about — and every consumer of the grade would then be consuming the price twice.
 *
 * A2, A5, Observer A4 (17.0a): AND IT IS THE STATEMENT THE ISSUER SHOWED IT, not the issuer's private
 * state read over its shoulder. The issuer pays for the opinion (A5) and opens its books to the
 * party it pays; the disclosure is on the record. What is measured is WHAT SERVICING ITS DEBT TOOK
 * against WHAT IT EARNED over the same quarter — a coverage ratio (Corporate Credit A3.b), one
 * number and the one an assessor actually uses; and whether it has MISSED A PAYMENT, which is not a
 * measure at all but a public event (XI-1) and is the one fact that overrides everything else.
 *
 * A3: and the answer is COARSE and STICKY. Coarse because seven bands is what a handful of public
 * facts supports; sticky because the state has to stay across a boundary for the assessor's own
 * patience before the published grade moves — otherwise a rating is a weekly restatement of a
 * balance sheet, and every mandate that refers to it churns with it (C1.a).
 */
import { asRatio, ratioOf, type Ratio, scale } from '../../core/measure.js';
import { atLeast, atMost } from '../../core/num.js';
import type { PartyId } from '../../core/ids.js';
import type { Statement } from '../../registry/statements.js';
import { GRADES, WORST, type AssessorDecl, type Grade } from './data.js';

/**
 * ONE HORIZON for the missed-payment half of the measure: what this assessor looks back over. A
 * missed payment ages out of it, which is what makes a grade a judgement about a party's state NOW
 * rather than a mark that never comes off (A2, A3). As long as the scale is coarse.
 */
export const MISSED_WINDOW = GRADES.length;

/** What this assessor measured, kept so that a published action can say what moved it (A4). */
export interface Measure {
  readonly strain: number;
  readonly missed: number;
  readonly grade: Grade;
}

/**
 * B1, A2: the measure and the band it falls in.
 *
 * STRAIN IS WHAT SERVICING ITS DEBT TOOK AGAINST WHAT THE ISSUER TOOK IN over the quarter it showed
 * — `service` against `earned` less what the marks did, both of them lines of the one statement.
 * That is a COVERAGE RATIO, which is the measure an assessor actually uses, and it is the one this
 * world can make honestly.
 *
 * IT USED TO BE AGAINST WHAT THE ISSUER IS WORTH, and item 12 recorded what that cost (12-17): every
 * treasury in the world graded `c` from period one, because a state's book equity is deeply negative
 * BY CONSTRUCTION — it owes its whole debt and owns nothing — and every firm downgraded together
 * whenever one common cost crossed them all in the same week. Neither of those was a signal about an
 * issuer. A state's capacity to pay is its TAX BASE, a firm's is what it sells, and until item 12a
 * built the equity ledger there was no read of either: what reached a party was recoverable only as
 * the bottom line of everything, marks included.
 *
 * THE MARKS ARE EXCLUDED and that is the distinction the whole measure turns on. A revaluation is
 * what the world now thinks a thing is worth and nobody handed it over; what an issuer can pay a
 * coupon out of is what somebody actually paid IT (Clearing D4, Reporting G2).
 *
 * Law 6: nothing here is clamped. An issuer taking in nothing against what it owes has no coverage
 * that means anything — the ratio is not defined — and gets the worst grade because that is the
 * thing the worst grade is FOR, not because a number was pushed back inside a range.
 */
export function assess(
  shown: Statement,
  /** XI-1: the payments the issuer publicly failed over the assessor's horizon (`missedWindow`). */
  missed: number,
  d: AssessorDecl,
): Measure {
  // Reporting G2, Clearing D4: what it can pay a coupon out of is what it earned before interest,
  // tax, wear and the marks — the statement's own sum (Law 4) — and the marks, which are what the
  // world now thinks a thing is worth and nobody handed over, are not in it.
  const takesIn = shown.summary.ebitda;
  if (missed > 0 || takesIn.pieces <= 0) {
    return { strain: asRatio(missed, 'the payments it missed'), missed, grade: WORST };
  }
  const strain = ratioOf(shown.summary.service, takesIn, 'what its debt took against what it earned');
  return { strain, missed, grade: bandOf(strain, d) };
}

/** B1, A3: which band a strain falls in, on this assessor's own geometrically widening scale. */
export function bandOf(strain: Ratio, d: AssessorDecl): Grade {
  let edge = asRatio(d.firstBoundary, 'where the first band ends');
  for (const grade of GRADES) {
    if (strain < edge) return grade;
    edge = scale(
      edge,
      asRatio(d.boundaryStep, 'how much wider the next band is'),
      'the next band is wider',
    );
  }
  return WORST;
}

/**
 * B2.a: AN INSTRUMENT IS NOT ITS ISSUER. What a holder of one line recovers when the issuer fails
 * is what its own ranking says it recovers, so a claim that ranks behind another is a worse thing
 * to hold than the issuer is a borrower — and a secured one is better. The grade on a line is the
 * issuer's, moved by where the line stands in the queue, which is the instrument's own public terms
 * (the kind's `ranking`) and never a second opinion about the issuer.
 */
export function forInstrument(issuer: Grade, seniority: number, secured: boolean): Grade {
  const at = GRADES.indexOf(issuer);
  // Secured lifts it a step; ranking behind the most senior claim drops it one per step down the
  // queue. Both are the queue itself (Bond N13), read rather than judged.
  const moved = at - (secured ? 1 : 0) + (seniority < 0 ? -seniority : 0);
  // The queue is as long as it is: a step past either end of the scale lands on the end of it,
  // because there is no grade beyond the best and none beyond the worst (Ratings A2).
  const step = atMost(
    atLeast(moved, 0, 'there is no grade above the best one'),
    GRADES.length - 1,
    'there is no grade below the worst one',
  );
  return GRADES[step] ?? WORST;
}

/** A4, E4: who this assessor has an opinion about — everything it is paid to have one about. */
export interface Opinion {
  readonly assessor: PartyId;
  readonly subject: PartyId;
  readonly grade: Grade;
  readonly measured: Measure;
}
