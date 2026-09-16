/**
 * The leveraged loan: a corporate claim whose coupon is a MARGIN OVER A TRANSACTED RATE.
 *
 * @spec Corporate Credit B4 Corporate Credit A2 Corporate Credit A2.a Corporate Credit A2.c Bond N5 Bond N5.b Bond N6 Indices A1 Indices D3 Law 3 Law 4 Law 8 Law 19
 *
 * B4, N5.b: fixed or floating, *"and floating is the norm in the loan market"*. A fixed line locks
 * a rate at issuance and carries it to maturity; this one promises a MARGIN over a named reference,
 * and what it pays for an accrual period is settled when that reference FIXES.
 *
 * THE REFERENCE IS TRANSACTED, never posted (Indices D3, Appendix B). It is the overnight book's own
 * cleared fixing in this money, published by the index system as `index.benchmark` — so a money
 * whose overnight book has not traded has no benchmark, and a line cannot be brought against one
 * that does not exist (Derivative E3's rule, and the same reason).
 *
 * WHAT FIXING DOES is written by the kernel (`ctx.fixCoupon`): the terms carry the rate in force for
 * the period the line has just entered, so what falls due, what has accrued and what a holder thinks
 * it is worth are all read off ONE number (Law 4). What it paid before is settled and in the ledger,
 * and nothing re-derives it.
 *
 * WHAT A HOLDER PROJECTS, and it is stated rather than hidden: the schedule beyond the current
 * accrual period is every remaining coupon AT THE RATE IN FORCE — what the line pays if the rate
 * never moves again. That is not a forecast of the benchmark and nothing here has one; a party that
 * wants a view of the rate forms its own (§46) and it moves the price it will pay, not the schedule.
 */
import { formatCivil, type Civil } from '../../calendar/civil.js';
import { addMonths } from '../../calendar/civil.js';
import { ANNUAL, rate, type Rate } from '../../core/rate.js';
import { asRatio, type Ratio, plus } from '../../core/measure.js';
import { instrumentId, type CurrencyCode, type InstrumentId, type PartyId } from '../../core/ids.js';
import { percent } from '../../core/format.js';
import { InvalidRegistry } from '../../core/errors.js';
import { compareCivil } from '../../calendar/civil.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import { accruedOf, cashFlowsOf, type CouponSchedule, dueOf, ontoTheGrid } from '../../registry/claims.js';
import { FACE_TICK } from '../../registry/grid.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { issuerName } from '../../registry/naming.js';
import { lastBenchmarkFix } from '../../registry/notices.js';
import type { MechanismContext } from '../../world/context.js';
import { instrumentKindId } from '../../core/ids.js';
import { CORPORATE_PAR, type Covenants } from './terms.js';

export const LEVERAGED_LOAN = instrumentKindId('corporate.leveragedLoan');

/**
 * Indices A1, D3: the name of the reference a line fixes on — one money's overnight book, said the
 * way the index system says it, so the line's terms and the fixing cannot disagree about which
 * rate is meant (Law 4). SECURED is the one a corporate margin is struck over: it is the book that
 * clears in every money, and an unsecured benchmark exists only where that book has traded.
 */
export const benchmarkNamed = (ccy: CurrencyCode): string => `${String(ccy)}:secured`;

export interface LeveragedLoanTerms extends Terms, CouponSchedule {
  readonly kind: typeof LEVERAGED_LOAN;
  readonly issuer: PartyId;
  readonly seniority: number;
  readonly covenants: Covenants;
  /** N5.b: the named reference it fixes on, which somebody else publishes (Indices A1). */
  readonly benchmark: string;
  /** N5.b: what this issuer promised OVER the reference, locked at issuance. */
  readonly margin: Rate;
  /** N6: the date the rate in force was last set, so a reader can see how old the fixing is. */
  readonly fixedOn: Civil;
}

export const isLeveragedLoan = (t: Terms): t is LeveragedLoanTerms =>
  'margin' in t && 'benchmark' in t && 'covenants' in t;

export function leveragedLoanTerms(i: Instrument): LeveragedLoanTerms {
  if (!isLeveragedLoan(i.terms)) {
    throw new InvalidRegistry('Corporate Credit B4', `${i.id} is not a leveraged loan`);
  }
  return i.terms;
}

/**
 * Law 9: a market names a floating line by its issuer, its margin and its maturity — the margin
 * rather than the coupon, because the coupon is what it happens to be paying this quarter and the
 * margin is what the issuer promised. One line per issuer per maturity, as the fixed lines are.
 */
export const leveragedLoanId = (issuer: PartyId, maturity: Civil): InstrumentId =>
  instrumentId(`loan:${issuer}:${formatCivil(maturity)}`);

export const leveragedLoan: InstrumentKindProfile = {
  id: LEVERAGED_LOAN,
  pricing: 'cleared',
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  owes: 'face',
  // N5.b: the kernel's door refuses to fix a coupon on a kind that did not say this.
  floats: true,
  unit: () => CORPORATE_PAR,
  ranking: (i) => ({
    seniority: isLeveragedLoan(i.terms) ? i.terms.seniority : 0,
    secured: [],
    claim: 'an unsecured claim on the estate, ranking where the loan says it ranks',
  }),
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  accelerates: true,
  validateTerms: (t) => {
    if (!isLeveragedLoan(t)) throw new InvalidRegistry('Corporate Credit B4', 'not loan terms');
    if (compareCivil(t.issueDate, t.maturity) >= 0) {
      throw new InvalidRegistry('Bond N4', 'a loan matures after it is drawn');
    }
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product
    if (t.margin.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a margin is quoted per annum');
    }
    if (t.benchmark.length === 0) {
      throw new InvalidRegistry('Bond N5.b', 'a floating line names the reference it fixes on');
    }
    if (t.covenants.leverage <= 0 || t.covenants.coverage <= 0) {
      throw new InvalidRegistry(
        'Corporate Credit B2',
        'a covenant line of nothing is a promise to nothing',
      );
    }
  },
  displayName: (i, namer) => {
    const who = issuerName(namer, i.id);
    if (!isLeveragedLoan(i.terms)) return `${who} loan`;
    // Law 9: a market names it by what the issuer promised — the margin — and the date.
    return `${who} +${percent(i.terms.margin.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  due: (i, period, cal, scale) =>
    isLeveragedLoan(i.terms) ? dueOf(i.terms, period, cal, ontoTheGrid(scale, i)) : [],
  accrued: (i, on, cal, scale) =>
    isLeveragedLoan(i.terms) ? accruedOf(i.terms, on, cal, ontoTheGrid(scale, i)) : 0,
  cashFlows: (i, after, cal, scale) =>
    isLeveragedLoan(i.terms) ? cashFlowsOf(i.terms, after, cal, ontoTheGrid(scale, i)) : [],
};

/** N5.b: the reference plus what this issuer promised over it — the rate in force, per annum. */
export function couponAt(fixing: Ratio, margin: Rate): Rate {
  return rate(
    plus(fixing, asRatio(margin.amount, 'the margin it promised'), 'what it pays for the period'),
    ANNUAL,
  );
}

/**
 * N5.b, N6, Law 19: EVERY LINE WHOSE ACCRUAL PERIOD HAS TURNED OVER, fixed at the rate the benchmark
 * published. The reset dates are the line's own coupon dates — what it pays for the period it has
 * just entered is set at the start of that period, which is what a fixing IS — and the rate is the
 * one somebody else published, read and never re-derived.
 *
 * A line whose benchmark has not fixed since it was drawn keeps the rate it was drawn at: the money
 * market did not trade, so nothing new was said about what money costs, and inventing a rate for it
 * would be the posted benchmark Appendix B forbids.
 */
export function fixResets(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isLeveragedLoan(i.terms)) continue;
    const t = i.terms;
    // N6: the accrual periods are the coupon dates. A line resets when one of them has passed since
    // the rate in force was set — never on a count of periods (Law 8, Money G3.b).
    const resets = ctx.calendar
      .schedule(t.issueDate, t.maturity, t.couponPeriodicity)
      .filter((d) => compareCivil(d, t.fixedOn) > 0 && compareCivil(d, today) <= 0);
    if (resets.length === 0) continue;
    const said = lastBenchmarkFix(ctx.journal, t.benchmark);
    if (!said.some) continue;
    const now = couponAt(said.value, t.margin);
    if (now.amount === t.coupon.amount) continue;
    ctx.fixCoupon(i.id, now, t.benchmark);
  }
}

/**
 * A2, A2.a, B4 (17.1): FIXED OR FLOATING IS A DECISION, and this is the reason behind it.
 *
 * An issuer compares what each shape costs it over the life of the line: the fixed coupon it would
 * have to strike today, against the rate in force plus its margin, carried forward at ITS OWN
 * outlook of the reference (§46 A2 — its own, formed from what it has seen the benchmark fix at, and
 * it can be wrong). Two issuers with different outlooks choose differently on the same day, which is
 * A2.c: the capital structure that results is an outcome of a decision meeting a price.
 *
 * With no outlook of the rate, what it knows is what the rate IS, and it compares on that. Nothing
 * here is a forecast the model handed it (§46 A4) and nothing is a rule about who floats.
 */
export function floatsRatherThanFixes(
  fixed: Ratio,
  fixing: Ratio,
  margin: Ratio,
  expected: Ratio | undefined,
): boolean {
  const reference = expected ?? fixing;
  return plus(reference, margin, 'what floating would cost it') < fixed;
}

/** The maturity a floating line is drawn to, from its own tenor in months (Law 8). */
export const loanMaturity = (on: Civil, months: number): Civil => addMonths(on, months);
