/**
 * Sovereign instruments: the bond and the bill, two instruments and not one with a flag
 * (Sovereign B1), answering the bond contract the sovereign's way.
 *
 * @spec Bond N9.b Sovereign F2 Sovereign B1 Sovereign B2 Sovereign B4 Sovereign B5 Sovereign B6 Sovereign B7 Sovereign F1 Sovereign F3 Bond N4 Bond N5 Bond N5.a Bond N5.c Bond N6 Bond N10 Bond N11 Bond N12 Bond N13 Bond N13.a Bond N14 Money G3.a Money G3.c Law 9 Law 15
 *
 * A module: it registers kinds and profiles and touches no kernel store. The auction, the curve and
 * the treasury's programme are separate modules (worklist 3); this one is only the paper.
 */
import type { Calendar } from '../../calendar/calendar.js';
import { FACE_TICK } from '../../registry/grid.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { InvalidRegistry } from '../../core/errors.js';
import { instrumentKindId, unitId } from '../../core/ids.js';
import { add, mul } from '../../core/num.js';
import { percent } from '../../core/format.js';
import type { Periodicity, Rate } from '../../core/rate.js';
import { issuerOf, type Terms } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import { issuerName } from '../../registry/naming.js';
import type { SystemModule } from '../../world/module.js';

export const SOVEREIGN_BOND = instrumentKindId('sovereign.bond');
export const SOVEREIGN_BILL = instrumentKindId('sovereign.bill');
export const PAR = unitId('par');

export interface SovereignBondTerms extends Terms {
  readonly kind: typeof SOVEREIGN_BOND;
  /** N5.a: fixed, locked at issuance, quoted per annum. */
  readonly coupon: Rate;
  /** N6: how often it pays. */
  readonly couponPeriodicity: Periodicity;
  /** N6: how interest accrues between payments. */
  readonly dayCount: DayCount;
  readonly issueDate: Civil;
  /** N4: the date the principal is due. */
  readonly maturity: Civil;
}

export interface SovereignBillTerms extends Terms {
  readonly kind: typeof SOVEREIGN_BILL;
  readonly issueDate: Civil;
  readonly maturity: Civil;
}

function isBond(t: Terms): t is SovereignBondTerms {
  // eslint-disable-next-line phoenix/no-kind-branch -- a profile's own type guard, the edge of the dispatch table
  return t.kind === SOVEREIGN_BOND;
}

function isBill(t: Terms): t is SovereignBillTerms {
  // eslint-disable-next-line phoenix/no-kind-branch -- a profile's own type guard, the edge of the dispatch table
  return t.kind === SOVEREIGN_BILL;
}

function validateDates(issue: Civil, maturity: Civil, what: string): void {
  if (compareCivil(maturity, issue) <= 0) {
    throw new InvalidRegistry('Bond N4', `${what}: maturity must be after the issue date`);
  }
}

export const sovereignBond: InstrumentKindProfile = {
  id: SOVEREIGN_BOND,
  pricing: 'cleared',
  // Law 8: government paper is quoted as a fraction of its own face, and it moves in
  // ten-thousandths of one — a basis point of price, which is the grid a sovereign book
  // actually quotes on. A cent of face would be a whole percentage point of a bond.
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  unit: () => PAR,
  // Bond N12, N13, N13.a as §8's table answers them, and every answer is stated rather than
  // implied. There are no covenants to breach, so nothing but a missed payment can be a default.
  // There is no estate — nothing of a state is seizable — so the claim is a negotiated exchange and
  // the sanction is exclusion from the market (Sovereign G3, G5). And the ranking never varies:
  // pari passu, always, which is why every line of an issuer carries the same seniority number.
  ranking: () => ({
    seniority: 0,
    secured: [],
    claim: 'nothing seizable: a negotiated exchange, and exclusion from the market until there is one',
  }),
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  // Sovereign G3: a missed payment on one line does not make the others due.
  accelerates: false,
  validateTerms: (t) => {
    if (!isBond(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bond terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bond');
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product kind
    if (t.coupon.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a bond coupon is quoted per annum');
    }
  },
  displayName: (i, namer) => {
    const who = issuerName(namer, i.id);
    if (!isBond(i.terms)) return `${who} bond`;
    // Law 9: issuer + coupon + maturity.
    return `${who} ${percent(i.terms.coupon.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  due: (i, period, cal) => {
    if (!isBond(i.terms)) return [];
    const t = i.terms;
    const out: DueAction[] = [];
    let prev = t.issueDate;
    for (const date of couponDates(t, cal)) {
      if (cal.place(date) === period) {
        // N6: the coupon for the accrual period, by the instrument's own day count (G3.c).
        const frac = yearFraction(t.dayCount, prev, date);
        const amountPerUnit = mul(t.coupon.amount, frac, 'coupon');
        // N6: a coupon of nothing is not a payment, and a line that promises none has none falling
        // due. A zero-coupon line is the ordinary shape of paper an issuer brings when the market
        // will pay above par for the principal alone.
        if (amountPerUnit > 0) out.push({ kind: 'coupon', date, amountPerUnit });
      }
      prev = date;
    }
    if (cal.place(t.maturity) === period) out.push({ kind: 'maturity', date: t.maturity });
    return out.sort((a, b) => compareCivil(a.date, b.date));
  },
  // N9.b: what the buyer owes the seller on top of the clean price, from the last coupon date to
  // the settlement date, on the line's own day count (G3.c).
  accrued: (i, on, cal) => {
    if (!isBond(i.terms)) return 0;
    const t = i.terms;
    let prev = t.issueDate;
    for (const date of couponDates(t, cal)) {
      if (compareCivil(date, on) > 0) break;
      prev = date;
    }
    if (compareCivil(on, prev) <= 0) return 0;
    return mul(t.coupon.amount, yearFraction(t.dayCount, prev, on), 'accrued');
  },
  // Every coupon left, and par at maturity (N5.a, N10). A yield is derived from these against the
  // price (Sovereign D2); nothing here reads a price.
  cashFlows: (i, after, cal) => {
    if (!isBond(i.terms)) return [];
    const t = i.terms;
    const out: CashFlow[] = [];
    let prev = t.issueDate;
    for (const date of couponDates(t, cal)) {
      const coupon = mul(t.coupon.amount, yearFraction(t.dayCount, prev, date), 'coupon');
      const isMaturity = compareCivil(date, t.maturity) === 0;
      if (compareCivil(date, after) > 0) {
        out.push({ date, perUnit: isMaturity ? add(coupon, 1, 'final flow') : coupon });
      }
      prev = date;
    }
    return out;
  },
};

/**
 * The coupon dates of a line, in order (N6). The schedule is generated from the issue date so
 * month-ends do not drift (Money G3.a); it is a read of the terms and is never stored.
 */
function couponDates(t: SovereignBondTerms, cal: Calendar): readonly Civil[] {
  return cal.schedule(t.issueDate, t.maturity, t.couponPeriodicity);
}

export const sovereignBill: InstrumentKindProfile = {
  id: SOVEREIGN_BILL,
  pricing: 'cleared',
  // Law 8: government paper is quoted as a fraction of its own face, and it moves in
  // ten-thousandths of one — a basis point of price, which is the grid a sovereign book
  // actually quotes on. A cent of face would be a whole percentage point of a bond.
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  unit: () => PAR,
  // Bond N12, N13, N13.a as §8's table answers them, and every answer is stated rather than
  // implied. There are no covenants to breach, so nothing but a missed payment can be a default.
  // There is no estate — nothing of a state is seizable — so the claim is a negotiated exchange and
  // the sanction is exclusion from the market (Sovereign G3, G5). And the ranking never varies:
  // pari passu, always, which is why every line of an issuer carries the same seniority number.
  ranking: () => ({
    seniority: 0,
    secured: [],
    claim: 'nothing seizable: a negotiated exchange, and exclusion from the market until there is one',
  }),
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  // Sovereign G3: a missed payment on one line does not make the others due.
  accelerates: false,
  validateTerms: (t) => {
    if (!isBill(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bill terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bill');
  },
  displayName: (i, namer) =>
    isBill(i.terms)
      ? `${issuerName(namer, i.id)} bill ${formatCivil(i.terms.maturity)}`
      : `${issuerName(namer, i.id)} bill`,
  // N5.c: no coupon; the return is the discount to par, and the bill accretes against its own print.
  due: (i, period, cal) =>
    isBill(i.terms) && cal.place(i.terms.maturity) === period
      ? [{ kind: 'maturity', date: i.terms.maturity }]
      : [],
  // F2: a bill accretes against its own cleared price; nothing accrues on the paper itself, so
  // nothing travels with a trade beyond the price.
  accrued: () => 0,
  // N5.c: one payment, par at maturity. The discount to it is the whole return.
  cashFlows: (i, after) =>
    isBill(i.terms) && compareCivil(i.terms.maturity, after) > 0
      ? [{ date: i.terms.maturity, perUnit: 1 }]
      : [],
};

export const sovereignInstruments: SystemModule = {
  id: 'sovereign-instruments',
  spec: 'Sovereign B',
  requires: [],
  instrumentKinds: [sovereignBond, sovereignBill],
  partyKinds: [],
  curveFamilies: [],
  // Bond N2, Law 8: par is money, so its smallest piece is money's — a line is issued, traded and
  // redeemed in the same grid the coupons on it are paid in.
  units: [{ id: PAR, name: 'units of par', perUnit: MONEY_PIECES }],
  params: [],
  phases: [],
  participants: [],
  families: [],
};
