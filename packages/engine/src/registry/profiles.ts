/**
 * Profiles: the behaviour that varies by kind, behind dispatch tables (Law 15).
 *
 * @spec Law 15 Law 9 XI-6 Equity F4 Fund Shares A3 Money B3 Money B3.a Money B3.b Money B3.c
 *
 * Every table is a Record over a closed union, so the compiler refuses a missing profile.
 */
import { formatCivil } from '../calendar/civil.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import type { Instrument } from '../register/instruments.js';
import type { InstrumentKind, PartyKind, Pricing } from './kinds.js';

export interface InstrumentProfile {
  /** How the instrument gets its value (XI-6). */
  readonly pricing: Pricing;
  /**
   * Whether a holding of it is a liability of the issuer. Money, bonds, loans, fund shares: yes.
   * A share is the residual claim, not a liability (Equity A1).
   */
  readonly liabilityOfIssuer: boolean;
  /** Law 9: the name a market would use, built from the instrument's own terms. */
  readonly displayName: (i: Instrument, issuerName: string) => string;
}

export const INSTRUMENT_PROFILES: Readonly<Record<InstrumentKind, InstrumentProfile>> =
  Object.freeze({
    money: {
      pricing: 'money',
      liabilityOfIssuer: true,
      displayName: (i, issuerName) => `${issuerName} money ${i.ccy}`,
    },
    'sovereign.bond': {
      pricing: 'cleared',
      liabilityOfIssuer: true,
      displayName: (i, issuerName) => {
        if (i.terms.kind !== 'sovereign.bond') return `${issuerName} bond`;
        // Law 9: issuer + coupon + maturity.
        const pct = (i.terms.coupon.amount * 100).toFixed(3).replace(/0+$/, '').replace(/\.$/, '');
        return `${issuerName} ${pct}% ${formatCivil(i.terms.maturity)}`;
      },
    },
    'sovereign.bill': {
      pricing: 'cleared',
      liabilityOfIssuer: true,
      displayName: (i, issuerName) => {
        if (i.terms.kind !== 'sovereign.bill') return `${issuerName} bill`;
        return `${issuerName} bill ${formatCivil(i.terms.maturity)}`;
      },
    },
  });

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

export interface MoneyIssuerProfile {
  readonly issuesMoney: boolean;
  readonly overdraft: (ctx: OverdraftContext) => OverdraftDecision;
}

const notAnIssuer: MoneyIssuerProfile = {
  issuesMoney: false,
  overdraft: (): OverdraftDecision => ({ allow: false }),
};

export const MONEY_ISSUER_PROFILES: Readonly<Record<PartyKind, MoneyIssuerProfile>> = Object.freeze(
  {
    centralBank: {
      issuesMoney: true,
      // B3.b: a bank overdrawn at the central bank is borrowing from it and the corridor prices it.
      // Until the corridor exists (worklist: money market, Central Bank D3.b) the overdraft is allowed
      // and recorded as a reserve overdraft; the Money audit family reports every one as unpriced.
      overdraft: (): OverdraftDecision => ({ allow: true, recordedAs: 'reserveOverdraft' }),
    },
    bank: {
      issuesMoney: true,
      // B3.a: a customer overdrawn is a credit decision by its bank. The credit decision is Banks
      // Lending (worklist: loans are rows); until then the bank refuses and the refusal is recorded.
      overdraft: (): OverdraftDecision => ({ allow: false }),
    },
    treasury: notAnIssuer,
    firm: notAnIssuer,
    household: notAnIssuer,
    smallBusiness: notAnIssuer,
  },
);
