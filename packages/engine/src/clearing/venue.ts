/**
 * A venue: a place where posted schedules clear into something that is not the transfer of an
 * instrument, and which the module that owns it therefore settles itself.
 *
 * @spec Clearing A1 Clearing B2 Labour A4 Labour C5 Labour D1 Law 4 Law 15 XI-10
 *
 * A market moves an instrument against money and the kernel settles it. A labour market strikes a
 * RELATIONSHIP — a firm, a worker, a wage, a start date — which persists and is paid period after
 * period (Labour A4, XI-10), so what a match produces is a row in somebody's register, not a leg.
 * The kernel still owns the book and the solver: schedules are posted here, one place, emptied at
 * the top of every period, and the venue's owner clears them with the same solver every market uses.
 *
 * A venue is declared like a market and is public: an employer's opening is a real posted intention
 * anyone can see (C5), and its key says what makes it itself — the region and the occupation — so
 * another system can find the venue it needs without knowing how this one names things.
 */
import type { CurrencyCode, UnitId, VenueId } from '../core/ids.js';

export interface VenueDecl {
  readonly id: VenueId;
  readonly name: string;
  /** Law 4: the module that clears it, which is the only one that may act on its matches. */
  readonly clearedBy: string;
  /** What a posted quantity is counted in: hours of an occupation's time (Labour A1). */
  readonly unit: UnitId;
  /** What a posted price is in: a wage per unit of time, in a currency (Labour A2). */
  readonly ccy: CurrencyCode;
  /** The dimensions that make this venue itself, as data (Law 15): region, occupation, ... */
  readonly key: Readonly<Record<string, string>>;
}

/** The venue whose key matches every dimension asked for, or none: a venue nobody declared. */
export function findVenue(
  venues: readonly VenueDecl[],
  key: Readonly<Record<string, string>>,
): VenueDecl | undefined {
  return venues.find((v) => Object.entries(key).every(([k, value]) => v.key[k] === value));
}
