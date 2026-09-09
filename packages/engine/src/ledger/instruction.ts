/**
 * The wire: every move of any asset, money included, is a numbered instruction with two named sides
 * per leg (Money D1). An instruction's legs settle together or not at all (Register C3, XI-5).
 *
 * @spec Goods B2 Goods E4 Goods F1 Bond N9.b Money C1 Money C1.a Money C1.b Money C1.c Money D1 Money D1.a Money D2 Money G4 Register C1 Register C2 Register C2.a Register C3 Register C3.a XI-5 XI-15
 *
 * Denomination on a cell (XI-15): a leg side on a cell carries the PER-MEMBER amount and the weight
 * it was struck at; the total is perMember x weight. Settlement refuses a cell side without one.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type { CurrencyCode, InstructionId, InstrumentId, PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';

/** Money B1: an account is (holder, issuer, currency). The currency is on the amount. */
export interface AccountRef {
  readonly holder: PartyId;
  /** The bank or central bank whose liability the money is. holder === issuer means the issuer's own money creation or destruction. */
  readonly issuer: PartyId;
}

/** XI-15: how a cell side is denominated. */
export interface CellSide {
  readonly perMember: number;
  readonly weight: number;
}

export interface MoneyLeg {
  readonly kind: 'money';
  readonly from: AccountRef;
  readonly to: AccountRef;
  readonly ccy: CurrencyCode;
  /** Total amount that moves. */
  readonly amount: number;
  readonly fromCell: Option<CellSide>;
  readonly toCell: Option<CellSide>;
}

export interface AssetLeg {
  readonly kind: 'asset';
  readonly from: PartyId;
  readonly to: PartyId;
  readonly instrument: InstrumentId;
  /** Total units that move, in the instrument's unit. */
  readonly qty: number;
  /** C2.a: the print if it is a trade, per unit in the instrument's currency; none for a transfer at carrying value. */
  readonly pricePerUnit: Option<number>;
  /**
   * Bond N9.b: what accrued since the last coupon and travelled with the paper, per unit. It is
   * part of the money leg's amount and not of the lot's basis; recorded here so the ledger says
   * what was paid for and nobody has to re-derive it (Law 19).
   */
  readonly accruedPerUnit: Option<number>;
  readonly fromCell: Option<CellSide>;
  readonly toCell: Option<CellSide>;
}

/**
 * A physical thing coming into existence or leaving it (Goods B, E4, F1). It is not a flow between
 * two parties, so it has one side: nobody is on the other end of a harvest or of a batch that
 * spoiled. What keeps it honest is the units identity — produced plus opening equals consumed plus
 * closing plus perished — which the units family checks, and the rule that a `create` may only
 * appear in the same instruction as the `destroy` legs of what it was made from (F1).
 */
export interface CreateLeg {
  readonly kind: 'create';
  readonly party: PartyId;
  readonly instrument: InstrumentId;
  readonly qty: number;
  /** What the units cost to make, per unit: the basis the lot carries (Goods E1). */
  readonly costPerUnit: number;
  readonly toCell: Option<CellSide>;
}

export interface DestroyLeg {
  readonly kind: 'destroy';
  readonly party: PartyId;
  readonly instrument: InstrumentId;
  readonly qty: number;
  /** Why the units left: consumed into something else, perished, scrapped. */
  readonly why: 'consumed' | 'perished' | 'scrapped';
  readonly fromCell: Option<CellSide>;
}

export type Leg = MoneyLeg | AssetLeg | CreateLeg | DestroyLeg;

/** C1.b / Register C2: why the units moved. */
export type Cause =
  | 'trade'
  | 'production'
  | 'issuance'
  | 'coupon'
  | 'maturity'
  | 'corporateAction'
  | 'default'
  | 'transfer'
  | 'seed';

export interface InstructionDraft {
  readonly legs: readonly Leg[];
  readonly cause: Cause;
  /** C1.b: a human-readable reason, so a unit is traceable to why it moved. */
  readonly reason: string;
}

export interface Instruction extends InstructionDraft {
  readonly id: InstructionId;
  readonly period: Period;
  readonly cycle: Cycle;
}

/** The named effects settlement produced when it applied an instruction (Audit B5: named events). */
export interface EquityEffect {
  readonly party: PartyId;
  /** Per member for a cell, in the party's home currency. */
  readonly delta: number;
}

export interface Settled {
  readonly outcome: 'settled';
  readonly instruction: Instruction;
  /** Register deltas actually applied, per member for cells: replayable (D1.a). */
  readonly deltas: readonly RegisterDelta[];
  readonly equity: readonly EquityEffect[];
  /** Interbank reserve legs settlement generated (Money C2.a). */
  readonly reserveLegs: readonly ReserveLeg[];
}

export type FailReason =
  | {
      readonly kind: 'insufficientUnits';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly short: number;
    }
  | {
      readonly kind: 'overdraftRefused';
      readonly party: PartyId;
      readonly issuer: PartyId;
      readonly ccy: CurrencyCode;
      readonly short: number;
    };

/** Money E1 / Register C3.b: a fail is a real, recorded state; nothing half-settles. */
export interface Failed {
  readonly outcome: 'failed';
  readonly instruction: Instruction;
  readonly reason: FailReason;
}

export type SettlementRecord = Settled | Failed;

export interface RegisterDelta {
  readonly party: PartyId;
  readonly instrument: InstrumentId;
  /** Per member for a cell. */
  readonly qty: number;
  /** 'holding' moves a holding; 'issued' moves the issuer's issued amount (an issuance or redemption). */
  readonly target: 'holding' | 'issued';
}

export interface ReserveLeg {
  readonly bank: PartyId;
  readonly centralBank: PartyId;
  readonly ccy: CurrencyCode;
  readonly amount: number;
}
