/**
 * THE NAMES OF COVER: what a policy is, what a unit of cover counts in, where cover is struck, and
 * how long a unit runs — spelled once, so a buyer in another module can post into the book.
 *
 * @spec Insurers A2 Insurers A4 Insurers A4.a Insurers B1 Law 4 Law 15
 *
 * The insurers module owns the MECHANISM — the quote, the clearing, the issue of a policy — and
 * re-exports these so there is one spelling. A firm covering its plant and a household covering
 * its debts are buyers in the same book, and neither may import the module that writes the cover
 * (ARCHITECTURE 4.9b): the name of the book is the kernel's, as a good's name is.
 */
import type { Period } from '../calendar/calendar.js';
import { agreementKindId, paramId, unitId, venueId, type CurrencyCode, type CurveFamilyId, type VenueId } from '../core/ids.js';
import type { Event } from '../journal/journal.js';
import type { AgreementTerms } from '../register/agreements.js';
import type { Civil } from '../calendar/civil.js';
import type { Qty } from '../core/tick.js';

/** Insurers A2, B1 (14.5): a policy is an AGREEMENT — what an insurer promises a named holder, with the schedule on it. */
export const POLICY_ROW = agreementKindId('insurers.policy');
export const COVER = unitId('cover');
/** Insurers B1: how long one unit of cover runs for, a convention of the contract (the insurers module declares it). */
export const COVER_TERM = paramId('insurers.coverTermPeriods');
export const coverVenue = (ccy: CurrencyCode): VenueId => venueId(`cover:${ccy}`);

const LIFECYCLE = 'households.lifecycle';

/**
 * Households F1.b, Insurers B3 (14.2): WHO DIED THIS PERIOD IN A COHORT, as the households module
 * published it — a count of people, read off the public lifecycle events and never a rate anybody
 * stated. It is what a cell observes of its own cohort's mortality and forms its outlook from.
 */
export function deathsIn(reads: { ofKindIn(kind: string, period: Period): readonly Event[] }, cohort: string, period: Period): number {
  let died = 0;
  for (const e of reads.ofKindIn(LIFECYCLE, period)) {
    if (e.data['event'] !== 'died' || e.data['cohort'] !== cohort) continue;
    const members = e.data['members'];
    if (typeof members === 'number') died += members;
  }
  return died;
}

/**
 * Insurers A2, B1, B2 (14.5): THE TERMS OF COVER — how much is covered, until when, and the curve
 * the promise is discounted at. What is owed NOW on the row is nothing until a loss makes a claim;
 * what the row is WORTH is the insurer's expected claims on it over what is left of the term, at
 * the curve — which is what the kernel marks it at (B2.a).
 */
export interface PolicyTerms extends AgreementTerms {
  readonly kind: typeof POLICY_ROW;
  /** Law 8: units of cover, each a unit of the money against a loss. */
  readonly cover: Qty;
  /** B1: the day the term ends; a loss after it is nobody's. */
  readonly to: Civil;
  readonly discountedAt: CurveFamilyId;
}

/** Law 15: a policy row is told by the shape of its terms, never by its kind id. */
export const isPolicyTerms = (t: AgreementTerms): t is PolicyTerms => 'cover' in t && 'to' in t && 'discountedAt' in t;
