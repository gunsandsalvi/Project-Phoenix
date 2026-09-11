/**
 * Reporting: what a public company's own books produced, published on its own calendar.
 *
 * @spec Reporting A1 Reporting A1.a Reporting A2 Reporting A2.a Reporting A3 Reporting A4 Reporting A4.a Reporting A5 Reporting G2 Reporting G4 Reporting G5 Reporting G6 Money G3.a Money G3.b Law 4 Law 9 Law 19
 *
 * A2 is the whole of it: THE REPORT IS A READ. Every figure in it is reachable from instructions
 * that settled and marks that were taken — income from the equity ledger's own entries (12a.1), the
 * balance sheet from the same function the `accounts` family checks against, the cash from the
 * wire's own legs. Nothing here composes a number, and A2.a is why: a figure that cannot be traced
 * to settled instructions is a second set of accounts.
 *
 * WHO REPORTS IS A STATE AND NOT A KIND (A1.a). A firm is public when it has a share line that a
 * market prices and somebody other than the firm holds it — three register reads, evaluated every
 * period, with no flag anywhere and no branch on what sort of party it is. A firm that ceases to be
 * public stops reporting in the period it stops, which is what a read gives you for free.
 *
 * AND THE CALENDAR IS DATES (G6). Companies' years end in different months, so reporting season is
 * something that happens continuously rather than once; a quarter is the pair of dates that bound
 * it; the periods in it are asked of the one calendar; and the lag between the close and the
 * publication is the only information asymmetry this world has.
 */
import { forbid } from '../../core/assert.js';
import { compareCivil } from '../../calendar/civil.js';
import type { PartyId } from '../../core/ids.js';
import { balanceSheet } from '../../audit/families/accounts.js';
import { FIRM } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { REPORTING_PARAMS, anchorOf, reportingParams } from './data.js';
import { publishableOn, quarterClosedBy, spanOf } from './fiscal.js';
import { cashOf, incomeOf, isPublic, listedLineOf } from './report.js';
import { guidanceOf, hasMoved, nextQuarter, type Guidance } from './guidance.js';

export * from './data.js';
export * from './fiscal.js';
export * from './report.js';
export * from './guidance.js';

/** What this module has already published, so a quarter is reported once (A1: on a calendar). */
interface Published {
  /** Company id to the labels of the quarters it has reported. */
  readonly done: Record<string, string[]>;
  /** A5: what each report SAID it earned, so a later disagreement with the books is a restatement. */
  readonly said: Record<string, number>;
  /** B2: the per-period figure this management last guided to, so a revision is a MOVE and not a date. */
  readonly guiding: Map<string, number>;
}

function published(ctx: MechanismContext): Published {
  return ctx.state<Published>('reporting', () => ({
    done: {},
    said: {},
    guiding: new Map<string, number>(),
  }));
}

/**
 * A1, A3, A4: publish, for every public company whose fiscal quarter closed at least the legislated
 * lag ago and which has not yet reported it.
 *
 * It runs after `corporateActions` because a coupon or a dividend that falls on the publication day
 * is part of the period it is published in, not of the one before — and because the balance sheet
 * the report carries must be the one the audit will check at the end of this same period.
 */
function publish(seed: string, ctx: MechanismContext): void {
  const lag = ctx.params.get(REPORTING_PARAMS.lag);
  const today = ctx.calendar.endOf(ctx.period);
  const state = published(ctx);
  for (const company of ctx.parties.ofKind(FIRM)) {
    if (!company.status.alive) continue;
    const line = listedLineOf(ctx, company.id);
    // G4: no report from a company whose shares nobody outside holds.
    if (!isPublic(ctx, company.id, line)) continue;
    const quarter = quarterClosedBy(anchorOf(seed, company.id), today);
    if (compareCivil(publishableOn(quarter, lag), today) > 0) continue;
    // Seed A2, Money G3: A COMPANY CANNOT REPORT ON A QUARTER THAT STARTED BEFORE THIS WORLD DID.
    // The first fiscal close after the epoch belongs to a quarter whose earlier weeks have no books
    // in them at all, and a report covering them would be a report about nothing — not a short
    // period, an absent one. So the first report a company publishes is for the first quarter that
    // opened on or after the epoch, which is between one and four quarters into the run depending on
    // where its own year end falls.
    if (compareCivil(quarter.begins, ctx.calendar.epoch) < 0) continue;
    const already = state.done[String(company.id)];
    if (already?.includes(quarter.label) === true) {
      // A5: RESTATEMENT. The entries are append-only and never edited (Register E2.a), so a figure
      // already published can only change one way: a later entry dated into a span already
      // reported. Re-reading the span every period is what catches it, and the original stands.
      restate(ctx, company.id, quarter, state);
      continue;
    }
    const earned = report(ctx, company.id, quarter.label, quarter, line);
    state.said[key(company.id, quarter.label)] = earned;
    if (already === undefined) state.done[String(company.id)] = [quarter.label];
    else already.push(quarter.label);
    // B1: and management guides to the quarter that opens next, in the lines the report carries.
    guide(ctx, company.id, nextQuarter(anchorOf(seed, company.id), quarter), state);
  }
  // B2: a revision is information, so it is looked for every period and not only on a report.
  revise(seed, ctx, state);
}

function key(company: PartyId, quarter: string): string {
  return `${company}|${quarter}`;
}

/**
 * A5: republished with a correction, dated, with the original standing (Register E2.a: a correction
 * is a new entry, never an erasure). A restatement is information about the management.
 */
function restate(
  ctx: MechanismContext,
  company: PartyId,
  quarter: ReturnType<typeof quarterClosedBy>,
  state: Published,
): void {
  const was = state.said[key(company, quarter.label)];
  if (was === undefined) return;
  const span = spanOf(quarter, ctx.calendar);
  const now = incomeOf(ctx, company, span.from, span.to).total;
  if (!hasMoved(now, was)) return;
  ctx.record(
    'reporting.restate',
    [company],
    { company, quarter: quarter.label, was, now, moved: now - was },
    true,
  );
  state.said[key(company, quarter.label)] = now;
}

/** B1, B4: what management expects of the coming quarter — its OWN outlook, published. */
function guide(
  ctx: MechanismContext,
  company: PartyId,
  coming: ReturnType<typeof quarterClosedBy>,
  state: Published,
): void {
  const said = guidanceOf(ctx, company, coming);
  if (said === undefined) {
    // B2: withdrawal. A management that no longer has a view of its own income has nothing to say,
    // and saying nothing is an event because the last thing it said is still standing.
    if (!state.guiding.has(String(company))) return;
    state.guiding.delete(String(company));
    ctx.record('reporting.guidance.withdrawn', [company], { company, quarter: coming.label }, true);
    return;
  }
  publishGuidance(ctx, company, said, state);
}

function publishGuidance(
  ctx: MechanismContext,
  company: PartyId,
  said: Guidance,
  state: Published,
): void {
  state.guiding.set(String(company), said.perPeriod);
  ctx.record('reporting.guidance', [company], { company, ...said }, true);
}

/**
 * B2: REVISED BETWEEN REPORTS, on a move and never on a schedule. A management whose own outlook has
 * moved past the dust of the arithmetic that produced it has told its own decisions something new,
 * and B4 says the audience is told the same thing.
 */
function revise(seed: string, ctx: MechanismContext, state: Published): void {
  const today = ctx.calendar.endOf(ctx.period);
  for (const company of ctx.parties.ofKind(FIRM)) {
    const standing = state.guiding.get(String(company.id));
    if (standing === undefined) continue;
    if (!company.status.alive || !isPublic(ctx, company.id, listedLineOf(ctx, company.id))) {
      state.guiding.delete(String(company.id));
      ctx.record('reporting.guidance.withdrawn', [company.id], { company: company.id }, true);
      continue;
    }
    const anchor = anchorOf(seed, company.id);
    const coming = nextQuarter(anchor, quarterClosedBy(anchor, today));
    const now = guidanceOf(ctx, company.id, coming);
    if (now === undefined || !hasMoved(now.perPeriod, standing)) continue;
    publishGuidance(ctx, company.id, now, state);
  }
}

/** The report itself: three statements, every line of them a read (A2). Returns the bottom line. */
function report(
  ctx: MechanismContext,
  company: PartyId,
  label: string,
  quarter: ReturnType<typeof quarterClosedBy>,
  line: ReturnType<typeof listedLineOf>,
): number {
  forbid(line !== undefined, 'Reporting A1', `${company} reports with no share line`);
  const span = spanOf(quarter, ctx.calendar);
  const income = incomeOf(ctx, company, span.from, span.to);
  const sheet = balanceSheet(ctx, company);
  const cash = cashOf(ctx, company, span.from, span.to);
  ctx.record(
    'reporting.report',
    [company],
    {
      company,
      quarter: label,
      opens: `${quarter.begins.y}-${quarter.begins.m}-${quarter.begins.d}`,
      closes: `${quarter.ends.y}-${quarter.ends.m}-${quarter.ends.d}`,
      from: span.from,
      to: span.to,
      // G2: the movement of the equity account, decomposed into what the instructions and the marks
      // did. `lines` is that decomposition in the words its writers used; `total` is the bottom line
      // and `revaluation` the part of it nobody was paid.
      income: income.lines.map((l) => ({ cause: l.cause, amount: l.amount, entries: l.entries })),
      earned: income.total,
      revaluation: income.revaluation,
      // A2, Law 4: the same read the `accounts` family checks the equity account against. A report
      // with its own balance sheet would be a second set of accounts able to disagree with the one
      // the audit proves (A2.a).
      assets: sheet.assets.value,
      liabilities: sheet.liabilities.value,
      ccy: sheet.ccy,
      cash: cash.map((c) => ({
        counterparty: c.counterparty,
        instrument: c.instrument,
        cause: c.cause,
        amount: c.amount,
        legs: c.legs,
      })),
      // G5: shares outstanding, so a reader can divide. Earnings per share is income over shares,
      // both of them reads; a stored quotient would be an outcome written down (Law 2).
      shares: line.issued,
    },
    true,
  );
  return income.total;
}

/**
 * A1, A2: the module. It requires `equity` because a share line is what makes a company public and
 * `firms` because a company is one, and it declares no kind and no party: the companies are already
 * here and reporting is something they do.
 */
export function reporting(seed: string): SystemModule {
  return {
    id: 'reporting',
    spec: 'Reporting',
    requires: ['firms', 'equity'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: reportingParams(),
    phases: [
      {
        name: 'reporting.publish',
        spec: 'Reporting A1 Reporting A3 Reporting A4',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          publish(seed, ctx);
        },
      },
    ],
    participants: [],
    families: [],
  };
}
