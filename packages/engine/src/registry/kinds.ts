/**
 * Kinds and their profiles: the dispatch tables behind Law 15.
 *
 * @spec Law 15 Law 9 XI-6 XI-15 Equity F4 Fund Shares A3 Money B3 Money B3.a Money B3.b Money B3.c Register E1 Register E2 Granularity
 *
 * A kind is registered at assembly by the module that owns it, with the whole of its behaviour in
 * one profile. The kernel never branches on a kind id; it asks the profile. Adding an instrument or
 * a party kind is one profile in one module; replacing a system replaces its profiles. Every profile
 * referenced by the state must exist at assembly (validated by the Registry), so an unknown kind is
 * a construction error and never a runtime surprise.
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import type { CurrencyCode, InstrumentKindId, PartyId, PartyKindId, UnitId } from '../core/ids.js';
import type { Instrument, Terms } from '../register/instruments.js';

/** Named individually or represented as cells with a weight (XI-15). */
export type Representation = 'named' | 'cell';

/** How an instrument gets its value (XI-6). */
export type Pricing = 'money' | 'cleared' | 'carriedAtCost';

/**
 * The stated, consistently applied lot-flow assumption (Register D4; Goods E5 forbids LIFO).
 * Weighted average arrives with inventory (Goods E5); until then FIFO is the only flow.
 */
export type LotFlow = 'FIFO';

/** What an instrument's terms say falls due in a period (Register E1, E2). */
export type DueAction =
  | { readonly kind: 'coupon'; readonly date: Civil; readonly amountPerUnit: number }
  | { readonly kind: 'maturity'; readonly date: Civil };

export interface InstrumentKindProfile {
  readonly id: InstrumentKindId;
  /** How the instrument gets its value (XI-6). */
  readonly pricing: Pricing;
  /**
   * Whether a holding of it is a liability of the issuer. Money, bonds, loans, fund shares: yes.
   * A share is the residual claim, not a liability (Equity A1).
   */
  readonly liabilityOfIssuer: boolean;
  /** The unit its quantity is counted in (Register A1.c), given its currency. */
  readonly unit: (ccy: CurrencyCode) => UnitId;
  /** Validate kind-specific terms at registration; throw InvalidRegistry otherwise. */
  readonly validateTerms: (terms: Terms) => void;
  /** Law 9: the name a market would use, built from the instrument's own terms. */
  readonly displayName: (i: Instrument, issuerName: string) => string;
  /** The dated actions the terms place in `period` (Money G3.a), in date order. */
  readonly due: (i: Instrument, period: Period, calendar: Calendar) => readonly DueAction[];
}

/**
 * What a money issuer does when a payment would take a holder's account below zero (Money B3).
 * Somebody lends it at a rate, or somebody refuses and the refusal is recorded (B3.c).
 */
export type OverdraftDecision =
  { readonly allow: true; readonly recordedAs: 'reserveOverdraft' } | { readonly allow: false };

export interface OverdraftContext {
  readonly holder: PartyId;
  readonly issuer: PartyId;
  readonly ccy: CurrencyCode;
  /** How far below zero the balance would go, per member of the holder. */
  readonly shortfall: number;
}

export interface PartyKindProfile {
  readonly id: PartyKindId;
  readonly representation: Representation;
  /** Present when parties of this kind issue money (Money A1): a bank, a central bank. */
  readonly moneyIssuer: { readonly overdraft: (ctx: OverdraftContext) => OverdraftDecision } | null;
}
