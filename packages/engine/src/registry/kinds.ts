/**
 * Kinds and their profiles: the dispatch tables behind Law 15.
 *
 * @spec Bond N9.b Law 15 Law 9 XI-6 XI-15 Equity F4 Fund Shares A3 Money B3 Money B3.a Money B3.b Money B3.c Register E1 Register E2 Granularity
 *
 * A kind is registered at assembly by the module that owns it, with the whole of its behaviour in
 * one profile. The kernel never branches on a kind id; it asks the profile. Adding an instrument or
 * a party kind is one profile in one module; replacing a system replaces its profiles. Every profile
 * referenced by the state must exist at assembly (validated by the Registry), so an unknown kind is
 * a construction error and never a runtime surprise.
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import type {
  CurrencyCode,
  InstrumentId,
  InstrumentKindId,
  PartyId,
  PartyKindId,
  UnitId,
} from '../core/ids.js';
import type { Failed } from '../ledger/instruction.js';
import type { Instrument, Terms } from '../register/instruments.js';
import type { Namer } from './naming.js';

/** Named individually or represented as cells with a weight (XI-15). */
export type Representation = 'named' | 'cell';

/** Where an instrument's price comes from (XI-6). */
export type Pricing = 'money' | 'cleared' | 'carriedAtCost';

/**
 * How a holder carries it, which is a different question from where its price comes from (Goods
 * E1, E2). A bond is carried at the mark and revalues with it; inventory is carried at what it cost
 * and is only ever written DOWN to what a market says (E2.c) — and it still clears in a market,
 * because a price and a carrying value are two things.
 */
export type Carry = 'mark' | 'cost';

/**
 * The stated, consistently applied lot-flow assumption (Register D4; Goods E5 forbids LIFO).
 * Weighted average arrives with inventory (Goods E5); until then FIFO is the only flow.
 */
export type LotFlow = 'FIFO';

/** One dated payment per unit the terms promise (Bond N5, N10): the curve reads these. */
export interface CashFlow {
  readonly date: Civil;
  /** Per unit of the instrument, in its currency. */
  readonly perUnit: number;
}

/** What an instrument's terms say falls due in a period (Register E1, E2). */
export type DueAction =
  | { readonly kind: 'coupon'; readonly date: Civil; readonly amountPerUnit: number }
  | { readonly kind: 'maturity'; readonly date: Civil };

/**
 * Bond N12: an instrument's own definition of failure to perform, met by something that happened.
 *
 * The definition belongs to the instrument, not to the kernel: a sovereign bond has no covenants to
 * breach, so nothing but a missed payment can be one; a loan (worklist 6) has both. The kernel knows
 * only that a payment failed, and asks. And it must be OBSERVABLE BY A HOLDER — which is why what
 * comes back is the words the definition uses, journalled publicly for everyone to react to, and not
 * a flag whose meaning lives in the code that set it.
 */
export interface DefaultDefinition {
  readonly met: string;
}

/**
 * Bond N13, N13.a: what a holder is entitled to on failure, and where that claim stands against the
 * issuer's others. Both are stated even when the answers are "nothing seizable" and "all equal" —
 * an unstated ranking is the one that silently becomes a waterfall the first time somebody needs one.
 */
export interface Ranking {
  /** N13.a: lower is more senior; equal numbers are pari passu. */
  readonly seniority: number;
  /** N13: what is pledged against it, which is nothing for an unsecured claim. */
  readonly secured: readonly { readonly instrument: InstrumentId; readonly qty: number }[];
  /** N13: what the holder is entitled to, in words. Never empty. */
  readonly claim: string;
}

export interface InstrumentKindProfile {
  readonly id: InstrumentKindId;
  /** Where the instrument's price comes from (XI-6). */
  readonly pricing: Pricing;
  /** How a holder carries it: at the mark, or at what it cost (Goods E1). */
  readonly carry: Carry;
  /**
   * Whether a holding of it is a liability of the issuer. Money, bonds, loans, fund shares: yes.
   * A share is the residual claim, not a liability (Equity A1).
   */
  readonly liabilityOfIssuer: boolean;
  /** The unit its quantity is counted in (Register A1.c), given its currency. */
  readonly unit: (ccy: CurrencyCode) => UnitId;
  /** Validate kind-specific terms at registration; throw InvalidRegistry otherwise. */
  readonly validateTerms: (terms: Terms) => void;
  /**
   * Law 9: the name a market would use, built from the instrument's own terms and the name of
   * whoever promised it — which is nobody for a physical thing (Goods A1), so the profile is given
   * the issuer's name only when there is an issuer and says how it names itself without one.
   */
  readonly displayName: (i: Instrument, namer: Namer) => string;
  /** The dated actions the terms place in `period` (Money G3.a), in date order. */
  readonly due: (i: Instrument, period: Period, calendar: Calendar) => readonly DueAction[];
  /**
   * Bond N9.b: interest accrued per unit since the last payment date, on a date. Zero for an
   * instrument that pays no coupon: a bill accretes against its own cleared price (Sovereign F2).
   */
  readonly accrued: (i: Instrument, on: Civil, calendar: Calendar) => number;
  /**
   * Every payment the terms promise strictly after a date, in date order (Sovereign D2: what a
   * yield is derived FROM). An instrument that promises nothing dated returns none of them.
   */
  readonly cashFlows: (i: Instrument, after: Civil, calendar: Calendar) => readonly CashFlow[];
  /**
   * Goods E2: what a lot carried at cost must be written down to when it is worth less than it
   * cost. The kernel asks the profile per lot and books the delta; a POSITIVE delta is refused
   * unless the kind says it marks to market both ways (E2.c: nobody but a dealer writes inventory
   * up). A kind that never writes down does not answer.
   */
  readonly revalue?: (
    i: Instrument,
    lot: { readonly qty: number; readonly basisPerUnit: number },
    marked: number,
  ) => number;
  /**
   * Goods E2.c: whether this kind may be carried above cost. A dealer's book marks both ways; an
   * ordinary holder's inventory does not.
   */
  readonly fairValueThroughIncome?: boolean;
  /**
   * Goods A1, E4: whether units of this kind are physical things that are made and used up, rather
   * than claims that are issued and redeemed. Only such a kind admits a create or a destroy leg;
   * for everything else, a unit that appeared without an issuer would be an invented claim.
   */
  readonly physical?: boolean;
  /**
   * Bond N13, N13.a: what a holder is entitled to on failure and where it ranks. Required, because
   * "stated even when the answer is nothing" is the whole point of the clause.
   */
  readonly ranking: (i: Instrument) => Ranking;
  /**
   * Bond N12: whether a payment that failed is a DEFAULT under this instrument's own definition.
   * The kernel asks after a coupon or a maturity instruction fails, and journals what comes back.
   * A kind that cannot default — money, a tonne of grain — does not answer.
   */
  readonly defaultOn?: (i: Instrument, failed: Failed) => DefaultDefinition | undefined;
  /**
   * Corporate Credit G2: whether a default on one of this issuer's instruments makes this one due
   * as well. Stated, and false is an answer: a sovereign has no cross-default (Sovereign G3).
   */
  readonly accelerates?: boolean;
}

/**
 * What a money issuer does when a payment would take a holder's account below zero (Money B3).
 * Somebody lends it at a rate, or somebody refuses and the refusal is recorded (B3.c).
 */
export type OverdraftDecision =
  | { readonly allow: true; readonly recordedAs: 'reserveOverdraft' | 'facilityDraw' }
  | { readonly allow: false };

/**
 * WHO answers Money B3 for an issuer's customers.
 *
 * A kind whose answer is its own states the function — the central bank's is, because lending
 * reserves to the banks that settle in them is what a central bank is (Central Bank D1). A kind
 * whose answer is a CREDIT DECISION says so and answers nothing: B3.a is explicit that a customer
 * overdrawn is borrowing and that it is its bank's decision, and a decision that weighs the room a
 * bank's own capital supports is not something a kind profile could ever know how to take. The
 * module that owns lending registers it at assembly, and assembly refuses a world where a kind says
 * this and nobody answers — a refusal that exists because a defaulted-to "no" would look exactly
 * like a bank with a credit standard.
 */
export type OverdraftPolicy = ((ctx: OverdraftContext) => OverdraftDecision) | 'aCreditDecision';



export interface OverdraftContext {
  readonly holder: PartyId;
  readonly issuer: PartyId;
  readonly ccy: CurrencyCode;
  /** How far below zero the balance would go, per member of the holder. */
  readonly shortfall: number;
  /**
   * Whether the holder issues money itself. A central bank lends reserves to the banks that settle
   * in them (Central Bank D1, D3); everyone else banking with it — the treasury above all — has an
   * account, not a facility (Central Bank E2, Treasury D3).
   */
  readonly holderIssuesMoney: boolean;
}

export interface PartyKindProfile {
  readonly id: PartyKindId;
  readonly representation: Representation;
  /** Present when parties of this kind issue money (Money A1): a bank, a central bank. */
  readonly moneyIssuer: { readonly overdraft: OverdraftPolicy } | null;
}
