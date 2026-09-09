/**
 * One naming grammar (Law 9, Observer F1): an instrument is displayed as a market would name it,
 * read from its own terms by its kind's profile. An identifier is never a name.
 *
 * @spec Law 9 Observer F1
 */
import type { PartiesReads } from '../parties/party.js';
import type { Instrument } from '../register/instruments.js';
import type { Registry } from './registry.js';

export function displayName(i: Instrument, parties: PartiesReads, registry: Registry): string {
  return registry.instrumentKind(i.kind).displayName(i, parties.get(i.issuer).name);
}
