/**
 * The registry: all DATA lives here (Law 15), and the dispatch tables of kind profiles that
 * modules register at assembly. Behaviour that varies by kind lives in those profiles.
 *
 * @spec Law 15 Currency A5 Currency A2 Seed B3 Appendix A XI-15
 *
 * The registry validates that its own references resolve at construction (Audit B6 at the data
 * level). References to parties (a currency's central bank, a cell's bank) are validated when the
 * world is built, because parties are state, not data.
 */
import { InvalidRegistry, Missing } from '../core/errors.js';
import type {
  CohortId,
  CurrencyCode,
  CurveFamilyId,
  InstrumentKindId,
  PartyId,
  PartyKindId,
  RegionId,
  UnitId,
} from '../core/ids.js';
import type { CurveFamilyDecl } from '../prices/curve.js';
import type { InstrumentKindProfile, LotFlow, PartyKindProfile } from './kinds.js';

export interface CurrencyDecl {
  readonly code: CurrencyCode;
  readonly name: string;
  /** The named issuer whose liability the currency is (Currency A2). */
  readonly centralBank: PartyId;
}

export interface RegionDecl {
  readonly id: RegionId;
  readonly name: string;
  /** The region determines its parties' money (Seed B3, Currency B1). */
  readonly ccy: CurrencyCode;
}

export interface UnitDecl {
  readonly id: UnitId;
  readonly name: string;
  /** A countable unit takes integer quantities only (shares, contracts, dwellings). */
  readonly countable: boolean;
}

/** A cohort is a life stage; ageing is a split at the boundary by date (Households F1.a). */
export interface CohortDecl {
  readonly id: CohortId;
  readonly name: string;
  /** Age at which a member enters this cohort, in years; the last cohort has no exit. */
  readonly fromAge: number;
}

/** The cell key dimensions are registry data (XI-15): lifting a relationship into the key is a data change. */
export type CellKeyDimension = 'region' | 'cohort' | 'bank';

export interface RegistryData {
  readonly currencies: readonly CurrencyDecl[];
  readonly regions: readonly RegionDecl[];
  readonly units: readonly UnitDecl[];
  readonly cohorts: readonly CohortDecl[];
  readonly cellKey: readonly CellKeyDimension[];
  /** The stated, consistently applied lot-flow assumption for securities (Register D4). */
  readonly lotFlow: LotFlow;
  /** The kind profiles the kernel and the assembled modules register (Law 15). */
  readonly instrumentKinds: readonly InstrumentKindProfile[];
  readonly partyKinds: readonly PartyKindProfile[];
  /** The curve families their owning modules declare (Sovereign D3.a: one owner each). */
  readonly curveFamilies: readonly CurveFamilyDecl[];
}

export class Registry {
  readonly currencies: ReadonlyMap<CurrencyCode, CurrencyDecl>;
  readonly regions: ReadonlyMap<RegionId, RegionDecl>;
  readonly units: ReadonlyMap<UnitId, UnitDecl>;
  readonly cohorts: readonly CohortDecl[];
  readonly cellKey: readonly CellKeyDimension[];
  readonly lotFlow: LotFlow;
  readonly curveFamilies: ReadonlyMap<CurveFamilyId, CurveFamilyDecl>;
  readonly instrumentKinds: ReadonlyMap<InstrumentKindId, InstrumentKindProfile>;
  readonly partyKinds: ReadonlyMap<PartyKindId, PartyKindProfile>;

  constructor(data: RegistryData) {
    this.currencies = unique(data.currencies, (c) => c.code, 'currency');
    this.regions = unique(data.regions, (r) => r.id, 'region');
    this.units = unique(data.units, (u) => u.id, 'unit');
    this.cohorts = [...data.cohorts];
    this.cellKey = [...data.cellKey];
    this.lotFlow = data.lotFlow;
    this.instrumentKinds = unique(data.instrumentKinds, (k) => k.id, 'instrument kind');
    this.partyKinds = unique(data.partyKinds, (k) => k.id, 'party kind');
    this.curveFamilies = unique(data.curveFamilies, (c) => c.id, 'curve family');

    for (const r of this.regions.values()) {
      if (!this.currencies.has(r.ccy)) {
        throw new InvalidRegistry(
          'Seed B3',
          `region ${r.id} names currency ${r.ccy}, which does not exist`,
        );
      }
    }
    for (const c of this.currencies.values()) {
      const unit = `ccy:${c.code}` as UnitId;
      if (!this.units.has(unit)) {
        throw new InvalidRegistry('Appendix A', `currency ${c.code} has no unit ${unit} declared`);
      }
    }
    if (this.cohorts.length === 0) {
      throw new InvalidRegistry('XI-15', 'at least one cohort is needed');
    }
    for (let i = 1; i < this.cohorts.length; i += 1) {
      const prev = this.cohorts[i - 1];
      const cur = this.cohorts[i];
      if (prev !== undefined && cur !== undefined && cur.fromAge <= prev.fromAge) {
        throw new InvalidRegistry('Households F1.a', 'cohorts must be in ascending age order');
      }
    }
    const dims = new Set(this.cellKey);
    if (dims.size !== this.cellKey.length) {
      throw new InvalidRegistry('XI-15', 'cell key dimension repeated');
    }
    if (!dims.has('region')) throw new InvalidRegistry('XI-15', 'the cell key must include region');
    if (!this.instrumentKinds.has('money' as InstrumentKindId)) {
      throw new InvalidRegistry('Money D2', 'the money instrument kind must be registered');
    }
    for (const f of this.curveFamilies.values()) {
      if (!this.currencies.has(f.ccy)) {
        throw new InvalidRegistry(
          'Sovereign D3.a',
          `curve family ${f.id} names currency ${f.ccy}, which does not exist`,
        );
      }
    }
  }

  /** Sovereign D3.a: the family, or nothing — a curve nobody declared is not a curve. */
  curveFamily(id: CurveFamilyId): CurveFamilyDecl {
    const f = this.curveFamilies.get(id);
    if (f === undefined) {
      throw new Missing('Sovereign D3.a', `curve family ${id} is not declared`, { id });
    }
    return f;
  }

  currency(code: CurrencyCode): CurrencyDecl {
    const c = this.currencies.get(code);
    if (c === undefined) {
      throw new Missing('Currency A5', `currency ${code} does not exist`, { code });
    }
    return c;
  }

  region(id: RegionId): RegionDecl {
    const r = this.regions.get(id);
    if (r === undefined) throw new Missing('Seed B3', `region ${id} does not exist`, { id });
    return r;
  }

  unit(id: UnitId): UnitDecl {
    const u = this.units.get(id);
    if (u === undefined) throw new Missing('Appendix A', `unit ${id} does not exist`, { id });
    return u;
  }

  cohort(id: CohortId): CohortDecl {
    const c = this.cohorts.find((x) => x.id === id);
    if (c === undefined) throw new Missing('XI-15', `cohort ${id} does not exist`, { id });
    return c;
  }

  /** The central bank that issues a currency (Currency A2). */
  centralBankOf(code: CurrencyCode): PartyId {
    return this.currency(code).centralBank;
  }

  /** The profile of an instrument kind (Law 15): the only way the kernel learns how a kind behaves. */
  instrumentKind(id: InstrumentKindId): InstrumentKindProfile {
    const k = this.instrumentKinds.get(id);
    if (k === undefined) {
      throw new Missing('Law 15', `instrument kind ${id} has no profile registered`, { id });
    }
    return k;
  }

  partyKind(id: PartyKindId): PartyKindProfile {
    const k = this.partyKinds.get(id);
    if (k === undefined) {
      throw new Missing('Law 15', `party kind ${id} has no profile registered`, { id });
    }
    return k;
  }

  issuesMoney(kind: PartyKindId): boolean {
    return this.partyKind(kind).moneyIssuer !== null;
  }
}

function unique<K, V>(items: readonly V[], key: (v: V) => K, what: string): ReadonlyMap<K, V> {
  const m = new Map<K, V>();
  for (const it of items) {
    const k = key(it);
    if (m.has(k)) throw new InvalidRegistry('Law 4', `${what} ${String(k)} declared twice`);
    m.set(k, it);
  }
  return m;
}
