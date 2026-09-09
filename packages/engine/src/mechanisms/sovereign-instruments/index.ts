/**
 * Sovereign instruments: the bond and the bill, two instruments and not one with a flag
 * (Sovereign B1), answering the bond contract the sovereign's way.
 *
 * @spec Sovereign B1 Sovereign B2 Sovereign B4 Sovereign B5 Sovereign B6 Sovereign B7 Sovereign F1 Sovereign F3 Bond N4 Bond N5 Bond N5.a Bond N5.c Bond N6 Bond N10 Bond N11 Bond N12 Bond N13 Bond N13.a Bond N14 Money G3.a Money G3.c Law 9 Law 15
 *
 * A module: it registers kinds and profiles and touches no kernel store. The auction, the curve and
 * the treasury's programme are separate modules (worklist 3); this one is only the paper.
 */
import { compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { InvalidRegistry } from '../../core/errors.js';
import { instrumentKindId, unitId } from '../../core/ids.js';
import { mul } from '../../core/num.js';
import { percent } from '../../core/format.js';
import type { Periodicity, Rate } from '../../core/rate.js';
import type { Terms } from '../../register/instruments.js';
import type { DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
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
  liabilityOfIssuer: true,
  unit: () => PAR,
  validateTerms: (t) => {
    if (!isBond(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bond terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bond');
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product kind
    if (t.coupon.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a bond coupon is quoted per annum');
    }
  },
  displayName: (i, issuerName) => {
    if (!isBond(i.terms)) return `${issuerName} bond`;
    // Law 9: issuer + coupon + maturity.
    return `${issuerName} ${percent(i.terms.coupon.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  due: (i, period, cal) => {
    if (!isBond(i.terms)) return [];
    const t = i.terms;
    const out: DueAction[] = [];
    let prev = t.issueDate;
    for (const date of cal.schedule(t.issueDate, t.maturity, t.couponPeriodicity)) {
      if (cal.place(date) === period) {
        // N6: the coupon for the accrual period, by the instrument's own day count (G3.c).
        const frac = yearFraction(t.dayCount, prev, date);
        out.push({ kind: 'coupon', date, amountPerUnit: mul(t.coupon.amount, frac, 'coupon') });
      }
      prev = date;
    }
    if (cal.place(t.maturity) === period) out.push({ kind: 'maturity', date: t.maturity });
    return out.sort((a, b) => compareCivil(a.date, b.date));
  },
};

export const sovereignBill: InstrumentKindProfile = {
  id: SOVEREIGN_BILL,
  pricing: 'cleared',
  liabilityOfIssuer: true,
  unit: () => PAR,
  validateTerms: (t) => {
    if (!isBill(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bill terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bill');
  },
  displayName: (i, issuerName) =>
    isBill(i.terms) ? `${issuerName} bill ${formatCivil(i.terms.maturity)}` : `${issuerName} bill`,
  // N5.c: no coupon; the return is the discount to par, and the bill accretes against its own print.
  due: (i, period, cal) =>
    isBill(i.terms) && cal.place(i.terms.maturity) === period
      ? [{ kind: 'maturity', date: i.terms.maturity }]
      : [],
};

export const sovereignInstruments: SystemModule = {
  id: 'sovereign-instruments',
  spec: 'Sovereign B',
  requires: [],
  instrumentKinds: [sovereignBond, sovereignBill],
  partyKinds: [],
  units: [{ id: PAR, name: 'units of par', countable: false }],
  params: [],
  phases: [],
  participants: [],
  families: [],
};
