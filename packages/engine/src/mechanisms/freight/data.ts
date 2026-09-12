/**
 * CARRIERS AND ROUTES: who moves things, between which two places, and how much they can move.
 *
 * @spec Freight A1 Freight A2 Freight A3 Freight A4 Freight B1 Freight B2 Freight B3 Freight B4 Capital Programme A4 Law 2 Law 15 Seed B1.a
 *
 * Moving a thing costs money and takes time, and a world where it does neither has one price
 * everywhere by construction — no location basis, no reason for a thing to be dearer where it is
 * short, and nothing for an arbitrageur to close. So a route is a real thing with a real limit: a
 * named carrier's plant, a capacity per period that is ITS OWN and not anybody else's (A4: capacity
 * on one route is not capacity on another), and a transit that takes periods.
 *
 * WHAT IS DECLARED IS A ROUTE AND A WIDTH (Law 2). How much a carrier can move is its PLANT, drawn
 * per carrier from the width below (Seed B1.a), and what it charges is cleared (D1) — there is no
 * freight rate anywhere in this file.
 */
import {
  marketId,
  paramId,
  venueId,
  type MarketId,
  type ParamId,
  type RegionId,
  type VenueId,
} from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { between, drawSize, type Spread, type Tail } from '../../rng/spread.js';
import type { CapitalKindDecl } from '../../registry/physical.js';

/** A4, Capital Programme A4: a ship is plant. Capacity on a route is a stock of it, with a life. */
export const VESSEL = 'vessel';

export const VESSEL_KIND: CapitalKindDecl = {
  id: VESSEL,
  name: 'vessels',
  unit: 'vessels',
  // A4.b, C1: built out of the one capital good this world makes, like every other kind of plant.
  madeFrom: 'machine',
  // Twenty-five years, which is what a hull lasts. It is why freight capacity answers a shortage
  // slowly and why a blocked route stays expensive for longer than the block does (B4, D3.a).
  usefulLifePeriods: 1300,
  buildLagPeriods: 26,
  // A vessel is at sea and is built for far more weather than a shed on land; what takes one is a
  // storm it could not run from (B4, Commodities Spot B3).
  standsWind: 5,
  // 13c.1: A hull is at sea: it stands on no ground, which is part of why shipping capacity is never a claim on a place.
  landPerUnit: null,
  windHardness: 8,
  why: 'Freight A4, B2: capacity is a stock of hulls with a life, and a route is served by the hulls that sail it. It is what makes freight a real limit on how much of a thing can be where it is wanted, rather than a fee on moving it.',
};

/** A4: a route is a PAIR OF PLACES and a time. Capacity on one is not capacity on another. */
export interface RouteDecl {
  readonly from: RegionId;
  readonly to: RegionId;
  /** A3: periods a thing spends in transit. It is on somebody's book the whole time (A3.a). */
  readonly transitPeriods: number;
  /** How many units of the thing one vessel moves per period on this leg. Technology. */
  readonly unitsPerVesselPerPeriod: number;
  /**
   * B4: what this PASSAGE is sailable in, as a multiple of an ordinary period's wind, and how
   * sharply it stops being. It is about the strait and not about the hull — a vessel that stands a
   * gale still does not sail a closed channel — so it is the route's own technology.
   *
   * Not a threshold: what sails is `exp(-(wind / this) ^ hardness)`, positive at every wind and
   * never one, so an ordinary week loses a little and a storm closes the leg (Law 6).
   */
  readonly sailsIn: number;
  readonly sailsHardness: number;
  readonly why: string;
}

/** The parameter a route declares about itself, under its own two places (XI-14). */
export const routeParam = (from: RegionId, to: RegionId, what: string): ParamId =>
  paramId(`freight.${from}.${to}.${what}`);

/** B1: a named carrier, with hulls of its own. A weight of one is a firm; these are firms. */
export interface CarrierDecl {
  readonly carrier: string;
  readonly name: string;
  readonly region: RegionId;
  readonly bank: string;
  /** B2: how many hulls it has, as a multiple of the smallest carrier's. Drawn, never typed. */
  readonly size: number;
  /** B3: the hours its own crews take per unit moved, as a ratio of what the route names. */
  readonly crewScale: number;
  readonly why: string;
}

export const CARRIER_SPREAD: { readonly size: Tail; readonly crewScale: Spread } = {
  size: {
    concentration: 1.6,
    why: 'Freight B1, Seed B4: shipping is a handful of large owners and a long tail of single-hull operators, and a uniform draw between two ends produces neither. What the exponent says is how much of the capacity the biggest carriers are — which is what makes a blocked route somebody in particular refusing.',
  },
  crewScale: {
    low: 0.85,
    high: 1.2,
    why: 'Freight B3, A3.a: two carriers on one route do not cost the same to run, which is why one of them is marginal and the other is not, and why the freight rate is a market rather than an average.',
  },
};

/** D1: one book per route, because room on one leg is not room on another (A4). */
export const freightVenue = (from: RegionId, to: RegionId): VenueId =>
  venueId(`venue.freight.${from}.${to}`);
export const freightMarket = (from: RegionId, to: RegionId): MarketId =>
  marketId(`mkt.freight.${from}.${to}`);

/** Seed A5, Audit D3: the carriers of a world, from its own seed, in their own labelled stream. */
export function drawCarriers(
  count: number,
  regions: readonly RegionId[],
  banks: readonly string[],
  seed: string,
): readonly CarrierDecl[] {
  const rng = prng(seed, 'freight');
  const out: CarrierDecl[] = [];
  for (let n = 0; n < count; n += 1) {
    const region = regions[n % regions.length];
    const bank = banks[n % banks.length];
    if (region === undefined || bank === undefined) continue;
    out.push({
      carrier: `carrier.${n + 1}`,
      name: `Carrier ${n + 1}`,
      region,
      bank,
      size: drawSize(rng, CARRIER_SPREAD.size),
      crewScale: between(rng, CARRIER_SPREAD.crewScale),
      why: 'Freight B1: a named owner of hulls, which is who a shipper is actually buying room from.',
    });
  }
  return out;
}
