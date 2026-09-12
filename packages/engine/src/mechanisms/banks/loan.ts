/**
 * A loan: a bilateral contract between a named bank and a named borrower, carried at what it cost.
 *
 * @spec Banks Lending A1 Banks Lending A1.a Banks Lending A1.b Banks Lending A2 Banks Lending A4 Banks Lending D1 Banks Lending D2 Banks Lending D3 Banks Lending E1 Banks Lending E2 Banks Lending F1 Banks Lending F1.a Bond N12 Bond N13 Bond N13.a Law 9 Law 15
 *
 * A1.a: it is NOT a security. It has no market, so it has no market price, and D1 says it is held
 * at amortised cost rather than marked to a market that does not exist. A1.b is that difference,
 * and it is why the kind carries at cost and answers `revalue`: a provision is a write-down of the
 * lot to what its holder expects to recover (D2), which is the same door inventory uses (Goods E2)
 * and is not a reserve sitting beside the loan absorbing things (D2.b).
 *
 * F1, F1.a: every loan is a ROW — an instrument in the same register as anything else, with a
 * lender of record, a borrower and its own terms. There is no book number anywhere; a bank's book
 * is the sum of the rows it holds, and that is a read.
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  currencyUnit,
  instrumentId,
  instrumentKindId,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { mul } from '../../core/num.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Namer } from '../../registry/naming.js';

export const LOAN = instrumentKindId('loan');

/** The period after this one; the calendar counts, this only names the next index (Money G3.a). */
const next = (p: Period): Period => asPeriod(p + 1);

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

/** F1.a: one row per (lender, borrower, drawing), named so a reader can see whose it is. */
export function loanId(lender: PartyId, borrower: PartyId, n: number): InstrumentId {
  return instrumentId(`loan:${lender}:${borrower}:${n}`);
}

/** Interest for the period this loan's terms place `on`, per unit of par outstanding (D3). */
function interestTo(t: LoanTerms, from: Civil, to: Civil): number {
  return mul(t.rate, yearFraction(t.dayCount, from, to), 'interest');
}

export const loanKind: InstrumentKindProfile = {
  id: LOAN,
  // D1: amortised cost. A1.a: no market, so it names none and nothing clears it.
  pricing: 'carriedAtCost',
  carry: 'cost',
  // The borrower owes it: it is the issuer's liability and the lender's asset.
  liabilityOfIssuer: true,
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (!isLoan(t)) throw new InvalidRegistry('Banks Lending A1', 'not loan terms');
    if (compareCivil(t.drawn, t.maturity) >= 0) {
      throw new InvalidRegistry('Banks Lending A2', 'a loan matures after it is drawn');
    }
    if (t.lender === t.borrower) {
      throw new InvalidRegistry('Banks Lending A1', 'a loan has two parties, and they differ');
    }
  },
  // Law 9: a market would name it by who owes it, at what, until when.
  displayName: (i: Instrument, namer: Namer) => {
    if (!isLoan(i.terms)) return String(i.id);
    const who = namer.issuer.some ? namer.issuer.value : String(i.terms.borrower);
    return `${who} ${percent(i.terms.rate)} ${formatCivil(i.terms.maturity)}`;
  },
  /**
   * D3: interest accrues and is received. It falls due every period the calendar places between the
   * drawing and the maturity, for exactly the days that period covers — so a loan drawn mid-period
   * pays for the days it was outstanding and not a period's worth (Law 8).
   */
  due: (i, period, cal) => {
    if (!isLoan(i.terms)) return [];
    const t = i.terms;
    const start = cal.startOf(period);
    const end = cal.startOf(next(period));
    const from = compareCivil(t.drawn, start) > 0 ? t.drawn : start;
    const to = compareCivil(t.maturity, end) < 0 ? t.maturity : end;
    if (compareCivil(from, to) >= 0) return [];
    const out: DueAction[] = [];
    const amountPerUnit = interestTo(t, from, to);
    if (amountPerUnit > 0) out.push({ kind: 'coupon', date: to, amountPerUnit });
    // A bullet: the principal falls due once, on the day the terms say (A2).
    if (cal.periodOf(t.maturity) === period) out.push({ kind: 'maturity', date: t.maturity });
    return out.sort((a, b) => compareCivil(a.date, b.date));
  },
  // Interest is settled every period it accrues, so nothing is ever outstanding between payments.
  accrued: () => 0,
  cashFlows: (i, after, cal) => {
    if (!isLoan(i.terms)) return [];
    const t = i.terms;
    const out: CashFlow[] = [];
    let from = compareCivil(t.drawn, after) > 0 ? t.drawn : after;
    for (let p = cal.periodOf(from); ; p = next(p)) {
      const end = cal.startOf(next(p));
      const to = compareCivil(t.maturity, end) < 0 ? t.maturity : end;
      if (compareCivil(from, to) >= 0) break;
      const coupon = interestTo(t, from, to);
      const last = compareCivil(to, t.maturity) === 0;
      out.push({ date: to, perUnit: last ? coupon + 1 : coupon });
      if (last) break;
      from = to;
    }
    return out;
  },
  /**
   * D2: the provision. The lot is written down to what its holder expects to recover, and the same
   * movement lands in the equity account — booked and visible, never a reserve quietly absorbing
   * (D2.b). The kernel asks per lot with what the holder says a unit is worth now.
   */
  carriedAt: (_i, _lot, recovery) => recovery,
  /**
   * D2.a: the provision moves when the assessment moves, and it moves BOTH WAYS — a bank that has
   * stopped expecting to lose on a borrower takes the charge back. Goods E2.c forbids writing
   * INVENTORY above cost because nobody but a dealer marks a thing it made up; a claim is not
   * inventory, and its carrying value is a belief about a payment rather than a price of a thing.
   * It can never exceed par all the same, because what the holder expects to recover cannot.
   */
  fairValueThroughIncome: true,
  // E1: a missed payment is a default. A covenant breach is the other half and needs covenants a
  // borrower's own accounts can be tested against (A5, worklist 13f).
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment on the loan fell due and the borrower did not make it' }
      : undefined,
  // Banks Lending states no cross-default; corporate paper does, and it arrives at 13f.
  accelerates: false,
  // N13, N13.a: what the lender is entitled to, and where it stands.
  ranking: (i) => {
    const t = isLoan(i.terms) ? i.terms : undefined;
    const secured = t?.security ?? [];
    return {
      // A4: a secured lender is ahead of an unsecured one, on the thing it is secured on.
      seniority: secured.length > 0 ? 0 : 1,
      secured,
      claim:
        secured.length > 0
          ? 'what the security fetches, and an unsecured claim for the rest'
          : 'an unsecured claim on whatever the estate realises',
    };
  },
};
