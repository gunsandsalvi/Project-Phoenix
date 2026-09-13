/**
 * WHAT A DATED CLAIM IS, and what it pays when: the sovereign's bill and bond, and the schedule any
 * coupon bond of any issuer runs on.
 *
 * @spec Sovereign A1 Sovereign B1 Sovereign B3 Bond N1 Bond N9 Bond N9.b Law 4 Law 9 Law 15
 *
 * The same argument `registry/physical.ts` makes, for a claim instead of a thing. The treasury
 * ISSUES these and has to name their kind and build their terms; the curve module reads them; the
 * banks that hold them weigh them. A module importing the module that owns a kind, to learn how to
 * spell it, is the crossing `phoenix/no-cross-module-import` exists to forbid, and ARCHITECTURE
 * §4.9b already took this decision for party kind ids: an id is a NAME, and a module that does not
 * own a kind still has to say it.
 *
 * What stays with `sovereign-instruments` is the MECHANISM — the two kind profiles, what each pays
 * and when, and the module that registers them.
 */
import { instrumentKindId, unitId } from '../core/ids.js';
import { compareCivil, type Civil } from '../calendar/civil.js';
import { InvalidRegistry } from '../core/errors.js';
import type { DayCount } from '../calendar/daycount.js';
import type { Periodicity, Rate } from '../core/rate.js';
import type { Terms } from '../register/instruments.js';
import type { CashFlow, DueAction } from './kinds.js';
import { add, mul } from '../core/num.js';
import { yearFraction } from '../calendar/daycount.js';

export const SOVEREIGN_BOND = instrumentKindId('sovereign.bond');
export const SOVEREIGN_BILL = instrumentKindId('sovereign.bill');
export const PAR = unitId('par');

/**
 * Bond N5.a, N6, Law 4 (13f): WHAT A COUPON BOND PROMISES, whoever issued it. A sovereign's and a
 * firm's differ in who can fail, what ranks where and what may be breached — never in when a coupon
 * falls or how it accrues, so there is ONE statement of the schedule and every bond reads it.
 */
export interface CouponSchedule {
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

export interface SovereignBondTerms extends Terms, CouponSchedule {
  readonly kind: typeof SOVEREIGN_BOND;
}

export interface SovereignBillTerms extends Terms {
  readonly kind: typeof SOVEREIGN_BILL;
  readonly issueDate: Civil;
  readonly maturity: Civil;
}

export function isBond(t: Terms): t is SovereignBondTerms {
   
  return t.kind === SOVEREIGN_BOND;
}

export function isBill(t: Terms): t is SovereignBillTerms {
   
  return t.kind === SOVEREIGN_BILL;
}

export function validateDates(issue: Civil, maturity: Civil, what: string): void {
  if (compareCivil(maturity, issue) <= 0) {
    throw new InvalidRegistry('Bond N4', `${what}: maturity must be after the issue date`);
  }
}


/**
 * N6: the coupon dates of a line, in order. Generated from the ISSUE DATE so month-ends do not
 * drift (Money G3.a); a read of the terms, never stored.
 */
export function couponDatesOf(t: CouponSchedule, cal: ScheduleCalendar): readonly Civil[] {
  return cal.schedule(t.issueDate, t.maturity, t.couponPeriodicity);
}

/** What this file needs of a calendar, and nothing else: it is registry data, not the period loop. */
export interface ScheduleCalendar {
  schedule(from: Civil, to: Civil, per: Periodicity): readonly Civil[];
  periodOf(date: Civil): number;
}

/** N6, N10: what falls due in a period — each coupon by its own accrual, and par at maturity. */
export function dueOf(
  t: CouponSchedule,
  period: number,
  cal: ScheduleCalendar,
): readonly DueAction[] {
  const out: DueAction[] = [];
  let prev = t.issueDate;
  for (const date of couponDatesOf(t, cal)) {
    if (cal.periodOf(date) === period) {
      const amountPerUnit = mul(t.coupon.amount, yearFraction(t.dayCount, prev, date), 'coupon');
      // N6: a coupon of nothing is not a payment. A zero-coupon line is the ordinary shape of paper
      // an issuer brings when the market will pay above par for the principal alone.
      if (amountPerUnit > 0) out.push({ kind: 'coupon', date, amountPerUnit });
    }
    prev = date;
  }
  if (cal.periodOf(t.maturity) === period) out.push({ kind: 'maturity', date: t.maturity });
  return out.sort((a, b) => compareCivil(a.date, b.date));
}

/**
 * N9.b: what the buyer owes the seller on top of the clean price, from the last coupon date to the
 * settlement date, on the line's own day count.
 */
export function accruedOf(t: CouponSchedule, on: Civil, cal: ScheduleCalendar): number {
  let prev = t.issueDate;
  for (const date of couponDatesOf(t, cal)) {
    if (compareCivil(date, on) > 0) break;
    prev = date;
  }
  if (compareCivil(on, prev) <= 0) return 0;
  return mul(t.coupon.amount, yearFraction(t.dayCount, prev, on), 'accrued');
}

/**
 * N5.a, N10: every coupon left, and par at maturity. A yield is DERIVED from these against a price
 * (Law 3); nothing here reads one.
 */
export function cashFlowsOf(
  t: CouponSchedule,
  after: Civil,
  cal: ScheduleCalendar,
): readonly CashFlow[] {
  const out: CashFlow[] = [];
  let prev = t.issueDate;
  for (const date of couponDatesOf(t, cal)) {
    const coupon = mul(t.coupon.amount, yearFraction(t.dayCount, prev, date), 'coupon');
    const isMaturity = compareCivil(date, t.maturity) === 0;
    if (compareCivil(date, after) > 0) {
      out.push({ date, perUnit: isMaturity ? add(coupon, 1, 'final flow') : coupon });
    }
    prev = date;
  }
  return out;
}
