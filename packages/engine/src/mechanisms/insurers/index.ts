/**
 * Insurers and pensions: the sector whose liability is a SCHEDULE, and therefore has duration.
 *
 * @spec Insurers B2 Insurers B2.b Insurers A1 Insurers A2 Insurers A2.a Insurers A3 Insurers A4 Insurers A4.a Insurers A4.b Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers C2 Insurers C2.a Insurers D2 Insurers E1 Insurers E2 XI-3 XI-11 Law 2 Law 3 Law 4 Law 6 Law 19
 *
 * B2.b IS THE CLAUSE THIS EXISTS FOR, and it says what the easy version gets wrong: a liability that
 * accumulates contributions minus benefits plus investment income has no schedule, no discount rate
 * and no discounting — so it never moves when rates move, and THE SECTOR'S DEFINING RISK
 * DISAPPEARS. This is the model's largest holder of duration; it must have duration.
 *
 * So a policy here is a promise to pay STATED AMOUNTS AT STATED FUTURE TIMES, and what it is worth
 * is those amounts discounted at a rate read from a market (B2). Falling rates raise it. That is
 * why a rate move is a solvency event for this sector and a P&L event for everybody else (B2.a,
 * D2), and it falls out of the arithmetic rather than being asserted anywhere.
 *
 * AND THE INSURER WEARS IT, which is the whole difference between this sector and a fund (A2.a).
 * The beneficiary does not absorb the investment result: the promise is fixed and the institution's
 * equity is what moves. In this world that is one declared fact — the kind says its issuer owes the
 * VALUE of what it promised rather than a face — and the revaluation pass does the rest. A sector
 * that passed the investment result straight through would be a fund wearing an insurer's name.
 *
 * A3, XI-3: equity is assets minus liabilities, it is a READ, and it can go negative — which is a
 * solvency event with consequences, because these institutions can fail like anything else.
 */
import { type AgreementKindDecl } from '../../register/agreements.js';
import { about } from '../../world/context.js';
import { yearFraction } from '../../calendar/daycount.js';
import { costOfCapital } from '../../registry/capital.js';
import { weatheredIn } from '../../registry/physical.js';
import {
  COVER,
  COVER_TERM,
  POLICY_ROW,
  coverVenue,
  isPolicyTerms,
  type PolicyTerms,
} from '../../registry/insurance.js';
import { period } from '../../calendar/calendar.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { valueAt } from '../../core/measure.js';
import { Missing } from '../../core/errors.js';
import type { SeedContext } from '../../world/context.js';
import type { RegionId } from '../../core/ids.js';
import {
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  noCash,
  type PerPiece,
  plus,
  type Ratio,
  ratioOf,
  scale,
} from '../../core/measure.js';
import { compareCivil, type Civil } from '../../calendar/civil.js';
import { partyKindId, type CurrencyCode, type PartyId } from '../../core/ids.js';
import { Unpriced } from '../../core/errors.js';
import { sum, atMost } from '../../core/num.js';
import { downTick, subQty } from '../../core/tick.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import type { CashFlow, PartyKindProfile } from '../../registry/kinds.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { allocate, meetCalls } from './allocate.js';
import {
  ENROLLED,
  FUNDED,
  PENSION,
  PENSION_PAID,
  PROMISED,
  SPONSORED,
  SPONSOR_CALLED,
  callSponsors,
  enrolSponsors,
  keepPromises,
  payPensions,
  pensionFundIdFor,
  pensionKind,
  pensionRowKind,
  sponsorshipRowKind,
} from './pensions.js';
import { PENSION_PARAMS } from '../../registry/insurance.js';

// 14.6: the second profile behind the dispatch table, and everything a test asks of it.
export {
  PENSION,
  pensionFundIdFor,
  pensionKind,
  pensionRowKind,
  sponsorshipRowKind,
  promiseOf,
  pensionPerMember,
  pensionSchedule,
  fundingRatioOf,
  promisesOf,
  PENSION_PAID,
  SPONSOR_CALLED,
  FUNDED,
  PROMISED,
  SPONSORED,
  ENROLLED,
} from './pensions.js';

/**
 * Law 9: AN INSURANCE COMPANY, named as the world names one. The deposit insurer this world already
 * has is a different institution with a different job — it stands behind a bank's depositors and
 * charges the bank for it — and two things called the same thing is how two facts become one.
 */
export const INSURANCE = partyKindId('insurance');
// 14.2, Law 4: the names of cover are the registry's, so a buyer in another module can post into the book.
export {
  COVER,
  COVER_TERM,
  POLICY_ROW,
  coverVenue,
  isPolicyTerms,
} from '../../registry/insurance.js';

/** B1: how long a unit of cover runs for, which is a convention of the contract (Law 2). */

/** A4: a claim this insurer actually paid, which is the only experience it has (A4.c). */
export const CLAIM_PAID = 'insurer.claim';

/** Law 9: named for who owes it and which book it is on. */
export const insuranceKind: PartyKindProfile = {
  /** item 15: what somebody else set it up to do, and it does not get to change it. */
  objective: 'itsMandate',
  id: INSURANCE,
  representation: 'named',
  moneyIssuer: null,
  /**
   * A3, XI-3: THESE INSTITUTIONS CAN FAIL, both ways. It can be unable to pay a claim that fell due
   * today while holding assets it cannot sell in time, and it can owe more than it has — which is
   * what a rate move does to it, and is the consequence B2.a exists to produce.
   */
  fails: ['cash', 'solvency'],
  borrows: true,
  buysOnTerms: false,
  // 17.8: Banks Lending A1.a, D4.a: a loan is not a security and is never distributed outside the banking system, so nobody of this kind is ever owed one.
  banking: false,
  depositClass: null,
};

/**
 * A2, B1, B2, B2.a, B2.b, Law 15 (14.5): A POLICY IS A ROW, AND THE KERNEL MARKS IT. What the row
 * is worth is the insurer's own expected claims on it — its outlook of what a unit of its cover
 * costs it a period (A4.c), on the cover, for every period left of the term — discounted at the
 * curve the row names: a schedule at a market rate, which is what B2 asks of a liability and why a
 * rate move is a solvency event for this sector (B2.a). An insurer that expects no claims carries
 * the promise at nothing; a term that has ended is worth nothing. The holder's side is the same
 * number the other way, so the two accounts move together (Law 5). A curve with nothing on it
 * prices nothing and says so (§5), never a zero that reads as a promise discharged.
 */
export const policyRowKind: AgreementKindDecl = {
  id: POLICY_ROW,
  what: 'cover written: what the weather takes from the holder, up to the cover, until the term ends',
  binds: 'whoeverSucceeds',
  valued: (row, at, reads) => {
    if (!isPolicyTerms(row.terms) || row.terms.cover <= 0) return noCash(row.ccy);
    const today = reads.on(at);
    if (compareCivil(row.terms.to, today) <= 0) return noCash(row.ccy);
    const expected = reads.outlook(row.debtor, about({ on: 'claims' }));
    if (!expected.some || expected.value.expected <= 0) return noCash(row.ccy);
    const perUnit = asPerPiece(
      expected.value.expected,
      'what a unit of cover is expected to cost a period',
    );
    const schedule: CashFlow[] = [];
    for (let p = at + 1; ; p += 1) {
      const date = reads.on(period(p));
      if (compareCivil(date, row.terms.to) > 0) break;
      schedule.push({ date, perUnit });
    }
    if (schedule.length === 0) return noCash(row.ccy);
    const priced = reads.curve(row.terms.discountedAt, at).priceOf(schedule, today);
    if (!priced.some) {
      throw new Unpriced('Insurers B2', `${row.id} is discounted at a curve with nothing on it`, {
        row: String(row.id),
        curve: String(row.terms.discountedAt),
      });
    }
    return valueAt(
      priced.value,
      row.terms.cover,
      row.ccy,
      'what the cover is expected to cost, discounted',
    );
  },
};

/* --------------------------------------------------------------------------------------------
 * WHAT IT CHARGES
 * ------------------------------------------------------------------------------------------ */

/**
 * A4.a, A4.b, Law 3: THE PRICE OF COVER, and a policy goes to the insurer that prices lower.
 *
 * A4.b says what the price is made of and it is two things, both the insurer's own: the claims a
 * unit of cover is expected to bring, which is ITS OWN experience of ITS OWN book (A4.c), and the
 * return required on the capital held against it. Worse experience or dearer capital quotes higher,
 * and neither is a number anybody typed — the first is a read of what its periods actually cost it
 * and the second is what its own money costs it.
 *
 * A4.a: an insurer with no surplus writes nothing and loses its renewals. It loses book before it
 * loses its licence, which is a consequence of having nothing to stand behind cover with rather
 * than a rule about solvency.
 */
export interface CoverQuote {
  readonly orders: readonly Order[];
  /** A4.a: why it wrote nothing, where it wrote nothing for a reason a reader should see. */
  readonly refused?: string;
}

/**
 * A4.a, A4.b, A4.c, XI-4 (14.4): WHAT IT WILL WRITE COVER AT, AND HOW MUCH. The price is two things
 * and only two: what a unit of cover is expected to cost it in claims over the term — its own
 * outlook of what a unit of its book has cost it a period, formed from every period it had cover
 * out (never the last claim), over the periods a unit runs — and the return required on the capital
 * held against the premium: what its capital costs it per annum (XI-4's read: its equity where a
 * market prices it, its debt at the margin otherwise, the sovereign curve when nothing else has
 * said), over the term's fraction of a year, on the surplus a unit of cover stands on. Its capacity
 * is its surplus (A4.a): a unit of money stands behind a unit of cover, and an insurer with no
 * surplus writes nothing. Worse experience or dearer capital quotes higher (A4.b); a ratio of its
 * capital enters as a level of the same thing per unit, and no ratio becomes a level on its own.
 */
export function quoteCover(view: ParticipantView): CoverQuote {
  const surplus = view.equity();
  // A4.a: nothing to stand behind it with, so nothing written. Not a threshold — an insurer with no
  // surplus has no capacity, which is arithmetic (Law 6).
  if (surplus.pieces <= 0) return { orders: [], refused: 'no surplus to stand behind cover with' };
  const capacity = downTick(surplus.pieces);
  if (capacity <= 0) return { orders: [], refused: 'a surplus below one unit of cover' };
  const term = view.params.periods(COVER_TERM);
  const seen = view.outlook(about({ on: 'claims' }));
  const experience = seen.some
    ? scale(
        asPerPiece(seen.value.expected, 'what a unit of its cover has cost it a period'),
        asRatio(term, 'the periods a unit runs'),
        'what a unit is expected to cost over the term',
      )
    : asPerPiece(0, 'it has never had cover out, so nothing has cost it anything yet');
  const cost = costOfCapital(view, period(view.period - 1), term);
  if (!cost.some) return { orders: [], refused: 'nothing has said what its capital costs' };
  const years = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(period(view.period + term)),
  );
  // A4.b: the capital a unit of cover stands on is its surplus over its capacity — one unit of
  // money behind one unit of cover, less what the whole pieces leave.
  const capitalPerUnit = ratioOf(
    heldAsMoney(view.registry.cashFor(surplus), surplus.ccy, 'its surplus'),
    heldAsMoney(capacity, surplus.ccy, 'the cover it can write'),
    'the capital behind a unit of cover',
  );
  const requiredOnCapital = asRatio(
    cost.value.perAnnum * years * capitalPerUnit,
    'the return its capital requires over the term, per unit of cover',
  );
  const price = coverPrice(experience, requiredOnCapital);
  if (price <= 0) return { orders: [], refused: 'a price of nothing is no quote' };
  // Clearing C3: it posts a price and a size, and the book decides whose cover gets written.
  return { orders: [{ party: view.self.id, side: 'sell', price, qty: capacity }] };
}

/**
 * E1, E2, A1: what must be true of this sector, MEASURED and never repaired. Each breaks silently:
 * a liability with no beneficiary is a promise to nobody, and a policy with no schedule is the cash
 * balance B2.b forbids — the shape that makes the sector's defining risk disappear.
 */
function promises(): Family {
  return {
    name: 'names',
    contributor: 'insurers',
    spec: 'Insurers A1 Insurers B1 Insurers E1 Insurers E2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      // E1 holds by construction — a row has a named creditor or it is not a row — so what is left
      // to check is B1: a live promise of cover is a promise of SOMETHING, until a day.
      for (const a of view.agreements.ofKind(POLICY_ROW)) {
        if (a.state !== 'performing' || !isPolicyTerms(a.terms)) continue;
        if (a.terms.cover <= 0) {
          out.push({
            family: 'names',
            spec: 'Insurers B1',
            owner: String(a.id),
            size: 0,
            unit: String(COVER),
            period: view.period,
            message: `${String(a.id)}: cover of nothing, still standing`,
          });
        }
      }
      // E2 (14.8): NO ASSET THAT IS NOT SOMEBODY'S LIABILITY OR A REAL THING. Every unit the sector
      // holds is of a live instrument that names its issuer, or of a kind whose profile says it is
      // a thing and nobody's liability (plant, stock, ground). It holds by construction of the
      // register and is guarded here because it breaks silently (Part II).
      for (const kind of [INSURANCE, PENSION]) {
        for (const p of view.parties.ofKind(kind)) {
          if (!p.status.alive) continue;
          for (const h of view.register.holdingsOf(p.id)) {
            const i = view.instruments.get(h.instrument);
            const real = !view.registry.instrumentKind(i.kind).liabilityOfIssuer;
            if (i.status.live && (i.issuer.some || real)) continue;
            out.push({
              family: 'names',
              spec: 'Insurers E2',
              owner: String(p.id),
              size: view.register.quantity(p.id, h.instrument),
              unit: String(h.instrument),
              period: view.period,
              message: `${String(p.id)} holds ${String(h.instrument)}, which is ${i.status.live ? 'nobody’s liability and not a thing' : 'an instrument that has ceased'}`,
            });
          }
        }
      }
      return out;
    },
  };
}

export function insurers(): SystemModule {
  return {
    id: 'insurers',
    spec: 'Insurers',
    // Item 14.0: AND FUNDS, because an institution does not invest itself — what it does with its
    // assets is hand them to a manager, so the doors it subscribes at have to exist before it looks
    // for one. It is a dependency of the ALLOCATION and not of the cover it writes.
    // 14.6: and the payroll the contributions ride on, and the people who retire.
    requires: ['sovereign-curve', 'funds', 'labour', 'households'],
    instrumentKinds: [],
    agreementKinds: [policyRowKind, pensionRowKind, sponsorshipRowKind],
    // Law 15: two profiles behind one dispatch table — the kernel asks the profile, never the id.
    partyKinds: [insuranceKind, pensionKind],
    curveFamilies: [],
    units: [{ id: COVER, name: 'units of cover', perUnit: MONEY_PIECES }],
    params: [
      {
        id: COVER_TERM,
        value: 52,
        unit: 'periods',
        dimension: 'periods',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Insurers B1: how long one unit of cover runs for. A convention of the contract, stated with it — a year, which is what a policy is written for. It is not a forecast of when a claim arrives: what a claim COSTS is read off what this insurer has actually paid (A4.c), and it is the price that carries it.',
      },
      {
        id: PENSION_PARAMS.employeeShare,
        value: 0.05,
        unit: 'share of the wage',
        dimension: 'ratio',
        kind: 'policy',
        owner: 'parliament',
        why: 'Insurers A4, D3 (14.6): the share of a wage a member pays into the scheme, deducted at the payroll. A rule of the scheme — POLICY, parliament’s from worklist 14 — and not a forecast of anything: what the fund comes to hold is what the payrolls carry, and what it owes is the promise, and neither is this number.',
      },
      {
        id: PENSION_PARAMS.employerShare,
        value: 0.1,
        unit: 'share of the wage',
        dimension: 'ratio',
        kind: 'policy',
        owner: 'parliament',
        why: 'Insurers A4, D3 (14.6): the share of a wage the employer pays into the scheme beside the member, out of its own account in the same instruction. A rule of the scheme, POLICY, parliament’s from worklist 14.',
      },
      {
        id: PENSION_PARAMS.replacementShare,
        value: 0.4,
        unit: 'share of a week of the benchmark trade',
        dimension: 'ratio',
        kind: 'policy',
        owner: 'parliament',
        why: 'Insurers A4, B1, Households F3 (14.6): what a retired member is paid a period, as a share of what a week of the trade most of the place’s people work in earns NOW — a flat pension indexed to the going wage, which is what a national scheme pays. A rule of the scheme, POLICY, parliament’s from worklist 14; the level of the pension itself is an OUTCOME of the wage.',
      },
      {
        id: PENSION_PARAMS.recoveryPeriods,
        value: 520,
        unit: 'periods',
        dimension: 'periods',
        kind: 'policy',
        owner: 'standardSetter',
        why: 'Insurers D3 (14.6): over how many periods a shortfall is called from the sponsors — a recovery plan of ten years, which is what a funding regulator gives a scheme. A rule, POLICY, and not a bound: the whole shortfall is called, a share of it a period, and a fund that stays short stays calling.',
      },
    ],
    phases: [
      {
        /**
         * B2, B2.b (item 14.0): AN INSTITUTION PUTS ITS ASSETS UNDER MANAGEMENT.
         *
         * *"Insurance companies and pension funds don't invest themselves. Their assets are always
         * third party managed"* (the owner). So this posts ONE subscription at the door of the pool
         * whose mandate is shaped like what this insurer promised, and everything after that is the
         * manager's. There is no portfolio here, no allocation rule and no view about a price.
         *
         * BEFORE the fund strikes, because a subscription is read at the strike — and after its own
         * period's flows, so what it puts to work is what it actually has.
         */
        name: 'insurers.allocate',
        spec: 'Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Fund Shares C1 Fund Shares C2 Private Equity A2.a XI-2',
        anchor: { before: 'funds.strike' },
        reads: [
          { kind: 'event', name: 'fund.called', of: 'thisPeriod' },
          { kind: 'event', name: 'insurer.claim', of: 'anyPeriod' },
          // B2, Fund Shares D2 (14.1): what each pool last published a share is worth, for the
          // doors it can subscribe at. Never declared, because no insurer had lived to allocate.
          { kind: 'event', name: 'fund.struck', of: 'anyPeriod' },
        ],
        // 14.7: what it put to work and what it kept back, and what it asked back after a missed call.
        writes: [
          { kind: 'event', name: 'insurer.allocated' },
          { kind: 'event', name: 'insurer.raised' },
        ],
        run: (ctx: MechanismContext): void => {
          // 14.6: a pension fund invests the same way — it does not invest itself either.
          for (const p of [...ctx.parties.ofKind(INSURANCE), ...ctx.parties.ofKind(PENSION)]) {
            if (!p.status.alive) continue;
            /**
             * §29 A2.a (item 13.5c): WHAT IT OWES BEFORE WHAT IT WOULD LIKE TO OWN. A call it could
             * not meet is money it must find, and asking for it back comes before putting anything
             * new to work — an investor that allocated while it was in default on a call would be
             * two decisions about one balance (Law 4).
             */
            meetCalls(ctx, p.id);
            allocate(ctx, p.id, ctx.registry.currencyOf(p.region));
          }
        },
      },
      {
        /**
         * A4, D3 (14.6): EVERY EMPLOYER WITH A PAYROLL SPONSORS THE FUND OF ITS PLACE, before the
         * payroll runs — the sponsorship row is what the wage instruction reads to carry the
         * contributions (`labour/matching.ts payFrom`), and what the fund calls when it is short.
         */
        name: 'pensions.enrol',
        spec: 'Insurers A4 Insurers D3',
        anchor: { before: 'labour.pay' },
        reads: [],
        writes: [
          { kind: 'event', name: SPONSORED },
          { kind: 'event', name: ENROLLED },
        ],
        run: (ctx: MechanismContext): void => {
          enrolSponsors(ctx);
        },
      },
      {
        /**
         * A2, B1, D3, Households F3 (14.6): THE PROMISE, THE PENSION AND THE CALL. After the
         * households have aged and buried this period, so the cell a promise opens to is the
         * standing cell of retired members as it now is; after the revaluation, so the shortfall
         * read is this period's marks.
         */
        name: 'pensions.promise',
        spec: 'Insurers A2 Insurers A4 Insurers B1 Insurers D1 Insurers D3 Insurers E1 Insurers E3 Households F3 Law 5',
        anchor: { after: 'households.lifecycle' },
        reads: [],
        writes: [
          { kind: 'event', name: PROMISED },
          { kind: 'event', name: PENSION_PAID },
          { kind: 'event', name: FUNDED },
          { kind: 'event', name: SPONSOR_CALLED },
        ],
        run: (ctx: MechanismContext): void => {
          keepPromises(ctx);
          payPensions(ctx);
          callSponsors(ctx);
        },
      },
      {
        name: 'insurers.claims',
        spec: 'Insurers A4 Insurers B4 Law 5',
        // After the weather has taken what it takes (the capital programme's phase sits at the same
        // anchor, earlier in the order), so a claim is on a loss that has happened and been said.
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'event', name: 'capital.weathered', of: 'thisPeriod' }],
        writes: [{ kind: 'event', name: CLAIM_PAID }],
        run: (ctx: MechanismContext): void => {
          payClaims(ctx);
          expireCover(ctx);
        },
      },
      {
        name: 'insurers.cover',
        spec: 'Insurers A4 Insurers A4.a Insurers A4.b Insurers A4.c Clearing C3',
        // Before the goods and paper sessions, because what an insurer writes this period is
        // capacity it then has to stand behind: a session it cannot see cannot be a reason.
        anchor: { before: 'markets' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [{ kind: 'event', name: 'fund.struck', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'insurer.unquoted' }],
        run: (ctx: MechanismContext): void => {
          /**
           * B-2, A-9: THE SECTOR RUNS. `phases: []` and `participants: []` meant nothing in this
           * module was ever asked anything, in any period of any run — the price of cover, the
           * capacity, the venue and the audit family were all reachable only from a test.
           *
           * What is here is the SELL side: every insurer quotes what a unit of cover costs it out
           * of its own claims experience and its own capital (A4.b), and what clears is written.
           * The BUY side — a firm that stands in a physical fact and would rather not (B4) — is
           * item 14's, and until it exists these sessions come back `noDemand`, which is a
           * measured state and not an absence.
           */
          for (const ccy of ctx.registry.currencies.keys()) {
            const bids: Order[] = [];
            for (const p of ctx.parties.ofKind(INSURANCE)) {
              if (!p.status.alive || ctx.registry.currencyOf(p.region) !== ccy) continue;
              const q = quoteCover(ctx.participant(p.id));
              bids.push(...q.orders);
              // A4.a (14.4): an insurer that writes nothing says why, in public.
              if (q.refused !== undefined)
                ctx.record(
                  'insurer.unquoted',
                  [p.id],
                  { insurer: p.id, ccy, why: q.refused },
                  true,
                );
            }
            runCover(ctx, ccy, bids);
          }
        },
      },
    ],
    participants: [],
    families: [promises()],
    seed(ctx: SeedContext): void {
      /**
       * B-2, A1: AN INSURER EXISTS BEFORE ANYBODY BUYS COVER. `insurers()` had no `seed`, so no
       * party of the kind was ever created in any world — the sector was a declaration and nothing
       * else. One per region that has a bank to hold its money, named and banked like any other
       * institution, because an insurer is one: it has a balance sheet, it can fail, and what it
       * writes is a claim on it.
       */
      // 14.2: THE BOOK IS OPEN BEFORE ANYBODY BIDS. A firm posts for cover in its own decide phase
      // and a household in its own, both before the insurers quote; a venue that opened only when
      // the quote came would refuse their postings.
      for (const ccy of ctx.registry.currencies.keys()) {
        ctx.openVenue({
          id: coverVenue(ccy),
          name: `cover in ${ccy}`,
          clearedBy: 'insurers',
          unit: COVER,
          ccy,
          key: { ccy },
        });
      }
      for (const region of ctx.registry.regions.values()) {
        const bank = [...ctx.parties.all()].find(
          (p) => p.region === region.id && ctx.registry.issuesMoney(p.kind) && p.bank !== p.id,
        );
        if (bank === undefined) continue;
        // 14.1: the foundation creates and funds it before the equity seed floats its line; a
        // world seeded without the foundation still gets one here, unfunded, as before.
        if (!ctx.parties.has(insurerIdFor(region.id))) {
          ctx.parties.add({
            id: insurerIdFor(region.id),
            kind: INSURANCE,
            region: region.id,
            name: `${region.name} Assurance`,
            bank: bank.id,
            representation: 'named',
            status: { alive: true, standing: 'good' },
          });
        }
        // 14.6: and the pension fund of the place, empty — the foundation makes it first where it runs.
        if (!ctx.parties.has(pensionFundIdFor(region.id))) {
          ctx.parties.add({
            id: pensionFundIdFor(region.id),
            kind: PENSION,
            region: region.id,
            name: `${region.name} Pension Fund`,
            bank: bank.id,
            representation: 'named',
            status: { alive: true, standing: 'good' },
          });
        }
      }
    },
  };
}

/** Law 9: an insurer is named for where it writes. An id is an id and never a display name. */
export function insurerIdFor(region: RegionId): PartyId {
  return `insurance.${region}` as PartyId;
}

/**
 * B2.a, D2: WHAT A RATE MOVE DOES TO A BOOK OF PROMISES — the number the whole sector turns on,
 * exposed so a test can put a case to it. Falling rates raise the liability; the institution's
 * equity falls by the same amount, because what it owes is what the promise is worth.
 */
export const presentValueOf = (
  schedule: readonly CashFlow[],
  discount: (date: Civil) => Ratio,
): PerPiece => sum(schedule.map((f) => scale(f.perUnit, discount(f.date), 'discounted'))).value;

/** A4.b: the two halves of a cover price, which are the only two things in it. */
export const coverPrice = (experience: PerPiece, requiredOnCapital: Ratio): PerPiece =>
  plus(
    experience,
    // A4.b: what its capital costs is a SHARE and what cover costs is a level, so the share is a
    // level of the same thing before they add — which is what "per unit of cover" means.
    asPerPiece(requiredOnCapital, 'what its capital costs, per unit of cover'),
    'what a unit of cover costs to write',
  );

/** D1: the mismatch, in the one unit that makes it comparable — what each side is worth now. */
export const gapOf = (assets: Cash, liabilities: Cash): Cash =>
  minus(assets, liabilities, 'what it has against what it owes');

/** C2.a: whether this book is one a matching buyer wants — long assets against long liabilities. */
export const wantsDuration = (schedule: readonly CashFlow[], horizon: Civil): boolean =>
  schedule.some((f) => compareCivil(f.date, horizon) > 0);

/** Kept where the venue is named, so the observer and a test spell it one way (Law 4). */
export const venueForCover = coverVenue;

export const runCover = (
  ctx: MechanismContext,
  ccy: CurrencyCode,
  bids: readonly Order[],
): void => {
  const venue = coverVenue(ccy);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      name: `cover in ${ccy}`,
      clearedBy: 'insurers',
      unit: COVER,
      ccy,
      key: { ccy },
    });
  }
  for (const b of bids) ctx.post(venue, b);
  const posted = ctx.posted(venue);
  const bidders = posted.filter((o) => o.side === 'buy').length;
  const askers = posted.filter((o) => o.side === 'sell').length;
  const outcome = clear(posted, 'proRata', 'sellersCompete');
  if (!isCleared(outcome)) {
    // 14.2: a session with buyers and no insurer willing to write (a quote of nothing is no quote,
    // A4.a) is a real state and it is said — the buyers were there and nobody wrote them cover.
    ctx.record(
      'cover.cleared',
      [],
      { ccy, outcome: outcome.kind, bids: bidders, asks: askers, written: 0 },
      true,
    );
    return;
  }
  /**
   * A-9, Law 5, Clearing D2: A SESSION THAT STRIKES A PRICE AND MOVES NO MONEY IS NOT A SESSION.
   *
   * This cleared the book and then DISCARDED `outcome.fills`, recording only how many there were.
   * Every other market in this world turns a fill into an instruction (D2, D3); this one turned it
   * into a count. So even in a world where somebody bid for cover, no policy was ever registered,
   * no premium was ever paid, and the number in the record was about a trade that did not happen.
   *
   * A fill is a policy: the buyer pays the premium and the insurer ISSUES it the cover, which is a
   * claim on the insurer for the schedule the policy carries (B1). Both legs, one instruction, same
   * period (Law 5).
   */
  /**
   * Clearing C3, Law 5 (14.5): FILLS ARE PAIRED BY THE SOLVER'S ALLOCATION. Every buyer's fill is
   * spread over the insurers that filled the other side in proportion to what each wrote, whole
   * units, the remainder to the first; each pair is one premium paid and one row opened — the
   * policy — in the same pass. A premium that cannot be paid opens nothing.
   */
  const sells = outcome.fills.filter((f) => f.side === 'sell' && f.qty > 0);
  const soldTotal = sum(sells.map((f) => f.qty)).value;
  let written = 0;
  if (soldTotal <= 0) return;
  const family = ctx.sovereignCurveIn(ccy);
  if (!family.some)
    throw new Missing('Insurers B2', `cover in ${ccy} has no curve to be discounted at`, { ccy });
  const to = ctx.calendar.startOf(period(ctx.period + ctx.params.periods(COVER_TERM)));
  for (const fill of outcome.fills) {
    if (fill.side !== 'buy' || fill.qty <= 0) continue;
    let left = downTick(fill.qty);
    sells.forEach((sell, n) => {
      if (left <= 0) return;
      const share =
        n === sells.length - 1
          ? left
          : atMost(
              downTick((fill.qty * sell.qty) / soldTotal),
              left,
              'no more than is left of the fill',
            );
      if (share <= 0) return;
      const premium = ctx.registry.cashFor(
        valueAt(outcome.price, share, ccy, 'the premium at inception'),
      );
      if (premium <= 0) return;
      const r = ctx.settle({
        legs: [
          {
            kind: 'money',
            from: ctx.accountOf(fill.party, ccy),
            to: ctx.accountOf(sell.party, ccy),
            // Treasury C1: what this money is to the party getting it — a premium is revenue.
            receipt: { of: 'sale' },
            ccy,
            amount: premium,
          },
        ],
        cause: 'corporateAction',
        reason: `${String(fill.party)} pays ${String(sell.party)} a premium for cover`,
      });
      if (r.outcome !== 'settled') return;
      const terms: PolicyTerms = {
        kind: POLICY_ROW,
        cover: share,
        to,
        discountedAt: family.value.id,
      };
      ctx.owes({
        debtor: sell.party,
        creditor: fill.party,
        ccy,
        // A2: it owes nothing NOW; what it owes is what a loss makes a claim for (14.3).
        owed: 0,
        terms,
        why: `${String(sell.party)} covers ${String(fill.party)} for ${String(share)} until ${String(to.y)}-${String(to.m)}-${String(to.d)}`,
      });
      left = subQty(left, share, 'what is left of the fill');
      written += 1;
    });
  }
  ctx.record(
    'cover.cleared',
    [],
    { ccy, outcome: outcome.kind, price: outcome.price, bids: bidders, asks: askers, written },
    true,
  );
};

/** Clearing D2: who was on the other side of this fill, from the book's own record of it. */
/**
 * Insurers A4, B4, Law 5 (14.3): THE CLAIM. What the weather took from a covered party this period,
 * at what its books carried it, is paid by the insurers whose cover it holds — oldest cover first,
 * up to the cover it holds — and the cover used is handed back in the same numbered instruction: a
 * unit of cover promised a unit of money against a loss, the loss came, the unit is paid and gone.
 * A claim the insurer cannot pay fails on the wire like any payment and is an arrear ranking as a
 * policyholder's (`register/arrears.ts`), which is how an insurer dies of a storm (A3, B4). One
 * event hitting many policies at once is one period's `capital.weathered` list, and every line of
 * it reaches here.
 */
function payClaims(ctx: MechanismContext): void {
  for (const loss of weatheredIn(ctx.journal, ctx.period)) {
    const holder = ctx.parties.resolve(loss.holder as PartyId);
    if (!holder.status.alive) continue;
    const ccy = ctx.registry.currencyOf(holder.region);
    let unpaid = ctx.registry.cashFor(
      asCash(loss.atCost, ccy, 'what the lost plant was on its books at'),
    );
    if (unpaid <= 0) continue;
    // Oldest cover first: the rows it holds, in the order they were opened.
    const rows = ctx.agreements
      .owedTo(holder.id)
      .filter(
        (a) =>
          a.state === 'performing' && a.ccy === ccy && isPolicyTerms(a.terms) && a.terms.cover > 0,
      )
      .sort((a, b) => a.since - b.since);
    for (const row of rows) {
      if (unpaid <= 0) break;
      const terms = row.terms;
      if (!isPolicyTerms(terms)) continue;
      const insurer = ctx.parties.get(row.debtor);
      if (!insurer.status.alive) continue;
      // COVER counts in the money's pieces: a unit of cover pays a unit of money.
      const units = atMost(
        terms.cover,
        unpaid,
        'it pays no more than the cover it holds, and no more than was lost',
      );
      if (units <= 0) continue;
      const r = ctx.settle({
        legs: [
          {
            kind: 'money',
            from: ctx.accountOf(insurer.id, ccy),
            to: ctx.accountOf(holder.id, ccy),
            receipt: { of: 'claim' },
            ccy,
            amount: units,
          },
        ],
        cause: 'corporateAction',
        reason: `${String(insurer.id)} pays ${String(holder.id)}'s claim on ${loss.vintage}`,
      });
      ctx.record(
        CLAIM_PAID,
        [insurer.id, holder.id],
        {
          insurer: insurer.id,
          holder: holder.id,
          policy: row.id,
          vintage: loss.vintage,
          capitalKind: loss.capitalKind,
          loss: loss.atCost,
          amount: units,
          paid: r.outcome === 'settled',
        },
        true,
      );
      if (r.outcome !== 'settled') break;
      // The cover used is gone: the row carries what is left, and a row with nothing left ends.
      const left = subQty(terms.cover, units, 'the cover left on the row');
      const restated: PolicyTerms = {
        kind: POLICY_ROW,
        cover: left,
        to: terms.to,
        discountedAt: terms.discountedAt,
      };
      if (left > 0) ctx.restate(row.id, restated);
      else ctx.endAgreement(row.id, 'the cover was paid out in full');
      unpaid = subQty(unpaid, units, 'what is still uncovered');
    }
  }
}

/** B1 (14.5): a term that has ended promises nothing; the row ends the period its day passes. */
function expireCover(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const row of ctx.agreements.ofKind(POLICY_ROW)) {
    if (row.state !== 'performing' || !isPolicyTerms(row.terms)) continue;
    if (compareCivil(row.terms.to, today) <= 0) ctx.endAgreement(row.id, 'the term ended');
  }
}

/**
 * B1, B2, B2.b: WHAT A UNIT OF COVER PROMISES. One payment, at the end of the term the exchange
 * writes cover for, discounted at the sovereign curve of its own money — which is what gives the
 * sector its duration (B2.b: a liability with no schedule is a cash balance).
 *
 * The TERM is a convention of the policy and not a forecast of when a claim arrives: what a claim
 * costs is `claimsSeen`, read off what this insurer has actually paid, and it is the PRICE that
 * carries it (A4.c). Nothing here draws from a distribution anybody stated.
 */

/** Law 9: a policy is named for who wrote it, for whom, and when. An id is never a display name. */
