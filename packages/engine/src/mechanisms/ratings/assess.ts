/**
 * The assessment itself: a grade made from state, by a party that cannot see a price.
 *
 * @spec Ratings A2 Ratings A2.a Ratings A3 Ratings B1 Ratings B2 Ratings B2.a Ratings B3 Ratings E3 Law 6 Law 19 XI-13
 *
 * A2.a: NOT FROM THE PRICE, and not by choosing not to look: the assessor decides from a view whose
 * prints, marks, indices and curves are all closed (`ctx.blind`). A grade made from a spread would
 * be the market's own opinion handed back to it with a letter on it — the fixed point XI-13 is
 * about — and every consumer of the grade would then be consuming the price twice.
 *
 * A2, Law 19: what is left is the issuer's own public state. WHAT IT HAS FALLING DUE against WHAT
 * IT IS WORTH, which is one number and the only one this world can make honestly yet; and whether
 * it has MISSED A PAYMENT, which is not a measure at all but an event (XI-1) and is the one fact
 * that overrides everything else.
 *
 * A3: and the answer is COARSE and STICKY. Coarse because seven bands is what a handful of public
 * facts supports; sticky because the state has to stay across a boundary for the assessor's own
 * patience before the published grade moves — otherwise a rating is a weekly restatement of a
 * balance sheet, and every mandate that refers to it churns with it (C1.a).
 */
import { period as asPeriod } from '../../calendar/calendar.js';
import { atLeast, atMost, div, mul } from '../../core/num.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import type { ParticipantView } from '../../world/context.js';
import { GRADES, WORST, type AssessorDecl, type Grade } from './data.js';

/** What this assessor measured, kept so that a published action can say what moved it (A4). */
export interface Measure {
  readonly strain: number;
  readonly missed: number;
  readonly grade: Grade;
}

/**
 * B1, A2: the measure and the band it falls in.
 *
 * STRAIN IS WHAT FALLS DUE AGAINST WHAT THE ISSUER TAKES IN — `owedIn` against `earned`, both of
 * them reads of the issuer's own account. That is a COVERAGE RATIO, which is the measure an assessor
 * actually uses, and it is the one this world can make honestly.
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
export function assess(blind: ParticipantView, d: AssessorDecl, ccy: CurrencyCode): Measure {
  // ONE HORIZON for both halves of the measure: what this assessor looks back over. A missed
  // payment ages out of it, which is what makes a grade a judgement about a party's state NOW
  // rather than a mark that never comes off (§44 A2, A3).
  const window = GRADES.length;
  const since = asPeriod(atLeast(blind.period - window, 0, 'there is no period before the world began'));
  const missed = blind.failedPayments(since).length;
  const takesIn = blind.earned(window);
  if (missed > 0 || takesIn <= 0) {
    return { strain: atLeast(missed, 0, 'nobody misses fewer payments than none'), missed, grade: WORST };
  }
  const strain = div(blind.owedIn(ccy), takesIn, 'what falls due against what it takes in');
  return { strain, missed, grade: bandOf(strain, d) };
}

/** B1, A3: which band a strain falls in, on this assessor's own geometrically widening scale. */
export function bandOf(strain: number, d: AssessorDecl): Grade {
  let edge = d.firstBoundary;
  for (const grade of GRADES) {
    if (strain < edge) return grade;
    edge = mul(edge, d.boundaryStep, 'the next band is wider');
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
  const step = atMost(atLeast(moved, 0, 'there is no grade above the best one'), GRADES.length - 1, 'there is no grade below the worst one');
  return GRADES[step] ?? WORST;
}

/** A4, E4: who this assessor has an opinion about — everything it is paid to have one about. */
export interface Opinion {
  readonly assessor: PartyId;
  readonly subject: PartyId;
  readonly grade: Grade;
  readonly measured: Measure;
}
