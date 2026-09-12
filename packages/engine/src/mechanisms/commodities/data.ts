/**
 * STORAGE: the covered space a thing waits in, and what a world declares about it.
 *
 * @spec Commodities Spot A3 Commodities Spot A4 Commodities Spot D2.a Commodities Spot D3 Capital Programme A4 Law 2 Law 15
 *
 * A stock that carries across periods is the state a spot price reads (D2.a), and a stock that
 * carries FOR NOTHING is a state nobody pays to hold — which makes holding free, makes carry zero
 * and makes a commodity curve a formula. So space is PLANT: a stock of productive assets with a
 * life, owned by a named party, built out of a good like any other capital (A4). What it costs to
 * hold a tonne for a week is then a price somebody pays somebody, cleared in a market, and never a
 * rate written down (D3).
 */
import { marketId, venueId, type MarketId, type RegionId, type VenueId } from '../../core/ids.js';
import { STORAGE, type CapitalKindDecl } from '../../registry/physical.js';

/**
 * A4: the kind. Space is not machinery, and a tonne of one is not a tonne of the other. The NAME is
 * the kernel's, because a firm deciding whether its line needs room has to spell it and a module
 * never imports another (4.9b); what a silo IS, and the market in what it lets, are this module's.
 */
export { STORAGE, spaceFor, type SpaceReads } from '../../registry/physical.js';

export const STORAGE_KIND: CapitalKindDecl = {
  id: STORAGE,
  name: 'grain silos',
  unit: 'silos',
  // A4.b, C1: built out of the one capital good this world makes. A silo is a made thing and
  // somebody sold it; what it is made of is that producer's business (Capital Programme A4.a).
  madeFrom: 'machine',
  // Twenty years. Space outlives the machinery that fills it by an order of magnitude, which is why
  // a world short of it stays short for a long time and why the carry it charges is worth watching.
  usefulLifePeriods: 1040,
  buildLagPeriods: 4,
  why: 'Commodities Spot A3, D3: a silo is a stock of productive asset with a life, and holding a tonne in somebody else s is a service they charge for. It is the plant that makes carry real: a world where space is free is a world where a commodity curve is arithmetic rather than a market. ONE SILO IS ONE MACHINE OF BUILDING (A4.b: a kind of capital is made from a good, one for one) and it holds five thousand tonnes, which is what makes the carry it charges a fraction of what is in it rather than a multiple.',
};

/** D3: one book per region, because space in one place is not space in another (A1.a). */
export const storageVenue = (region: RegionId): VenueId => venueId(`venue.storage.${region}`);
export const storageMarket = (region: RegionId): MarketId => marketId(`mkt.storage.${region}`);
