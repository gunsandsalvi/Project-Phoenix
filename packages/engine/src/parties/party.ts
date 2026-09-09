/**
 * Parties: named individually, or a cell standing for a population with a weight (XI-15).
 *
 * @spec XI-15 Households A2.e Households A2.f Small-Business Pools A6 Register F2 Seed B2 Seed B3 Law 15
 *
 * A cell is one possible member carried with a multiplicity. It is homogeneous by construction: the
 * register stores per-member state for it, and the cell's totals are `weight x member` at read.
 * The weight is a count and changes by exactly five events: entry, death, promotion, split, merge.
 * Which representation a kind takes is its profile's to say (Law 15).
 */
import { forbid } from '../core/assert.js';
import type { Period } from '../calendar/calendar.js';
import { Forbidden, Missing } from '../core/errors.js';
import type { CohortId, PartyId, PartyKindId, RegionId } from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import type { Registry } from '../registry/registry.js';

export type PartyStatus =
  | { readonly alive: true }
  | { readonly alive: false; readonly ceasedIn: Period; readonly successor: PartyId };

/** The declared key dimensions (XI-15): region, cohort, and the member's bank. */
export interface CellKey {
  readonly region: RegionId;
  readonly cohort: CohortId;
  readonly bank: PartyId;
}

interface PartyBase {
  readonly id: PartyId;
  readonly kind: PartyKindId;
  readonly region: RegionId;
  /** A display name, never a key (Appendix A, Keys). */
  readonly name: string;
  /**
   * Where the party keeps its money (Money B1): the issuer of its account in its home currency. A
   * bank and the treasury bank at the central bank; the central bank is its own issuer.
   */
  readonly bank: PartyId;
  readonly status: PartyStatus;
}

export interface NamedParty extends PartyBase {
  readonly representation: 'named';
}

export interface CellParty extends PartyBase {
  readonly representation: 'cell';
  /** How many real parties this cell IS. A count, never a share (Appendix A). */
  readonly weight: number;
  readonly key: CellKey;
}

export type Party = NamedParty | CellParty;

/** The five events that may change a weight, and nothing else (XI-15). */
export type WeightEventKind = 'entry' | 'death' | 'promotion' | 'split' | 'merge';

export interface WeightEvent {
  readonly kind: WeightEventKind;
  readonly party: PartyId;
  readonly before: number;
  readonly after: number;
  readonly period: Period;
  readonly cause: string;
}

export function weightOf(p: Party): number {
  return p.representation === 'cell' ? p.weight : 1;
}

export class Parties {
  private readonly map = new Map<PartyId, Party>();

  constructor(private readonly registry: Registry) {}

  add(p: Party): void {
    forbid(!this.map.has(p.id), 'Seed B2', `party ${p.id} already exists`, { id: p.id });
    const profile = this.registry.partyKind(p.kind);
    forbid(
      profile.representation === p.representation,
      'XI-15',
      `a ${p.kind} is represented as ${profile.representation}, not ${p.representation}`,
      { id: p.id },
    );
    this.registry.region(p.region);
    if (p.representation === 'cell') {
      positiveCount(p.weight, `weight of ${p.id}`);
      forbid(
        p.key.bank === p.bank,
        'XI-15',
        `cell ${p.id} banks at ${p.bank} but its key says ${p.key.bank}`,
      );
      forbid(
        p.key.region === p.region,
        'XI-15',
        `cell ${p.id} is in ${p.region} but its key says ${p.key.region}`,
      );
      this.registry.cohort(p.key.cohort);
    }
    this.map.set(p.id, Object.freeze({ ...p }));
  }

  has(id: PartyId): boolean {
    return this.map.has(id);
  }

  get(id: PartyId): Party {
    const p = this.map.get(id);
    if (p === undefined) throw new Missing('Audit B6', `party ${id} does not exist`, { id });
    return p;
  }

  cell(id: PartyId): CellParty {
    const p = this.get(id);
    if (p.representation !== 'cell') {
      throw new Forbidden('XI-15', `${id} is a named party, not a cell`);
    }
    return p;
  }

  all(): readonly Party[] {
    return [...this.map.values()];
  }

  /** Alive parties only; a ceased party keeps its identity (Seed B2) but takes no part. */
  alive(): readonly Party[] {
    return this.all().filter((p) => p.status.alive);
  }

  /** Alive parties of one kind, in insertion order (deterministic). */
  ofKind(kind: PartyKindId): readonly Party[] {
    return this.alive().filter((p) => p.kind === kind);
  }

  /**
   * Apply a weight event. This is the only writer of a weight (XI-15). The caller journals it.
   * The register copies per-member state on split and checks identity on merge (world/cells.ts).
   */
  applyWeight(ev: WeightEvent): void {
    const p = this.cell(ev.party);
    forbid(
      p.weight === ev.before,
      'XI-15',
      `weight event on ${ev.party} expected ${ev.before}, is ${p.weight}`,
    );
    positiveCount(ev.after, `weight of ${ev.party} after ${ev.kind}`);
    this.map.set(p.id, Object.freeze({ ...p, weight: ev.after }));
  }

  /** A party that ceases leaves its holdings to a named successor, never to nobody (Register F2). */
  cease(id: PartyId, period: Period, successor: PartyId): void {
    const p = this.get(id);
    forbid(p.status.alive, 'Register F2', `party ${id} has already ceased`);
    forbid(this.has(successor), 'Register F2', `successor ${successor} of ${id} does not exist`);
    forbid(successor !== id, 'Register F2', `party ${id} cannot succeed itself`);
    this.map.set(
      id,
      Object.freeze({ ...p, status: { alive: false, ceasedIn: period, successor } }),
    );
  }

  /** Resolve a reference through successors (Audit B6: every issuer exists or has a successor). */
  resolve(id: PartyId): Party {
    let p = this.get(id);
    let hops = 0;
    while (!p.status.alive) {
      p = this.get(p.status.successor);
      hops += 1;
      if (hops > this.map.size) {
        throw new Forbidden('Register F2', `successor chain from ${id} is cyclic`);
      }
    }
    return p;
  }
}

/** The read-only face of the party store. */
export type PartiesReads = Pick<
  Parties,
  'has' | 'get' | 'cell' | 'all' | 'alive' | 'ofKind' | 'resolve'
>;
