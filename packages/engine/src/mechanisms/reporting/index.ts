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
import { compareCivil } from '../../calendar/civil.js';
import type { PartyId } from '../../core/ids.js';
import type { MechanismContext } from '../../world/context.js';
import type { Event } from '../../journal/journal.js';
import type { SystemModule } from '../../world/module.js';
import { REPORTING_PARAMS, reportingParams } from './data.js';
import { anchorOf, publishableOn, quarterClosedBy, spanOf } from '../../calendar/fiscal.js';
import { period as asPeriod } from '../../calendar/calendar.js';
import { INTERIM } from '../../registry/statements.js';
import { namesQuotedIn } from '../../registry/banking.js';
import { partyId } from '../../core/ids.js';
import { isPublic, keepsAccounts, listedLineOf } from './report.js';
import { earnedOver, prepareStatement } from './statement.js';
import type { Instrument } from '../../register/instruments.js';
import { guidanceOf, hasMoved, nextQuarter, type Guidance } from './guidance.js';

export * from './data.js';
export * from '../../calendar/fiscal.js';
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
  const lag = ctx.params.days(REPORTING_PARAMS.lag);
  const today = ctx.calendar.endOf(ctx.period);
  const state = published(ctx);
  // 17.0a, THE OWNER'S RULE: EVERY COMPANY PREPARES FULL QUARTERLY FINANCIALS, and whether they are
  // published or shared is a separate act. So every living party that is not an estate closes its
  // books on its own fiscal calendar; the statement is written under its own name, PUBLIC where the
  // company has paper anybody can buy (A1.a) and private otherwise, and then SHOWN to whoever has a
  // reason to see it — its lenders of record and the banks that keep its accounts. G4 still holds:
  // nothing is PUBLISHED by a company whose paper nobody outside holds.
  for (const company of ctx.parties.all()) {
    if (!company.status.alive || !keepsAccounts(ctx, company.id)) continue;
    const line = listedLineOf(ctx, company.id);
    const isOpen = isPublic(ctx, company.id);
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
    const earned = report(ctx, company.id, quarter.label, quarter, line, isOpen);
    state.said[key(company.id, quarter.label)] = earned;
    if (already === undefined) state.done[String(company.id)] = [quarter.label];
    else already.push(quarter.label);
    // B1: and a PUBLIC company's management guides to the quarter that opens next, in the lines
    // the report carries. A private company's outlook is its own.
    if (isOpen && line !== undefined)
      guide(ctx, company.id, nextQuarter(anchorOf(seed, company.id), quarter), state);
  }
  // B2: a revision is information, so it is looked for every period and not only on a report.
  revise(seed, ctx, state);
}

/**
 * Reporting A2, A3, Corporate Credit A4, §29 E1 (17b′.1): THE BOOKS A LENDER ASKED TO SEE.
 *
 * The owner's finding: *"a lender needs to see accounts before a buyout or any type of large
 * lending, even if just a current snapshot."* A company's first fiscal close is one to four quarters
 * after this world opens — a quarter that began before the epoch is a quarter with no books in it —
 * and in that window it has nothing to show anybody. A bank that committed a facility in it would be
 * lending large against nothing, and nobody does that.
 *
 * So a company that has asked for a COMMITMENT prepares MANAGEMENT ACCOUNTS: the same statement
 * `report` prepares, over the span from its last close (or from the opening) to now. It is PRIVATE
 * and stays private — a snapshot handed to a lender is not a quarter anybody may trade on — and it
 * is shown through the same door the quarterly one is, plus to every bank that has quoted the name,
 * which is who it is asking.
 *
 * WHICH ASKS GET ONE IS NOT A SIZE ANYBODY DECLARED (Law 6). It is `wants: 'commitment'`, and in
 * this world that IS the large borrowing: a facility a lender's capital stands behind before a penny
 * moves, as against a line drawn and repaid at the borrower's option. A company that wants a week's
 * working capital is not asked for its books and does not prepare any.
 *
 * IT RUNS IN THIS PHASE, after revaluation, for the reason the quarterly report does: a balance
 * sheet is struck when the marks are final (Clearing F1.a). The lender reads it in the NEXT period,
 * when it decides — which is the lag Reporting B2.a wants anyway, because a covenant a lender could
 * test before the accounts were struck is not a covenant.
 *
 * It takes no fiscal anchor, unlike `publish`: a snapshot is struck AS AT TODAY and belongs to no
 * quarter, which is the whole of what makes it a snapshot.
 */
function prepareForLenders(ctx: MechanismContext): void {
  const today = ctx.calendar.endOf(ctx.period);
  for (const req of ctx.requests(ctx.period)) {
    if (req.wants !== 'commitment') continue;
    const company = ctx.parties.resolve(req.borrower).id;
    const who = ctx.parties.get(company);
    /**
     * A3, 17.0a: ASKING IS THE REASON. `keepsAccounts` asks whether anybody is already OWED this
     * company's books — a lender of record, or the people it employs — and a company that owes
     * nobody anything yet keeps none. A company asking a bank to commit is in exactly that
     * position and is exactly the one that must produce them: it owes its books to the lender it
     * is asking. An ESTATE is still the exception, as it is for the quarterly report — it is
     * realising a book, not borrowing against one (XI-8).
     */
    if (!who.status.alive || ctx.registry.partyKind(who.kind).terminal === true) continue;
    /**
     * A2: IT DOES NOT PREPARE WHAT IT ALREADY HAS. Accounts struck at the close of the period that
     * just ended are this morning's, and a second set for the same day would be two answers to one
     * question (Law 4). What it has is the fresher of its quarterly report and its last snapshot.
     */
    const held = ctx.published.latestAccounts(company);
    if (held !== undefined && held.to >= ctx.period) continue;
    // The span since its books were last struck, or since this world opened if they never were.
    const from = held === undefined ? asPeriod(0) : asPeriod(held.to + 1);
    if (from > ctx.period) continue;
    const label = `interim ${today.y}-${today.m}-${today.d}`;
    const prepared = prepareStatement(ctx, company, label, { from, to: ctx.period });
    const statement = ctx.record(
      INTERIM,
      [company],
      {
        ...prepared,
        opens: `${ctx.calendar.startOf(from).y}-${ctx.calendar.startOf(from).m}-${ctx.calendar.startOf(from).d}`,
        closes: `${today.y}-${today.m}-${today.d}`,
      },
      // A3, G4: PRIVATE, always. It is not a quarter and nothing may trade on it.
      false,
    );
    share(ctx, company, statement);
    // Corporate Credit A4: and to the banks it is ASKING, which are the ones that quoted it — the
    // whole point of preparing them. A bank that never quoted this name is not owed its books.
    for (const e of namesQuotedIn(ctx.journal, ctx.period)) {
      const bank = e.data['bank'];
      if (typeof bank !== 'string' || e.subjects[0] !== String(company)) continue;
      const to = ctx.parties.resolve(partyId(bank)).id;
      if (to !== company) ctx.disclose(company, to, statement);
    }
  }
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
  const now = earnedOver(ctx, company, span.from, span.to);
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
  // Law 8, Currency A3: what it guided is money, and money on the record names its currency.
  const ccy = ctx.registry.currencyOf(ctx.parties.get(company).region);
  ctx.record('reporting.guidance', [company], { company, ...said, ccy }, true);
}

/**
 * B2: REVISED BETWEEN REPORTS, on a move and never on a schedule. A management whose own outlook has
 * moved past the dust of the arithmetic that produced it has told its own decisions something new,
 * and B4 says the audience is told the same thing.
 */
function revise(seed: string, ctx: MechanismContext, state: Published): void {
  const today = ctx.calendar.endOf(ctx.period);
  for (const company of ctx.parties.all()) {
    const standing = state.guiding.get(String(company.id));
    if (standing === undefined) continue;
    if (!company.status.alive || !isPublic(ctx, company.id)) {
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
  line: Instrument | undefined,
  isOpen: boolean,
): number {
  const span = spanOf(quarter, ctx.calendar);
  // A2: THE WHOLE STATEMENT IS ONE READ, prepared for every company alike (17.0a); the journal
  // carries it in pieces beside its one currency (Law 8), and `statementOf` is the one parser.
  const prepared = prepareStatement(ctx, company, label, span);
  const statement = ctx.record(
    'reporting.report',
    [company],
    {
      ...prepared,
      opens: `${quarter.begins.y}-${quarter.begins.m}-${quarter.begins.d}`,
      closes: `${quarter.ends.y}-${quarter.ends.m}-${quarter.ends.d}`,
    },
    isOpen,
  );
  share(ctx, company, statement);
  return prepared.earned;
}

/**
 * Observer A3, A4, Corporate Credit A4 (17.0a): WHO IS SHOWN THE STATEMENT, and why. A lender of
 * record — the holder of a claim on the company that no market prices, so a bilateral one — is
 * owed the books it lent against (Banks Lending A5: a covenant is tested on them). The bank that
 * keeps the company's account sees its flows anyway and is the bank that quotes it. Each is a
 * disclosure the kernel records, so what a lender knows about a name is on the record; a public
 * company's statement is public and the disclosure is what makes the private one reachable.
 */
function share(ctx: MechanismContext, company: PartyId, statement: Event): void {
  const shown = new Set<string>();
  const showTo = (to: PartyId): void => {
    if (to === company || shown.has(String(to))) return;
    shown.add(String(to));
    ctx.disclose(company, to, statement);
  };
  for (const i of ctx.instruments.issuedBy(company)) {
    if (!i.status.live || i.market.some) continue;
    if (!ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    for (const holder of ctx.register.holdersOf(i.id)) showTo(holder);
  }
  for (const h of ctx.register.holdingsOf(company)) {
    const i = ctx.instruments.get(h.instrument);
    if (ctx.registry.instrumentKind(i.kind).pricing !== 'money' || !i.issuer.some) continue;
    showTo(i.issuer.value);
  }
}

/**
 * A1, A2: the module. It requires `equity` because a share line is what makes a company public and
 * `firms` because a company is one, and it declares no kind and no party: the companies are already
 * here and reporting is something they do.
 */
export function reporting(seed: string): SystemModule {
  return {
    id: 'reporting',
    nouns: [
      {
        name: 'reporting',
        kind: 'working',
        holds:
          'which quarters each company has already reported, what each report SAID it earned, and the figure management is currently guiding to',
        why: 'BOOKKEEPING ABOUT PUBLISHING, not the statement. The statement itself goes to the JOURNAL as `reporting.report` — income lines, earned, revaluation, assets, liabilities — where five other modules already read it (`research`, `control`, `corporate-bond`, `guidance`, the observer). What is kept here is which quarters are done, what each said so a later disagreement is a restatement, and the standing guidance: state one phase hands to a later one. It was declared a noun at item 0 on the belief that accounts were unreadable outside this module, and that belief was wrong (`docs/RECORD.md` item 3).',
      },
    ],
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
        spec: 'Reporting A1 Reporting A3 Reporting A4 Clearing F1.a',
        /**
         * A1, A4, Clearing F1.a, item 0 (stop 18): A BALANCE SHEET IS STRUCK AT A CLOSE, so it is
         * published after the marks are taken and not before the session that makes them.
         *
         * It ran after `corporateActions`, which is the top of the period: the statement asks what
         * every holding is worth (`balanceSheet`), and a line whose market had not yet sat this
         * period had no mark to give — `NotYetProduced [Clearing F1.a] cp:firm.7:2026-07-06 has no
         * print for period 20`, and the run stopped the first time a company's quarter closed while
         * it held one. F1.a says the fix for that is the ORDER, and ARCHITECTURE 4.8 says where: a
         * read AT MARKS belongs after revaluation, which is the moment the marks are final.
         *
         * What reads the report — the covenant test, research, guidance, control — reads it with
         * the lag publishing has, which is what B2.a asks for anyway: a covenant a lender could
         * test before the accounts were struck is not a covenant.
         */
        anchor: { after: 'revaluation' },
        // A2: the statement is a READ, and these are the public records it reads — the deals that
        // named this company (§35), what it declared on its shares (Equity D3), and what a bank
        // published about its own regulation. Everything else in it comes from the register, the
        // ledger, the agreements, the contracts, the employment rows and the party's own outlooks.
        reads: [
          { kind: 'event', name: 'bank.capital', of: 'anyPeriod' },
          // 17b′.1: who asked for a commitment today, and which banks have quoted them.
          { kind: 'event', name: 'credit.quoted', of: 'thisPeriod' },
          { kind: 'event', name: 'credit.request', of: 'thisPeriod' },
          { kind: 'event', name: 'bank.liquidity', of: 'anyPeriod' },
          { kind: 'event', name: 'control.acquired', of: 'anyPeriod' },
          { kind: 'event', name: 'control.combined', of: 'anyPeriod' },
          { kind: 'event', name: 'control.contested', of: 'anyPeriod' },
          { kind: 'event', name: 'control.failed', of: 'anyPeriod' },
          { kind: 'event', name: 'control.owned', of: 'anyPeriod' },
          { kind: 'event', name: 'control.tender', of: 'anyPeriod' },
          { kind: 'event', name: 'payout.declared', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'disclosed' },
          { kind: 'event', name: 'reporting.guidance' },
          { kind: 'event', name: 'reporting.interim' },
          { kind: 'event', name: 'reporting.report' },
        ],
        run: (ctx: MechanismContext): void => {
          publish(seed, ctx);
          // 17b′.1: and the books a lender asked to see, for whoever asked for a commitment today.
          prepareForLenders(ctx);
        },
      },
    ],
    participants: [],
    families: [],
  };
}
