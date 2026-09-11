/**
 * What a world's assessors are like, drawn rather than written out.
 *
 * @spec Ratings A1 Ratings A3 Ratings A5 Ratings B3 Ratings D5 Seed B1.a Seed B4 Law 2 Law 15
 *
 * A5, XI-13: THE ASSESSOR IS PAID BY THE ISSUERS IT RATES, which is a reason to rate generously and
 * is stated as one. What counters it is its own record — a grade it gave that defaulted is an event
 * anybody can read (E3) — and nothing else: there is no regulator here and no competitor's opinion
 * weighted against it. That is the conflict the real arrangement has, kept rather than tidied away.
 *
 * Seed B4: there is MORE THAN ONE of them and they are not alike. Two assessors with the same
 * thresholds and the same patience are one assessor with two names, and a world where every grade
 * is unanimous has no second opinion in it at all (XI-13). So each draws its own methodology.
 */
import { partyKindId, paramId, type ParamId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { between, betweenWhole, type Spread } from '../../rng/spread.js';

export const ASSESSOR = partyKindId('assessor');

/**
 * A3, B1: THE SCALE, coarse and ordinal, best first. Coarse because an assessment made from a
 * handful of public facts does not carry twenty distinctions, and because what consumes a grade —
 * a mandate boundary, a risk weight, a haircut — is a step function and a finer scale would only
 * move the steps around. The names are labels; what they order is the list.
 */
export const GRADES = ['aaa', 'aa', 'a', 'bbb', 'bb', 'b', 'c'] as const;
export type Grade = (typeof GRADES)[number];

/** The worst grade: where an issuer that has missed a payment goes, whatever else is true of it. */
export const WORST: Grade = 'c';

export interface AssessorDecl {
  readonly assessor: string;
  readonly name: string;
  readonly bank: string;
  /**
   * A3: HOW LONG THE STATE HAS TO STAY ACROSS A BOUNDARY before it moves the grade, in periods. It
   * is what makes a rating sticky rather than a restatement of this week's balance sheet, and it is
   * this assessor's own: two that reacted at the same speed would downgrade in the same session and
   * a mandate boundary would be crossed by every holder at once for no reason but that (C1.a).
   */
  readonly patience: number;
  /**
   * A2, B1: WHERE THIS ASSESSOR PUTS THE FIRST BOUNDARY, on the measure it makes of an issuer —
   * what falls due against what the issuer is worth. The rest of the scale is this times the step
   * below, so one number and one ratio state a whole methodology rather than seven thresholds that
   * could be inconsistent with each other.
   */
  readonly firstBoundary: number;
  readonly boundaryStep: number;
  /** A5: what it charges an issuer for a rating, per period, per rating it publishes. */
  readonly fee: number;
  readonly why: string;
}

export const ratingParam = (assessor: string, what: string): ParamId =>
  paramId(`rating.${what}.${assessor}`);

export const RATING_PARAMS = {
  riskWeight: (grade: Grade): ParamId => paramId(`regulation.riskWeight.${grade}`),
} as const;

export const ASSESSOR_SPREAD: Readonly<Record<'patience' | 'firstBoundary' | 'boundaryStep' | 'fee', Spread>> = {
  patience: {
    low: 2,
    high: 6,
    why: 'Ratings A3: how many periods the state has to stay across a boundary before this assessor moves the grade. Two to six weeks: long enough that a grade is not a weekly restatement of a balance sheet, short enough that a real deterioration is published while it still matters. Its own, because assessors that all reacted together would make every downgrade a single event that every mandated holder acts on in one session (C1.a).',
  },
  firstBoundary: {
    low: 0.02,
    high: 0.08,
    why: "Ratings A2, B1: where this assessor puts the line between its best grade and the next, on what an issuer has falling due against what it is worth. Two to eight per cent: the width is the disagreement (XI-13), and the level is the assessor's own methodology, which is what it sells. It is a SHAPE — a claim about where the answer is — and it dies when a grade can be measured against the defaults that followed it (E3, Part XII).",
  },
  boundaryStep: {
    low: 1.6,
    high: 3,
    why: 'Ratings A3, B1: how much wider each band is than the one above it. A scale whose bands were equal would put six of its seven grades inside the first few per cent of strain and never use the rest; one that widens geometrically spends its resolution where issuers actually sit. The ratio is the assessor’s own, so two of them disagree about how far apart bbb and bb are as well as about where aaa ends.',
  },
  fee: {
    low: 0.00002,
    high: 0.00008,
    why: "Ratings A5: what this assessor charges an issuer for a rating, per period, as a share of what the issuer is worth. Small, because it is a fee for an opinion and not a cost of capital, and a share rather than a flat amount because a rating on a large issuer is worth more to sell. This is the CONFLICT (A5): the assessor's income comes from the parties it grades, and nothing in this world makes it independent.",
  },
};

/** Seed B1.a: as many assessors as the world asks for, each with its own methodology. */
export function drawAssessors(count: number, banks: readonly string[], seed: string): readonly AssessorDecl[] {
  const rng = prng(seed, 'assessors');
  const out: AssessorDecl[] = [];
  for (let n = 0; n < count; n += 1) {
    const bank = banks[n % banks.length];
    if (bank === undefined) break;
    const own = rng.derive(`assessor/${n}`);
    const id = `assessor.${String.fromCharCode(97 + n)}`;
    out.push({
      assessor: id,
      name: `${String.fromCharCode(65 + n)} Ratings`,
      bank,
      patience: betweenWhole(own, ASSESSOR_SPREAD.patience),
      firstBoundary: between(own, ASSESSOR_SPREAD.firstBoundary),
      boundaryStep: between(own, ASSESSOR_SPREAD.boundaryStep),
      fee: between(own, ASSESSOR_SPREAD.fee),
      why: 'Ratings A1, A5: a named opinion, sold to the issuers it is about.',
    });
  }
  return out;
}

/**
 * Seed B4, XI-13: THREE of them. One is a posted number every mandate refers to and nobody
 * disagrees with, which is the fixed point again; two can only ever agree or split; three can
 * disagree in the way a real market of opinions does, and the SPREAD of their grades on one issuer
 * is a read about this world (E4) rather than a number anybody stated.
 */
export const ASSESSOR_COUNT = 3;
