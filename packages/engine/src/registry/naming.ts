/**
 * One naming grammar (Law 9, Observer F1): an instrument is displayed as a market would name it,
 * read from its own terms by its kind's profile. An identifier is never a name.
 *
 * @spec Law 9 Observer F1 Goods A1 Goods C2 Register B3
 *
 * The profile composes the name; the kernel resolves the parts that are state — who promised it,
 * and what the place it trades in is called. A claim is named by its issuer (Bond N1); a physical
 * thing was promised by nobody and is named by what it is and where (Goods A1).
 */
import { Missing } from '../core/errors.js';
import type { RegionId } from '../core/ids.js';
import { none, some, type Option } from '../core/option.js';
import type { PartiesReads } from '../parties/party.js';
import type { Instrument } from '../register/instruments.js';
import type { Contract } from './derivatives.js';
import type { Registry } from './registry.js';

/** The named parts of the world a profile may put in a display name. */
export interface Namer {
  /** The name of whoever promised it, when somebody did. */
  readonly issuer: Option<string>;
  /** What a region is called, for a thing a market names by where it is. */
  readonly region: (id: RegionId) => string;
}

export function displayName(i: Instrument, parties: PartiesReads, registry: Registry): string {
  return registry.instrumentKind(i.kind).displayName(i, {
    issuer: i.issuer.some ? some(parties.get(i.issuer.value).name) : none<string>(),
    region: (id) => registry.region(id).name,
  });
}

/**
 * Law 9, Derivative D12: the same grammar for a CONTRACT. It has no issuer — nobody promised it,
 * two parties agreed it — so the profile is handed none, and what it names itself by is its own
 * identity: what it settles against, its term and its strike.
 */
export function contractName(c: Contract, registry: Registry): string {
  return registry.derivativeKind(c.kind).displayName(c, {
    issuer: none<string>(),
    region: (id) => registry.region(id).name,
  });
}

/** For a kind named by its issuer: having none is a defect in the instrument, not an empty name. */
export function issuerName(n: Namer, what: string): string {
  if (!n.issuer.some) throw new Missing('Law 9', `${what} is named by its issuer and has none`);
  return n.issuer.value;
}
