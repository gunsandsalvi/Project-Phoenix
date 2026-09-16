/**
 * What a LOAN is, and what a COMMITMENT TO LEND one is, as shapes the kernel names.
 *
 * @spec Banks Lending A1 Banks Lending A2 Banks Lending A4 Banks Lending A3.a Corporate Credit C9 Banks Lending F1 Housing C1 Private Equity B2 Private Equity B2.b Private Equity E1 Law 4 Law 15
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
import type { Qty } from '../core/tick.js';
import type { Cash, Ratio } from '../core/measure.js';
import {
  agreementKindId,
  instrumentId,
  instrumentKindId,
  paramId,
  type InstrumentId,
  type PartyId,
} from '../core/ids.js';
import type { Civil } from '../calendar/civil.js';
import type { Period } from '../calendar/calendar.js';
import type { AgreementTerms } from '../register/agreements.js';
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
  /**
   * A2: the rate struck at origination, per annum. It is what the negotiation produced (C2.a).
   *
   * A `Ratio`, so it can never be spent or posted as a level: what a rate earns over a span is the
   * rate SCALED by the span, and what that comes to per unit is par scaled by the result. A-44 and
   * A-58 are both this distinction read the wrong way round.
   */
  readonly rate: Ratio;
  readonly drawn: Civil;
  readonly maturity: Civil;
  readonly dayCount: DayCount;
  /**
   * Bond F3, Small-Business Pools B1, Housing C2 (11.2): bullet at maturity, or a schedule. A
   * term loan repays its principal straight-line over the periods it has left, so the borrower
   * pays interest AND principal every period; a line is drawn and repaid at the borrower's option
   * and falls due once. Which it is, is a term struck at origination like the rate.
   */
  readonly amortising: boolean;
  /** A4: what it is secured on, which is nothing for an unsecured loan — stated either way. */
  readonly security: readonly { readonly instrument: InstrumentId; readonly qty: Qty }[];
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

/* --- A COMMITMENT TO LEND, which is not a loan and is not nothing --------------------------- */

/**
 * §29 B2, B2.b, E1, Corporate Credit C9: A LENDER HAS AGREED TO LEND AND HAS NOT LENT.
 *
 * *"No buyout without a lender who agreed to lend"* (E1), and the whole of why that has to be an
 * agreement rather than a row is the CONDITION: the money must not exist unless the deal closes. A
 * loan written the period before a tender is money a failed tender has to hand back; a commitment
 * drawn inside the instruction that completes the tender is money that was never made.
 *
 * It is here rather than in the module that draws it because two of them name it: the lender
 * DECIDES it — the same credit decision it takes about a row, at the size and rate it would have
 * lent at — and the borrower DRAWS it, and a module may not import another to find out what it has
 * been promised (ARCHITECTURE 4.9b).
 *
 * It is the shape `short-term-debt`'s BACKSTOP also has, and the two are NOT merged here: a
 * backstop is a standing line with a commitment fee on undrawn headroom and this one lapses, so
 * merging them is a change to §18's economics and does not belong in §29's item (finding 21.71).
 */
export const FACILITY = agreementKindId('credit.facility');

export interface FacilityTerms extends AgreementTerms {
  readonly kind: typeof FACILITY;
  /** The most the lender committed. Its capital stands behind it before a penny is drawn. */
  readonly limit: Cash;
  /** What a drawing costs per annum — the rate that lender quoted this name when it committed. */
  readonly rate: Ratio;
  /**
   * B2.b: A COMMITMENT IS MADE FOR A DEAL AND LAPSES IF THE DEAL DOES NOT CLOSE. It is what makes
   * this an underwritten commitment rather than a standing line nobody pays for: the lender's
   * capital is behind it for the period the deal has to happen in, and after that it is not.
   */
  readonly until: Period;
  /**
   * A2: how long the drawing runs for, struck when the line was committed rather than when it is
   * drawn. Commitment papers say the term, and the party that DRAWS the line is not always the
   * party that decided it — so the term travels with the promise (Law 4).
   */
  readonly maturity: Civil;
}

/** Structural, like `isLoan`: a size, a price for drawing it, and a date it stops standing. */
export const isFacility = (t: AgreementTerms): t is FacilityTerms =>
  'limit' in t && 'rate' in t && 'until' in t && 'maturity' in t;

/**
 * F1.a: the row a facility is drawn into — one per (lender, borrower), named so a reader sees whose
 * it is and what it stands behind. One spelling, because the lender's headroom read and the
 * borrower's drawing are two modules asking about one row (Law 4).
 */
export function facilityLoanId(lender: PartyId, borrower: PartyId): InstrumentId {
  return instrumentId(`loan:${String(lender)}:${String(borrower)}:facility`);
}

/* --- How long a borrowing runs for, where more than one module has to say it ------------------ */

/**
 * Corporate Credit A2, Housing C2, Bond F3 (17b.8): THE TERMS A BORROWER ASKS FOR, in months.
 *
 * A tenor is a decision about a NEED and the need is the borrower's: a roof is paid for over
 * decades and a stock of grain over weeks. Every loan in this world ran for twelve months because
 * one parameter in the banks module said so (finding 21.60(a)), which made the term a fact about
 * the lender.
 *
 * The ids are here rather than in the module that declares each because more than one module has to
 * say the same one: a mortgage is asked for by `housing` for a landlord and by `households` for a
 * family, and the two must not be two different lengths of the same product (Law 4, ARCHITECTURE
 * 4.9b). The VALUES are declared by the module that owns the product.
 */
export const TERM_MONTHS = {
  /** Housing C2: what a family or a landlord borrows over to buy a roof. Declared by `housing`. */
  mortgage: paramId('housing.mortgageMonths'),
  /** Corporate Credit C9: a working-capital line, drawn and repaid at the borrower's option. */
  working: paramId('lending.loanMonths'),
} as const;
