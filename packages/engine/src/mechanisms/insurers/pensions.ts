/**
 * Pensions: the second profile behind the sector's dispatch table (14.6).
 *
 * @spec Insurers A1 Insurers A2 Insurers A2.a Insurers A3 Insurers A4 Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers B3 Insurers D1 Insurers D3 Insurers E1 Insurers E3 Households F3 XI-3 XI-15 Law 2 Law 5 Law 8 Law 15 Law 19
 *
 * A PENSION FUND IS AN INSTITUTION WHOSE LIABILITY IS A SCHEDULE (B1), like an insurer's, and it
 * differs from an insurer in exactly what its profile says (Law 15): it writes no cover and quotes
 * nothing; what it owes is a pension — so much per member per period to a named retired cell, for
 * as long as its members live — and what stands behind it when it is short is not its own capital
 * but a SPONSOR: the employers whose payrolls contribute to it (D3). So it fails on cash and not on
 * solvency: a fund whose promises outrun its assets is a fund with a shortfall, which is the
 * sponsors' to make good, and one that cannot pay a pension that fell due has failed.
 *
 * THE PROMISE IS TO MEMBERS, AND WHO IS A MEMBER IS A FACT ABOUT THE PERSON (XI-15): a lattice
 * dimension the seed states and a lifetime carries, so a working member ages into a retired member
 * and the standing cell of retired members is the one the fund owes. The seed's own retired paid
 * nothing in and are promised nothing — a scheme that owed them would be a seeded promise (22a).
 *
 * WHAT A MEMBER IS PAID is a share of what a week of the trade most of the fund's members worked
 * in earns NOW — a flat pension indexed to the going wage, which is what a national earnings-related
 * scheme pays (Law 1) — read at every payment and never a number carried on the row (Law 19). The
 * row is worth those payments to the members alive, decayed period by period at the fund's own
 * outlook of the cohort's mortality (B3, §46), as far as the mortality table lets anybody live,
 * discounted at the sovereign curve of its money (B2): a schedule at a market rate, so falling rates
 * raise it (B2.a) and the sector keeps its duration (B2.b). The kernel marks it like a policy.
 *
 * A SHORTFALL HAS A CONSEQUENCE (D3): the funding ratio is a read — assets at mark over promises at
 * the curve, published every period — and when the fund's equity is negative it calls its sponsors
 * for the shortfall over the recovery plan, each for its share of what its payroll carried in this
 * period. A call the sponsor cannot pay fails on the wire like any payment, and the arrear is the
 * fund's claim on it.
 */
import { yearFraction } from '../../calendar/daycount.js';
import { period } from '../../calendar/calendar.js';
import { Unpriced } from '../../core/errors.js';
import {
  cohortId,
  partyKindId,
  type CurrencyCode,
  type PartyId,
  type RegionId,
} from '../../core/ids.js';
import {
  asCash,
  asPerPiece,
  asRatio,
  negated,
  noCash,
  plus,
  ratioOf,
  valueAt,
  type Cash,
  type PerPiece,
  type Ratio,
} from '../../core/measure.js';
import { addTo, atMost, div, mul, raised, sub, sum, zeroIfNone } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty, downTick, subQty, type Qty } from '../../core/tick.js';
import { isMoneyLeg } from '../../ledger/instruction.js';
import { keyOf, weightOf } from '../../parties/party.js';
import type { Agreement, AgreementKindDecl, RowValuationReads } from '../../register/agreements.js';
import {
  PENSION_DIM,
  PENSION_MEMBER,
  PENSION_NONE,
  PENSION_PARAMS,
  PENSION_ROW,
  SPONSORSHIP_ROW,
  endOfMortalityTable,
  isPensionTerms,
  sponsorshipOf,
  type PensionTerms,
  type SponsorshipTerms,
} from '../../registry/insurance.js';
import type { CashFlow, PartyKindProfile } from '../../registry/kinds.js';
import { OCCUPATIONS } from '../../registry/occupations.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';
import { HOURS } from '../../registry/wages.js';
import { about } from '../../world/context.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';

/** Law 9: A PENSION FUND, named as the world names one; an insurer is a different institution. */
export const PENSION = partyKindId('pension');

/** Law 9: a fund is named for the place whose payrolls feed it. An id is an id and never a display name. */
export function pensionFundIdFor(region: RegionId): PartyId {
  return `pension.${region}` as PartyId;
}

export const PENSION_PAID = 'pension.paid';
export const SPONSOR_CALLED = 'pension.call';
export const FUNDED = 'pension.funded';
export const PROMISED = 'pension.promised';
export const SPONSORED = 'pension.sponsored';

/**
 * A3, D3, XI-3, Law 15: THE SECOND PROFILE. It fails on cash — a pension that fell due and was not
 * paid — and NOT on solvency: its equity is a read and it can go negative (A3), and what that means
 * for a fund is a shortfall the sponsors are called for (D3), not a licence lost. It borrows
 * nothing: what it has is what was paid in and what that earned.
 */
export const pensionKind: PartyKindProfile = {
  objective: 'itsMandate',
  id: PENSION,
  representation: 'named',
  moneyIssuer: null,
  fails: ['cash'],
  borrows: false,
  buysOnTerms: false,
  // 17.8: Banks Lending A1.a, D4.a: a loan is not a security and is never distributed outside the banking system, so nobody of this kind is ever owed one.
  banking: false,
  depositClass: null,
};

/* --------------------------------------------------------------------------------------------
 * WHAT A MEMBER IS PAID, AND WHAT THE PROMISE IS WORTH
 * ------------------------------------------------------------------------------------------ */

/** The reads a pension is sized from — the same set whether the kernel marks the row or the fund pays it. */
export interface PensionReads {
  goingRate(occupation: string, region: RegionId): PerPiece | undefined;
  readonly registry: { currencyOf(region: RegionId): CurrencyCode };
  readonly params: Pick<RowValuationReads['params'], 'ratio' | 'amount'>;
}

/**
 * A4, Households F3, Law 8, Law 19: WHAT ONE RETIRED MEMBER IS PAID A PERIOD — the replacement
 * share of what a week of the trade the promise is indexed to earns now, in whole pieces of the
 * money. Nothing where no hour has ever cleared in that trade: a pension indexed to a rate nobody
 * has struck cannot be sized, and that is a refusal, not a zero.
 */
export function pensionPerMember(reads: PensionReads, terms: PensionTerms): Option<Qty> {
  const rate = reads.goingRate(terms.indexedTo, terms.region);
  if (rate === undefined) return none();
  const week = valueAt(
    rate,
    reads.params.amount(PEOPLE_PARAMS.hoursPerMember, HOURS),
    reads.registry.currencyOf(terms.region),
    'what a week of the trade earns',
  );
  const share = reads.params.ratio(PENSION_PARAMS.replacementShare);
  return some(downTick(mul(week.pieces, share, 'the pension a period')));
}

/**
 * B1, B3: THE SCHEDULE — what one member is expected to draw in each future period: the pension,
 * decayed by the chance a member has died by then at the fund's own outlook of the cohort's
 * mortality per period, as far as the mortality table lets anybody live. Pure arithmetic on what
 * the fund expects; nothing here is a rate anybody stated.
 */
export function pensionSchedule(
  perMember: Qty,
  mortalityPerPeriod: Ratio,
  dates: readonly CashFlow['date'][],
): CashFlow[] {
  const survives = sub(
    1,
    mortalityPerPeriod,
    'the share of the cohort that lives through a period',
  );
  return dates.map((date, n) => ({
    date,
    perUnit: asPerPiece(
      mul(
        perMember,
        raised(survives, n + 1, 'the share alive by then'),
        'what one member is expected to draw',
      ),
      'a member’s expected pension that period',
    ),
  }));
}

/** B1: the days the promise can still be paid on — every period from the next to the table's end. */
function paymentDays(
  terms: PensionTerms,
  at: number,
  reads: RowValuationReads,
): CashFlow['date'][] {
  const end = endOfMortalityTable(reads.params);
  if (end === undefined) return [];
  const today = reads.on(period(at));
  const yearsLeft = sub(end, terms.entersAt, 'the years the youngest member could still live');
  const out: CashFlow['date'][] = [];
  for (let p = at + 1; ; p += 1) {
    const date = reads.on(period(p));
    if (yearFraction('ACT/365F', today, date) > yearsLeft) break;
    out.push(date);
  }
  return out;
}

/**
 * A2, A2.a, B1, B2, B2.a, B2.b, E3 (14.6): WHAT THE ROW IS WORTH NOW, which is what the kernel
 * marks it at every revaluation. The members alive to be promised (a read of the cell's weight),
 * times what one is expected to draw in each period left, at the curve the row names. The fund
 * wears the result (A2.a): its equity moves by the mark, the members' account the other way, and
 * nothing is stored but the last mark (E3). A fund with no outlook of the cohort's mortality yet
 * expects the deaths it has seen, which is none. A curve with nothing on it prices nothing and
 * says so (§5).
 */
export function promiseOf(row: Agreement, at: number, reads: RowValuationReads): Cash {
  if (!isPensionTerms(row.terms)) return noCash(row.ccy);
  const members = reads.weightOf(row.creditor);
  if (members <= 0) return noCash(row.ccy);
  const perMember = pensionPerMember(reads, row.terms);
  if (!perMember.some || perMember.value <= 0) return noCash(row.ccy);
  const outlook = reads.outlook(row.debtor, about({ on: 'mortality', cohort: row.terms.cohort }));
  const mortality = asRatio(
    outlook.some ? outlook.value.expected : 0,
    'the share of the cohort it expects to die a period',
  );
  const days = paymentDays(row.terms, at, reads);
  if (days.length === 0) return noCash(row.ccy);
  const schedule = pensionSchedule(perMember.value, mortality, days);
  const priced = reads
    .curve(row.terms.discountedAt, period(at))
    .priceOf(schedule, reads.on(period(at)));
  if (!priced.some) {
    throw new Unpriced('Insurers B2', `${row.id} is discounted at a curve with nothing on it`, {
      row: String(row.id),
      curve: String(row.terms.discountedAt),
    });
  }
  return valueAt(
    priced.value,
    asQty(members, 'the members promised'),
    row.ccy,
    'what the promise is worth, discounted',
  );
}

export const pensionRowKind: AgreementKindDecl = {
  id: PENSION_ROW,
  what: 'a pension: so much per member per period to a retired cell of members, for as long as they live',
  binds: 'whoeverSucceeds',
  valued: promiseOf,
};

/**
 * D3: the sponsor's covenant. It owes nothing until the fund is short; what it is then called for
 * is settled on the wire and what fails is an arrear the kernel writes — so the row itself carries
 * no value, and what the sponsor's balance sheet does not show is 14.8's finding.
 */
export const sponsorshipRowKind: AgreementKindDecl = {
  id: SPONSORSHIP_ROW,
  what: 'an employer whose payroll contributes to a pension fund, and is called when it is short',
  binds: 'aGoingConcern',
};

/* --------------------------------------------------------------------------------------------
 * THE PHASES
 * ------------------------------------------------------------------------------------------ */

/**
 * A4, D3 (14.6): EVERY EMPLOYER WITH A PAYROLL IS A SPONSOR of the fund of its place, from the
 * first period it has a row — the sponsorship row is what the payroll reads to know where the
 * contributions go (`labour/matching.ts payFrom`), and what the fund reads to know whom to call.
 */
export function enrolSponsors(ctx: MechanismContext): void {
  for (const row of ctx.employment.all()) {
    const employer = ctx.parties.resolve(row.employer);
    if (!employer.status.alive) continue;
    const fundId = pensionFundIdFor(employer.region);
    if (!ctx.parties.has(fundId) || !ctx.parties.get(fundId).status.alive) continue;
    enrolMembers(ctx, row.worker);
    if (sponsorshipOf(ctx.agreements, employer.id) !== undefined) continue;
    const terms: SponsorshipTerms = { kind: SPONSORSHIP_ROW, standsBehind: fundId };
    ctx.owes({
      debtor: employer.id,
      creditor: fundId,
      ccy: ctx.registry.currencyOf(employer.region),
      owed: 0,
      terms,
      why: `${String(employer.id)}'s payroll contributes to ${String(fundId)}`,
    });
    ctx.record(SPONSORED, [employer.id, fundId], { sponsor: employer.id, fund: fundId }, true);
  }
}

/**
 * A2, XI-15 (14.6): A PERSON WHOSE WAGE CARRIES A CONTRIBUTION IS A MEMBER, for life. The worker
 * cell of an employment row is one job's people (12b.2a), so the whole of it is enrolled: it moves
 * to the member key of its lattice, a dated event with its cause, and carries the key through
 * every job, spell and cohort after. Nobody the seed placed is a member until a payroll enrols
 * them, so nobody is promised what nobody paid in for.
 */
export const ENROLLED = 'pension.enrolled';
function enrolMembers(ctx: MechanismContext, worker: PartyId): void {
  const cell = ctx.parties.resolve(worker);
  if (cell.representation !== 'cell' || !cell.status.alive) return;
  if (cell.key[PENSION_DIM] !== PENSION_NONE) return;
  const moved = ctx.cells.reKey(
    cell.id,
    weightOf(cell),
    { [PENSION_DIM]: PENSION_MEMBER },
    'enrolled in the pension scheme',
  );
  ctx.record(
    ENROLLED,
    [cell.id, moved],
    { cell: cell.id, now: moved, members: weightOf(cell) },
    true,
  );
}

/** XI-15, Households F3: a living cell of retired MEMBERS — the only party a pension is owed to. */
function isRetiredMembers(ctx: MechanismContext, party: PartyId): boolean {
  const p = ctx.parties.resolve(party);
  if (p.representation !== 'cell' || !p.status.alive) return false;
  // Law 15: a cell of a kind that is not keyed on membership is nobody's member — read, not asked.
  if (p.key[PENSION_DIM] !== PENSION_MEMBER) return false;
  if (p.key['estate'] !== 'living') return false;
  const cohort = p.key['cohort'];
  if (cohort === undefined) return false;
  return (
    ctx.registry.cohort(cohortId(cohort)).fromAge >= ctx.params.years(PEOPLE_PARAMS.retirementAge)
  );
}

/** Law 9: the trade most of a place's people work in now — what a flat pension there is indexed to. */
function benchmarkTrade(ctx: MechanismContext, region: RegionId): string | undefined {
  let best: { readonly id: string; readonly heads: number } | undefined;
  for (const o of OCCUPATIONS) {
    const heads = sum(ctx.employment.inTrade(o.id, region).map((r) => r.headcount)).value;
    if (best === undefined || heads > best.heads) best = { id: o.id, heads };
  }
  // A place where nobody works has no trade to index a pension to; the promise waits for one.
  return best === undefined || best.heads <= 0 ? undefined : best.id;
}

/**
 * A2, B1, E1, Households F3 (14.6): THE PROMISE OPENS when a cell of members is retired and the
 * fund of its place owes it nothing yet — one row per (fund, cell), owing nothing NOW, the
 * schedule read off it. And it ENDS when there is nobody left to promise: a cell that died whole
 * has handed its rows to the probate office, and a pension to an office is a promise to nobody.
 */
export function keepPromises(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const row of ctx.agreements.ofKind(PENSION_ROW)) {
    if (row.state !== 'performing') continue;
    if (!isRetiredMembers(ctx, row.creditor)) ctx.endAgreement(row.id, 'nobody left to promise');
  }
  for (const cell of ctx.parties.all()) {
    if (cell.representation !== 'cell' || !isRetiredMembers(ctx, cell.id)) continue;
    const fundId = pensionFundIdFor(cell.region);
    if (!ctx.parties.has(fundId) || !ctx.parties.get(fundId).status.alive) continue;
    const standing = ctx.agreements
      .owedTo(cell.id)
      .some((a) => a.state === 'performing' && a.debtor === fundId && isPensionTerms(a.terms));
    if (standing) continue;
    const ccy = ctx.registry.currencyOf(cell.region);
    const family = ctx.sovereignCurveIn(ccy);
    if (!family.some)
      throw new Unpriced('Insurers B2', `a pension in ${ccy} has no curve to be discounted at`, {
        ccy,
      });
    const cohort = keyOf(cell, 'cohort');
    const indexedTo = benchmarkTrade(ctx, cell.region);
    if (indexedTo === undefined) continue;
    const terms: PensionTerms = {
      kind: PENSION_ROW,
      cohort,
      entersAt: ctx.registry.cohort(cohortId(cohort)).fromAge,
      region: cell.region,
      indexedTo,
      discountedAt: family.value.id,
    };
    ctx.owes({
      debtor: fundId,
      creditor: cell.id,
      ccy,
      owed: 0,
      terms,
      why: `${String(fundId)} promises ${String(cell.id)}'s members a pension from ${String(today.y)}-${String(today.m)}-${String(today.d)}`,
    });
    ctx.record(
      PROMISED,
      [fundId, cell.id],
      { fund: fundId, cell: cell.id, cohort, members: weightOf(cell), indexedTo: terms.indexedTo },
      true,
    );
  }
}

/**
 * A4, Households F3, Law 5, Law 8 (14.6): THE PENSION IS PAID — one money leg from the fund to the
 * cell for every row, the members alive times what one is paid, whole pieces a member. A pension
 * the fund cannot pay fails on the wire and its arrear ranks as a wage's; a fund that still cannot
 * pay it has failed (A3, XI-3).
 */
export function payPensions(ctx: MechanismContext): void {
  const reads: PensionReads = {
    goingRate: (o, r) => ctx.employment.goingRate(o, r),
    registry: ctx.registry,
    params: ctx.params,
  };
  for (const row of ctx.agreements.ofKind(PENSION_ROW)) {
    if (row.state !== 'performing' || !isPensionTerms(row.terms)) continue;
    const fund = ctx.parties.get(row.debtor);
    if (!fund.status.alive) continue;
    const cell = ctx.parties.resolve(row.creditor);
    const members = weightOf(cell);
    const perMember = pensionPerMember(reads, row.terms);
    if (!perMember.some || perMember.value <= 0 || members <= 0) continue;
    const amount = asQty(perMember.value * members, 'the pensions of the members');
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(fund.id, row.ccy),
          to: ctx.accountOf(cell.id, row.ccy),
          receipt: { of: 'pension' },
          ccy: row.ccy,
          amount,
        },
      ],
      cause: 'transfer',
      reason: `${String(fund.id)} pays ${String(cell.id)}'s pensions`,
    });
    ctx.record(
      PENSION_PAID,
      [fund.id, cell.id],
      {
        fund: fund.id,
        cell: cell.id,
        row: row.id,
        members,
        perMember: perMember.value,
        amount,
        paid: r.outcome === 'settled',
      },
      true,
    );
  }
}

/** D1, E3: promises at the curve — the kernel's marks on the rows it owes, read and never re-derived (Law 19). */
export function promisesOf(
  ctx: Pick<MechanismContext, 'agreements'>,
  fund: PartyId,
  ccy: CurrencyCode,
): Cash {
  let total = 0;
  for (const row of ctx.agreements.owedBy(fund)) {
    if (row.state !== 'performing' || !isPensionTerms(row.terms)) continue;
    total += zeroIfNone(ctx.agreements.markOf(row.id));
  }
  return asCash(total, ccy, 'what it has promised, at the curve');
}

/**
 * D1, D3, E3 (14.6): THE FUNDING RATIO — assets at mark over promises at the curve, a read every
 * period and never a stored number. Assets are what the fund's equity and its promises come to
 * together, because a fund owes nothing but its promises and what it failed to pay of them.
 */
export function fundingRatioOf(view: ParticipantView, promises: Cash): Option<Ratio> {
  if (promises.pieces <= 0) return none();
  const assets = plus(view.equity(), promises, 'what it holds, at mark');
  return some(ratioOf(assets, promises, 'assets over promises'));
}

/**
 * D3, Law 5 (14.6): A SHORTFALL IS CALLED FROM THE SPONSORS. Where the fund's equity is negative it
 * is short by that much, and it calls a recovery period's share of it from the employers whose
 * payrolls fed it this period, each in proportion to what its payroll carried in (read off the
 * ledger's contribution legs, Law 19), whole pieces, the remainder to the first. A sponsor with no
 * payroll this period is not called: it has nothing to be apportioned on. A call a sponsor cannot
 * pay fails on the wire and its arrear is the fund's claim on it.
 */
export function callSponsors(ctx: MechanismContext): void {
  for (const fund of ctx.parties.ofKind(PENSION)) {
    if (!fund.status.alive) continue;
    const ccy = ctx.registry.currencyOf(fund.region);
    const view = ctx.participant(fund.id);
    const promises = promisesOf(ctx, fund.id, ccy);
    const equity = view.equity();
    const ratio = fundingRatioOf(view, promises);
    ctx.record(
      FUNDED,
      [fund.id],
      ratio.some
        ? {
            fund: fund.id,
            ccy,
            equity: equity.pieces,
            promises: promises.pieces,
            ratio: ratio.value,
          }
        : { fund: fund.id, ccy, equity: equity.pieces, promises: promises.pieces },
      true,
    );
    if (equity.pieces >= 0) continue;
    const shortfall = negated(equity, 'what it is short by');
    const call = downTick(
      div(
        shortfall.pieces,
        ctx.params.periods(PENSION_PARAMS.recoveryPeriods),
        'this period’s share of the recovery',
      ),
    );
    if (call <= 0) continue;
    const account = ctx.accountOf(fund.id, ccy);
    const carried = new Map<PartyId, number>();
    for (const r of ctx.ledger.inPeriod(ctx.period)) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (!isMoneyLeg(leg) || leg.receipt.of !== 'contribution') continue;
        if (leg.to.holder !== account.holder || leg.ccy !== ccy) continue;
        const payer = ctx.parties.resolve(leg.from.holder).id;
        if (payer === fund.id) continue;
        addTo(carried, payer, leg.amount);
      }
    }
    const sponsors = [...carried.entries()].filter(
      ([sponsor, paid]) => paid > 0 && sponsorshipOf(ctx.agreements, sponsor) !== undefined,
    );
    const total = sum(sponsors.map(([, paid]) => paid)).value;
    if (total <= 0) {
      ctx.record(
        SPONSOR_CALLED,
        [fund.id],
        {
          fund: fund.id,
          ccy,
          shortfall: shortfall.pieces,
          call,
          why: 'no payroll contributed this period, so there is nobody to apportion the call on',
        },
        true,
      );
      continue;
    }
    let left: Qty = call;
    sponsors.forEach(([sponsor, paid], n) => {
      if (left <= 0) return;
      const share =
        n === sponsors.length - 1
          ? left
          : atMost(downTick((call * paid) / total), left, 'no more than is left of the call');
      if (share <= 0) return;
      const r = ctx.settle({
        legs: [
          {
            kind: 'money',
            from: ctx.accountOf(sponsor, ccy),
            to: account,
            receipt: { of: 'contribution' },
            ccy,
            amount: share,
          },
        ],
        cause: 'transfer',
        reason: `${String(fund.id)} calls ${String(sponsor)} for its share of the shortfall`,
      });
      ctx.record(
        SPONSOR_CALLED,
        [fund.id, sponsor],
        {
          fund: fund.id,
          sponsor,
          ccy,
          shortfall: shortfall.pieces,
          call,
          amount: share,
          paid: r.outcome === 'settled',
        },
        true,
      );
      left = subQty(left, share, 'what is left of the call');
    });
  }
}
