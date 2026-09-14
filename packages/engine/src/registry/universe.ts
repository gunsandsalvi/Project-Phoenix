/**
 * The investment universe: what any asset IS, read off the asset itself.
 *
 * @spec Fund Shares A4 Corporate Credit A4 Corporate Credit A4.b Ratings A3 Ratings B1 Bond N4 Bond N13.a Law 2 Law 4 Law 15 Law 19
 *
 * A MANDATE SAYS WHAT A FUND MAY HOLD, and until now it said it as a list of instrument kind ids —
 * `['sovereign.bill']`, `['good.grain']`. That is a kind branch wearing a registry's clothes: it
 * goes stale the moment somebody issues something new, it cannot express "credit, three to seven
 * years, senior" at all, and every vehicle in the world has to be described by enumerating the
 * world. What replaces it is a LANGUAGE — a small schedule of dimensions that any asset answers for
 * itself, and a blueprint that states bands over them (`registry/blueprint.ts`).
 *
 * THE ONE RULE THAT MAKES IT WORK: EVERY DIMENSION IS A READ, NEVER A LABEL. Nothing here is stored
 * on an instrument, assigned by a seed, or maintained by a module. A table mapping instruments to
 * asset classes would be a second representation of facts the world already declares (Law 4), and
 * it would be wrong in the two places it matters most:
 *
 *   - A FIVE-YEAR BOND BECOMES A THREE-YEAR BOND. Duration is the distance from today to the last
 *     thing it pays, so it falls every period on its own. A stored tenor would not.
 *   - A COMPANY FALLS OUT OF LARGE-CAP BY FALLING. Size is what the market says the issuer is
 *     worth, so it is an OUTCOME (Law 2) and a bad year moves a name out of a mandate — which is
 *     the mechanism, not an accident of it.
 *
 * WHAT IT IS COSTS THREE READS AND NO TABLE. Whether somebody promised it, whether the promise ends,
 * and what kind of party made it. Everything else — a bill, commercial paper, a covered bond, a
 * securitisation note, a private company — falls out of those three plus the dimensions below, which
 * is the test this file was written against: a kind invented next year classifies itself.
 */
import type { Civil } from '../calendar/civil.js';
import { dayNumber } from '../calendar/civil.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import { none, some, type Option } from '../core/option.js';
import type { Instrument } from '../register/instruments.js';
import { GRADES, rankOf, type Grade } from './grades.js';

/**
 * What an asset IS, at the coarsest useful grain. Not a label on the instrument: the answer is
 * computed from three facts the world already declares, so nothing can disagree with it.
 */
export type AssetClass =
  /** Nobody promised it. A tonne of grain is not anybody's liability (Goods A1). */
  | 'thing'
  /** Somebody promised it and the promise never ends: a residual claim (Equity A1, A4). */
  | 'residual'
  /** A dated promise by a state. */
  | 'government'
  /** A dated promise by a company or a bank. */
  | 'corporate'
  /** A dated promise by a vehicle, whose assets are the pool and nothing else (XI-11). */
  | 'structured';

/** The party kinds the classification distinguishes. Data, and the only place it is written down. */
export interface ObligorKinds {
  readonly sovereign: readonly string[];
  readonly vehicle: readonly string[];
}

/**
 * Law 15: WHICH PROMISES ARE A STATE'S AND WHICH ARE A POOL'S, as data rather than as a branch.
 *
 * A treasury's paper and a company's differ in what standing behind it means — there is no estate
 * behind a state (Sovereign G3, G5) and nothing but the pool behind a vehicle (XI-11) — and that is
 * a fact about the PARTY, so it is read from the party's kind and stated once here.
 */
export const OBLIGORS: ObligorKinds = {
  sovereign: ['treasury', 'centralBank'],
  vehicle: ['vehicle'],
};

/** Everything the universe can say about one asset. Every field is a read; nothing is stored. */
export interface Classified {
  readonly what: AssetClass;
  readonly ccy: CurrencyCode;
  /** Bond N4: years from today to the last thing it promises. None for a thing or a residual. */
  readonly durationYears: Option<number>;
  /** N13.a: where a claim on it stands, and whether anything is pledged against it. */
  readonly seniority: Option<number>;
  readonly secured: boolean;
  /** Whether a market prices it: the difference between a public company and a private one. */
  readonly listed: boolean;
  /** Ratings A3: the LOWEST grade anybody published on the obligor. None if nobody has looked. */
  readonly grade: Option<Grade>;
  readonly obligor: Option<PartyId>;
}

/** What the classification needs to know about the world, so the reads have one source each. */
export interface UniverseReads {
  readonly on: Civil;
  partyKind(party: PartyId): string;
  /** A1.a: whether the kind promises anything, and what it promises when, from its own profile. */
  promises(i: Instrument): boolean;
  lastFlow(i: Instrument): Option<Civil>;
  ranking(i: Instrument): Option<{ readonly seniority: number; readonly secured: boolean }>;
  /** Ratings A3: every grade any assessor has published on this name, in no particular order. */
  gradesOn(obligor: PartyId): readonly string[];
}

const DAYS_PER_YEAR = 365;

/**
 * A4, Law 19: WHAT THIS ASSET IS, asked of the asset.
 *
 * The three structural reads, in the order that makes them a decision tree with no overlap: did
 * somebody promise it, does the promise end, and whose promise is it. A kind that answers those
 * three answers this, whether or not it existed when this was written.
 */
export function classify(i: Instrument, reads: UniverseReads): Classified {
  const rank = reads.ranking(i);
  const last = reads.lastFlow(i);
  return {
    what: classOf(i, reads, last),
    ccy: i.ccy,
    durationYears: yearsTo(last, reads.on),
    seniority: rank.some ? some(rank.value.seniority) : none<number>(),
    secured: rank.some && rank.value.secured,
    // Clearing D1: a market prices it, or nothing does. The same read 10c uses for securitisable.
    listed: i.market.some,
    grade: i.issuer.some ? lowestGrade(reads.gradesOn(i.issuer.value)) : none<Grade>(),
    obligor: i.issuer,
  };
}

function classOf(i: Instrument, reads: UniverseReads, last: Option<Civil>): AssetClass {
  // Goods A1: nobody issued a tonne of wheat, so there is no promise and no obligor to have one.
  if (!reads.promises(i) || !i.issuer.some) return 'thing';
  // Equity A4: it is PERPETUAL — no maturity, no redemption — which is why equity is a different
  // instrument and not a long bond. A claim that never ends is the residual.
  if (!last.some) return 'residual';
  const kind = reads.partyKind(i.issuer.value);
  if (OBLIGORS.sovereign.includes(kind)) return 'government';
  // XI-11: a vehicle's paper is a claim on a POOL and on nothing else, which is why it is its own
  // class: its standing is where it attaches, not where it ranks among a company's creditors.
  if (OBLIGORS.vehicle.includes(kind)) return 'structured';
  return 'corporate';
}

/** Bond N4: how long it has left, in years — a distance that falls every period on its own. */
function yearsTo(last: Option<Civil>, on: Civil): Option<number> {
  if (!last.some) return none<number>();
  const days = dayNumber(last.value) - dayNumber(on);
  return days <= 0 ? none<number>() : some(days / DAYS_PER_YEAR);
}

/**
 * Ratings A3, A4.b, Corporate Credit A4: THE LOWEST GRADE ANYBODY HAS PUBLISHED ON THIS NAME.
 *
 * This world has several assessors and they are drawn to disagree, which is A4.b's requirement and
 * the reason a book has two sides. What a MANDATE is written to is the conservative one of them:
 * the lowest available, which is what a fund's investors agreed it may hold.
 *
 * IT IS A SELECTION AND NOT A BLEND, which is what keeps it inside Law 2 and Appendix B's "no
 * decision at an average": every candidate is one named assessor's own opinion that it can be wrong
 * about, and what comes out is one of them rather than a number none of them holds.
 *
 * AND IT MAKES A DOWNGRADE TRANSMIT. One assessor moving is enough to put a name below a mandate's
 * line, so the name has to be sold — which is the channel §44 exists for. `middleGrade` in
 * `grades.ts` makes the opposite trade deliberately (a downgrade two assessors must agree on, so
 * one moving changes nothing) and is what the CDS index and the index system divide on. Two
 * conventions for two questions; see item 10e.7, which is where the owner settles whether this
 * world keeps both.
 *
 * A NAME NOBODY HAS ASSESSED HAS NO GRADE, which is a different answer from the worst one (App A).
 */
export function lowestGrade(published: readonly string[]): Option<Grade> {
  let worst = -1;
  for (const g of published) {
    const at = rankOf(g);
    // A grade nobody's scale knows is not an opinion about credit; it is a typo (Law 8).
    if (at > worst) worst = at;
  }
  // The scale is ordinal and BEST FIRST, so the lowest grade is the one with the biggest rank.
  const lowest = GRADES[worst];
  return lowest === undefined ? none<Grade>() : some(lowest);
}
