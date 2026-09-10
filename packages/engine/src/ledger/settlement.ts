/**
 * Settlement: the one rule that applies an instruction, and the only writer of holdings.
 *
 * @spec Goods E4 Commodities Spot F1 Treasury D3 Central Bank E2 Money C2 Money C2.a Money C2.b Money C2.c Money C4 Money C4.a Money C4.b Money D1 Money D3 Money D4 Money E1 Money E1.a Money E1.b Money E2 Money E3 Money E4 Money B3 Money B3.a Money B3.b Money B3.c Register B1 Register B3 Register C1 Register C2.a Register C3 Register C3.a Register C3.b Register C4 Register D4 Audit B5 XI-5 XI-15 Equity C4 Equity F4
 *
 * Payer minus, payee plus (C2). For a money leg between accounts at different issuers the interbank
 * reserve leg is generated here (C2.a); a same-issuer payment moves no reserves (C2.b). All legs of
 * an instruction are pre-checked and then applied together: if any leg cannot apply, none does and a
 * Failed record is written (Register C3.b, Money E1). Settlement is final (E2).
 *
 * Money moving between two issuers is that money's redemption at the first issuer and its issuance at
 * the second, with reserves moving to compensate. Money is CREATED only by a leg whose from-account is
 * an issuer's own (C4), and DESTROYED only by a leg whose to-account is: the Money audit family reads
 * those legs against the change in the stock (C4.c).
 *
 * The equity account of every party the instruction touches moves by the named effect of the legs
 * (Audit B5): what came in at its price or carrying value, minus what went out at its carrying value,
 * minus own liabilities issued, plus own liabilities redeemed. An exchange at a price nets to zero; a
 * transfer is income to one side and expense to the other; a sale away from the mark is a realised
 * gain or loss. Nothing else moves an equity account except revaluation (world/revalue.ts).
 */
import { issuedBy, issuerOf } from '../register/instruments.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import { assertNever, forbid, impossible } from '../core/assert.js';
import { Forbidden, Mismatch, Missing } from '../core/errors.js';
import {
  currencyUnit,
  type CurrencyCode,
  type InstrumentId,
  type LienId,
  moneyInstrumentId,
  type PartyId,
  type UnitId,
} from '../core/ids.js';
import { addTo, dustOf, finite, mul, sum, zeroIfNone } from '../core/num.js';
import { onTick } from '../core/tick.js';
import type { Journal } from '../journal/journal.js';
import type { Parties, Party } from '../parties/party.js';
import { weightOf } from '../parties/party.js';
import type { PriceStore } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Instruments } from '../register/instruments.js';
import type { DrawnLot, Register } from '../register/register.js';
import type { Registry } from '../registry/registry.js';
import type { OverdraftContext, OverdraftDecision } from '../registry/kinds.js';
import type { PartyKindId } from '../core/ids.js';
import type {
  AccountRef,
  AssetLeg,
  AssumeLeg,
  CellSide,
  CreateLeg,
  DestroyLeg,
  EquityEffect,
  FailReason,
  Instruction,
  InstructionDraft,
  MoneyLeg,
  PledgeLeg,
  RegisterDelta,
  ReleaseLeg,
  ReserveLeg,
  SettlementRecord,
} from './instruction.js';
import { subjectsOf } from './instruction.js';
import type { Ledger } from './ledger.js';

export interface SettlementDeps {
  readonly registry: Registry;
  readonly calendar: Calendar;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly prices: PriceStore;
  readonly valuation: Valuation;
  readonly ledger: Ledger;
  readonly journal: Journal;
  /**
   * Money B3.a: the credit decision behind an overdraft, for an issuer kind that says its answer is
   * one. It is a function of the party kind and not of the instrument, because it is the ISSUER's
   * decision — the bank's — and every bank of a kind decides the same way from its own state.
   */
  creditDecision(kind: PartyKindId): (ctx: OverdraftContext) => OverdraftDecision;
}

/** One register-level operation an instruction expands into; quantities are per member for cells. */
type Op =
  | {
      readonly op: 'debit';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
      readonly money: boolean;
    }
  | {
      readonly op: 'credit';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
      /** Total units on the leg (weight x per member for a cell), for the issuer's side. */
      readonly totalQty: number;
      readonly money: boolean;
      /** Basis per unit, or 'carrying' to take the giver's carrying value (a transfer). */
      readonly basis: number | 'carrying';
      /** The debit whose carrying value a 'carrying' basis reads; -1 for an issuance. */
      readonly fromDebit: number;
    }
  | {
      readonly op: 'issue';
      readonly issuer: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
      readonly valuePerUnit: number | 'carrying';
      readonly fromDebit: number;
    }
  | {
      readonly op: 'redeem';
      readonly issuer: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
      readonly valuePerUnit: number | 'carrying';
      readonly fromDebit: number;
    }
  /** Goods E4: units of a physical kind coming into existence or leaving it; nobody issued them. */
  | {
      readonly op: 'exist';
      readonly holder: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
    }
  /** XI-8: the issuer of record changes; the holders and the units do not. */
  | {
      readonly op: 'reseat';
      readonly from: PartyId;
      readonly to: PartyId;
      readonly instrument: InstrumentId;
    }
  /** Register D5: units bound to a beneficiary, and freed. Nothing changes hands either way. */
  | {
      readonly op: 'bind';
      readonly pledgor: PartyId;
      readonly beneficiary: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: number;
      readonly secures: string;
    }
  | {
      readonly op: 'free';
      readonly pledgor: PartyId;
      readonly instrument: InstrumentId;
      readonly lien: LienId;
    };

export class Settlement {
  constructor(private readonly d: SettlementDeps) {}

  /**
   * Apply an instruction at (period, cycle). Returns the record written to the ledger. Contract
   * violations in the draft (a one-sided leg, a currency mismatch, a cell side without a per-member
   * amount) throw; an economic impossibility (short units, refused overdraft) is a Failed record.
   */
  settle(draft: InstructionDraft, period: Period, cycle: Cycle): SettlementRecord {
    const instruction: Instruction = Object.freeze({
      ...draft,
      legs: draft.legs.map((l) => Object.freeze({ ...l })),
      id: this.d.ledger.allocate(),
      period,
      cycle,
    });
    this.validate(instruction);
    const ops = this.expand(instruction);
    const reserveLegs = this.reserveLegs(instruction);

    const fail = this.precheck(instruction, ops);
    if (fail !== undefined) {
      const record: SettlementRecord = { outcome: 'failed', instruction, reason: fail };
      this.d.ledger.append(record);
      this.d.journal.record(
        period,
        cycle,
        'instruction.failed',
        subjectsOf(instruction),
        {
          id: instruction.id,
          cause: instruction.cause,
          reason: fail,
        },
        false,
      );
      return record;
    }

    const applied = this.apply(instruction, ops);
    const record: SettlementRecord = {
      outcome: 'settled',
      instruction,
      deltas: applied.deltas,
      equity: applied.equity,
      reserveLegs,
    };
    this.d.ledger.append(record);
    this.d.journal.record(
      period,
      cycle,
      'instruction.settled',
      subjectsOf(instruction),
      {
        id: instruction.id,
        cause: instruction.cause,
        legs: instruction.legs.length,
      },
      false,
    );
    return record;
  }

  // ---- validation ----------------------------------------------------------------------------

  private validate(ins: Instruction): void {
    forbid(ins.legs.length > 0, 'Money D1', `instruction ${ins.id} has no legs`);
    forbid(ins.reason.length > 0, 'Money C1.b', `instruction ${ins.id} carries no reason`);
    for (const leg of ins.legs) {
      switch (leg.kind) {
        case 'money':
          this.validateMoney(leg, ins);
          break;
        case 'asset':
          this.validateAsset(leg, ins);
          break;
        case 'create':
        case 'destroy':
          this.validatePhysical(leg, ins);
          break;
        case 'assume':
          this.validateAssume(leg, ins);
          break;
        case 'pledge':
        case 'release':
          this.validateLien(leg, ins);
          break;
        default:
          assertNever(leg, 'Leg');
      }
    }
  }

  private validateMoney(leg: MoneyLeg, ins: Instruction): void {
    impossible(
      finite(leg.amount, 'money leg amount') > 0,
      'Money C1',
      `money leg amount must be positive, got ${leg.amount}`,
    );
    forbid(
      leg.from.holder !== leg.to.holder || leg.from.issuer !== leg.to.issuer,
      'Money C1.a',
      `instruction ${ins.id}: a payment from an account to itself is not a payment`,
    );
    this.d.registry.currency(leg.ccy);
    for (const acct of [leg.from, leg.to]) this.validateAccount(acct, leg.ccy, ins);
    this.validateCellSide(leg.from.holder, leg.fromCell, leg.amount, ins);
    this.validateCellSide(leg.to.holder, leg.toCell, leg.amount, ins);
    const unit = currencyUnit(leg.ccy);
    this.onTheGrid(leg.amount, unit, ins);
    if (leg.fromCell.some) this.onTheGrid(leg.fromCell.value.perMember, unit, ins);
    if (leg.toCell.some) this.onTheGrid(leg.toCell.value.perMember, unit, ins);
  }

  private validateAccount(acct: AccountRef, ccy: CurrencyCode, ins: Instruction): void {
    const holder = this.alive(acct.holder, ins);
    const issuer = this.alive(acct.issuer, ins);
    forbid(
      this.d.registry.issuesMoney(issuer.kind),
      'Money A1.d',
      `instruction ${ins.id}: ${issuer.id} (${issuer.kind}) does not issue money`,
    );
    const inst = moneyInstrumentId(issuer.id, ccy);
    forbid(
      this.d.instruments.has(inst),
      'Money A1',
      `instruction ${ins.id}: ${issuer.id} issues no money in ${ccy}`,
    );
    forbid(
      holder.representation === 'named' || holder.id !== issuer.id,
      'Money A1.d',
      `instruction ${ins.id}: a cell cannot issue money`,
    );
  }

  /**
   * Goods E4, Commodities Spot F1: a physical thing is made or used up on one book. Only a kind
   * that says its units are physical admits it — a claim that appeared with nobody on the other
   * side is invented money (Money C1) — and units enter the world only through a PRODUCTION event
   * that says so. What a batch had to draw to make them is the recipe's, which is the good's own
   * technology and lives in its terms: the kernel never looks inside terms (Law 15), so that is
   * checked where the recipe is readable, by the goods module's own audit contribution (Goods B2).
   * A thing made from labour and land alone consumes no units at all, and requiring it to destroy
   * something would have made the first stage of every production chain impossible.
   */
  private validatePhysical(leg: CreateLeg | DestroyLeg, ins: Instruction): void {
    impossible(
      finite(leg.qty, 'physical leg qty') > 0,
      'Register C1',
      `a create or destroy moves a positive quantity, got ${leg.qty}`,
    );
    const inst = this.d.instruments.get(leg.instrument);
    forbid(inst.status.live, 'Register B4', `instruction ${ins.id}: ${inst.id} has ceased`);
    forbid(
      this.d.registry.instrumentKind(inst.kind).physical === true,
      'Goods A1',
      `instruction ${ins.id}: ${inst.id} is a claim, and a claim is issued and redeemed, never made`,
    );
    this.alive(leg.party, ins);
    const side = leg.kind === 'create' ? leg.toCell : leg.fromCell;
    this.validateCellSide(leg.party, side, leg.qty, ins);
    this.onTheGrid(leg.qty, inst.unit, ins);
    if (side.some) this.onTheGrid(side.value.perMember, inst.unit, ins);
    if (leg.kind === 'create') {
      impossible(
        finite(leg.costPerUnit, 'cost per unit') >= 0,
        'Goods E1',
        `what a unit cost to make cannot be negative`,
      );
      forbid(
        ins.cause === 'production' || ins.cause === 'seed',
        'Commodities Spot F1',
        `instruction ${ins.id}: ${inst.id} would come into the world by ${ins.cause}; units are produced`,
      );
    }
  }

  /**
   * XI-8, Firm Birth D5: who owes a line changes. The line must be one this party actually issued,
   * both books must be somebody who exists, and it must not be a party assuming its own paper —
   * which would be a reference that never resolves and a liability that netted itself away.
   */
  private validateAssume(leg: AssumeLeg, ins: Instruction): void {
    const inst = this.d.instruments.get(leg.instrument);
    forbid(inst.status.live, 'Register B4', `instruction ${ins.id}: ${inst.id} has ceased`);
    forbid(
      issuedBy(inst, leg.from),
      'Register F2',
      `instruction ${ins.id}: ${leg.from} is not the issuer of ${inst.id}`,
    );
    forbid(leg.from !== leg.to, 'Register F2', `instruction ${ins.id}: ${leg.from} assumes its own paper`);
    this.alive(leg.from, ins);
    this.alive(leg.to, ins);
  }

  /**
   * Register D5, D5.b: an encumbrance names the holder whose units are bound and the party they are
   * bound TO, and both are somebody. A party binding units to itself is not securing anything; a
   * lien on a line that has ceased is a claim on nothing.
   */
  private validateLien(leg: PledgeLeg | ReleaseLeg, ins: Instruction): void {
    const inst = this.d.instruments.get(leg.instrument);
    forbid(inst.status.live, 'Register B4', `instruction ${ins.id}: ${inst.id} has ceased`);
    forbid(
      leg.pledgor !== leg.beneficiary,
      'Register D5',
      `instruction ${ins.id}: ${leg.pledgor} would pledge to itself`,
    );
    this.alive(leg.pledgor, ins);
    this.alive(leg.beneficiary, ins);
    if (leg.kind !== 'pledge') return;
    impossible(
      finite(leg.qty, 'pledge leg qty') > 0,
      'Register D5',
      `a lien binds a positive quantity, got ${leg.qty}`,
    );
    forbid(
      leg.secures.length > 0,
      'Register D5.b',
      `instruction ${ins.id}: a lien says what it secures`,
    );
    this.validateCellSide(leg.pledgor, leg.pledgorCell, leg.qty, ins);
    this.onTheGrid(leg.qty, inst.unit, ins);
    if (leg.pledgorCell.some) this.onTheGrid(leg.pledgorCell.value.perMember, inst.unit, ins);
  }

  private validateAsset(leg: AssetLeg, ins: Instruction): void {
    impossible(
      finite(leg.qty, 'asset leg qty') > 0,
      'Register C1',
      `asset leg qty must be positive, got ${leg.qty}`,
    );
    forbid(
      leg.from !== leg.to,
      'Register C1',
      `instruction ${ins.id}: a transfer needs two named sides`,
    );
    const inst = this.d.instruments.get(leg.instrument);
    forbid(inst.status.live, 'Register B4', `instruction ${ins.id}: ${inst.id} has ceased`);
    forbid(
      inst.kind !== 'money',
      'Money D2',
      `instruction ${ins.id}: money moves by a money leg, not an asset leg`,
    );
    this.alive(leg.from, ins);
    this.alive(leg.to, ins);
    if (leg.pricePerUnit.some) {
      impossible(
        finite(leg.pricePerUnit.value, 'price') >= 0,
        'Law 6',
        `a price cannot be negative`,
      );
    }
    this.validateCellSide(leg.from, leg.fromCell, leg.qty, ins);
    this.validateCellSide(leg.to, leg.toCell, leg.qty, ins);
    this.onTheGrid(leg.qty, inst.unit, ins);
    if (leg.fromCell.some) this.onTheGrid(leg.fromCell.value.perMember, inst.unit, ins);
    if (leg.toCell.some) this.onTheGrid(leg.toCell.value.perMember, inst.unit, ins);
  }

  /**
   * Law 8, Money A2: A QUANTITY THAT DOES NOT EXIST CANNOT MOVE. Every unit has a smallest piece
   * (registry: `tickExponent`), so a leg carrying a thousandth of a cent or a millionth of a share
   * is not a small movement — it is a movement of something that is not there.
   *
   * It throws rather than rounding, and that is the point: the kernel rounding somebody's payment
   * for them would be the kernel deciding what they paid. Whoever builds the leg decides — pay the
   * tick below or the tick above, and give the odd tick to somebody named (core/tick.ts).
   */
  private onTheGrid(qty: number, unit: UnitId, ins: Instruction): void {
    const tick = this.d.registry.tick(unit);
    impossible(
      onTick(qty, tick),
      'Law 8',
      `instruction ${ins.id}: ${qty} is not a whole number of ${unit} (${tick} at a time)`,
    );
  }

  /** XI-15: a cell side carries a per-member amount at the current weight; a named side carries none. */
  private validateCellSide(
    party: PartyId,
    side: MoneyLeg['fromCell'],
    total: number,
    ins: Instruction,
  ): void {
    const p = this.d.parties.get(party);
    if (p.representation === 'cell') {
      forbid(
        side.some,
        'XI-15',
        `instruction ${ins.id}: a movement on cell ${party} must be denominated per member`,
      );
      const cs: CellSide = side.value;
      forbid(
        cs.weight === p.weight,
        'XI-15',
        `instruction ${ins.id}: cell ${party} weight is ${p.weight}, leg struck at ${cs.weight}`,
      );
      const expected = mul(cs.perMember, cs.weight, 'cell total');
      forbid(
        Math.abs(expected - total) <= dustOf(2, Math.abs(total) + Math.abs(expected)),
        'XI-15',
        `instruction ${ins.id}: cell ${party} per-member ${cs.perMember} x ${cs.weight} is not ${total}`,
      );
    } else {
      forbid(
        !side.some,
        'XI-15',
        `instruction ${ins.id}: ${party} is named; it has no per-member side`,
      );
    }
  }

  private alive(id: PartyId, ins: Instruction): Party {
    const p = this.d.parties.get(id);
    if (!p.status.alive) {
      // Money E4: a ceased party's legs are settled or refused by name; its estate must have re-seated
      // them. Addressing the dead party directly is a defect in the mechanism that drafted this.
      throw new Forbidden(
        'Money E4',
        `instruction ${ins.id} addresses ${id}, which ceased in period ${p.status.ceasedIn}`,
        {
          party: id,
          successor: p.status.successor,
        },
      );
    }
    return p;
  }

  // ---- expansion -----------------------------------------------------------------------------

  private expand(ins: Instruction): Op[] {
    const ops: Op[] = [];
    for (const leg of ins.legs) {
      switch (leg.kind) {
        case 'money':
          this.expandMoney(leg, ops);
          break;
        case 'asset':
          this.expandAsset(leg, ops);
          break;
        case 'create':
          ops.push({
            op: 'credit',
            party: leg.party,
            instrument: leg.instrument,
            qty: leg.toCell.some ? leg.toCell.value.perMember : leg.qty,
            totalQty: leg.qty,
            money: false,
            basis: leg.costPerUnit,
            fromDebit: -1,
          });
          ops.push({ op: 'exist', holder: leg.party, instrument: leg.instrument, qty: leg.qty });
          break;
        case 'destroy': {
          ops.push({
            op: 'debit',
            party: leg.party,
            instrument: leg.instrument,
            qty: leg.fromCell.some ? leg.fromCell.value.perMember : leg.qty,
            money: false,
          });
          ops.push({ op: 'exist', holder: leg.party, instrument: leg.instrument, qty: -leg.qty });
          break;
        }
        case 'assume':
          ops.push({
            op: 'reseat',
            from: leg.from,
            to: leg.to,
            instrument: leg.instrument,
          });
          break;
        case 'pledge':
          ops.push({
            op: 'bind',
            pledgor: leg.pledgor,
            beneficiary: leg.beneficiary,
            instrument: leg.instrument,
            qty: leg.pledgorCell.some ? leg.pledgorCell.value.perMember : leg.qty,
            secures: leg.secures,
          });
          break;
        case 'release':
          ops.push({
            op: 'free',
            pledgor: leg.pledgor,
            instrument: leg.instrument,
            lien: leg.lien,
          });
          break;
        default:
          assertNever(leg, 'Leg');
      }
    }
    return ops;
  }

  private expandMoney(leg: MoneyLeg, ops: Op[]): void {
    const i1 = leg.from.issuer;
    const i2 = leg.to.issuer;
    const m1 = moneyInstrumentId(i1, leg.ccy);
    const m2 = moneyInstrumentId(i2, leg.ccy);
    const cb = this.d.registry.centralBankOf(leg.ccy);
    const perFrom = leg.fromCell.some ? leg.fromCell.value.perMember : leg.amount;
    const perTo = leg.toCell.some ? leg.toCell.value.perMember : leg.amount;

    // Payer minus (C2), or creation if the payer is the issuer itself (C4).
    if (leg.from.holder === i1)
      ops.push({
        op: 'issue',
        issuer: i1,
        instrument: m1,
        qty: leg.amount,
        valuePerUnit: 1,
        fromDebit: -1,
      });
    else
      ops.push({ op: 'debit', party: leg.from.holder, instrument: m1, qty: perFrom, money: true });

    // Payee plus (C2), or destruction if the payee is the issuer itself.
    if (leg.to.holder === i2)
      ops.push({
        op: 'redeem',
        issuer: i2,
        instrument: m2,
        qty: leg.amount,
        valuePerUnit: 1,
        fromDebit: -1,
      });
    else
      ops.push({
        op: 'credit',
        party: leg.to.holder,
        instrument: m2,
        qty: perTo,
        totalQty: leg.amount,
        money: true,
        basis: 1,
        fromDebit: -1,
      });

    if (i1 !== i2) {
      // Across issuers (C2.a): a bank's deposit money leaves its books and is destroyed there; the
      // payee's bank creates its own; and reserves move between them at their central bank. Money
      // whose issuer IS the central bank does not leave its issuer: it changes holder, which the
      // reserve legs below already are.
      const reserves = moneyInstrumentId(cb, leg.ccy);
      if (i1 !== cb) {
        ops.push({
          op: 'redeem',
          issuer: i1,
          instrument: m1,
          qty: leg.amount,
          valuePerUnit: 1,
          fromDebit: -1,
        });
        ops.push({ op: 'debit', party: i1, instrument: reserves, qty: leg.amount, money: true });
      }
      if (i2 !== cb) {
        ops.push({
          op: 'issue',
          issuer: i2,
          instrument: m2,
          qty: leg.amount,
          valuePerUnit: 1,
          fromDebit: -1,
        });
        ops.push({
          op: 'credit',
          party: i2,
          instrument: reserves,
          qty: leg.amount,
          totalQty: leg.amount,
          money: true,
          basis: 1,
          fromDebit: -1,
        });
      }
    }
  }

  private reserveLegs(ins: Instruction): ReserveLeg[] {
    const out: ReserveLeg[] = [];
    for (const leg of ins.legs) {
      if (leg.kind !== 'money' || leg.from.issuer === leg.to.issuer) continue;
      const cb = this.d.registry.centralBankOf(leg.ccy);
      if (leg.from.issuer !== cb)
        out.push({ bank: leg.from.issuer, centralBank: cb, ccy: leg.ccy, amount: -leg.amount });
      if (leg.to.issuer !== cb)
        out.push({ bank: leg.to.issuer, centralBank: cb, ccy: leg.ccy, amount: leg.amount });
    }
    return out;
  }

  private expandAsset(leg: AssetLeg, ops: Op[]): void {
    const inst = this.d.instruments.get(leg.instrument);
    const perFrom = leg.fromCell.some ? leg.fromCell.value.perMember : leg.qty;
    const perTo = leg.toCell.some ? leg.toCell.value.perMember : leg.qty;
    const price: number | 'carrying' = leg.pricePerUnit.some ? leg.pricePerUnit.value : 'carrying';
    let debitIndex = -1;
    if (issuedBy(inst, leg.from)) {
      if (price === 'carrying') {
        throw new Missing('Register C2.a', `an issuance of ${inst.id} needs a price`, {
          instrument: inst.id,
        });
      }
      ops.push({
        op: 'issue',
        issuer: issuerOf(inst),
        instrument: inst.id,
        qty: leg.qty,
        valuePerUnit: price,
        fromDebit: -1,
      });
    } else {
      debitIndex = ops.length;
      ops.push({ op: 'debit', party: leg.from, instrument: inst.id, qty: perFrom, money: false });
    }
    if (issuedBy(inst, leg.to)) {
      ops.push({
        op: 'redeem',
        issuer: issuerOf(inst),
        instrument: inst.id,
        qty: leg.qty,
        valuePerUnit: price,
        fromDebit: debitIndex,
      });
    } else {
      ops.push({
        op: 'credit',
        party: leg.to,
        instrument: inst.id,
        qty: perTo,
        totalQty: leg.qty,
        money: false,
        basis: price,
        fromDebit: debitIndex,
      });
    }
  }

  // ---- pre-check: every leg or none (XI-5) ---------------------------------------------------

  private precheck(ins: Instruction, ops: readonly Op[]): FailReason | undefined {
    const net = new Map<
      string,
      { party: PartyId; instrument: InstrumentId; delta: number; money: boolean }
    >();
    for (const op of ops) {
      if (op.op !== 'debit' && op.op !== 'credit') continue;
      const key = `${op.party}|${op.instrument}`;
      const cur = net.get(key) ?? {
        party: op.party,
        instrument: op.instrument,
        delta: 0,
        money: op.money,
      };
      cur.delta = finite(cur.delta + (op.op === 'debit' ? -op.qty : op.qty), 'net delta');
      net.set(key, cur);
    }
    // Money Market B3.c: what is already bound cannot be bound again, and what this instruction is
    // about to move cannot be bound either. Free units are the register's answer less whatever this
    // same instruction takes out of the holding — one question asked once, before anything moves.
    for (const leg of ins.legs) {
      if (leg.kind !== 'pledge') continue;
      const per = leg.pledgorCell.some ? leg.pledgorCell.value.perMember : leg.qty;
      const moving = zeroIfNone(net.get(`${leg.pledgor}|${leg.instrument}`)?.delta);
      const free = finite(
        this.d.register.free(leg.pledgor, leg.instrument) + (moving < 0 ? moving : 0),
        'free to pledge',
      );
      // The comparison is EXACT and against the register's own read, because the register asks it
      // again when it binds (Law 4: one question, one answer). A band here would let an
      // instruction pass this check and throw inside the walk that applies it.
      if (per <= free) continue;
      return {
        kind: 'insufficientCollateral',
        party: leg.pledgor,
        instrument: leg.instrument,
        short: finite(per - free, 'collateral short'),
      };
    }
    for (const n of net.values()) {
      if (n.delta >= 0) continue;
      const free = this.d.register.free(n.party, n.instrument);
      const after = finite(free + n.delta, 'balance after');
      // C4: whether a holder can deliver is the REGISTER's question, and it is asked once — here,
      // before anything moves, and again by the walk that moves it, with the same answer. Two
      // readers with two tolerances would let an instruction pass this check and then throw
      // inside the walk, which is one fact with two writers (Law 4).
      if (!n.money) {
        if (this.d.register.deliverable(n.party, n.instrument, -n.delta)) continue;
        return {
          kind: 'insufficientUnits',
          party: n.party,
          instrument: n.instrument,
          short: -after,
        };
      }
      if (after >= 0 || Math.abs(after) <= dustOf(ops.length, Math.abs(free) + Math.abs(n.delta)))
        continue;
      const inst = this.d.instruments.get(n.instrument);
      const issuerId = issuerOf(inst);
      const issuer = this.d.parties.get(issuerId);
      const moneyIssuer = this.d.registry.partyKind(issuer.kind).moneyIssuer;
      if (moneyIssuer === null) {
        return {
          kind: 'overdraftRefused',
          party: n.party,
          issuer: issuerId,
          ccy: inst.ccy,
          short: -after,
        };
      }
      const context = {
        holder: n.party,
        issuer: issuerId,
        ccy: inst.ccy,
        shortfall: -after,
        holderIssuesMoney: this.d.registry.issuesMoney(this.d.parties.get(n.party).kind),
      };
      // B3.a: a kind whose answer is a credit decision does not answer here; the module that owns
      // lending does, and assembly has already refused a world where nobody does.
      const answer =
        moneyIssuer.overdraft === 'aCreditDecision'
          ? this.d.creditDecision(issuer.kind)
          : moneyIssuer.overdraft;
      const decision = answer(context);
      if (!decision.allow) {
        return {
          kind: 'overdraftRefused',
          party: n.party,
          issuer: issuerId,
          ccy: inst.ccy,
          short: -after,
        };
      }
      // B3.c: never a silent negative. Recorded here; priced by the corridor when it exists.
      this.d.journal.record(
        ins.period,
        ins.cycle,
        'reserve.overdraft',
        [n.party, issuerId],
        {
          instruction: ins.id,
          holder: n.party,
          issuer: issuerId,
          ccy: inst.ccy,
          shortfallPerMember: -after,
          recordedAs: decision.recordedAs,
        },
        true,
      );
    }
    return undefined;
  }

  // ---- apply ---------------------------------------------------------------------------------

  private apply(
    ins: Instruction,
    ops: readonly Op[],
  ): { deltas: RegisterDelta[]; equity: EquityEffect[] } {
    const deltas: RegisterDelta[] = [];
    const equity = new Map<PartyId, number>();
    const drawnByOp = new Map<number, DrawnLot[]>();
    const carryingOf = (opIndex: number): number => {
      const drawn = drawnByOp.get(opIndex);
      if (drawn === undefined) throw new Missing('Register D4', `no drawn lots for op ${opIndex}`);
      const op = ops[opIndex];
      if (op?.op !== 'debit') throw new Missing('Register D4', `op ${opIndex} is not a debit`);
      const total = sum(drawn.map((l) => l.qty)).value;
      const value = sum(
        drawn.map((l) =>
          mul(l.qty, this.d.valuation.carryingPerUnit(op.instrument, l, ins.period), 'carrying'),
        ),
      ).value;
      return total === 0 ? 0 : value / total;
    };
    // Law 7: what each party's equity NETTED to, and what it passed THROUGH getting there. A
    // trade takes a book down by the price and up by the value in one instruction, and the
    // rounding that leaves behind is the price's, not the difference's.
    const gross = new Map<PartyId, number>();
    const bump = (party: PartyId, delta: number): void => {
      addTo(equity, party, delta);
      addTo(gross, party, Math.abs(delta));
    };

    ops.forEach((op, index) => {
      switch (op.op) {
        case 'debit': {
          if (op.money) {
            this.d.register.moneyDelta(op.party, op.instrument, -op.qty, ins.period, true);
            drawnByOp.set(index, [
              { lot: 0 as never, qty: op.qty, basisPerUnit: 1, acquired: ins.period },
            ]);
            bump(op.party, -op.qty);
          } else {
            const drawn = this.d.register.debit(op.party, op.instrument, op.qty);
            drawnByOp.set(index, drawn);
            const carrying = sum(
              drawn.map((l) =>
                mul(
                  l.qty,
                  this.d.valuation.carryingPerUnit(op.instrument, l, ins.period),
                  'carrying',
                ),
              ),
            ).value;
            bump(op.party, -carrying);
          }
          deltas.push({
            party: op.party,
            instrument: op.instrument,
            qty: -op.qty,
            weight: weightOf(this.d.parties.get(op.party)),
            target: 'holding',
          });
          break;
        }
        case 'credit': {
          const basis = op.basis === 'carrying' ? carryingOf(op.fromDebit) : op.basis;
          if (op.money)
            this.d.register.moneyDelta(op.party, op.instrument, op.qty, ins.period, true);
          else this.d.register.credit(op.party, op.instrument, op.qty, basis, ins.period);
          bump(op.party, mul(op.qty, basis, 'credit value'));
          deltas.push({
            party: op.party,
            instrument: op.instrument,
            qty: op.qty,
            weight: weightOf(this.d.parties.get(op.party)),
            target: 'holding',
          });
          if (!op.money && op.fromDebit >= 0) {
            // A holder-to-holder transfer re-marks the issuer's liability from the giver's carrying
            // value to the receiver's basis: the liability is the same number read from the other
            // side (Register B3), so the change lands on the issuer.
            const inst = this.d.instruments.get(op.instrument);
            if (this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer) {
              bump(
                issuerOf(inst),
                mul(op.totalQty, carryingOf(op.fromDebit) - basis, 'issuer re-mark'),
              );
            }
          }
          break;
        }
        case 'issue': {
          this.d.instruments.adjustIssued(op.instrument, op.qty);
          deltas.push({
            party: op.issuer,
            instrument: op.instrument,
            qty: op.qty,
            weight: 1,
            target: 'issued',
          });
          const inst = this.d.instruments.get(op.instrument);
          if (this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer) {
            const per = op.valuePerUnit === 'carrying' ? carryingOf(op.fromDebit) : op.valuePerUnit;
            bump(op.issuer, -mul(op.qty, per, 'issue value'));
          }
          break;
        }
        case 'exist': {
          // Goods E4: nothing issued these units, so the delta is booked against the party whose
          // book they appeared on or left; the units family is what checks the identity.
          this.d.instruments.adjustIssued(op.instrument, op.qty);
          deltas.push({
            party: op.holder,
            instrument: op.instrument,
            qty: op.qty,
            weight: 1,
            target: 'issued',
          });
          break;
        }
        case 'redeem': {
          this.d.instruments.adjustIssued(op.instrument, -op.qty);
          deltas.push({
            party: op.issuer,
            instrument: op.instrument,
            qty: -op.qty,
            weight: 1,
            target: 'issued',
          });
          const inst = this.d.instruments.get(op.instrument);
          if (this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer) {
            // The issuer's liability is the same number read from the holder's side (Register B3).
            const per =
              op.fromDebit >= 0
                ? carryingOf(op.fromDebit)
                : op.valuePerUnit === 'carrying'
                  ? 1
                  : op.valuePerUnit;
            bump(op.issuer, mul(op.qty, per, 'redeem value'));
          }
          break;
        }
        case 'bind': {
          // Register D5: the units stay where they are and stay on the same book — an encumbrance
          // moves nothing, so no equity account is touched and no delta is a movement of value.
          this.d.register.pledge(
            op.pledgor,
            op.instrument,
            op.qty,
            op.beneficiary,
            op.secures,
            ins.period,
          );
          break;
        }
        case 'free': {
          this.d.register.release(op.pledgor, op.instrument, op.lien);
          break;
        }
        case 'reseat': {
          const inst = this.d.instruments.get(op.instrument);
          const liability = this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer;
          // Register B3: what the obligation is worth is what its holders carry it at — read from
          // their side, never re-derived from the issuer's (Law 19). A line nobody owes (a good) is
          // re-seated with no equity effect at all, which is a different answer from zero.
          const owed = !liability
            ? 0
            : sum(
                this.d.register.holdersOf(op.instrument).map((h) => {
                  const held = this.d.register.holding(h, op.instrument);
                  if (!held.some) return 0;
                  return mul(
                    this.d.valuation.valueOfLots(op.instrument, held.value.lots, ins.period),
                    weightOf(this.d.parties.get(h)),
                    'liability held',
                  );
                }),
              ).value;
          this.d.instruments.reseat(op.instrument, op.to);
          bump(op.from, owed);
          bump(op.to, -owed);
          break;
        }
        default:
          assertNever(op, 'Op');
      }
    });

    const effects: EquityEffect[] = [];
    for (const [party, delta] of equity) {
      if (delta === 0) continue;
      this.checkHomeCurrency(party, ins);
      this.d.register.moveEquity({
        party,
        delta,
        cause: `instruction ${ins.id}`,
        through: zeroIfNone(gross.get(party)),
      });
      effects.push({ party, delta });
    }
    return { deltas, equity: effects };
  }

  /**
   * Equity accounts are kept in the party's home money. Until the currency layer revalues foreign
   * positions (Currency D2, worklist: currency layer) an instruction may not touch an instrument in
   * another money: two currencies are never added (Money A2.b).
   */
  private checkHomeCurrency(party: PartyId, ins: Instruction): void {
    const home = this.d.registry.region(this.d.parties.get(party).region).ccy;
    for (const leg of ins.legs) {
      const ccy = leg.kind === 'money' ? leg.ccy : this.d.instruments.get(leg.instrument).ccy;
      const touches =
        leg.kind === 'money'
          ? leg.from.holder === party ||
            leg.to.holder === party ||
            leg.from.issuer === party ||
            leg.to.issuer === party
          : leg.kind === 'asset' || leg.kind === 'assume'
            ? leg.from === party || leg.to === party
            : leg.kind === 'pledge' || leg.kind === 'release'
              ? leg.pledgor === party || leg.beneficiary === party
              : leg.party === party;
      if (touches && ccy !== home) {
        throw new Mismatch(
          'Money A2.b',
          `instruction ${ins.id}: ${party} books in ${home}; leg is in ${ccy} (currency layer not built)`,
          {
            party,
            home,
            ccy,
          },
        );
      }
    }
  }
}


/** Helpers for mechanisms building legs (XI-15: a cell side is per member). */
export function cellSide(p: Party, perMember: number): CellSide | undefined {
  if (p.representation !== 'cell') return undefined;
  return { perMember, weight: p.weight };
}

/**
 * Law 8, XI-15: what can actually be moved to or from a party, per member and in total.
 *
 * A member of a cell is a real holder with a real account, so its share is a whole number of the
 * unit's smallest piece like anybody else's — and the total is that times the weight, which is
 * therefore on the grid too. What the fraction below a tick would have been is not moved: it does
 * not exist, so it stays where it was.
 */
export function shareFor(
  registry: Pick<Registry, 'deliverable'>,
  party: Party,
  unit: UnitId,
  perMemberWanted: number,
): { readonly perMember: number; readonly total: number } {
  const perMember = registry.deliverable(unit, perMemberWanted);
  return { perMember, total: mul(perMember, weightOf(party), `total for ${party.id}`) };
}

/** Total moved on a party's side: weight x per member for a cell. */
export function totalFor(p: Party, perMember: number): number {
  return mul(perMember, weightOf(p), `total for ${p.id}`);
}
