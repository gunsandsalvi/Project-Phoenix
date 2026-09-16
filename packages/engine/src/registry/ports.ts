/**
 * THE NAMES OF A PORT: who owns the quay of a place, how many berths it works a period, and how
 * many calls it has worked this period — spelled once, read off the ledger (Law 19).
 *
 * @spec Freight B2 Freight B4 Freight D6 Law 4 Law 19
 *
 * ITEM 15.2: A PORT HAS AN OWNER AND A BERTH. The place's local authority owns its quay (15.1's
 * party present in each place), and a berth works one CALL a period — a vessel loading to sail or
 * landing what it carried. Congestion is an OUTCOME of that count and nothing else: a vessel that
 * arrives at a quay whose berths are all worked this period waits at anchor until one is free, and a
 * cargo that cleared a session at a port with no berth left does not load. No parameter says how
 * congested a port is; the calls this period are counted off the ledger's voyage legs, and what
 * waits is what the count turned away.
 */
import { paramId, type PartyId, type PlaceId, type VoyageId } from '../core/ids.js';
import type { Period } from '../calendar/calendar.js';
import type { Leg } from '../ledger/instruction.js';
import type { Voyage } from '../register/voyages.js';
import { placeAt, type GeographyDecl } from './geography.js';
import { authorityIdFor } from './land.js';
import type { RegionId } from '../core/ids.js';

/**
 * 15.2, Freight B1, B2: HOW MANY BERTHS A PLACE'S QUAY WORKS A PERIOD. A SHAPE with a scheduled
 * death: a berth is capital the port authority builds and wears out, and until it is a vintage of
 * the authority's own (22a's opening states the port as it states every stock) it is this number.
 */
export const PORT_BERTHS = paramId('port.berthsPerPlace');

/** 15.2: the quay of a place is its authority's. */
export const portOwnerOf = (place: PlaceId): PartyId => authorityIdFor(place as RegionId);

export interface CallReads {
  inPeriod(period: Period): readonly { readonly outcome: string; readonly instruction: { readonly legs: readonly Leg[] } }[];
}
export interface VoyageLookup {
  get(id: VoyageId): Voyage;
}

/** The place a voyage ends in — the quay it lands at. */
export function landsAt(g: GeographyDecl, v: Pick<Voyage, 'tiles'>): PlaceId | undefined {
  const last = v.tiles[v.tiles.length - 1];
  return last === undefined ? undefined : placeAt(g, last);
}

/** The place a voyage starts from — the quay it loads at. */
export function sailsFrom(g: GeographyDecl, tiles: readonly Voyage['tiles'][number][]): PlaceId | undefined {
  const first = tiles[0];
  return first === undefined ? undefined : placeAt(g, first);
}

/**
 * 15.2, Law 19: THE CALLS A QUAY HAS WORKED THIS PERIOD — every vessel that loaded to sail from it
 * and every one that landed at it, counted off the settled voyage legs of the period. Nothing is
 * kept between the reads; the ledger is the one record of what moved.
 */
export function callsAt(reads: CallReads, voyages: VoyageLookup, g: GeographyDecl, period: Period, place: PlaceId): number {
  let calls = 0;
  for (const r of reads.inPeriod(period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (leg.kind !== 'voyage') continue;
      if (leg.act === 'sail' && sailsFrom(g, leg.tiles) === place) calls += 1;
      if (leg.act === 'land' && landsAt(g, voyages.get(leg.voyage)) === place) calls += 1;
    }
  }
  return calls;
}
