/**
 * What a LOAN is, as a shape the kernel names.
 *
 * @spec Banks Lending A1 Banks Lending A2 Banks Lending A4 Banks Lending F1 Housing C1 Law 4 Law 15
 *
 * The SHAPE of a credit row — who lent, who owes, at what rate, until when, and what it is secured
 * on — is kernel data for the reason a good's terms are (ARCHITECTURE 4.9b): more than one module
 * has to name the same thing. The banks module writes loans and prices them; the housing module
 * has to be able to ask whether a row is secured on the roof it is about to foreclose, and a module
 * may not import another module to find out (`phoenix/no-cross-module-import`).
 *
 * What is here is the NAME and the READ. The mechanism — origination, the quote, the standard, the
 * provision — stays where it belongs, in `mechanisms/banks`, which re-exports these so a loan has
 * one definition and one spelling (Law 4).
 */
import { instrumentKindId, type InstrumentId, type PartyId } from '../core/ids.js';
import type { Civil } from '../calendar/civil.js';
import type { DayCount } from '../calendar/daycount.js';
import { InvalidRegistry } from '../core/errors.js';
import type { Instrument, Terms } from '../register/instruments.js';

export const LOAN = instrumentKindId('loan');

/** A2: fixed at origination — principal, maturity, rate, currency; and A4: secured or not. */
export interface LoanTerms extends Terms {
  readonly kind: typeof LOAN;
  /** A1: the bank of record. The borrower is the instrument's issuer (it owes the money). */
  readonly lender: PartyId;
  readonly borrower: PartyId;
  /** A2: the rate struck at origination, per annum. It is what the negotiation produced (C2.a). */
  readonly rate: number;
  readonly drawn: Civil;
  readonly maturity: Civil;
  readonly dayCount: DayCount;
  /** A4: what it is secured on, which is nothing for an unsecured loan — stated either way. */
  readonly security: readonly { readonly instrument: InstrumentId; readonly qty: number }[];
}

/**
 * Whether these terms are a loan's. Structural, not a kind comparison: what makes a loan a loan is
 * that it names a lender and a borrower and says what it is secured on (Law 15, A1, A4).
 */
export function isLoan(t: Terms): t is LoanTerms {
  return 'lender' in t && 'borrower' in t && 'security' in t;
}

export function loanTerms(i: Instrument): LoanTerms {
  if (!isLoan(i.terms)) {
    throw new InvalidRegistry('Banks Lending A1', `${i.id} is not a loan`);
  }
  return i.terms;
}
