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
import { none, some, type Option } from '../core/option.js';

export const LOAN = instrumentKindId('loan');

/** A2: fixed at origination — principal, maturity, rate, currency; and A4: secured or not. */
export interface LoanTerms extends Terms {
  readonly kind: typeof LOAN;
  /**
   * A1, Banks Lending D4: WHO WROTE IT, which is not always who owns it now. A loan can be sold
   * (D4) and securitised (XI-11), and when it is, the row's units move in the register like any
   * other units — so who the money is owed TO is `creditorOf`, read off the register, and this is
   * the party that originated it and whose standard it was written to. Two different facts, two
   * different names; one of them was doing both jobs and would have become a lie the first time a
   * row left a bank's book (Law 4, Law 19).
   */
  readonly originator: PartyId;
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
  return 'originator' in t && 'borrower' in t && 'security' in t;
}

/**
 * Banks Lending A1, D4, XI-11: WHO THE MONEY IS OWED TO, right now. The register is the one writer
 * of who holds what (Law 4), so this asks it rather than reading a name off the terms — which is
 * what makes a sale a real transfer instead of a relabelling, and what makes the originator's
 * capital fall because the row genuinely left its book.
 *
 * NOBODY is a real answer and not a failure: a row that has been repaid to the last unit is owed to
 * no one, and so is one that has been written and not yet drawn. TWO is not: a loan is a bilateral
 * row and a second holder of it would be two lenders of one loan, so that throws where it happens.
 */
export function creditorOf(
  holdersOf: (i: InstrumentId) => readonly PartyId[],
  i: Instrument,
): Option<PartyId> {
  const held = holdersOf(i.id);
  if (held.length > 1) {
    throw new InvalidRegistry(
      'Banks Lending A1',
      `${i.id} is owed to ${held.length} parties; a loan is a row between two`,
    );
  }
  const first = held[0];
  return first === undefined ? none<PartyId>() : some(first);
}

export function loanTerms(i: Instrument): LoanTerms {
  if (!isLoan(i.terms)) {
    throw new InvalidRegistry('Banks Lending A1', `${i.id} is not a loan`);
  }
  return i.terms;
}
