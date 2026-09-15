/**
 * A share: the residual claim on a firm, counted in shares, perpetual, carrying a vote.
 *
 * @spec Equity A1 Equity A1.a Equity A1.b Equity A2 Equity A2.a Equity A3 Equity A4 Equity A5 Equity A6 Equity B2 Equity C3 Equity D4 Equity F1 Equity F2 Equity F3 Bond N12 Bond N13 Bond N13.a Law 9 Law 15 XI-15
 *
 * It is NOT a liability of its issuer (A1): a liability is a number the issuer owes and would have
 * to find, and the whole of what a share is, is the claim on whatever is left when every number
 * anybody does owe has been found. So the kernel books nothing against the firm when it issues one
 * and nothing when it buys one back: what the firm gets is the cash, and what it gives up is the
 * cash, which is exactly D1 and D2.b.
 *
 * It promises NOTHING dated (A4): no maturity, no coupon, no cash flow anybody can discount. A
 * dividend is a decision taken period by period and is not a term of the instrument — which is why
 * `due` is empty and why a share cannot default (Bond N12): there is no promise to break. What
 * happens instead is E4, and it happens through the ranking: a share ranks behind every debt
 * (A1.a), so an estate reaches it last, and reaching it last with nothing left is the wipe.
 */
import { scaleQty, type Qty } from '../../core/tick.js';
import { asCash, asRatio, over, pricedAt, type PerPiece } from '../../core/measure.js';
import type { InstrumentKindId, PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { CENT_TICK } from '../../registry/grid.js';
import { instrumentKindId } from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { issuerName } from '../../registry/naming.js';
import { SHARES } from '../../registry/profiles.js';

export const SHARE: InstrumentKindId = instrumentKindId('equity.share');

/** A5: the vote rides with the share, and how many votes a share carries is a term of it. */
export interface ShareTerms extends Terms {
  readonly kind: typeof SHARE;
  /** A6: whose residual claim it is. The line's issuer of record can change (an estate succeeds a
   * dead firm); who PROMISED it cannot, and this is that. */
  readonly issuer: PartyId;
  readonly votesPerShare: number;
}

/**
 * Whether these terms are a share's. Structural, not a kind comparison (Law 15): what makes a share
 * a share is that it names whose residual claim it is and what its holder gets to vote with.
 */
export function isShare(t: Terms): t is ShareTerms {
  return 'issuer' in t && 'votesPerShare' in t;
}

/** The terms of a line that is a share; asking of anything else is a defect in the caller. */
export function shareTerms(i: Instrument): ShareTerms {
  if (!isShare(i.terms)) {
    throw new InvalidRegistry('Equity A1', `${i.id} is not a share`, { instrument: i.id });
  }
  return i.terms;
}

/**
 * F3, A5, XI-15: what a holder can cast — every share it holds, times the votes a share carries. A
 * cell's holding is its members' TOTAL (0f.1), so a represented holder casts exactly what its
 * members hold between them and is not disenfranchised by its representation; a majority is
 * therefore a thing that can be bought (A5.a). This multiplied the total by the weight once more,
 * and the first rung of the ladder (0g.1) cast more votes than there are safe integers.
 */
export function votesOf(units: Qty, terms: ShareTerms): Qty {
  return scaleQty(units, terms.votesPerShare, 'votes');
}

export const shareKind: InstrumentKindProfile = {
  id: SHARE,
  // B2: a price clears per share, per period, out of the schedules holders and buyers post (B1).
  pricing: 'cleared',
  // Law 8: a share is quoted in cents, which is what every share market this world is modelled on
  // quotes in.
  priceTick: CENT_TICK,
  // C3, C4: marked at that price, and the change in the mark is P&L reaching the holder's income —
  // for every holder class without exception, which is what carrying it at the mark means.
  carry: 'mark',
  liabilityOfIssuer: false,
  // A1: a share is the residual claim and not a liability, so its issuer owes nothing against it.
  owes: 'face',
  unit: () => SHARES,
  // D4: the count of a line can be restated without anything else about it changing. It is the one
  // kind in this world that can: par restated is a different promise, and a tonne is a tonne.
  splits: true,
  /**
   * A1.a, A1.b, F2: last. Every number the issuer owes is reached before it, and what it gets is
   * what is left — which can be nothing and is never less than nothing, because a claim that ranks
   * last is redeemed at what the estate could pay and no holder is called on for the rest (A1.b:
   * limited liability is a real property, and here it is the arithmetic of ranking last).
   */
  ranking: () => ({
    seniority: 2,
    secured: [],
    claim: 'whatever is left of the firm once every other claim on it has been paid, and nothing if nothing is',
  }),
  validateTerms: (t: Terms) => {
    if (!isShare(t)) throw new InvalidRegistry('Equity A1', 'share terms name no issuer');
    if (!(t.votesPerShare > 0)) {
      throw new InvalidRegistry('Equity A5', 'a share carries a vote', { votes: t.votesPerShare });
    }
  },
  // Law 9, A6: a market names a share by its issuer, and by nothing else.
  displayName: (i, namer) => `${issuerName(namer, i.id)} shares`,
  // A4: perpetual. Nothing falls due on a share, ever: a dividend is a decision (D3), taken period
  // by period by the firm, and the module that owns the decision pays it (Register E1).
  due: () => [],
  accrued: () => 0,
  // Sovereign D2's question — what is a yield derived FROM — has no answer here, and that is the
  // answer: a share promises no dated payment, so nothing about it can be discounted into a price
  // by anybody but a participant with an opinion (B3).
  cashFlows: () => [],
  /**
   * Equity B1, B3, §46 A3, Capital Programme B1: WHAT A SHARE IS WORTH TO A HOLDER THAT REQUIRES
   * THIS, and it is the same question the same holder asks of a machine — what it expects to get out
   * of it, against what its own money costs it. One valuation technology, in one place (Law 4).
   *
   * What it would get is what the company PUBLISHED (Reporting A2), annualised by the span of its own
   * report; there is no second set of accounts, no forecast and no multiple. What it requires is the
   * caller's, out of its own circumstances, which is what makes two holders want the same share at
   * different levels and what gives the book two sides (§46 A3). Nothing here is a price and nothing
   * here reaches one (Law 3): it is one participant's opinion, and a market decides.
   *
   * `cashFlows` above says a share promises no dated payment, which is true and was the whole
   * problem: every read that discounted a promise found nothing here, so no fund could hold a share
   * and the only bid in the equity book came from parties extrapolating its own price. A company
   * that has published nothing is still worth nothing THROUGH THIS DOOR — that is an absence of
   * evidence and it is returned as one, not as a zero (App A).
   */
  worthTo: (i, required, reads): Option<PerPiece> => {
    const issuer = i.issuer;
    if (!issuer.some) return none<PerPiece>();
    const said = reads.lastReport(issuer.value);
    if (!said.some || said.value.earned <= 0 || said.value.periods <= 0) return none<PerPiece>();
    // Law 8: the periodicity is part of the number. The report covers a span of PERIODS and a
    // required return is quoted per YEAR, so the calendar puts them in one unit — never a factor.
    const ofAYear = yearFraction(
      'ACT/365F',
      reads.calendar.startOf(reads.period),
      reads.calendar.startOf(period(reads.period + said.value.periods)),
    );
    if (ofAYear <= 0) return none<PerPiece>();
    const annual = over(
      asCash(said.value.earned, 'what it published it earned'),
      asRatio(ofAYear, 'the fraction of a year that was'),
      'what it earns a year, as it published it',
    );
    const whole = over(
      annual,
      asRatio(required, 'what this holder requires'),
      'what that stream is worth at what this holder requires',
    );
    const shares = reads.issued(i.id);
    if (shares <= 0) return none<PerPiece>();
    return some(pricedAt(whole, shares, 'what one share of it is worth to this holder'));
  },
};
