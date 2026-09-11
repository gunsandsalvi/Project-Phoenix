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
import { div, mul } from '../../core/num.js';
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
 * STRAIN is what falls due in the issuer's own money, this period and next, against what the issuer
 * is worth — `owedIn` against `equity`, both of them reads of the issuer's own account. Negative
 * strain is an issuer holding more of its own money than it owes, which is the top of the scale;
 * the bands widen geometrically from the assessor's own first boundary, so the scale spends its
 * resolution where issuers actually sit rather than putting six of seven grades inside a few
 * per cent (B3: an assessor's methodology is its own).
 *
 * Law 6: nothing here is clamped. A party worth nothing has no strain that means anything — the
 * ratio is not defined — and gets the worst grade because an issuer with no capital behind what it
 * owes is the thing the worst grade is FOR, not because a number was pushed back inside a range.
 */
export function assess(blind: ParticipantView, d: AssessorDecl, ccy: CurrencyCode): Measure {
  const missed = blind.failedPayments(GRADES.length).length;
  const worth = blind.equity();
  if (missed > 0 || worth <= 0) {
    return { strain: missed > 0 ? missed : 0, missed, grade: WORST };
  }
  const strain = div(blind.owedIn(ccy), worth, 'what falls due against what it is worth');
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
  const bounded = moved < 0 ? 0 : moved;
  return GRADES[bounded < GRADES.length ? bounded : GRADES.length - 1] ?? WORST;
}

/** A4, E4: who this assessor has an opinion about — everything it is paid to have one about. */
export interface Opinion {
  readonly assessor: PartyId;
  readonly subject: PartyId;
  readonly grade: Grade;
  readonly measured: Measure;
}
