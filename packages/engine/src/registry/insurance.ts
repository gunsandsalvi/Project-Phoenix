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
import { agreementKindId, paramId, unitId, venueId, type CurrencyCode, type CurveFamilyId, type ParamId, type PartyId, type RegionId, type VenueId } from '../core/ids.js';
import type { Event } from '../journal/journal.js';
import type { Agreement, AgreementTerms } from '../register/agreements.js';
import type { Civil } from '../calendar/civil.js';
import { downTick, type Qty } from '../core/tick.js';
import type { Ratio } from '../core/measure.js';
import { atLeast } from '../core/num.js';

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

/* --------------------------------------------------------------------------------------------
 * PENSIONS (14.6): the names of a promise to pay a retired member, and of the employer behind it.
 * ------------------------------------------------------------------------------------------ */

/**
 * Insurers A2, B1 (14.6): A PENSION IS A ROW — what a named fund promises a named retired cell: so
 * much per member per period, for as long as its members live. The row owes nothing NOW; what it
 * pays each period is read at the payment, and what it is WORTH is the schedule of expected
 * payments — the members alive, decayed by the fund's own outlook of their mortality — at the curve.
 */
export const PENSION_ROW = agreementKindId('insurers.pension');
/**
 * Insurers D3 (14.6): THE SPONSOR STANDS BEHIND THE FUND. A row from an employer to the fund its
 * payroll contributes to, owing nothing until the fund is short — and what it is then called for
 * is its share of the shortfall, a payment that fails on the wire like any other (an arrear).
 */
export const SPONSORSHIP_ROW = agreementKindId('insurers.sponsorship');

/** Insurers A4, D3: the scheme's rules — POLICY, declared by the insurers module and read at the payroll. */
export const PENSION_PARAMS = {
  /** The share of a wage the member pays in. */
  employeeShare: paramId('pensions.contribution.employeeShare'),
  /** The share of a wage the employer pays in beside it. */
  employerShare: paramId('pensions.contribution.employerShare'),
  /** What a retired member is paid a period, as a share of what a week of the benchmark trade earns now. */
  replacementShare: paramId('pensions.replacementShare'),
  /** Over how many periods a shortfall is called from the sponsors (a recovery plan). */
  recoveryPeriods: paramId('pensions.recoveryPeriods'),
} as const;

/** XI-15 (14.6): the lattice dimension that says whether a person is a member of the scheme. */
export const PENSION_DIM = 'pension';
export const PENSION_MEMBER = 'member';
export const PENSION_NONE = 'none';

export interface PensionTerms extends AgreementTerms {
  readonly kind: typeof PENSION_ROW;
  /** Households F1.a: the cohort the members are in — whose mortality the schedule decays at. */
  readonly cohort: string;
  /** The age the cohort enters at, in years — how far past today the youngest member could live. */
  readonly entersAt: number;
  readonly region: RegionId;
  /** Law 9: the trade whose going rate the pension is indexed to — the one most of its members worked in. */
  readonly indexedTo: string;
  readonly discountedAt: CurveFamilyId;
}

export interface SponsorshipTerms extends AgreementTerms {
  readonly kind: typeof SPONSORSHIP_ROW;
  /** The fund the sponsor stands behind (the row's creditor, named on the terms as its shape). */
  readonly standsBehind: PartyId;
}

/** Law 15: a row is told by the shape of its terms, never by its kind id. */
export const isPensionTerms = (t: AgreementTerms): t is PensionTerms => 'cohort' in t && 'indexedTo' in t && 'discountedAt' in t;
export const isSponsorshipTerms = (t: AgreementTerms): t is SponsorshipTerms => 'standsBehind' in t;

/** Insurers D3 (14.6): the fund this employer's payroll contributes to, off its rows — or none. */
export function sponsorshipOf(reads: { owedBy(party: PartyId): readonly Agreement[] }, employer: PartyId): Agreement | undefined {
  return reads.owedBy(employer).find((a) => a.state === 'performing' && isSponsorshipTerms(a.terms));
}

/**
 * Insurers A4, Labour E1, Law 8 (14.6): WHAT A WAGE CARRIES INTO THE FUND, per member, in whole
 * pieces of the money — the member's share deducted from the wage and the employer's paid beside
 * it. A share below one piece a member carries nothing: there is no such coin.
 */
export function contributionsOn(wagePerMember: Qty, shares: { readonly employee: Ratio; readonly employer: Ratio }): { readonly employee: Qty; readonly employer: Qty } {
  return {
    employee: downTick(wagePerMember * shares.employee),
    employer: downTick(wagePerMember * shares.employer),
  };
}

/**
 * Households F1.b, Law 4: THE ONE SPELLING of a mortality band's parameter — declared by the
 * households module per five-year band, read by the pension schedule for where the table ends.
 */
export const mortalityParam = (band: { readonly fromAge: number; readonly toAge: number }): ParamId =>
  paramId(`households.mortality.${String(band.fromAge)}-${String(band.toAge)}`);

const MORTALITY_PARAM = /^households\.mortality\.(\d+)-(\d+)$/;

/**
 * Insurers B3 (14.6): THE AGE THE MORTALITY TABLE ENDS AT — the last band's upper edge, read off
 * the bands the households module declared. Nobody in this world is older than the table says
 * anybody can be, so a pension promised past it is promised to nobody; the schedule stops there.
 * A world with no table has no end, which is a refusal.
 */
export function endOfMortalityTable(params: { all(): readonly { readonly id: ParamId }[] }): number | undefined {
  let end: number | undefined;
  for (const d of params.all()) {
    const m = MORTALITY_PARAM.exec(String(d.id));
    if (m?.[2] === undefined) continue;
    const to = Number(m[2]);
    end = end === undefined ? to : atLeast(end, to, 'the later end');
  }
  return end;
}
