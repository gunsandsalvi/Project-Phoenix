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
import { ARREAR, arrearId, isArrear, type ArrearTerms } from '../register/arrears.js';
import { issuedBy, issuerOf } from '../register/instruments.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import { assertNever, forbid, impossible } from '../core/assert.js';
import { FAILED, RESERVE_OVERDRAFT, SETTLED } from './facts.js';
import { Forbidden, Missing } from '../core/errors.js';
import {
  type ContractId,
  currencyUnit,
  type CurrencyCode,
  type DerivativeKindId,
  type InstrumentId,
  type LienId,
  moneyInstrumentId,
  type PartyId,
  type UnitId,
} from '../core/ids.js';
import {
  absolute,
  asPerPiece,
  asRatio,
  asTotal,
  type Cash,
  heldAsMoney,
  minus,
  negated,
  type Total,
  type PerPiece,
  pricedAt,
  scale,
  valueAt,
  noCash,
  sumCash,
} from '../core/measure.js';
import { addTo, atMost, finite, sum, zeroIfNone } from '../core/num.js';
import { none, some } from '../core/option.js';
import { negQty, onTick, type Qty } from '../core/tick.js';
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
  Realised,
  AccountRef,
  AssetLeg,
  AssumeLeg,
  CreateLeg,
  DestroyLeg,
  EquityEffect,
  FailReason,
  Instruction,
  InstructionDraft,
  ContractLeg,
  MoneyLeg,
  OpenContractLeg,
  PledgeLeg,
  RegisterDelta,
  ReleaseLeg,
  ReserveLeg,
  SettlementRecord,
  VoyageLeg,
} from './instruction.js';
import { isMoneyLeg, subjectsOf, type Failed } from './instruction.js';
import type { Ledger } from './ledger.js';
import type { Contracts } from '../register/contracts.js';
import type { Voyages } from '../register/voyages.js';
import type { Contract, DerivativeKindProfile, Underlying } from '../registry/derivatives.js';

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
  /**
   * Derivative X1, D1, D11: the contract store and what a row is carried at. Settlement is its one
   * writer, as it is the register's: a position opening or closing on two balance sheets is a
   * change of state and goes over the wire (Money D1, D4).
   */
  readonly contracts: Contracts;
  /** 13c.1: where what is on its way has got to (Freight A3). */
  readonly voyages: Voyages;
  contractCarrying(c: Contract, at: Period): Cash;
  derivativeKind(kind: DerivativeKindId): DerivativeKindProfile;
  /**
   * Derivative D3.a, Derivative Layer G4: whether this world actually produces the thing a contract
   * settles against — a market that clears it, an index that reads it, a party whose events it
   * records. A derivative on a price nothing clears prices itself, and the refusal is at the site
   * the contract is written rather than at the first mark nobody can take.
   */
  underlyingExists(u: Underlying): string | undefined;
}

/** One register-level operation an instruction expands into; quantities are per member for cells. */
type Op =
  | {
      readonly op: 'debit';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: Qty;
      readonly money: boolean;
    }
  | {
      readonly op: 'credit';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: Qty;
      /** Total units on the leg (weight x per member for a cell), for the issuer's side. */
      readonly totalQty: Qty;
      readonly money: boolean;
      /** Basis per unit, or 'carrying' to take the giver's carrying value (a transfer). */
      readonly basis: PerPiece | 'carrying';
      /** The debit whose carrying value a 'carrying' basis reads; -1 for an issuance. */
      readonly fromDebit: number;
    }
  | {
      readonly op: 'issue';
      readonly issuer: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: Qty;
      readonly valuePerUnit: PerPiece | 'carrying';
      readonly fromDebit: number;
    }
  | {
      readonly op: 'redeem';
      readonly issuer: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: Qty;
      readonly valuePerUnit: PerPiece | 'carrying';
      readonly fromDebit: number;
    }
  /** Goods E4: units of a physical kind coming into existence or leaving it; nobody issued them. */
  | {
      readonly op: 'exist';
      readonly holder: PartyId;
      readonly instrument: InstrumentId;
      readonly qty: Qty;
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
      readonly qty: Qty;
      readonly secures: string;
    }
  | {
      readonly op: 'free';
      readonly pledgor: PartyId;
      readonly instrument: InstrumentId;
      readonly lien: LienId;
    }
  /** Derivative D1, D11: a contract appears on two books at its struck value, or leaves them. */
  | {
      readonly op: 'writeContract';
      readonly leg: OpenContractLeg;
    }
  | {
      readonly op: 'tearUpContract';
      readonly contract: ContractId;
      readonly why: string;
    }
  /** Freight A1, A3: a journey opens, gets on, loses something or arrives. */
  | {
      readonly op: 'voyage';
      readonly leg: VoyageLeg;
    }
  /** Derivative Layer B4: the obligation leaves one balance sheet and lands on another. */
  | {
      readonly op: 'novateContract';
      readonly contract: ContractId;
      readonly from: PartyId;
      readonly to: PartyId;
    };

/**
 * Register B3, XI-3, §48 (13f): whether what this kind's ISSUER owes follows a price, or is the
 * face it promised. One read, three call sites, so a liability cannot be face on one of them and
 * market on another (Law 4).
 */
const owesItsValue = (k: {
  readonly liabilityOfIssuer: boolean;
  readonly owes: 'face' | 'value';
}): boolean => k.liabilityOfIssuer && k.owes === 'value';

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
      this.d.journal.say(
        period,
        cycle,
        FAILED,
        subjectsOf(instruction),
        { id: instruction.id, cause: instruction.cause, reason: fail.kind },
        false,
      );
      this.writeArrears(record, period, cycle);
      return record;
    }

    const applied = this.apply(instruction, ops);
    const record: SettlementRecord = {
      outcome: 'settled',
      instruction,
      deltas: applied.deltas,
      equity: applied.equity,
      realised: applied.realised,
      contracts: applied.contracts,
      reserveLegs,
    };
    this.d.ledger.append(record);
    this.d.journal.say(
      period,
      cycle,
      SETTLED,
      subjectsOf(instruction),
      { id: instruction.id, cause: instruction.cause, legs: instruction.legs.length },
      false,
    );
    return record;
  }

  /**
   * Money E1, E1.b (12a.1): A PAYER THAT COULD NOT PAY OWES A ROW. For every money leg the payer
   * that was refused could not fund, an arrear is issued by the payer to the payee for the amount,
   * in the money it was due in, in the same pass as the fail — one writer, settlement. It is an
   * issuance like any other, so the register and the ledger carry it and the audit reads it; a
   * fail that was not the payer's money (a delivery that was short) leaves nobody in arrears.
   */
  private writeArrears(failed: Failed, period: Period, cycle: Cycle): void {
    if (failed.reason.kind !== 'overdraftRefused') return;
    // A missed payment ON an arrear is the same arrear still standing, never a row on a row.
    if (
      failed.instruction.legs.some(
        (l) =>
          l.kind === 'asset' &&
          this.d.instruments.has(l.instrument) &&
          isArrear(this.d.instruments.get(l.instrument)),
      )
    )
      return;
    const payer = failed.reason.party;
    let n = 0;
    for (const leg of failed.instruction.legs) {
      if (!isMoneyLeg(leg) || leg.from.holder !== payer || leg.to.holder === payer) continue;
      if (
        !this.d.parties.get(payer).status.alive ||
        !this.d.parties.get(leg.to.holder).status.alive
      )
        continue;
      // Law 8: a money leg's amount is already whole pieces of its money.
      const amount = leg.amount;
      if (amount <= 0) continue;
      n += 1;
      const id = arrearId(failed.instruction.id, n);
      if (this.d.instruments.has(id)) continue;
      const terms: ArrearTerms = {
        kind: ARREAR,
        failed: failed.instruction.id,
        // 0i.5: every money leg says what it is, so an arrear always has a class and there is
        // no `unclassified` tier for it to fall into.
        class: leg.receipt.of,
        payer,
        payee: leg.to.holder,
      };
      this.d.instruments.add({
        id,
        kind: ARREAR,
        issuer: some(payer),
        ccy: leg.ccy,
        terms,
        market: none(),
      });
      this.settle(
        {
          legs: [
            {
              kind: 'asset',
              from: payer,
              to: leg.to.holder,
              instrument: id,
              qty: amount,
              pricePerUnit: some(asPerPiece(1, 'a piece of money owed is a piece of money')),
              accruedPerUnit: none(),
            },
          ],
          cause: 'default',
          reason: `${String(payer)} owes ${String(leg.to.holder)} ${String(amount)} it could not pay on instruction ${String(failed.instruction.id)}`,
        },
        period,
        cycle,
      );
    }
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
        case 'contract':
          this.validateContract(leg, ins);
          break;
        case 'voyage':
          this.validateVoyage(leg);
          break;
        default:
          assertNever(leg, 'Leg');
      }
    }
  }

  /**
   * Freight A1, E1: what a voyage leg has to be true about before anything is applied. The rest —
   * that a voyage exists, that it has not already landed, that it does not lose more than it is
   * carrying — the store itself refuses at its own door.
   */
  /** Law 19: which plant a voyage bound is read off the lien it holds, never restated on the row. */
  private hullsOf(row: { readonly carrier: PartyId; readonly hulls: LienId }): InstrumentId {
    for (const holding of this.d.register.holdingsOf(row.carrier)) {
      if (holding.liens.some((l) => l.id === row.hulls)) return holding.instrument;
    }
    throw new Missing('Freight E2', `voyage lien ${row.hulls} binds nothing`);
  }

  private validateVoyage(leg: VoyageLeg): void {
    if (leg.act !== 'sail') return;
    impossible(
      finite(leg.km, 'the distance of a voyage') > 0,
      'Freight E1',
      'no instantaneous transport: a voyage covers a real distance',
    );
    impossible(leg.tiles.length > 0, 'Freight A1', 'a voyage has a path over real ground');
    impossible(
      leg.hulls > 0,
      'Freight E2',
      'no shipment without capacity: a voyage commits hulls somebody owns',
    );
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
    const unit = currencyUnit(leg.ccy);
    this.onTheGrid(leg.amount, unit, ins);
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
    this.onTheGrid(leg.qty, inst.unit, ins);
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
    forbid(
      leg.from !== leg.to,
      'Register F2',
      `instruction ${ins.id}: ${leg.from} assumes its own paper`,
    );
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
    this.onTheGrid(leg.qty, inst.unit, ins);
  }

  /**
   * Derivative D1, D1.a, D2, D5, G1, G4: A CONTRACT HAS TWO NAMED LIVE SIDES, a notional in a unit,
   * a money its legs move in, and an underlying this world produces somewhere else.
   *
   * Every one of these is a contract violation and not an outcome: a payoff received from nobody is
   * invented money (D1.a), and a derivative settling against a price nothing clears is a contract
   * that prices itself (G4, D3.a). None of them is something a participant could legitimately have
   * tried, so none of them is a `Failed` record.
   */
  private validateContract(leg: ContractLeg, ins: Instruction): void {
    if (leg.act !== 'open') {
      forbid(
        this.d.contracts.has(leg.contract),
        'Derivative D11',
        `instruction ${ins.id}: no contract ${leg.contract}`,
      );
      const row = this.d.contracts.get(leg.contract);
      forbid(
        row.state === 'open',
        'Derivative D11',
        `instruction ${ins.id}: ${leg.contract} is already terminated`,
      );
      if (leg.act === 'close') {
        forbid(leg.why.length > 0, 'Money C1.b', `instruction ${ins.id}: a termination says why`);
        return;
      }
      forbid(
        row.a === leg.from || row.b === leg.from,
        'Derivative Layer B4',
        `instruction ${ins.id}: ${leg.from} is not a side of ${leg.contract}`,
      );
      this.alive(leg.to, ins);
      return;
    }
    forbid(
      leg.a !== leg.b,
      'Derivative D1',
      `instruction ${ins.id}: a contract has two counterparties, and they differ`,
    );
    this.alive(leg.a, ins);
    this.alive(leg.b, ins);
    if (leg.house !== null) this.alive(leg.house, ins);
    impossible(
      finite(leg.notional, 'contract notional') > 0,
      'Derivative D2',
      `a contract has a notional and it is positive, got ${leg.notional}`,
    );
    finite(leg.value.pieces, 'what the contract is worth at inception');
    finite(leg.struckAt.level, 'the level it was struck at');
    this.d.registry.currency(leg.ccy);
    const profile = this.d.derivativeKind(leg.derivative);
    profile.validateTerms(leg.terms);
    this.onTheGrid(leg.notional, profile.unit, ins);
    // G4, D3.a: the underlying is asked of the profile against the contract as it will be, so a
    // kind whose underlying is read off its terms is checked on the terms it is being written with.
    const asIfOpen: Contract = {
      id: 'unwritten' as Contract['id'],
      pairedWith: none<ContractId>(),
      kind: leg.derivative,
      a: leg.a,
      b: leg.b,
      terms: leg.terms,
      ccy: leg.ccy,
      notional: leg.notional,
      struckAt: leg.struckAt,
      basis: leg.value,
      opened: ins.period,
      state: 'open',
      terminated: none(),
      house: leg.house,
    };
    const missing = this.d.underlyingExists(profile.underlying(asIfOpen));
    if (missing !== undefined) {
      throw new Forbidden(
        'Derivative Layer G4',
        `instruction ${ins.id}: ${leg.derivative} settles against ${missing}, which this world does not produce`,
      );
    }
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
    this.onTheGrid(leg.qty, inst.unit, ins);
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
    impossible(
      onTick(qty),
      'Law 8',
      `instruction ${ins.id}: ${qty} is not a whole number of pieces of ${unit}`,
    );
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
            // 0f.1: the register holds TOTALS; the cell side on the leg is a statement the check
            // above holds it to, not a quantity anything is written in.
            qty: leg.qty,
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
            qty: leg.qty,
            money: false,
          });
          ops.push({
            op: 'exist',
            holder: leg.party,
            instrument: leg.instrument,
            qty: negQty(leg.qty, 'what leaves the world'),
          });
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
            qty: leg.qty,
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
        case 'voyage':
          ops.push({ op: 'voyage', leg });
          break;
        case 'contract':
          ops.push(
            leg.act === 'open'
              ? { op: 'writeContract', leg }
              : leg.act === 'close'
                ? { op: 'tearUpContract', contract: leg.contract, why: leg.why }
                : { op: 'novateContract', contract: leg.contract, from: leg.from, to: leg.to },
          );
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
    // 0f.1: the register holds TOTALS, for a cell as for a named party.
    const perFrom = leg.amount;
    const perTo = leg.amount;

    // Payer minus (C2), or creation if the payer is the issuer itself (C4).
    if (leg.from.holder === i1)
      ops.push({
        op: 'issue',
        issuer: i1,
        instrument: m1,
        qty: leg.amount,
        valuePerUnit: asPerPiece(1, 'one of itself'),
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
        valuePerUnit: asPerPiece(1, 'one of itself'),
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
        basis: asPerPiece(1, 'one of itself'),
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
          valuePerUnit: asPerPiece(1, 'one of itself'),
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
          valuePerUnit: asPerPiece(1, 'one of itself'),
          fromDebit: -1,
        });
        ops.push({
          op: 'credit',
          party: i2,
          instrument: reserves,
          qty: leg.amount,
          totalQty: leg.amount,
          money: true,
          basis: asPerPiece(1, 'one of itself'),
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
        out.push({
          bank: leg.from.issuer,
          centralBank: cb,
          ccy: leg.ccy,
          amount: negQty(leg.amount, 'what left this bank'),
        });
      if (leg.to.issuer !== cb)
        out.push({ bank: leg.to.issuer, centralBank: cb, ccy: leg.ccy, amount: leg.amount });
    }
    return out;
  }

  private expandAsset(leg: AssetLeg, ops: Op[]): void {
    const inst = this.d.instruments.get(leg.instrument);
    const perFrom = leg.qty;
    const perTo = leg.qty;
    const price: PerPiece | 'carrying' = leg.pricePerUnit.some
      ? leg.pricePerUnit.value
      : 'carrying';
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
      cur.delta = finite(
        cur.delta + (op.op === 'debit' ? negQty(op.qty, 'what leaves') : op.qty),
        'net delta',
      );
      net.set(key, cur);
    }
    // Money Market B3.c: what is already bound cannot be bound again, and what this instruction is
    // about to move cannot be bound either. Free units are the register's answer less whatever this
    // same instruction takes out of the holding — one question asked once, before anything moves.
    for (const leg of ins.legs) {
      if (leg.kind !== 'pledge') continue;
      const per = leg.qty;
      const moving = zeroIfNone(net.get(`${leg.pledgor}|${leg.instrument}`)?.delta);
      const free = finite(
        this.d.register.free(leg.pledgor, leg.instrument) +
          atMost(moving, 0, 'units this instruction ADDS are not there to pledge until it settles'),
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
    // Register D5, XI-8 (12c.3): what this same instruction FREES is deliverable by it — an estate
    // takes a dead party's pledged units and their lien in one numbered pass, so the collateral is
    // never briefly nobody's. The walk applies the release before the debit, and asks the register
    // again (Law 4: the same question, once here and once there).
    const freed = new Map<string, number>();
    for (const leg of ins.legs) {
      if (leg.kind !== 'release') continue;
      const held = this.d.register.holding(leg.pledgor, leg.instrument);
      const lien = held.some ? held.value.liens.find((l) => l.id === leg.lien) : undefined;
      if (lien === undefined) continue;
      addTo(freed, `${leg.pledgor}|${leg.instrument}`, lien.qty);
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
        const unbound = zeroIfNone(freed.get(`${n.party}|${n.instrument}`));
        if (unbound >= -n.delta) continue;
        if (
          this.d.register.deliverable(
            n.party,
            n.instrument,
            finite(-n.delta - unbound, 'what it must deliver beyond what this frees'),
          )
        )
          continue;
        return {
          kind: 'insufficientUnits',
          party: n.party,
          instrument: n.instrument,
          short: finite(-after - unbound, 'units short'),
        };
      }
      // Law 8: a balance is a count of the money's own smallest piece, and so is every delta that
      // reaches it. An account is overdrawn by a whole piece or it is not overdrawn (item 13b.1).
      if (after >= 0) continue;
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
      // B3.c: never a silent negative. It is public because a drawing is information — a depositor
      // answers it (Money Market D5.a) and a rival bank sees it (E2.a). What it IS, is whatever the
      // module that allowed it writes behind it before the period closes.
      forbid(decision.allow, 'Money B3.c', 'an overdraft was neither allowed nor refused');
      this.d.journal.say(
        ins.period,
        ins.cycle,
        RESERVE_OVERDRAFT,
        [n.party, issuerId],
        {
          instruction: ins.id,
          holder: n.party,
          issuer: issuerId,
          ccy: inst.ccy,
          // 21a: a holding is a TOTAL, so what the account is short of is one too. This was called
          // `shortfallPerMember` and stopped being true at that item, with nothing to catch it.
          shortfall: -after,
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
  ): {
    deltas: RegisterDelta[];
    equity: EquityEffect[];
    contracts: ContractId[];
    realised: Realised[];
  } {
    const deltas: RegisterDelta[] = [];
    /**
     * Treasury C1, Law 19: what a seller made, from the one pass that knows both halves.
     *
     * The money leg said the money is proceeds of a sale; the debit below says what the lots it drew
     * were carried at. Put side by side here and nowhere else, so nobody re-derives either — and so
     * a tax on a gain is a tax on a gain rather than on the whole gross (A-37, A-46).
     */
    const sold = new Map<string, Cash>();
    const realised: Realised[] = [];
    const equity = new Map<PartyId, Total<'money:piece'>>();
    /** Derivative D1: the rows this instruction wrote, so its drafter can name what it opened. */
    const written: ContractId[] = [];
    const drawnByOp = new Map<number, DrawnLot[]>();
    const carryingOf = (opIndex: number): PerPiece => {
      const drawn = drawnByOp.get(opIndex);
      if (drawn === undefined) throw new Missing('Register D4', `no drawn lots for op ${opIndex}`);
      const op = ops[opIndex];
      if (op?.op !== 'debit') throw new Missing('Register D4', `op ${opIndex} is not a debit`);
      const total = sum(drawn.map((l) => l.qty)).value;
      const value = sumCash(
        ccyOf(op.instrument),
        drawn.map((l) =>
          valueAt(
            this.d.valuation.carryingPerUnit(op.instrument, l, ins.period),
            l.qty,
            ccyOf(op.instrument),
            'carrying',
          ),
        ),
        'carrying',
      ).value;
      return total === 0
        ? asPerPiece(0, 'nothing was drawn, so nothing was carried')
        : pricedAt(value, total, 'what a unit of it was carried at');
    };
    // Law 7: what each party's equity NETTED to, and what it passed THROUGH getting there. A
    // trade takes a book down by the price and up by the value in one instruction, and the
    // rounding that leaves behind is the price's, not the difference's.
    const gross = new Map<PartyId, number>();
    // Currency A3 (16.0): the money a line is in, for what a leg on it comes to.
  /**
   * Law 4, Law 19, Law 18 (0g.26): WHAT MONEY A LINE IS IN, ASKED ONCE PER INSTRUCTION.
   *
   * An instruction is ATOMIC: nothing is issued, reseated or ceased while it applies, so a line's
   * currency cannot change between two of its legs. It was read from the register two and three
   * times per leg — once for the sum's currency, once inside the map for each drawn lot, once more
   * in the bump — and the estate hand-over that carries a whole cell's book is one instruction with
   * **five thousand four hundred and eighty-nine legs**. Measured: that single instruction cost
   * **231,250 kernel reads, and sixty of them are 13,875,032 of the period's reads.**
   *
   * The cache lives and dies with the instruction, so there is nothing for a later period to read
   * stale: it is a local for a loop, not a store.
   */
    const ccySeen = new Map<InstrumentId, CurrencyCode>();
    const ccyOf = (instrument: InstrumentId): CurrencyCode => {
      const had = ccySeen.get(instrument);
      if (had !== undefined) return had;
      const ccy = this.d.instruments.get(instrument).ccy;
      ccySeen.set(instrument, ccy);
      return ccy;
    };
    /**
     * Currency B1, C5, Law 18 (0g.26): AND WHICH MONEY A PARTY REPORTS IN, once per party per
     * instruction, for the same reason — a party's region does not change mid-instruction, and the
     * estate instruction bumps ONE party's account five thousand times.
     */
    const homeSeen = new Map<PartyId, CurrencyCode>();
    const homeOf = (party: PartyId): CurrencyCode => {
      const had = homeSeen.get(party);
      if (had !== undefined) return had;
      const home = this.d.valuation.ownMoneyOf(party);
      homeSeen.set(party, home);
      return home;
    };
    /**
     * 21a, A-1, XI-15: THE ONE DOOR TO AN EQUITY ACCOUNT, and it takes a TOTAL.
     *
     * There were two — `bumpTotal`, which divided by the weight, and `bumpPerMember`, which did
     * not — because a cell's equity account was kept per member while everything else it held
     * became a total at 0f.1. Every writer had to know which of the two its `Cash` was, and the
     * record of what that cost is 21.101 (a probate office crediting nine hundred households with
     * the whole estate each) and 21.104 (the account and the sheet drifting apart by the
     * divide-and-multiply itself). The account is a total now, so there is one denomination, one
     * door, and nothing to say.
     *
     * A contract's value comes in here too: it belongs to no instrument, being a bilateral
     * obligation and not a holding (Derivative X1), and its money is the contract's own (D5) — but
     * it is the party's total the same way, so it takes the same door.
     */
    const bumpIn = (party: PartyId, delta: Cash): void => {
      const own = inOwn(party, delta);
      addTo(equity, party, asTotal<'money:piece'>(own.pieces, 'what the account moves by'));
      addTo(gross, party, absolute(own, 'what it passed through').pieces);
    };
    /**
     * Currency C5, Money A2.b: AN EQUITY ACCOUNT IS KEPT IN ITS PARTY'S OWN MONEY, so what a leg in
     * another money did to it is converted at the rate that same period settles at — the one rate
     * (C5), asked for once, here. Two currencies are still never added; two amounts in ONE are.
     *
     * For everything in a party's own money this is the number that went in, unchanged, and for a
     * world with one currency it never runs. Where the single-currency guard used to stand, and it
     * does the thing the guard was standing in for.
     */
    const inOwn = (party: PartyId, delta: Cash): Cash =>
      // A-51: through the kernel's one door. This was the same conversion written out here, and it
      // was the only place in the engine that did it — six readers of a party's own book did not.
      this.d.valuation.inMoney(delta, homeOf(party), ins.period);

    ops.forEach((op, index) => {
      switch (op.op) {
        case 'debit': {
          if (op.money) {
            this.d.register.moneyDelta(
              op.party,
              op.instrument,
              negQty(op.qty, 'what leaves'),
              ins.period,
            );
            drawnByOp.set(index, [
              {
                lot: 0 as never,
                qty: op.qty,
                // Money D2: one of itself, and it is the only price in this world that is written.
                basisPerUnit: asPerPiece(1, 'money is worth one of itself'),
                acquired: ins.period,
              },
            ]);
            bumpIn(
              op.party,
              negated(heldAsMoney(op.qty, ccyOf(op.instrument), 'what leaves'), 'what leaves'),
            );
          } else {
            const drawn = this.d.register.debit(op.party, op.instrument, op.qty);
            drawnByOp.set(index, drawn);
            const carrying = sumCash(
              ccyOf(op.instrument),
              drawn.map((l) =>
                valueAt(
                  this.d.valuation.carryingPerUnit(op.instrument, l, ins.period),
                  l.qty,
                  ccyOf(op.instrument),
                  'carrying',
                ),
              ),
              'carrying',
            ).value;
            // What this party's units of this line cost it, kept for the disposal pairing below.
            sold.set(`${op.party}/${op.instrument}`, carrying);
            bumpIn(op.party, negated(carrying, 'what it gave up'));
          }
          deltas.push({
            party: op.party,
            instrument: op.instrument,
            qty: negQty(op.qty, 'the other way'),
            target: 'holding',
          });
          break;
        }
        case 'credit': {
          const basis = op.basis === 'carrying' ? carryingOf(op.fromDebit) : op.basis;
          if (op.money) this.d.register.moneyDelta(op.party, op.instrument, op.qty, ins.period);
          else this.d.register.credit(op.party, op.instrument, op.qty, basis, ins.period);
          bumpIn(op.party, valueAt(basis, op.qty, ccyOf(op.instrument), 'credit value'));
          deltas.push({
            party: op.party,
            instrument: op.instrument,
            qty: op.qty,
            target: 'holding',
          });
          if (!op.money && op.fromDebit >= 0) {
            /**
             * Register B3 (13f): A HOLDER-TO-HOLDER TRANSFER CHANGES NOTHING THE ISSUER OWES, when
             * what it owes is the face. Two parties agreeing a price between themselves is not an
             * event on the borrower's book — it still has to find the whole amount on the day — so
             * the issuer is not a side of their trade and its equity does not move with it.
             *
             * Where the claim IS the book (a fund share), the liability really is the same number
             * read from the other side and the re-mark does land on the issuer.
             */
            const inst = this.d.instruments.get(op.instrument);
            if (owesItsValue(this.d.registry.instrumentKind(inst.kind))) {
              bumpIn(
                issuerOf(inst),
                valueAt(
                  minus(carryingOf(op.fromDebit), basis, 'what the re-mark moved it by'),
                  op.totalQty,
                  inst.ccy,
                  'issuer re-mark',
                ),
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
            target: 'issued',
          });
          const inst = this.d.instruments.get(op.instrument);
          const issuedKind = this.d.registry.instrumentKind(inst.kind);
          if (issuedKind.liabilityOfIssuer) {
            /**
             * Register B3 (13f): WHAT IT TOOK ON IS THE FACE, and the cash it got is whatever the
             * book gave it. A bond brought at 98 leaves its issuer owing 100 and holding 98, and
             * the two is a DISCOUNT ON ISSUE it wears on the day — real, and the reason a issuer
             * with a poor name pays for it at the moment it borrows rather than never.
             *
             * This used to book the liability at the price, so an issue at any price was equity-
             * neutral and a deteriorating name could raise money for ever at no cost to its book.
             */
            const per = owesItsValue(issuedKind)
              ? op.valuePerUnit === 'carrying'
                ? carryingOf(op.fromDebit)
                : op.valuePerUnit
              : asPerPiece(1, 'at what it promised');
            bumpIn(
              op.issuer,
              negated(valueAt(per, op.qty, ccyOf(op.instrument), 'issue value'), 'what it took on'),
            );
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
            target: 'issued',
          });
          break;
        }
        case 'redeem': {
          this.d.instruments.adjustIssued(op.instrument, negQty(op.qty, 'what ceased to exist'));
          deltas.push({
            party: op.issuer,
            instrument: op.instrument,
            qty: negQty(op.qty, 'the other way'),
            target: 'issued',
          });
          const inst = this.d.instruments.get(op.instrument);
          const redeemedKind = this.d.registry.instrumentKind(inst.kind);
          if (redeemedKind.liabilityOfIssuer) {
            // Register B3: the obligation that goes away is the one it carried — the face of it,
            // unless the claim was the book, in which case it is what the book was worth.
            const per = owesItsValue(redeemedKind)
              ? op.fromDebit >= 0
                ? carryingOf(op.fromDebit)
                : op.valuePerUnit === 'carrying'
                  ? asPerPiece(1, 'at what it promised')
                  : op.valuePerUnit
              : asPerPiece(1, 'at what it promised');
            bumpIn(op.issuer, valueAt(per, op.qty, ccyOf(op.instrument), 'redeem value'));
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
          const seatedKind = this.d.registry.instrumentKind(inst.kind);
          /**
           * Register B3 (13f): WHAT THE OBLIGATION IS, read from the holders' side because that is
           * where the balances are (Law 19) — and it is the FACE of them unless the claim is the
           * book itself, in which case it is what the book is worth. A line nobody owes (a good) is
           * re-seated with no equity effect at all, which is a different answer from zero.
           *
           * The number an estate takes on has to be the same number the dead party was carrying,
           * or the two books disagree by the difference and the balance-sheet identity fires on the
           * estate — which is how this one was found.
           */
          const seatedCcy = ccyOf(op.instrument);
          const owed: Cash = !seatedKind.liabilityOfIssuer
            ? noCash(seatedCcy)
            : sumCash(
                seatedCcy,
                this.d.register.holdersOf(op.instrument).map((h) => {
                  const held = this.d.register.holding(h, op.instrument);
                  if (!held.some) return noCash(seatedCcy);
                  return scale(
                    owesItsValue(seatedKind)
                      ? this.d.valuation.valueOfLots(op.instrument, held.value.lots, ins.period)
                      : heldAsMoney(
                          sum(held.value.lots.map((l) => l.qty)).value,
                          seatedCcy,
                          'the face it holds',
                        ),
                    asRatio(weightOf(this.d.parties.get(h)), 'the people it stands for'),
                    'liability held',
                  );
                }),
                'what the dead party owed on it',
              ).value;
          this.d.instruments.reseat(op.instrument, op.to);
          bumpIn(op.from, owed);
          bumpIn(op.to, negated(owed, 'and onto the new one'));
          break;
        }
        case 'writeContract': {
          // D1: the row exists from this instruction on, and it is an asset to one side and a
          // liability to the other at every instant from now — including this one, which is why
          // the value lands on both equity accounts here rather than waiting for a revaluation.
          const leg = op.leg;
          const row = this.d.contracts.open(
            {
              kind: leg.derivative,
              a: leg.a,
              b: leg.b,
              terms: leg.terms,
              ccy: leg.ccy,
              notional: leg.notional,
              struckAt: leg.struckAt,
              basis: leg.value,
              house: leg.house,
            },
            ins.period,
          );
          written.push(row.id);
          bumpIn(leg.a, leg.value);
          bumpIn(leg.b, negated(leg.value, 'and the other side of it'));
          break;
        }
        case 'voyage': {
          // Freight A3: a journey opening, getting on, losing something or arriving. NOTHING HERE
          // MOVES VALUE — the cargo leaving, the freight being paid and what a storm destroyed are
          // ordinary legs of the same instruction, drafted beside this one. What this writes is
          // WHERE the thing is, which is a fact the register has no room for (Law 4).
          const leg = op.leg;
          if (leg.act === 'sail') {
            // E2: the hulls are bound where they stand. The register already refuses to move
            // encumbered units, so this IS "a hull cannot be in two trades at once" (Law 12).
            const lien = this.d.register.pledge(
              leg.carrier,
              leg.hullInstrument,
              leg.hulls,
              leg.shipper,
              `voyage of ${leg.qty} ${leg.cargo}`,
              ins.period,
            );
            this.d.voyages.open(
              {
                carrier: leg.carrier,
                shipper: leg.shipper,
                by: leg.by,
                tiles: leg.tiles,
                km: leg.km,
                cargo: leg.cargo,
                qty: leg.qty,
                hulls: lien.id,
                freight: leg.freight,
              },
              ins.period,
            );
          } else if (leg.act === 'advance') {
            this.d.voyages.advance(leg.voyage, leg.km);
          } else if (leg.act === 'lose') {
            this.d.voyages.lose(leg.voyage, leg.units);
          } else {
            const row = this.d.voyages.get(leg.voyage);
            this.d.voyages.land(leg.voyage);
            this.d.register.release(row.carrier, this.hullsOf(row), row.hulls);
          }
          break;
        }
        case 'novateContract': {
          const row = this.d.contracts.get(op.contract);
          const carrying = this.d.contractCarrying(row, ins.period);
          // B4: what the leaving side was carrying goes off its book and onto the new one. The sign
          // is the side it was on: `a` holds +mark, `b` holds −mark (D1).
          const held = row.a === op.from ? carrying : negated(carrying, 'the side it was on');
          this.d.contracts.novate(op.contract, op.from, op.to);
          bumpIn(op.from, negated(held, 'off its book'));
          bumpIn(op.to, held);
          break;
        }
        case 'tearUpContract': {
          // D11: it ceases to exist on both books at once, and what leaves each book is what that
          // book was carrying it at (Law 19: the kernel reads it rather than being told).
          const row = this.d.contracts.get(op.contract);
          const carrying = this.d.contractCarrying(row, ins.period);
          this.d.contracts.close(op.contract, ins.period);
          bumpIn(row.a, negated(carrying, 'off its book'));
          bumpIn(row.b, carrying);
          break;
        }
        default:
          assertNever(op, 'Op');
      }
    });

    const effects: EquityEffect[] = [];
    for (const [party, delta] of equity) {
      if (delta === 0) continue;
      this.d.register.moveEquity({
        party,
        period: ins.period,
        cycle: ins.cycle,
        delta,
        // Reporting A2, G2: the cause is the instruction's OWN reason, which is the sentence its
        // writer wrote about why the units moved (C1.b). A report groups by it, so what a company
        // says it earned is what its own mechanisms said they were doing — never a chart of
        // accounts the report invented on top of them.
        cause: ins.reason,
        instruction: ins.id,
        through: zeroIfNone(gross.get(party)),
      });
      effects.push({ party, delta });
    }
    /**
     * Pair each disposal's proceeds with what the seller's own debit cost. The two halves are found
     * by the party they belong to — the seller is the `to` of the money and the `from` of the asset
     * — which is not an inference about shape: the market DECLARED the money a disposal, and this
     * only supplies the number the register alone holds.
     */
    for (const leg of ins.legs) {
      if (leg.kind !== 'money' || leg.receipt.of !== 'disposal') continue;
      const seller = leg.to.holder;
      for (const asset of ins.legs) {
        if (asset.kind !== 'asset' || asset.from !== seller) continue;
        const basis = sold.get(`${seller}/${asset.instrument}`);
        if (basis === undefined) continue;
        realised.push({
          party: seller,
          instrument: asset.instrument,
          proceeds: heldAsMoney(leg.amount, leg.ccy, 'what the seller was paid'),
          basis,
        });
      }
    }
    /**
     * C2, XI-5 (18.2): THE TWO ROWS ONE CLEARED FILL MADE, TOLD ABOUT EACH OTHER.
     *
     * A house is buyer to the seller and seller to the buyer, so one fill writes two rows in one
     * instruction — and everything that has to treat them as one trade had to find the sibling
     * afterwards by comparing fields that happened to agree, including `struckAt === struckAt`,
     * which compared two objects by identity. The instruction knows, so it says so here. Nothing
     * else may: the pairing is a fact about how the trade was written (Law 4, one writer).
     */
    if (written.length === 2) {
      const [first, second] = written;
      if (first !== undefined && second !== undefined) {
        const left = this.d.contracts.get(first);
        const right = this.d.contracts.get(second);
        if (left.house !== null && left.house === right.house) {
          this.d.contracts.pair(first, second);
        }
      }
    }
    return { deltas, equity: effects, contracts: written, realised };
  }

  /**
   * Money A2.b, Currency C5, D2: THE SINGLE-CURRENCY GUARD IS GONE, and what replaces it is the
   * rate.
   *
   * It read: "equity accounts are kept in the party's home money, so until the currency layer
   * revalues foreign positions an instruction may not touch an instrument in another money." That
   * was true and it was a PLACEHOLDER wearing a contract's clothes — it did not check an invariant,
   * it refused a world that had not been built. The world is built now: every position in a money
   * that is not its holder's own revalues to the rate in force at the close of each period
   * (`world/revalue.ts`), and what a balance sheet in two currencies comes to is one conversion at
   * one rate, in one place (`Valuation.inMoney`).
   *
   * Two currencies are STILL never added (A2.b). What is added is two amounts in ONE currency, one
   * of which was converted at the rate the same period settled at (C5) — which is what an accountant
   * means by it and what the clause is about.
   */
}
