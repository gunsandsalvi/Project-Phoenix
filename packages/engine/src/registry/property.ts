/**
 * THE NAMES OF COMMERCIAL PROPERTY: the landlord kind, the lease row, the lettings venue of a
 * place, the rent it last struck, and what a tenant's own plan said it was short of — spelled
 * once, so a tenant in another module and the capital programme can read them without importing
 * the module that lets the space (ARCHITECTURE 4.9b).
 *
 * @spec Housing A3 Capital Programme A2 Capital Programme C1 Law 4 Law 15 Law 19
 */
import { agreementKindId, paramId, partyKindId, venueId, type PartyId, type RegionId, type VenueId } from '../core/ids.js';
import { asPerPiece, type PerPiece } from '../core/measure.js';
import { none, some, type Option } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { Period } from '../calendar/calendar.js';
import type { Agreement } from '../register/agreements.js';
import { isLeaseTerms, type LeaseTerms } from './physical.js';

export { isLeaseTerms, type LeaseTerms } from './physical.js';

/** Item 15.3: A LANDLORD — a mass sector, cells of them with a lattice, owning space built to let. */
export const LANDLORD = partyKindId('landlord');
/** Item 15.3: a lease of plant, with a term (`registry/physical.ts LeaseTerms`). */
export const LEASE_ROW = agreementKindId('property.lease');
/** The kind of plant this sector lets: the room a shop, a surgery or an office is (Capital Programme A2). */
export const PREMISES = 'premises';
/** Law 9: a place has one lettings book for premises, named for it. */
export const lettingsVenue = (region: RegionId): VenueId => venueId(`lettings.premises:${region}`);

export const PROPERTY_PARAMS = {
  /** Seed C4: how many landlords bank at each bank at the opening. A shape with its death at 22a. */
  landlordsPerBank: paramId('property.seed.landlordsPerBank'),
  /** Seed C4: how many units of premises a landlord opens holding. A shape with its death at 22a. */
  premisesPerLandlord: paramId('property.seed.premisesPerLandlord'),
  /** A convention of the contract: how long a lease runs for. */
  leaseTerm: paramId('property.leaseTermPeriods'),
  /** PREFERENCE: how far a landlord counts the rent when it prices a building. */
  horizon: paramId('property.horizonPeriods'),
  /** PREFERENCE: what it costs a landlord to move the account its rents land in (Banks Funding A1.b). */
  switchingCost: paramId('property.switchingCost'),
} as const;

export const PROPERTY_EDGES = {
  size: [paramId('property.lattice.size.1')],
} as const;

/** The public print of a place's lettings book. */
export const RENT_PRINT = 'property.rent';
const FIRM_PLAN = 'firms.plan';

/**
 * 15.3, Law 19, Law 8: the rent a place's book LAST STRUCK, per unit of premises per period, or
 * nothing where it has never struck one. A print that cleared nothing carries the last level and
 * the period it was struck in (`struckIn`), so the level is there and its age is visible.
 */
export function rentStruckIn(reads: { lastPublicAbout(kind: string, subject: string): Option<Event> }, region: RegionId): Option<PerPiece> {
  const said = reads.lastPublicAbout(RENT_PRINT, String(lettingsVenue(region)));
  if (!said.some) return none<PerPiece>();
  const rent = said.value.data['rentPerUnit'];
  return typeof rent === 'number' && rent > 0 ? some(asPerPiece(rent, 'the rent a unit of premises let at, per period')) : none<PerPiece>();
}

/** 15.3: what a firm's own plan this period said — where its capacity binds and what a unit earns it. */
export interface PlanRead {
  readonly output: string;
  readonly bound: string;
  readonly capacity: number;
  readonly runRate: number;
  readonly expectedPrice: number;
  readonly unitCost: number;
}

export function planOf(reads: { lastOwnSince(kind: string, since: Period): Option<Event> }, period: Period): Option<PlanRead> {
  const said = reads.lastOwnSince(FIRM_PLAN, period);
  if (!said.some || said.value.data['planned'] !== true) return none<PlanRead>();
  const d = said.value.data;
  const output = d['output'];
  const bound = d['bound'];
  const capacity = d['capacity'];
  const runRate = d['runRate'];
  const expectedPrice = d['expectedPrice'];
  const unitCost = d['unitCost'];
  if (typeof output !== 'string' || typeof bound !== 'string') return none<PlanRead>();
  if (typeof capacity !== 'number' || typeof runRate !== 'number' || typeof expectedPrice !== 'number' || typeof unitCost !== 'number') return none<PlanRead>();
  return some({ output, bound, capacity, runRate, expectedPrice, unitCost });
}

/** 15.3: the units of a kind of plant a landlord has let, off its rows. */
export function unitsLetBy(rows: readonly Agreement[], landlord: PartyId, capitalKind: string): number {
  let out = 0;
  for (const a of rows) {
    if (a.state !== 'performing' || a.creditor !== landlord || !isLeaseTerms(a.terms)) continue;
    if (a.terms.capitalKind === capitalKind) out += a.terms.units;
  }
  return out;
}

/** 15.3: the leases a tenant holds, as rows. */
export function leasesHeldBy(rows: readonly Agreement[], tenant: PartyId): readonly (Agreement & { readonly terms: LeaseTerms })[] {
  const out: (Agreement & { readonly terms: LeaseTerms })[] = [];
  for (const a of rows) {
    if (a.state !== 'performing' || a.debtor !== tenant || !isLeaseTerms(a.terms)) continue;
    out.push(a as Agreement & { readonly terms: LeaseTerms });
  }
  return out;
}
