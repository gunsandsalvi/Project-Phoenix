/**
 * What a household needs a roof for, and what a tenancy is.
 *
 * @spec Housing A1 Housing A2 Housing A3 Households A2.b Households E1 Law 2 Law 15
 *
 * Data only (Law 15). How many dwellings a household needs is a PREFERENCE in physical units, for
 * the reason the consumption basket is (13c.2): a share of income spent on rent is an outcome of a
 * quantity meeting a price, and writing the share down is writing down the answer.
 *
 * It is stated PER MEMBER because a cell is one possible household carried with a multiplicity
 * (XI-15), so what it says is how many people live under one roof — the oldest ratio in housing and
 * the one that turns a population into a number of dwellings.
 */
import { paramId, type ParamId, type RegionId, type VenueId, venueId } from '../../core/ids.js';

/** 13d: the good a roof is. It is built, it stands where it was built, and it wears out. */
export const DWELLING = 'dwelling';

/** A2, A3: one book per place, because a roof in one place is not a roof in another (A1). */
export const rentVenue = (region: RegionId): VenueId => venueId(`rent.${region}`);

export const HOUSING_PARAMS = {
  perMember: (cohort: string): ParamId => paramId(`housing.dwellingsPerMember.${cohort}`),
} as const;

export interface TenureDecl {
  readonly cohort: string;
  /**
   * A1, E1: dwellings one member of this cohort lives in. The reciprocal of household size, which
   * is what it is: about two and a half people to a dwelling for a working household, fewer for a
   * retired one because the children have gone.
   */
  readonly perMember: number;
  readonly why: string;
}

export const TENURE: readonly TenureDecl[] = [
  {
    cohort: 'working',
    perMember: 0.4,
    why: 'Two and a half people to a dwelling. A real-world primitive imported as one (Law 2): it is a fact about how people live, not about this world’s rents.',
  },
  {
    cohort: 'retired',
    perMember: 0.6,
    why: 'Fewer to a dwelling once the children have gone, which is why an ageing population needs MORE dwellings for the same number of people — and that is a mechanism this world now has rather than a claim anybody makes.',
  },
];
