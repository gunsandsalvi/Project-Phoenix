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
import { legKey, placeAt, type GeographyDecl } from './geography.js';
import { asPerPiece, type PerPiece } from '../core/measure.js';
import { none, some, type Option } from '../core/option.js';
import type { Event } from '../journal/journal.js';
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

/** Freight D1 (16.3): the one public record of every leg's session in a period, keyed by the leg. */
export const FREIGHT_SESSION = 'freight.session';

/**
 * Freight C1, C3, Law 19 (16.3): THE RATE A LEG LAST STRUCK, per unit carried, and the period it was
 * struck in — read off the freight module's own record by the leg's name, so a merchant or a
 * shipper in another module reads the print and never a rate table. Nothing where the leg has never
 * cleared: a rate nobody paid is not a rate (Law 3).
 */
export function freightRateOn(
  reads: { lastOf(kind: string, subject: string): Event | undefined },
  from: RegionId,
  to: RegionId,
): Option<{ readonly rate: PerPiece; readonly period: Period }> {
  const said = reads.lastOf(FREIGHT_SESSION, legKey(from, to));
  if (said === undefined) return none();
  const byLeg = said.data['byLeg'];
  if (typeof byLeg !== 'object' || byLeg === null) return none();
  const leg = (byLeg as Record<string, unknown>)[legKey(from, to)];
  if (typeof leg !== 'object' || leg === null) return none();
  const rate = (leg as Record<string, unknown>)['rate'];
  if (typeof rate !== 'number' || rate <= 0) return none();
  return some({ rate: asPerPiece(rate, `what the leg ${String(from)} to ${String(to)} last cleared at`), period: said.period });
}
