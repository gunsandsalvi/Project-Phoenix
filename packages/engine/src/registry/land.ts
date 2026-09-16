/**
 * THE NAMES OF THE GROUND: what a place's land is called, where it is struck, who holds what nobody
 * has built on, and what a planning policy is called — spelled once, so a firm's investment
 * decision and the capital programme can read them without importing the module that sells it.
 *
 * @spec Capital Programme A2 Capital Programme C1 Goods B4 Law 4 Law 15
 *
 * The land module owns the MECHANISM — the line, the market, the seller and the seed; the firm's
 * bid for ground is part of its project (`registry/capital.ts project`) and the refusal to stand
 * plant on ground it does not hold is the capital programme's (ARCHITECTURE 4.9b).
 */
import { instrumentId, instrumentKindId, marketId, paramId, partyKindId, unitId, type InstrumentId, type MarketId, type PartyId, type RegionId } from '../core/ids.js';
import { upTick, type Qty } from '../core/tick.js';
import { HECTARES_PER_KM2 } from './grid.js';
import { groundUnder, type GroundReads } from './physical.js';

/** A place's ground, as a line. One per place: land in one place is not land in another (A1.a). */
export const LAND = instrumentKindId('land');
export const HECTARE = unitId('km2');
export const landId = (region: RegionId): InstrumentId => instrumentId(`land.${region}`);
export const landMarket = (region: RegionId): MarketId => marketId(`mkt.land.${region}`);

/**
 * Item 15.1: THE LOCAL AUTHORITY — the party present in each place that holds what nobody has built
 * on there and sells it, so a hectare of one place sold to a firm of that place is a trade inside
 * it and not a cross-border one (a country's one treasury selling every place's ground was).
 */
export const LOCAL_AUTHORITY = partyKindId('localAuthority');
export const authorityIdFor = (region: RegionId): PartyId => `authority.${region}` as PartyId;

/**
 * Item 15.1: THE PLANNING POLICY — how much of a place's unbuilt ground its authority releases a
 * period, in hectares. POLICY, parliament's from worklist 14: it is a rule about consents and not a
 * forecast of demand, and the authority never offers everything it holds.
 */
export const PLANNING_RELEASE = paramId('land.planning.releasePerPeriod');

/** Law 8: whole hectares, and UP — plant that stands on a fraction of one stands on the whole one. */
export function hectaresOf(km2: number): Qty {
  return upTick(km2 * HECTARES_PER_KM2);
}

/** Capital Programme C1: the ground so many pieces of a kind of plant stand on, in whole hectares. */
export function hectaresUnder(reads: GroundReads, capitalKind: string, units: Qty): Qty {
  return hectaresOf(groundUnder(reads, [{ capitalKind, units }]));
}
