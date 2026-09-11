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
import { downTick, piecesPerUnit, toTick, toTickOf } from '../core/tick.js';
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
import type { Qty } from '../core/tick.js';

export interface CurrencyDecl {
  readonly code: CurrencyCode;
  readonly name: string;
  /** The named issuer whose liability the currency is (Currency A2). */
  readonly centralBank: PartyId;
  /**
   * Law 1, Law 8: THE SMALLEST INCREMENT A RATE QUOTED IN THIS MONEY MOVES BY, per unit of the
   * money being bought — what a dealer calls a pip.
   *
   * It belongs to the QUOTE money and not to the pair, which is the real convention and not a
   * simplification: every pair quoted in dollars moves in ten-thousandths of one and every pair
   * quoted in yen moves in hundredths of one, because what the increment is worth is a fact about
   * the money the price is IN. How fine it is, is a RESOLUTION (Law 2), and `resolution.tickShift`
   * moves every tick in the world together so the invariance can be run.
   */
  readonly quoteTick: number;
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
   * Law 1, Law 8: HOW MANY INDIVISIBLE PIECES ONE OF THESE IS DIVIDED INTO — a hundred cents to the
   * USD, a thousand kilos to the tonne, one machine to a machine.
   *
   * A QUANTITY OF THIS UNIT IS A COUNT OF THOSE PIECES and is always a whole number: the state
   * holds 18849 cents, never 188.49 of anything. That is the only way the arithmetic is exact —
   * integers add and compare exactly in binary — so no balance drifts, no check needs a tolerance,
   * and nothing finer than a piece can be paid, lent, refused or left over anywhere in the world.
   *
   * The NAME is what a person reads (USD, tonnes); this is how the number relates to it, and it is
   * used at the two boundaries where the two meet: a seed declaring an amount and a report showing
   * one. How fine it is, is a RESOLUTION (Law 2): the same world declared in tenths of a cent must
   * follow the same path, and the registry's shift moves every unit's subdivision together so that
   * invariance can be run and measured.
   */
  readonly perUnit: number;
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
  /** Law 2: what every unit's subdivision is multiplied by, for the invariance test. */
  readonly pieceShift: number;
  /** Law 2: what every declared price tick is DIVIDED by, which is the same test for the price grid. */
  readonly tickShift: number;
  readonly regions: ReadonlyMap<RegionId, RegionDecl>;
  readonly units: ReadonlyMap<UnitId, UnitDecl>;
  readonly cohorts: readonly CohortDecl[];
  readonly cellKey: readonly CellKeyDimension[];
  readonly lotFlow: LotFlow;
  readonly curveFamilies: ReadonlyMap<CurveFamilyId, CurveFamilyDecl>;
  readonly instrumentKinds: ReadonlyMap<InstrumentKindId, InstrumentKindProfile>;
  readonly partyKinds: ReadonlyMap<PartyKindId, PartyKindProfile>;

  private readonly subdivisions = new Map<UnitId, number>();

  /**
   * Law 2's two resolution knobs: `pieceShift` is how many times finer every unit's PIECES are, and
   * `tickShift` how many times finer every quoted PRICE is. They are separate because the grids are
   * separate — a share is indivisible and still quotes in cents — and each is tested the same way,
   * by declaring the same world finer and showing its path does not turn on the number.
   */
  constructor(data: RegistryData, pieceShift: number, tickShift: number) {
    this.pieceShift = pieceShift;
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
    // Law 8: a subdivision that is not a whole number of pieces is refused at assembly rather
    // than discovered later in a balance that will not add up.
    for (const u of this.units.values()) piecesPerUnit(u.perUnit, pieceShift);
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
      // Law 8, worklist 12b.1: A KIND SOMEBODY QUOTES HAS A SMALLEST INCREMENT AND EVERY OTHER KIND
      // HAS NONE, and both mistakes are refused here rather than found later in a price with sixteen
      // figures in it.
      //
      // QUOTED means SOMEBODY CAN POST A LIMIT IN IT. Money is worth one of itself (Money D2) and a
      // kind carried at cost is never offered at a level, so neither has a grid. A DERIVED kind has
      // one, because its shares trade: an exchange-traded fund is a claim on a book AND a line in a
      // market, and Fund Shares E2 is the clause that says those are two numbers about one thing.
      //
      // WHICH IS EXACTLY WHERE THE GRID GOES AND WHERE IT DOES NOT. The traded number is posted by a
      // party and is on the grid like every other posted level; the DERIVED value is arithmetic on a
      // book (B1) that nobody offers, and it is not. Rounding that one was tried and the audit
      // refused it within two periods: it leaves `assets - shares x value` belonging to nobody, and
      // a fund's equity is zero BY CONSTRUCTION (A3) — a residual with no holder, which is Law 2's
      // defect made by putting a grid on a number nobody posts.
      const quoted = k.pricing === 'cleared' || k.pricing === 'derived';
      if (quoted && k.priceTick === undefined) {
        throw new InvalidRegistry(
          'Law 8',
          `instrument kind ${k.id} can be posted at a level and declares no price tick`,
        );
      }
      if (!quoted && k.priceTick !== undefined) {
        throw new InvalidRegistry(
          'Law 8',
          `instrument kind ${k.id} declares a price tick and is never posted at one (${k.pricing})`,
        );
      }
      if (k.priceTick !== undefined && !(k.priceTick > 0)) {
        throw new InvalidRegistry('Law 8', `instrument kind ${k.id} has a price tick of ${k.priceTick}`);
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
   * Law 8: how many indivisible pieces one NAMED unit of this is — a hundred cents to the USD. It
   * is asked at the two boundaries where a person's number meets the state's: `pieces` converts a
   * declared amount into the count the state holds, and `named` converts it back for a report.
   * Nothing in between ever divides by it: every quantity in the engine is already a count.
   */
  subdivision(id: UnitId): number {
    const known = this.subdivisions.get(id);
    if (known !== undefined) return known;
    const made = piecesPerUnit(this.unit(id).perUnit, this.pieceShift);
    this.subdivisions.set(id, made);
    return made;
  }

  /** What a declared amount of this unit IS, as a count of pieces: 188.49 USD is 18849 cents. */
  pieces(id: UnitId, named: number): Qty {
    return toTick(named * this.subdivision(id));
  }

  /**
   * Law 8: a PRICE declared as money per named unit, as the state holds one — money pieces per
   * piece of the thing. Prices are ratios and the state's two subdivisions are both in them, so a
   * price of 400 USD the tonne is four hundredths of a cent the gram, and value = units x price
   * comes out in cents without anybody converting anything downstream.
   */
  priceOf(ccy: CurrencyCode, unit: UnitId, perNamedUnit: number): number {
    return (perNamedUnit * this.subdivision(currencyUnit(ccy))) / this.subdivision(unit);
  }

  /**
   * Law 8, Clearing C4.c: THE SMALLEST INCREMENT THIS KIND IS QUOTED IN, in the same terms a price
   * is held in — money pieces per piece of the thing — so a level is on the grid when it is a whole
   * number of these.
   *
   * It is `priceOf` applied to the declared tick, because a tick IS a price: the smallest one that
   * is not nothing. Law 2's `tickShift` divides it, which is how the same world is declared with a
   * finer grid and its path shown not to turn on the number.
   */
  tickFor(kind: InstrumentKindId, ccy: CurrencyCode): number {
    const k = this.instrumentKind(kind);
    const declared = k.priceTick;
    if (declared === undefined) {
      throw new InvalidRegistry('Law 8', `instrument kind ${kind} is never posted at a level and has no tick`);
    }
    return this.priceOf(ccy, k.unit(ccy), declared / this.tickShift);
  }

  /**
   * Law 8: a stated level, put on this kind's quote grid where it HAS one.
   *
   * A kind that is never quoted has no quote grid and this passes its number through: what is
   * stated for a lot of work-in-progress or a loan row is a COST, not a level a market could print,
   * and a cost lands on the money's own grid at the moment it is paid (`Registry.cashFor`). Giving
   * it a quote tick would be inventing a market for a thing that has none.
   */
  onQuoteGrid(kind: InstrumentKindId, ccy: CurrencyCode, price: number): number {
    return this.instrumentKind(kind).priceTick === undefined
      ? price
      : toTickOf(price, this.tickFor(kind, ccy));
  }

  /**
   * Spot FX C1, Law 8: the same question for a RATE, whose grid belongs to the money it is quoted
   * in (`CurrencyDecl.quoteTick`) — a pip. The base money's unit is what one of is being bought,
   * so the two subdivisions in the ratio are the two moneys' own.
   */
  rateTickFor(base: CurrencyCode, quote: CurrencyCode): number {
    return this.priceOf(
      quote,
      currencyUnit(base),
      this.currency(quote).quoteTick / this.tickShift,
    );
  }

  /** The other way, for a reader: 18849 cents is 188.49 USD. Never used to decide anything. */
  named(id: UnitId, pieces: number): number {
    return pieces / this.subdivision(id);
  }

  /**
   * Money A2, Law 8: the most of this money that EXISTS at or below the amount asked for. Whoever
   * pays rounds down, because paying up would be paying a tick nobody has; whoever splits a payment
   * between several payees uses `splitOnTick` instead, so that the parts sum to exactly the whole
   * and the odd tick has a named holder.
   */
  payable(_ccy: CurrencyCode, amount: number): Qty {
    return downTick(amount);
  }

  /**
   * Law 8: what a computed VALUE comes to in money — the nearest whole piece, up or down. Used
   * where a quantity meets a price and the answer is what somebody owes: rounding it always down
   * would hand the payer a fraction of a piece on every trade it ever did.
   */
  cashFor(_ccy: CurrencyCode, value: number): Qty {
    return toTick(value);
  }

  /** Register A1.c: the same question for units of anything else — the most that can be delivered. */
  deliverable(_unit: UnitId, qty: number): Qty {
    return downTick(qty);
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
