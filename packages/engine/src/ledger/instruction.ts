/**
 * The wire: every move of any asset, money included, is a numbered instruction with two named sides
 * per leg (Money D1). An instruction's legs settle together or not at all (Register C3, XI-5).
 *
 * @spec Goods B2 Goods E4 Commodities Spot F1 Bond N9.b Money C1 Money C1.a Money C1.b Money C1.c Money D1 Money D1.a Money D2 Money G4 Register C1 Register C2 Register C2.a Register C3 Register C3.a XI-5 XI-15
 *
 * Denomination on a cell (XI-15): a leg side on a cell carries the PER-MEMBER amount and the weight
 * it was struck at; the total is perMember x weight. Settlement refuses a cell side without one.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type {
  ContractId,
  CurrencyCode,
  DerivativeKindId,
  InstructionId,
  InstrumentId,
  LienId,
  PartyId,
} from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { ContractTerms } from '../registry/derivatives.js';

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
 * A physical thing coming into existence or leaving it (Goods B, E4). It is not a flow between
 * two parties, so it has one side: nobody is on the other end of a harvest or of a batch that
 * spoiled. What keeps it honest is the units identity — produced plus opening equals consumed plus
 * closing plus perished — which the units family checks, and the rule that a `create` may only
 * appear in the same instruction as the `destroy` legs of what it was made from (Commodities
 * Spot F1: units cannot be conjured).
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

/**
 * XI-8, Firm Birth D5, Banks Capital D6: an issuer died and its paper did not die with it — whoever
 * succeeded it OWES the line from now on. Nothing moves in the register: the same holders hold the
 * same units of the same instrument, on the same terms. What moves is the obligation, and it moves
 * between two named balance sheets — which is why this is a leg and not a door. The liability
 * leaves one book and lands on the other in the same numbered instruction (Law 5), at what the
 * holders carry it at, because a liability is the same number read from the other side (Register B3).
 */
export interface AssumeLeg {
  readonly kind: 'assume';
  /** Who has owed it until now. */
  readonly from: PartyId;
  /** Who owes it from now on. */
  readonly to: PartyId;
  readonly instrument: InstrumentId;
}

/**
 * Money Market B3.c, Register D5: units BOUND to a named beneficiary, and freed again.
 *
 * Nothing changes hands: the pledgor still holds the paper, still carries it, still collects what
 * it pays. What changes is that the units are no longer free, so they can neither be sold nor
 * pledged a second time — which is the whole of B3.c, and the reason a solvent bank can run out of
 * the ability to borrow. It is a leg rather than a door because it names two parties and belongs to
 * the same numbered instruction as the money it secures: collateral that could be bound in one pass
 * and lent against in another is collateral that was briefly nobody's.
 *
 * `secures` is Register D5.b's traceable chain, in the words of whoever bound it — the row this
 * collateral stands behind — and it is how the release finds the lien it is freeing.
 */
export interface PledgeLeg {
  readonly kind: 'pledge';
  readonly pledgor: PartyId;
  readonly beneficiary: PartyId;
  readonly instrument: InstrumentId;
  /** Total units bound, in the instrument's unit. */
  readonly qty: number;
  readonly secures: string;
  readonly pledgorCell: Option<CellSide>;
}

/** The other half of a pledge: the named lien ends and the units are free again (Register D5). */
export interface ReleaseLeg {
  readonly kind: 'release';
  readonly pledgor: PartyId;
  readonly beneficiary: PartyId;
  readonly instrument: InstrumentId;
  readonly lien: LienId;
}

/**
 * Derivative D1, D11, X1, Derivative Layer B2: A POSITION OPENING OR CLOSING ON TWO BOOKS AT ONCE.
 *
 * A contract is not a holding, so nothing about it is an asset leg — and it is not nothing either:
 * the moment two parties agree terms, one of them has an asset and the other a liability of the
 * same size (D1), and on termination it ceases to exist on both books at once (D11). Both are
 * changes of balance sheet, and a change of balance sheet that does not go over the wire is exactly
 * what Money D1 exists to prevent. So it is a leg, in the same numbered instruction as the premium
 * or the close-out payment it comes with (Law 5), and settlement is the one writer of the contract
 * store as it is of the register.
 *
 * `value` is what the contract is worth TO `a` at the moment it is written — zero for a contract
 * struck at par (D7.b), the premium for one bought outright. It is the position's BASIS in the
 * sense Register D4 means: what it cost, which is what the equity account has recognised until a
 * mark moves it. On a CLOSE nothing states it: what leaves the two books is what they were carrying
 * it at, which the kernel reads for itself (Law 19).
 */
export interface OpenContractLeg {
  readonly kind: 'contract';
  readonly act: 'open';
  readonly a: PartyId;
  readonly b: PartyId;
  readonly derivative: DerivativeKindId;
  readonly terms: ContractTerms;
  readonly ccy: CurrencyCode;
  readonly notional: number;
  /** D7: the rate, spread or strike the two sides cleared at. */
  readonly struckAt: number;
  /**
   * Clearing E1, Expectations A2: WHAT THE LEVEL IS A LEVEL OF — the book's own line, which is the
   * subject its print is written against. A party that struck a contract saw this price, and that
   * is one more thing it has observed about this book.
   */
  readonly book: InstrumentId;
  /** What it is worth to `a` at inception, in `ccy`. */
  readonly value: number;
  /** C2: the house both sides face when it is cleared; null bilaterally. */
  readonly house: PartyId | null;
}

export interface CloseContractLeg {
  readonly kind: 'contract';
  readonly act: 'close';
  readonly contract: ContractId;
  readonly why: string;
}

/**
 * Derivative Layer B4: a position moves to a new counterparty, with the old one's consent, and it
 * is a real change of who faces whom. The consent is a decision the leaving party's own module
 * takes before the leg is drafted; what happens here is that the obligation leaves one balance
 * sheet at what it was carried at and lands on another — which is the same shape as an `assume`,
 * and it is a leg for the same reason (Law 5: both ends in one numbered instruction).
 */
export interface NovateContractLeg {
  readonly kind: 'contract';
  readonly act: 'novate';
  readonly contract: ContractId;
  readonly from: PartyId;
  readonly to: PartyId;
}

export type ContractLeg = OpenContractLeg | CloseContractLeg | NovateContractLeg;

export type Leg =
  | MoneyLeg
  | AssetLeg
  | CreateLeg
  | DestroyLeg
  | AssumeLeg
  | PledgeLeg
  | ReleaseLeg
  | ContractLeg;

/** Which side of the wire a leg is, for readers that must tell them apart (Law 15's dispatch). */
export const isMoneyLeg = (leg: Leg): leg is MoneyLeg => leg.kind === 'money';
export const isAssetLeg = (leg: Leg): leg is AssetLeg => leg.kind === 'asset';
export const isCreateLeg = (leg: Leg): leg is CreateLeg => leg.kind === 'create';
export const isDestroyLeg = (leg: Leg): leg is DestroyLeg => leg.kind === 'destroy';
export const isAssumeLeg = (leg: Leg): leg is AssumeLeg => leg.kind === 'assume';
export const isPledgeLeg = (leg: Leg): leg is PledgeLeg => leg.kind === 'pledge';
export const isReleaseLeg = (leg: Leg): leg is ReleaseLeg => leg.kind === 'release';
export const isContractLeg = (leg: Leg): leg is ContractLeg => leg.kind === 'contract';

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
  /**
   * Derivative D1, Law 19: the contract rows this instruction opened, in leg order. A module that
   * drafted a trade needs to know which row it now has a side of, and reading it back off the
   * store by guessing at the identity would be re-deriving what settlement already knows.
   */
  readonly contracts: readonly ContractId[];
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
    }
  /**
   * Money Market B3.c, C4.b: the pledgor does not have the free units to bind. A bank out of
   * unencumbered eligible paper cannot borrow against it, and that is an outcome of the borrowing,
   * not a violation of anything: the instruction fails and the row is never written.
   */
  | {
      readonly kind: 'insufficientCollateral';
      readonly party: PartyId;
      readonly instrument: InstrumentId;
      readonly short: number;
    };

/** Money E1 / Register C3.b: a fail is a real, recorded state; nothing half-settles. */
export interface Failed {
  readonly outcome: 'failed';
  readonly instruction: Instruction;
  readonly reason: FailReason;
}

/** Money E1.b: a payment that did not arrive, from the side that was owed it. */
export interface Unpaid {
  readonly payer: PartyId;
  readonly payee: PartyId;
  readonly amount: number;
  readonly ccy: CurrencyCode;
  readonly reason: string;
}

/**
 * Money E1.b: what a failed instruction did not pay, and to whom — the payee's receivable that did
 * not arrive, read off the instruction that failed rather than stored beside it (Law 19: the legs
 * are the source, and a second copy of them would be a second thing to keep true).
 */
export function unpaid(f: Failed): readonly Unpaid[] {
  return f.instruction.legs.filter(isMoneyLeg).map((l) => ({
    payer: l.from.holder,
    payee: l.to.holder,
    amount: l.amount,
    ccy: l.ccy,
    reason: f.instruction.reason,
  }));
}

export type SettlementRecord = Settled | Failed;

export interface RegisterDelta {
  readonly party: PartyId;
  readonly instrument: InstrumentId;
  /** Per member for a cell. */
  readonly qty: number;
  /**
   * XI-15: the multiplicity this delta was struck at, one for a named party. A cell's weight can
   * change later in the same period — it splits when part of it takes a job — so a reader that
   * multiplied by today's weight would be reconstructing a different instruction from the one that
   * settled (Law 19: read what was recorded).
   */
  readonly weight: number;
  /** 'holding' moves a holding; 'issued' moves the issuer's issued amount (an issuance or redemption). */
  readonly target: 'holding' | 'issued';
}

export interface ReserveLeg {
  readonly bank: PartyId;
  readonly centralBank: PartyId;
  readonly ccy: CurrencyCode;
  readonly amount: number;
}

/**
 * Every name an instruction touches: the parties on both sides of every leg and the instruments
 * that moved. It is what an event about the instruction is filed under, and what tells a party
 * whether the instruction was its own (Observer A4).
 */
export function subjectsOf(ins: Instruction): string[] {
  const s = new Set<string>();
  for (const leg of ins.legs) {
    if (isMoneyLeg(leg)) {
      s.add(leg.from.holder);
      s.add(leg.to.holder);
    } else if (isAssetLeg(leg) || isAssumeLeg(leg)) {
      s.add(leg.from);
      s.add(leg.to);
      s.add(leg.instrument);
    } else if (isPledgeLeg(leg) || isReleaseLeg(leg)) {
      s.add(leg.pledgor);
      s.add(leg.beneficiary);
      s.add(leg.instrument);
    } else if (isContractLeg(leg)) {
      if (leg.act === 'open') {
        s.add(leg.a);
        s.add(leg.b);
      } else {
        s.add(leg.contract);
        if (leg.act === 'novate') {
          s.add(leg.from);
          s.add(leg.to);
        }
      }
    } else {
      s.add(leg.party);
      s.add(leg.instrument);
    }
  }
  return [...s];
}
