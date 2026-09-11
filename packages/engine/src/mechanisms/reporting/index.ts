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

export * from './data.js';
export * from './fiscal.js';
export * from './report.js';

/** What this module has already published, so a quarter is reported once (A1: on a calendar). */
interface Published {
  /** Company id to the labels of the quarters it has reported. */
  readonly done: Record<string, string[]>;
}

function published(ctx: MechanismContext): Published {
  return ctx.state<Published>('reporting', () => ({ done: {} }));
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
    if (already?.includes(quarter.label) === true) continue;
    report(ctx, company.id, quarter.label, quarter, line);
    if (already === undefined) state.done[String(company.id)] = [quarter.label];
    else already.push(quarter.label);
  }
}

/** The report itself: three statements, every line of them a read (A2). */
function report(
  ctx: MechanismContext,
  company: PartyId,
  label: string,
  quarter: ReturnType<typeof quarterClosedBy>,
  line: ReturnType<typeof listedLineOf>,
): void {
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
