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
import { downTick, isTick, tickFromExponent, toTick } from '../core/tick.js';
import { currencyUnit } from '../core/ids.js';
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
  /**
   * Law 1, Law 8: THE SMALLEST PIECE OF THIS UNIT THAT EXISTS, as the exponent of a halving — the
   * tick is 2^-exponent, so 0 is whole units (a contract, a dwelling) and 20 is about a millionth.
   *
   * Every quantity of the unit is a whole number of ticks: there is no half-cent and no thousandth
   * of a share. It is a power of two because only then do whole ticks add EXACTLY in binary
   * floating point, which is what lets a balance moved a million times be compared with no
   * tolerance at all (Law 7) instead of with a derived band.
   *
   * How fine it is, is a RESOLUTION (Law 2): change it and the world's path must not move. The
   * shift below moves every unit's grid together so that invariance can be run and measured.
   */
  readonly tickExponent: number;
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
  /** Law 2: how many halvings every unit's grid is shifted by, for the invariance test. */
  readonly tickShift: number;
  readonly regions: ReadonlyMap<RegionId, RegionDecl>;
  readonly units: ReadonlyMap<UnitId, UnitDecl>;
  readonly cohorts: readonly CohortDecl[];
  readonly cellKey: readonly CellKeyDimension[];
  readonly lotFlow: LotFlow;
  readonly curveFamilies: ReadonlyMap<CurveFamilyId, CurveFamilyDecl>;
  readonly instrumentKinds: ReadonlyMap<InstrumentKindId, InstrumentKindProfile>;
  readonly partyKinds: ReadonlyMap<PartyKindId, PartyKindProfile>;

  private readonly ticks = new Map<UnitId, number>();

  constructor(data: RegistryData, tickShift = 0) {
    this.tickShift = tickShift;
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
    for (const u of this.units.values()) {
      // Law 7: a grid that is not a power of two drifts off itself, and then the exactness this
      // whole idea buys is gone. It is refused at assembly rather than discovered in a balance.
      const tick = tickFromExponent(u.tickExponent + tickShift);
      if (!isTick(tick)) {
        throw new InvalidRegistry('Law 8', `unit ${u.id} has a tick of ${tick}, which is not a power of two`);
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
    for (const k of this.instrumentKinds.values()) {
      // XI-6, Fund Shares B1: a kind that says its value is derived and derives nothing is an
      // unpriced position pretending to be a priced one, and it would read as a hole at every mark.
      if (k.pricing === 'derived' && k.derive === undefined) {
        throw new InvalidRegistry('XI-6', `instrument kind ${k.id} is derived and derives nothing`);
      }
      if (k.pricing !== 'derived' && k.derive !== undefined) {
        throw new InvalidRegistry(
          'Law 4',
          `instrument kind ${k.id} derives a value but is priced by ${k.pricing}`,
        );
      }
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

  /**
   * Law 8: the smallest amount of this unit that exists. Every quantity in the state is a whole
   * number of it, and the wire refuses one that is not (Money C1).
   */
  tick(id: UnitId): number {
    const known = this.ticks.get(id);
    if (known !== undefined) return known;
    const made = tickFromExponent(this.unit(id).tickExponent + this.tickShift);
    this.ticks.set(id, made);
    return made;
  }

  /**
   * Money A2, Law 8: the most of this money that EXISTS at or below the amount asked for. Whoever
   * pays rounds down, because paying up would be paying a tick nobody has; whoever splits a payment
   * between several payees uses `splitOnTick` instead, so that the parts sum to exactly the whole
   * and the odd tick has a named holder.
   */
  payable(ccy: CurrencyCode, amount: number): number {
    return downTick(amount, this.tick(currencyUnit(ccy)));
  }

  /**
   * Law 8: what a computed VALUE comes to in money — the nearest whole piece, up or down. Used
   * where a quantity meets a price and the answer is what somebody owes: rounding it always down
   * would hand the payer a fraction of a piece on every trade it ever did.
   */
  cashFor(ccy: CurrencyCode, value: number): number {
    return toTick(value, this.tick(currencyUnit(ccy)));
  }

  /** Register A1.c: the same question for units of anything else — the most that can be delivered. */
  deliverable(unit: UnitId, qty: number): number {
    return downTick(qty, this.tick(unit));
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
