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
import { asPerPiece, asRatio, minus, plus, scale, type PerPiece, type Ratio } from '../../core/measure.js';
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import { percent } from '../../core/format.js';
import { currencyUnit, instrumentId, type InstrumentId, type PartyId } from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { none, some } from '../../core/option.js';
import { LOAN, isLoan, rateOn, type LoanTerms } from '../../registry/credit.js';
import { issuerOf, type Instrument } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Namer } from '../../registry/naming.js';

/**
 * Law 4, ARCHITECTURE 4.9b: THE SHAPE IS THE KERNEL'S AND THE MECHANISM IS THIS MODULE'S. What a
 * loan IS — who lent, who owes, and what it is secured on — is named in `registry/credit.ts`
 * because more than one module has to read it: housing has to ask whether a row is secured on the
 * roof it is foreclosing, and a module may not import another module to find out. Everything else
 * about a loan is here, and this re-export is what keeps one spelling.
 */
export { LOAN, creditorOf, isLoan, loanTerms, rateOn, type LoanTerms } from '../../registry/credit.js';

/** The period after this one; the calendar counts, this only names the next index (Money G3.a). */
const next = (p: Period): Period => asPeriod(p + 1);

/** F1.a: one row per (lender, borrower, drawing), named so a reader can see whose it is. */
export function loanId(lender: PartyId, borrower: PartyId, n: number): InstrumentId {
  return instrumentId(`loan:${lender}:${borrower}:${n}`);
}

/**
 * Interest over a span, as a SHARE of the par outstanding (D3).
 *
 * A rate is per annum and a span is a fraction of one, so what the span earns is the rate scaled by
 * it — dimensionless either way, and never a level (A-44, A-58 are both a rate read as one).
 */
function interestTo(t: LoanTerms, from: Civil, to: Civil): Ratio {
  return scale(rateOn(t), asRatio(yearFraction(t.dayCount, from, to), 'the span of a year'), 'interest');
}

/**
 * PAR: one unit of a loan is one unit of the money it is written in.
 *
 * This is where a share of par becomes money per unit, and it is named rather than assumed because
 * a rate and a level are not the same thing. `E-9`: no crossing is needed here and the reason is
 * this kind's OWN declaration — `unit: (ccy) => currencyUnit(ccy)`, so a piece of a loan is a piece
 * of its money and `priceOf` would multiply by one. A bond is the other case: its unit is `PAR`,
 * declared separately, and its coupon and redemption cross at the door (`registry/claims.ts`).
 */
const PAR: PerPiece = asPerPiece(1, 'par: one unit of a loan is one piece of its money');

export const loanKind: InstrumentKindProfile = {
  id: LOAN,
  // D1: amortised cost. A1.a: no market, so it names none and nothing clears it.
  pricing: 'carriedAtCost',
  /**
   * Corporate Credit B4, Bond N5.b (17d.2): *"fixed or floating, and floating is the norm in the
   * loan market."* A loan's coupon is a margin over what money cost over the accrual just ended, so
   * it RESETS — and the kernel is the one writer of what a fixing does to a line (`ctx.fixCoupon`).
   * A row written where the benchmark had never fixed floats over nothing and never resets, which
   * its own terms say (`floatsOver`) rather than a flag anybody sets later.
   */
  floats: true,
  carry: 'cost',
  // The borrower owes it: it is the issuer's liability and the lender's asset.
  liabilityOfIssuer: true,
  // Register B3, XI-3: A BORROWER OWES THE PRINCIPAL. What its lender thinks the row is worth is
  // the lender's provision (D2) and moves the LENDER's book; the borrower still has to find the
  // whole of it on the day, and a borrower whose debt was written down as it deteriorated would be
  // growing more solvent the closer it came to failing.
  owes: 'face',
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t, issuer) => {
    if (!isLoan(t)) throw new InvalidRegistry('Banks Lending A1', 'not loan terms');
    if (compareCivil(t.drawn, t.maturity) >= 0) {
      throw new InvalidRegistry('Banks Lending A2', 'a loan matures after it is drawn');
    }
    // 21.70: who owes it is the ISSUER, so the two parties to the row are the originator and the
    // issuer. It used to compare the originator against a `borrower` on the terms, which was the
    // issuer written down a second time.
    if (!issuer.some) {
      throw new InvalidRegistry('Banks Lending A1', 'a loan is owed by somebody');
    }
    if (t.originator === issuer.value) {
      throw new InvalidRegistry('Banks Lending A1', 'a loan has two parties, and they differ');
    }
  },
  // Law 9: a market would name it by who owes it, at what, until when.
  displayName: (i: Instrument, namer: Namer) => {
    if (!isLoan(i.terms)) return String(i.id);
    // 21.70: the issuer IS the borrower, so a loan with no issuer's name has no name to fall back
    // on and says its own id — the fallback used to read the second copy of the same party.
    const who = namer.issuer.some ? namer.issuer.value : String(i.id);
    return `${who} ${percent(rateOn(i.terms))} ${formatCivil(i.terms.maturity)}`;
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
    const amountPerUnit = scale(PAR, interestTo(t, from, to), 'the interest one unit earned');
    if (amountPerUnit > 0) out.push({ kind: 'coupon', date: to, amountPerUnit });
    const matures = cal.periodOf(t.maturity);
    // F3 (11.2): an amortiser repays a slice of what is outstanding every period it has left —
    // one over the periods to and including the maturity, read off the calendar each time, so a
    // further drawing on the row is repaid over the same remaining term and nothing stores a
    // schedule (Law 19). The interest above is on what was outstanding through the period; the
    // slice goes after it on the same day.
    if (t.amortising && matures > period) {
      out.push({
        kind: 'amortisation',
        date: to,
        unitsPerUnit: asRatio(1 / (matures - period + 1), 'the slice of what is left'),
      });
    }
    // A bullet: the principal falls due once, on the day the terms say (A2); an amortiser's last
    // slice is whatever is left.
    if (matures === period) out.push({ kind: 'maturity', date: t.maturity });
    return out.sort((a, b) => compareCivil(a.date, b.date));
  },
  // Interest is settled every period it accrues, so nothing is ever outstanding between payments.
  accrued: () => 0,
  cashFlows: (i, after, cal) => {
    if (!isLoan(i.terms)) return [];
    const t = i.terms;
    const out: CashFlow[] = [];
    let from = compareCivil(t.drawn, after) > 0 ? t.drawn : after;
    const matures = cal.periodOf(t.maturity);
    // F3 (11.2): what is left of ONE unit after the slices an amortiser has repaid; a bullet keeps
    // the whole of it to the end.
    let left = PAR;
    for (let p = cal.periodOf(from); ; p = next(p)) {
      const end = cal.startOf(next(p));
      const to = compareCivil(t.maturity, end) < 0 ? t.maturity : end;
      if (compareCivil(from, to) >= 0) break;
      const coupon = scale(left, interestTo(t, from, to), 'what is left of a unit earned over the span');
      const last = compareCivil(to, t.maturity) === 0;
      const slice = t.amortising && !last
        ? scale(left, asRatio(1 / (matures - p + 1), 'the slice of what is left'), 'the principal repaid on the day')
        : asPerPiece(0, 'no principal until the end');
      // Item 16: what ONE unit pays on that day, which is money per piece — the par it redeems at
      // is one of them, and the coupon is the rest.
      out.push({
        date: to,
        perUnit: last ? plus(coupon, left, 'the last one returns what is left too') : plus(coupon, slice, 'interest and the slice'),
      });
      if (last) break;
      left = minus(left, slice, 'what is left after the slice');
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
  /**
   * E3, 21.59 (17.7): WHAT A RE-AGREEMENT OF A LOAN MAY CHANGE, which is time and price and
   * nothing else.
   *
   * A loan is the one claim in this world that two named parties can sit down and re-agree, because
   * it is the one with exactly two of them (A1). What they can agree is how long the borrower has
   * and what it pays for it — and, with them, whether the principal comes back in slices or at the
   * end, which is the same conversation about time.
   *
   * What they cannot agree here is everything that would make it a DIFFERENT claim. Who wrote it
   * and who owes it are the two parties themselves. What it is secured on is a lien in the register
   * and a pledge is an act with two sides, not a line of terms (A4, XI-5); a workout that takes
   * security takes it the same way the original drawing did. The day count is the convention every
   * accrual on the row has already been struck under, and the drawing date is when the money
   * actually moved — neither is negotiable after the fact (Law 8, Law 19). And the maturity moves
   * OUT only: a date brought forward is an acceleration, which has its own path and its own event.
   */
  reagree: (was, now) => {
    if (!isLoan(was) || !isLoan(now)) {
      return some('a loan is re-agreed as a loan');
    }
    // 21.70: who OWES it is the issuer and cannot be re-agreed here at all — the clause that
    // refused a change of borrower was a guard keeping two copies of one fact in step, and it goes
    // with the copy. The originator is a term and still may not change.
    if (was.originator !== now.originator) {
      return some('a re-agreement is between the two parties that are already on it');
    }
    if (was.dayCount !== now.dayCount) {
      return some('the convention every accrual so far was struck under does not change');
    }
    if (compareCivil(was.drawn, now.drawn) !== 0) {
      return some('the day the money moved is not negotiable');
    }
    if (was.security.length !== now.security.length) {
      return some('security is a lien in the register, pledged by an act with two sides');
    }
    if (compareCivil(now.maturity, was.maturity) < 0) {
      return some('a date brought forward is an acceleration, not an agreement');
    }
    return none<string>();
  },
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
