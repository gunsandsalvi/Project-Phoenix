/**
 * THE LATTICE A POPULATION LIVES ON — what makes one cell a different cell from another.
 *
 * @spec XI-15 Households A2.e Households A2.f Small-Business Pools A6.a Law 2 Law 15
 *
 * A cell's identity is its KEY on a declared lattice (0f). A dimension is either CATEGORICAL — a
 * fact with a name, moved by the one event that changes it (a hire, a default, a cohort's date
 * crossing) — or BANDED: a quantity read off the cell's own state against a set of edges, and the
 * edges are RESOLUTION (Law 2), tested by invariance: refine every edge by two and the world's
 * aggregates move by less than derived dust, or the lattice is a shape and not a resolution.
 *
 * At most one live cell per key, kept by the kernel: a move onto an occupied key merges. The
 * dimensions are DATA — a kind declares its own — and nothing anywhere branches on which (Law 15).
 * `PartyKindProfile.cellKey` (0b) said which of a closed list of four a kind keyed on; this is
 * that list opened, with the quantities and the events that make a dimension a dimension.
 */
import type { ParamId, PartyId, PartyKindId, CurrencyCode, InstrumentId } from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { Holding } from '../register/register.js';

/** What a dimension read may look at: a cell's own state and the public record, nothing private. */
export interface LatticeReads {
  cashPerMember(cell: PartyId, ccy: CurrencyCode): number;
  perMember(cell: PartyId, instrument: InstrumentId): number;
  holdingsOf(cell: PartyId): readonly Holding[];
  /** What a lot of it is worth at the read, in the cell's own money, or nothing where unpriced. */
  worthPerMember(cell: PartyId, instrument: InstrumentId): Option<number>;
  /** §46: what the cell expects of its own income per period, or nothing where it has no outlook. */
  expectedIncome(cell: PartyId): Option<number>;
  lastEvent(kind: string, subject: PartyId): Option<Event>;
  homeCurrency(cell: PartyId): CurrencyCode;
  period: number;
}

export interface CategoricalDim {
  readonly dim: string;
  /** The one event that moves a member across this dimension (XI-15: a categorical move is owned). */
  readonly movedBy: string;
  /**
   * How a cell the seed places without this dimension is placed on it: a READ of the opening record
   * (no hire yet is unemployed; no default yet is a clean record), never an assumption. A dimension
   * the seed itself supplies — region, bank, cohort, line — has none, and the registry refuses a
   * seeded cell that lacks one of those.
   */
  readonly opening?: (reads: LatticeReads, cell: PartyId) => string;
  readonly why: string;
}

/** The band a cell is in when its quantity cannot be read yet: no history is a real state. */
export const UNREAD = 'unread';

export interface BandedDim {
  readonly dim: string;
  /** The quantity, per member, this dimension bands — or nothing where the cell has no history. */
  readonly quantity: (reads: LatticeReads, cell: PartyId) => Option<number>;
  /** RESOLUTION: one declared edge per parameter, ascending. */
  readonly edges: readonly ParamId[];
  readonly why: string;
}

export interface LatticeDecl {
  readonly kind: PartyKindId;
  readonly categorical: readonly CategoricalDim[];
  readonly banded: readonly BandedDim[];
}

/** Every dimension of a lattice, categorical first, in declaration order: the shape of a key. */
export function latticeDimensions(l: LatticeDecl): readonly string[] {
  return [...l.categorical.map((d) => d.dim), ...l.banded.map((d) => d.dim)];
}

/**
 * XI-15: WHICH BAND A QUANTITY IS IN — the count of edges at or below it, as a string, because a
 * key is names. Edges are ascending; a quantity below the first is band 0.
 */
export function bandOf(edges: readonly number[], x: number): string {
  let n = 0;
  for (const e of edges) if (x >= e) n += 1;
  return String(n);
}
