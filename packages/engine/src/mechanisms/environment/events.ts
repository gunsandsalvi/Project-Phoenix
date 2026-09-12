/**
 * HOW THE PHYSICAL STATE CROSSES: as a public event, read by everybody and written by nobody else.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 4 Observer A3
 *
 * Four systems need the same fact and none of them may own it (Law 4). A module never imports
 * another (ARCHITECTURE 4.9b), so what crosses is an EVENT — the same door `bank.capital` and
 * `bank.dealing` already cross — and these are its readers. A reader that drew its own hazard
 * instead would be a second representation of one physical thing, which is the defect this module
 * exists to prevent, one level above a number.
 */
import type { Period } from '../../calendar/calendar.js';
import type { RegionId } from '../../core/ids.js';
import type { Event } from '../../journal/journal.js';
import { factId, type FactId } from './data.js';

export const ENVIRONMENT_STATE = 'environment.state';

/** What a reader needs of the journal: the period's events of a kind, and nothing else. */
export interface EventReads {
  readonly period: Period;
  readonly journal: { ofKind(kind: string): readonly Event[] };
}

/**
 * B3, B4: what the world is doing to this region this period, as a multiple of what it normally
 * does. `undefined` is a world with no environment module in it — a scale model built to exercise
 * one door — and a reader says what it does about that rather than assuming an ordinary period,
 * because "normal" is an answer and a missing module is not one.
 */
export function conditionsIn(
  reads: EventReads,
  region: RegionId,
): ReadonlyMap<FactId, number> | undefined {
  for (const e of reads.journal.ofKind(ENVIRONMENT_STATE)) {
    if (e.period !== reads.period || e.subjects[0] !== String(region)) continue;
    const facts = e.data['facts'];
    if (typeof facts !== 'object' || facts === null) continue;
    const out = new Map<FactId, number>();
    for (const [id, value] of Object.entries(facts as Record<string, unknown>)) {
      if (typeof value === 'number') out.set(factId(id), value);
    }
    return out;
  }
  return undefined;
}

/** One fact of one region, or none because this world does not have that fact (or that module). */
export function conditionOf(
  reads: EventReads,
  region: RegionId,
  fact: FactId,
): number | undefined {
  return conditionsIn(reads, region)?.get(fact);
}
