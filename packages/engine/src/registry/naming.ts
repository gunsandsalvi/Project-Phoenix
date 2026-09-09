/**
 * One naming grammar (Law 9, Observer F1): an instrument is displayed as a market would name it,
 * read from its own terms. An identifier is never a name.
 *
 * @spec Law 9 Observer F1
 */
import type { Parties } from '../parties/party.js';
import type { Instrument } from '../register/instruments.js';
import { INSTRUMENT_PROFILES } from './profiles.js';

export function displayName(i: Instrument, parties: Parties): string {
  return INSTRUMENT_PROFILES[i.kind].displayName(i, parties.get(i.issuer).name);
}
