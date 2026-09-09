/**
 * The register: who holds what, in what units, with what basis and what encumbrance.
 *
 * @spec Register A1 Register A1.c Register A3 Register B2 Register B3 Register C1 Register C4 Register D1 Register D2 Register D2.a Register D4 Register D5 Register D5.a Register D5.b Audit B5 Audit B5.b XI-15
 *
 * A holding is (holder, instrument) -> lots and liens. For a cell the quantities are PER MEMBER; the
 * cell's total is weight x member at read (XI-15). Both directions are indexed and both are written by
 * the one mutation path, which only settlement and the cell events call.
 *
 * The equity book is the stated equity account per party (Audit B5): a balance moved only by named
 * events (settlement's realised effects, revaluation, capital), never a stored total of anything.
 */
import type { Period } from '../calendar/calendar.js';
import { forbid, impossible } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import type { InstrumentId, LienId, LotId, PartyId } from '../core/ids.js';
import { finite, sum } from '../core/num.js';
import { type Option, none, some } from '../core/option.js';
import type { Parties } from '../parties/party.js';
import { weightOf } from '../parties/party.js';

export interface Lot {
  readonly id: LotId;
  /** Units, per member for a cell. */
  readonly qty: number;
  /** What those units cost, per unit, in the instrument's currency (Register D4). */
  readonly basisPerUnit: number;
  readonly acquired: Period;
}

export interface Lien {
  readonly id: LienId;
  /** Units encumbered, per member for a cell. */
  readonly qty: number;
  /** Who the units are bound to (D5.b: the chain is traceable). */
  readonly beneficiary: PartyId;
  readonly reason: string;
  readonly created: Period;
}

export interface Holding {
  readonly holder: PartyId;
  readonly instrument: InstrumentId;
  readonly lots: readonly Lot[];
  readonly liens: readonly Lien[];
}

/** Units drawn from a lot by a debit, with the basis they carried. */
export interface DrawnLot {
  readonly lot: LotId;
  readonly qty: number;
  readonly basisPerUnit: number;
  readonly acquired: Period;
}

export interface EquityMove {
  readonly party: PartyId;
  /** Per member for a cell, in the party's home currency. */
  readonly delta: number;
  readonly cause: string;
}

export class Register {
  private readonly byHolder = new Map<PartyId, Map<InstrumentId, MutableHolding>>();
  private readonly byInstrument = new Map<InstrumentId, Set<PartyId>>();
  private readonly equityAccount = new Map<PartyId, number>();
  private nextLot = 1;
  private nextLien = 1;

  constructor(private readonly parties: Parties) {}

  // ---- reads -------------------------------------------------------------------------------

  holding(holder: PartyId, instrument: InstrumentId): Option<Holding> {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? none() : some(snapshot(h));
  }

  /** Units held per member (a party that holds none holds zero: that is a quantity, not a missing value). */
  quantity(holder: PartyId, instrument: InstrumentId): number {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? 0 : sum(h.lots.map((l) => l.qty)).value;
  }

  /** Units held by the whole party: weight x member (XI-15). */
  totalQuantity(holder: PartyId, instrument: InstrumentId): number {
    return this.quantity(holder, instrument) * weightOf(this.parties.get(holder));
  }

  encumbered(holder: PartyId, instrument: InstrumentId): number {
    const h = this.byHolder.get(holder)?.get(instrument);
    return h === undefined ? 0 : sum(h.liens.map((l) => l.qty)).value;
  }

  /** D5.a: free units are held minus encumbered, and only free units can move. */
  free(holder: PartyId, instrument: InstrumentId): number {
    return this.quantity(holder, instrument) - this.encumbered(holder, instrument);
  }

  /** D1: what does this party hold? */
  holdingsOf(holder: PartyId): readonly Holding[] {
    const m = this.byHolder.get(holder);
    return m === undefined ? [] : [...m.values()].map(snapshot);
  }

  /** D2: who holds this instrument? */
  holdersOf(instrument: InstrumentId): readonly PartyId[] {
    const s = this.byInstrument.get(instrument);
    return s === undefined ? [] : [...s];
  }

  /** B2: the sum of holdings, weight x member for cells, as a Sum with its dust. */
  heldTotal(instrument: InstrumentId): { value: number; dust: number; terms: number } {
    return sum(this.holdersOf(instrument).map((h) => this.totalQuantity(h, instrument)));
  }

  allHoldings(): readonly Holding[] {
    const out: Holding[] = [];
    for (const m of this.byHolder.values()) for (const h of m.values()) out.push(snapshot(h));
    return out;
  }

  /** The stated equity account, per member (Audit B5). Missing until the seed states it. */
  equity(party: PartyId): number {
    const e = this.equityAccount.get(party);
    if (e === undefined)
      throw new Missing('Audit B5', `equity account of ${party} has not been stated`);
    return e;
  }

  hasEquityAccount(party: PartyId): boolean {
    return this.equityAccount.has(party);
  }

  // ---- writes (settlement, seed, cell events only) ------------------------------------------

  /** State the equity account once (Seed C1: at period zero the equity is the read). */
  stateEquity(party: PartyId, value: number): void {
    forbid(
      !this.equityAccount.has(party),
      'Audit B5.b',
      `equity of ${party} already stated; move it by events`,
    );
    this.parties.get(party);
    this.equityAccount.set(party, finite(value, `equity of ${party}`));
  }

  /** Move the equity account by a named event (Audit B5). */
  moveEquity(move: EquityMove): void {
    const cur = this.equity(move.party);
    this.equityAccount.set(move.party, finite(cur + move.delta, `equity of ${move.party}`));
  }

  credit(
    holder: PartyId,
    instrument: InstrumentId,
    qty: number,
    basisPerUnit: number,
    period: Period,
  ): Lot {
    impossible(qty > 0, 'Register C1', `a credit moves a positive quantity, got ${qty}`, {
      holder,
      instrument,
    });
    finite(basisPerUnit, 'basis');
    this.parties.get(holder);
    const h = this.mutable(holder, instrument);
    const lot: Lot = Object.freeze({
      id: this.nextLot as LotId,
      qty,
      basisPerUnit,
      acquired: period,
    });
    this.nextLot += 1;
    h.lots.push(lot);
    return lot;
  }

  /**
   * Draw units from lots first-in-first-out (Register D4; Registry.lotFlow). Throws if the free
   * quantity is short: a party cannot deliver what it does not hold (C4: no short by accident).
   */
  debit(holder: PartyId, instrument: InstrumentId, qty: number): DrawnLot[] {
    impossible(qty > 0, 'Register C1', `a debit moves a positive quantity, got ${qty}`, {
      holder,
      instrument,
    });
    const freeNow = this.free(holder, instrument);
    forbid(
      qty <= freeNow || withinDebitDust(qty, freeNow),
      'Register C4',
      `${holder} cannot deliver ${qty} of ${instrument}: free ${freeNow}`,
      { holder, instrument, qty, free: freeNow },
    );
    const h = this.mutable(holder, instrument);
    const drawn: DrawnLot[] = [];
    let remaining = qty;
    while (remaining > 0 && h.lots.length > 0) {
      const lot = h.lots[0];
      if (lot === undefined) break;
      const take =
        lot.qty <= remaining || withinDebitDust(lot.qty, remaining) ? lot.qty : remaining;
      drawn.push({
        lot: lot.id,
        qty: take,
        basisPerUnit: lot.basisPerUnit,
        acquired: lot.acquired,
      });
      remaining = finite(remaining - take, 'debit remaining');
      if (take === lot.qty) h.lots.shift();
      else h.lots[0] = Object.freeze({ ...lot, qty: finite(lot.qty - take, 'lot qty') });
      if (withinDebitDust(remaining, 0)) remaining = 0;
    }
    impossible(
      remaining === 0,
      'Register C4',
      `debit of ${qty} left ${remaining} unmatched on ${holder}`,
      {
        holder,
        instrument,
      },
    );
    if (h.lots.length === 0 && h.liens.length === 0) this.drop(holder, instrument);
    return drawn;
  }

  /**
   * Money is one balance per account, basis one (Money D2): a single lot whose quantity may go
   * negative only when the issuer's overdraft decision allowed it (Money B3). Returns the balance
   * after the move.
   */
  moneyDelta(
    holder: PartyId,
    instrument: InstrumentId,
    delta: number,
    period: Period,
    allowNegative: boolean,
  ): number {
    finite(delta, `money delta on ${holder}`);
    this.parties.get(holder);
    const h = this.mutable(holder, instrument);
    forbid(
      h.lots.length <= 1,
      'Money D2',
      `money holding ${holder}/${instrument} has ${h.lots.length} lots`,
    );
    const current = h.lots[0];
    const before = current === undefined ? 0 : current.qty;
    const after = finite(before + delta, `balance of ${holder}/${instrument}`);
    const encumbered = sum(h.liens.map((l) => l.qty)).value;
    forbid(
      allowNegative || after - encumbered >= 0 || withinDebitDust(after - encumbered, 0),
      'Money B3.c',
      `${holder} would be overdrawn ${after} on ${instrument} with no lender and no recorded refusal`,
      { holder, instrument, before, delta },
    );
    if (after === 0 && h.liens.length === 0) {
      this.drop(holder, instrument);
      return 0;
    }
    const acquired = current === undefined ? period : current.acquired;
    const lot: Lot = Object.freeze({
      id: current === undefined ? (this.nextLot as LotId) : current.id,
      qty: after,
      basisPerUnit: 1,
      acquired,
    });
    if (current === undefined) this.nextLot += 1;
    h.lots = [lot];
    return after;
  }

  /** D5: bind units to a beneficiary; pledged paper can be neither sold nor counted free. */
  pledge(
    holder: PartyId,
    instrument: InstrumentId,
    qty: number,
    beneficiary: PartyId,
    reason: string,
    period: Period,
  ): Lien {
    impossible(qty > 0, 'Register D5', `a lien binds a positive quantity, got ${qty}`);
    this.parties.get(beneficiary);
    const freeNow = this.free(holder, instrument);
    forbid(
      qty <= freeNow,
      'Register D5.a',
      `${holder} cannot pledge ${qty} of ${instrument}: free ${freeNow}`,
    );
    const h = this.mutable(holder, instrument);
    const lien: Lien = Object.freeze({
      id: this.nextLien as LienId,
      qty,
      beneficiary,
      reason,
      created: period,
    });
    this.nextLien += 1;
    h.liens.push(lien);
    return lien;
  }

  release(holder: PartyId, instrument: InstrumentId, lien: LienId): void {
    const h = this.byHolder.get(holder)?.get(instrument);
    if (h === undefined)
      throw new Missing('Register D5', `lien ${lien} on ${holder}/${instrument} does not exist`);
    const idx = h.liens.findIndex((l) => l.id === lien);
    if (idx < 0)
      throw new Missing('Register D5', `lien ${lien} on ${holder}/${instrument} does not exist`);
    h.liens.splice(idx, 1);
    if (h.lots.length === 0 && h.liens.length === 0) this.drop(holder, instrument);
  }

  /** Copy per-member state to a new party (a split: XI-15). The equity account is copied too. */
  copyMemberState(from: PartyId, to: PartyId): void {
    const src = this.byHolder.get(from);
    forbid(
      !this.byHolder.has(to),
      'XI-15',
      `${to} already has holdings; a split creates a fresh party`,
    );
    if (src !== undefined) {
      const dst = new Map<InstrumentId, MutableHolding>();
      for (const [inst, h] of src) {
        dst.set(inst, { holder: to, instrument: inst, lots: [...h.lots], liens: [...h.liens] });
        this.index(inst).add(to);
      }
      this.byHolder.set(to, dst);
    }
    if (this.equityAccount.has(from)) this.equityAccount.set(to, this.equity(from));
  }

  /** Remove every trace of a party that has merged away; the caller has verified identical state. */
  forget(party: PartyId): void {
    const m = this.byHolder.get(party);
    if (m !== undefined) for (const inst of m.keys()) this.index(inst).delete(party);
    this.byHolder.delete(party);
    this.equityAccount.delete(party);
  }

  // ---- internals ---------------------------------------------------------------------------

  private mutable(holder: PartyId, instrument: InstrumentId): MutableHolding {
    let m = this.byHolder.get(holder);
    if (m === undefined) {
      m = new Map();
      this.byHolder.set(holder, m);
    }
    let h = m.get(instrument);
    if (h === undefined) {
      h = { holder, instrument, lots: [], liens: [] };
      m.set(instrument, h);
      this.index(instrument).add(holder);
    }
    return h;
  }

  private index(instrument: InstrumentId): Set<PartyId> {
    let s = this.byInstrument.get(instrument);
    if (s === undefined) {
      s = new Set();
      this.byInstrument.set(instrument, s);
    }
    return s;
  }

  private drop(holder: PartyId, instrument: InstrumentId): void {
    this.byHolder.get(holder)?.delete(instrument);
    this.byInstrument.get(instrument)?.delete(holder);
  }
}

interface MutableHolding {
  readonly holder: PartyId;
  readonly instrument: InstrumentId;
  lots: Lot[];
  liens: Lien[];
}

function snapshot(h: MutableHolding): Holding {
  return { holder: h.holder, instrument: h.instrument, lots: [...h.lots], liens: [...h.liens] };
}

/**
 * A debit that asks for the whole balance may differ from the sum of lots by the dust of that sum
 * (Law 7); anything beyond dust is a genuine short (C4).
 */
function withinDebitDust(a: number, b: number): boolean {
  const s = sum([a, b]);
  return Math.abs(a - b) <= s.dust + 2 * Number.EPSILON * (Math.abs(a) + Math.abs(b));
}

/**
 * The read-only face of the register. The World exposes only this; the store with its writes is
 * handed to settlement, the cell events and the seed, and to nothing else (Law 4: one writer).
 */
export type RegisterReads = Pick<
  Register,
  | 'holding'
  | 'quantity'
  | 'totalQuantity'
  | 'encumbered'
  | 'free'
  | 'holdingsOf'
  | 'holdersOf'
  | 'heldTotal'
  | 'allHoldings'
  | 'equity'
  | 'hasEquityAccount'
>;

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export function registerReads(store: Register): RegisterReads {
  return Object.freeze({
    holding: (holder: PartyId, instrument: InstrumentId) => store.holding(holder, instrument),
    quantity: (holder: PartyId, instrument: InstrumentId) => store.quantity(holder, instrument),
    totalQuantity: (holder: PartyId, instrument: InstrumentId) =>
      store.totalQuantity(holder, instrument),
    encumbered: (holder: PartyId, instrument: InstrumentId) => store.encumbered(holder, instrument),
    free: (holder: PartyId, instrument: InstrumentId) => store.free(holder, instrument),
    holdingsOf: (holder: PartyId) => store.holdingsOf(holder),
    holdersOf: (instrument: InstrumentId) => store.holdersOf(instrument),
    heldTotal: (instrument: InstrumentId) => store.heldTotal(instrument),
    allHoldings: () => store.allHoldings(),
    equity: (party: PartyId) => store.equity(party),
    hasEquityAccount: (party: PartyId) => store.hasEquityAccount(party),
  });
}
