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
import { cohortId, type PartyId, type PartyKindId, regionId, type RegionId } from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import type { CellKeyDimension, Registry } from '../registry/registry.js';

export type PartyStatus =
  | { readonly alive: true }
  | { readonly alive: false; readonly ceasedIn: Period; readonly successor: PartyId };

/**
 * XI-15: a cell's key is the value of every dimension the REGISTRY declares, and of nothing else.
 * It is a map and not a shape on purpose: the dimensions are registry data, so lifting a
 * relationship from a register row into the key is a data change and a re-stratification event,
 * never a change to a mechanism. A key shaped as an interface made that claim false — it said
 * region, cohort and bank in the kernel, and a world that wanted a fourth had to change this file.
 */
export type CellKey = Readonly<Partial<Record<CellKeyDimension, string>>>;

/** XI-15: what one declared dimension of a cell's key says. Absent means the world does not key on it. */
export function keyOf(p: CellParty, dim: CellKeyDimension): string {
  const v = p.key[dim];
  if (v === undefined) {
    throw new Missing('XI-15', `cell ${p.id} has no ${dim}: this world does not key cells on it`, {
      id: p.id,
    });
  }
  return v;
}

/** What a key dimension MEANS, which is the part that cannot be data (Law 15: one dispatch table). */
interface KeyDimensionTerms {
  /**
   * Where the same fact ALSO lives on the party, when it lives twice: the two copies must agree,
   * which is Law 4 held at the door. A dimension with nowhere else to live — a wealth band, a
   * tenure — has none, and having nowhere else to live is precisely why it is a dimension.
   */
  readonly alsoOn?: (p: PartyBase) => string;
  /** Whether the value names something the registry declares, asked without throwing. */
  readonly declared?: (r: Registry, value: string) => boolean;
}

/**
 * XI-15: the one place that says how a key dimension is checked. Which dimensions a world keys its
 * cells on is registry data; what each one means is here, so adding one is a row in the registry
 * and an entry in this table, and never a branch in a mechanism.
 */
const KEY_DIMENSIONS: Readonly<Record<CellKeyDimension, KeyDimensionTerms>> = {
  region: { alsoOn: (p) => p.region, declared: (r, v) => r.regions.has(regionId(v)) },
  cohort: { declared: (r, v) => r.cohorts.some((c) => c.id === cohortId(v)) },
  bank: { alsoOn: (p) => p.bank },
};

/**
 * XI-15: everything wrong with a cell's key, or nothing. One writer of the rule and two readers
 * with different postures: the door throws on the first fault, the names family reports each as a
 * finding — because `add` is not the only writer of a party (`bankAt` rewrites the key) and a rule
 * enforced at one door and nowhere after it is a rule that holds until something else writes.
 */
export function cellKeyFaults(registry: Registry, p: CellParty): readonly string[] {
  const faults: string[] = [];
  const declared = new Set<string>(registry.cellKey);
  for (const dim of registry.cellKey) {
    const value = p.key[dim];
    if (value === undefined) {
      faults.push(`cell ${p.id} has no ${dim}, and this world keys its cells on ${dim}`);
      continue;
    }
    const terms = KEY_DIMENSIONS[dim];
    const also = terms.alsoOn?.(p);
    if (also !== undefined && also !== value) {
      faults.push(`cell ${p.id} keys on ${dim} ${value} but is ${also}`);
    }
    if (terms.declared !== undefined && !terms.declared(registry, value)) {
      faults.push(`cell ${p.id} keys on ${dim} ${value}, which the registry does not declare`);
    }
  }
  for (const dim of Object.keys(p.key)) {
    if (!declared.has(dim)) {
      faults.push(`cell ${p.id} carries ${dim} in its key, which this world does not key cells on`);
    }
  }
  return faults;
}

/**
 * XI-15: change one dimension of a key, and only where this world keys on it. A world that does
 * not stratify on where a cell banks does not acquire the dimension because a cell moved bank.
 */
function rekey(registry: Registry, key: CellKey, dim: CellKeyDimension, value: string): CellKey {
  if (!registry.cellKey.includes(dim)) return key;
  return { ...key, [dim]: value };
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
      const faults = cellKeyFaults(this.registry, p);
      forbid(faults.length === 0, 'XI-15', faults.join('; '), { id: p.id });
    }
    this.map.set(p.id, Object.freeze({ ...p }));
  }

  has(id: PartyId): boolean {
    return this.map.has(id);
  }

  /**
   * XI-15: two cells have the same key when every dimension THIS WORLD declares agrees. Read off
   * the declared list, because a world that keys on a fourth dimension and merged on three would
   * merge two populations that differ on the thing it stratified them by.
   */
  sameKey(a: CellParty, b: CellParty): boolean {
    return this.registry.cellKey.every((dim) => a.key[dim] === b.key[dim]);
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
    // XI-8, Firm Birth D5: every reference resolves to the estate or the successor — and an estate
    // that has paid everything away has nobody left to succeed IT. A kind that says it is terminal
    // may end there, and the names family checks it ended holding nothing (D6.a). Everything else
    // must name somebody: a party that succeeded itself would be a reference that never resolves.
    forbid(
      successor !== id || this.registry.partyKind(p.kind).terminal === true,
      'Register F2',
      `party ${id} cannot succeed itself`,
    );
    this.map.set(
      id,
      Object.freeze({ ...p, status: { alive: false, ceasedIn: period, successor } }),
    );
  }

  /**
   * Banks Funding E1, E3.a: a depositor moves its account to another bank.
   *
   * Where a party banks is one fact with one writer (Law 4), and it is the account every payment TO
   * that party lands in (Money B1), so a party that left its money behind would be a party whose
   * own decisions could not see it. The balance therefore moves with the account, in the
   * instruction the caller settles first: this only records where the party banks from now on, and
   * it is called only after that money has actually arrived.
   *
   * A cell's key names its bank (XI-15), so the key moves with it. Every member of a cell is
   * identical, so a cell either moves or it does not — half a cell moving is a SPLIT, and then the
   * new cell moves, which is the five weight events doing exactly what they are for.
   */
  rebank(id: PartyId, to: PartyId): void {
    const p = this.get(id);
    forbid(p.status.alive, 'Banks Funding E1', `${id} has ceased and banks nowhere`);
    const bank = this.get(to);
    forbid(bank.status.alive, 'Banks Funding E1', `${id} cannot bank at ${to}, which has ceased`);
    forbid(
      this.registry.issuesMoney(bank.kind),
      'Money A1.d',
      `${id} cannot bank at ${to}, which issues no money`,
    );
    forbid(p.bank !== to, 'Banks Funding E1', `${id} already banks at ${to}`);
    this.map.set(
      id,
      Object.freeze(
        p.representation === 'cell'
          ? { ...p, bank: to, key: rekey(this.registry, p.key, 'bank', to) }
          : { ...p, bank: to },
      ),
    );
  }

  /** Resolve a reference through successors (Audit B6: every issuer exists or has a successor). */
  resolve(id: PartyId): Party {
    let p = this.get(id);
    let hops = 0;
    while (!p.status.alive) {
      // A terminal cessation is where the chain ENDS: an estate that paid everything away is
      // succeeded by nobody, and a reference to it resolves to it (XI-8, Firm Birth D5).
      if (p.status.successor === p.id) return p;
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

/**
 * A real read-only facade: entry, death and the weight events are not reachable through it, at
 * runtime as well as in the types. It is what a PARTICIPANT is handed — who somebody is, is public
 * (Observer A3) — and what a party may not do to the world's population is then a fact, not a
 * promise about how the type is used.
 */
export function partiesReads(parties: Parties): PartiesReads {
  return Object.freeze({
    has: (id: PartyId) => parties.has(id),
    get: (id: PartyId) => parties.get(id),
    cell: (id: PartyId) => parties.cell(id),
    all: () => parties.all(),
    alive: () => parties.alive(),
    ofKind: (kind: PartyKindId) => parties.ofKind(kind),
    resolve: (id: PartyId) => parties.resolve(id),
  });
}
