/**
 * The treasury: what it owes, what it collects, and the constraint that it must raise money before
 * it spends it.
 *
 * @spec Treasury D6 Sovereign A1.c Sovereign A3.b Treasury A1 Treasury A1.a Treasury A2 Treasury A3 Treasury A3.a Treasury B1 Treasury B2 Treasury B4 Treasury C1 Treasury C1.a Treasury C3 Treasury D1 Treasury D2 Treasury D2.a Treasury D3 Treasury D4 Treasury D4.a Treasury D4.b Treasury D5 Treasury D5.a Treasury E1 Treasury E2 Treasury E3 Sovereign A1 Sovereign A1.a Sovereign A1.b Sovereign A2 Sovereign A2.a Sovereign A2.b Sovereign A2.c Sovereign A3 Sovereign A3.a Sovereign B3.a Sovereign C1 Sovereign C1.a Sovereign C1.b Sovereign C5 Sovereign C7 Sovereign F4 Sovereign F5 XI-9
 *
 * The programme is a READ, recomputed every period and journalled, never stored: what falls due
 * over its own horizon out of the paper it has actually issued (D4.a), what its standing mandate
 * pays out, what it collected last period, and the buffer it wants (D4.b). The difference is the
 * need (D1), and it is raised by ANNOUNCING an offer before the session (Sovereign C1) — a size and
 * a walk-away, never a price: the treasury chooses the size and the maturity, the market chooses
 * the price (D2.a).
 *
 * There is no overdraft anywhere in here (D3, Sovereign A3.b): an outlay the account cannot meet
 * FAILS, is journalled as a shortfall, and the next programme sees it. That is the constraint the
 * whole system hangs on (XI-9), and it is what makes a failed auction cost something.
 */
import { about } from '../../world/context.js';
import { fxPairId } from '../../core/ids.js';
import {
  atMostCash,
  negated,
  noCash,
  sumCash,
  type Cash,
  type PerPiece,
  type Ratio,
  acrossMembers,
  amountOf,
  asAmount,
  asCash,
  asPerMember,
  asRatio,
  asTotal,
  eachMember,
  heldAsMoney,
  minus,
  over,
  plus,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import { assertNever } from '../../core/assert.js';
import type { Civil } from '../../calendar/civil.js';
import { addMonths, compareCivil, formatCivil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { period, type Period } from '../../calendar/calendar.js';
import { Impossible, Missing } from '../../core/errors.js';
import {
  currencyUnit,
  instrumentId,
  marketId,
  moneyInstrumentId,
  paramId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
} from '../../core/ids.js';
import { addTo, atLeast, combineDust, div, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../../core/rate.js';
import { curveFamilyOf, priceAt } from '../../prices/curve.js';
import { struckIn } from '../../prices/price-store.js';
import { isAssetLeg, isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import { displayName } from '../../registry/naming.js';
import { weightOf, type CellParty, gridPerMember } from '../../parties/party.js';
import { HOUSEHOLD, TREASURY } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { Family, Violation } from '../../audit/audit.js';
import { issuedBy, type Instrument } from '../../register/instruments.js';
import { isArrear } from '../../register/arrears.js';
import { failedWhy, sovereignIn } from '../../world/failure.js';
import type { SystemModule } from '../../world/module.js';
import type { Order } from '../../clearing/solver.js';
import type { MarketDecl } from '../../clearing/market.js';
import {
  SOVEREIGN_BILL,
  SOVEREIGN_BOND,
  type SovereignBillTerms,
  type SovereignBondTerms,
} from '../../registry/claims.js';
import { findVenue } from '../../clearing/venue.js';
import { isGoodTerms } from '../../registry/physical.js';
import {
  GRID_DAY,
  GRID_MONTHS,
  LONG_TENORS,
  MONTHS_PER_YEAR,
  PROCUREMENT,
  PUBLIC_OCCUPATION,
  SHORT_TENORS,
  TENOR_WINDOW_YEARS,
} from './data.js';
import { downTick, type Qty, upTick } from '../../core/tick.js';
import { ownPayrollOf, wageFacingParty, wholePeople } from '../../registry/wages.js';
import { netChange } from '../../register/employment.js';

/** The name of the store a treasury's programme phase leaves its need in (declared in the nouns). */
export const PROGRAMME = 'treasury.programme.need';

/** Sovereign D1, Law 8: what this treasury needs this period, and the period it reckoned it in. */
export interface ProgrammeNeed {
  at: Period | undefined;
  /** Negative is what it has OVER, which is what it can buy its own paper back with (D4.b). */
  /** Missing until a programme has run this period (16.0): a need is money of a named currency. */
  need: Cash | undefined;
}

/** An empty slot: a treasury that has published no programme this period has no period and no need. */
export const nothingNeeded = (): ProgrammeNeed => ({ at: undefined, need: undefined });

export const TREASURY_PARAMS = {
  bufferPeriods: paramId('treasury.buffer.periods'),
  horizon: paramId('treasury.programme.horizon'),
  tenorMixShort: paramId('treasury.tenorMix.short'),
  auctionEvery: paramId('treasury.auction.everyPeriods'),
  transfers: paramId('treasury.outlays.transfers.perMember'),
  publicService: paramId('treasury.publicService.hours'),
  purchases: paramId('treasury.purchases.perPeriod'),
  taxInterest: paramId('treasury.tax.interestIncome'),
  taxIncome: paramId('treasury.tax.income'),
  taxConsumption: paramId('treasury.tax.consumption'),
  concession: paramId('treasury.walkAway.concession'),
  dealershipShare: paramId('sovereign.primaryDealers.minBidShare'),
  buybackStale: paramId('treasury.buyback.stalePeriods'),
} as const;

interface Line {
  readonly id: string;
  readonly maturity: Civil;
  /** Units of par in issue on this line. A count, so a tenor mix is a share of two of them. */
  readonly issued: Qty;
  readonly tenorYears: number;
  readonly short: boolean;
}

/**
 * Sovereign D3, D3.c, Law 4, Law 19: THE CONVENTION ITS OWN CURVE IS BUILT ON.
 *
 * Every consumer of a curve compounds and counts days the way that curve family says (D3.c), and a
 * treasury that pricing its own paper on a different one would be two conventions for one promise.
 * It is read off the family this issuer's own paper makes, which is where the declaration lives —
 * so this module no longer imports the module that declares it (`13b.1`), and an issuer whose
 * curve is a different convention gets that one without a line changing.
 */
function dayCountOn(
  reads: Pick<MechanismContext, 'registry'>,
  issuer: PartyId,
  ccy: CurrencyCode,
): DayCount {
  return reads.registry.curveFamily(curveFamilyOf(issuer, ccy)).dayCount;
}

/** Every live line this issuer has out, with what it owes on it (E1: read from the register). */
function linesOf(ctx: MechanismContext, issuer: PartyId, on: Civil): Line[] {
  const out: Line[] = [];
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, issuer) || !i.status.live) continue;
    const flows = ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar, ctx.registry);
    const last = flows[flows.length - 1];
    if (last === undefined) continue;
    const tenorYears = yearFraction(dayCountOn(ctx, issuer, i.ccy), on, last.date);
    out.push({
      id: i.id,
      maturity: last.date,
      issued: i.issued,
      tenorYears,
      short: tenorYears <= 1,
    });
  }
  return out;
}

/** What the treasury must pay over its horizon out of its own paper (B2, B4, D4.a). */
function debtService(ctx: MechanismContext, issuer: PartyId, on: Civil, horizon: Period): Cash {
  // Currency B1, C4: what falls due across its lines, REPORTED in the money it reports in (16.4 will let it owe in another).
  const home = ctx.registry.currencyOf(ctx.parties.get(issuer).region);
  const terms: Cash[] = [];
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, issuer) || !i.status.live) continue;
    for (const f of ctx.registry
      .instrumentKind(i.kind)
      .cashFlows(i, on, ctx.calendar, ctx.registry)) {
      if (ctx.calendar.periodOf(f.date) <= horizon) {
        terms.push(
          ctx.valuation.inMoney(valueAt(f.perUnit, i.issued, i.ccy, 'service'), home, ctx.period),
        );
      }
    }
  }
  return sumCash(home, terms, 'what its debt asks').value;
}

/**
 * B1, B3: what the standing mandate costs a period — the transfers it pays every member of every
 * household, the wage bill of the people it employs, and the budget it buys real things with. The
 * wage bill is a READ of what it actually paid last period, not a rate on a headcount (B2's rule
 * applied to labour): what it owes its own staff is what its own rows say.
 */
function mandatePerPeriod(ctx: MechanismContext, id: PartyId): Cash {
  const ccy = ctx.registry.currencyOf(ctx.parties.get(id).region);
  const money = currencyUnit(ccy);
  const transfers = heldAsMoney(
    ctx.params.amount(TREASURY_PARAMS.transfers, money),
    ccy,
    'what it pays a member a period',
  );
  const terms: Cash[] = [
    heldAsMoney(ctx.params.amount(TREASURY_PARAMS.purchases, money), ccy, 'what it buys with'),
    lastWageBill(ctx, id, ccy),
  ];
  for (const p of itsPeople(ctx, id)) {
    terms.push(
      asCash(
        acrossMembers(
          asPerMember<'money:piece'>(transfers.pieces, 'what one member is paid'),
          p.weight,
          'transfers',
        ),
        ccy,
        'transfers',
      ),
    );
  }
  return sumCash(ccy, terms, 'what its mandate asks a period').value;
}

/** What its own payroll came to last time it was paid, read from its own record (Law 19). */
function lastWageBill(ctx: MechanismContext, id: PartyId, ccy: CurrencyCode): Cash {
  const own = ownPayrollOf(ctx, id, ctx.period, ccy);
  return own.some ? own.value.due : noCash(ccy);
}

/**
 * What an hour costs it: what its own payroll paid for one, or what its region last printed.
 *
 * This was `currentPayroll` + a copy of `wageFacing`, which made the file the THIRD writer of one
 * formula (Law 4, 0e′.1). Its fallback also took the last print ANYWHERE, so a state read another
 * country's wage whenever that country printed later; the registry's is its own region's.
 */
function wageItFaces(ctx: MechanismContext, id: PartyId): PerPiece | undefined {
  const region = ctx.parties.get(id).region;
  const facing = wageFacingParty(ctx, id, ctx.period, region);
  return facing.some ? facing.value : undefined;
}

/** What it collected last period, which is what it has to go on until it has an outlook (§46 C5). */
function lastReceipts(ctx: MechanismContext, ccy: CurrencyCode): Cash {
  const events = ctx.journal.ofKind('treasury.receipts');
  const last = events[events.length - 1];
  if (last === undefined) return noCash(ccy);
  const total = last.data['total'];
  // Item 16: money re-entering from what the treasury itself published.
  return typeof total === 'number'
    ? asCash(total, ccy, 'what it collected last period')
    : noCash(ccy);
}

/** The first grid date on or after a target date (B3.a: which line a tenor lands on). */
function gridDate(target: Civil): Civil {
  for (let year = target.y; year <= target.y + 1; year += 1) {
    for (const m of GRID_MONTHS) {
      const c: Civil = { y: year, m, d: GRID_DAY };
      if (compareCivil(c, target) >= 0) return c;
    }
  }
  throw new Impossible('Sovereign B3.a', 'no maturity grid date after the target', { target });
}

export const treasury: SystemModule = {
  // Money E1 (12a.9): a levy that fails is an ARREAR the payer issued to the state — settlement
  // writes it in the pass of the fail, ranked as tax in an estate (`register/arrears.ts`). The
  // agreement this module wrote beside it was the same debt twice (Law 4); the row is the read.
  agreementKinds: [],
  id: 'treasury',
  nouns: [
    {
      name: PROGRAMME,
      kind: 'working',
      holds: 'what each treasury reckoned it needs this period, and the period it reckoned it in',
      why: 'it is how this module gets from its programme phase to its own buy-in within one period (0e\u2032.4). What the WORLD reads is the published `treasury.programme`, which is still written and is the whole point of C1.a; this is the same number the same treasury reads about itself, and it was its own public event read back by its writer in the period it wrote it.',
    },
  ],
  spec: 'Treasury, Sovereign A, C, XI-9',
  requires: ['sovereign-instruments', 'sovereign-curve'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: TREASURY_PARAMS.bufferPeriods,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Treasury D4.b: it holds a cash buffer because the alternative is dependence on every single auction clearing. How many periods of known outlays it wants in hand is its own patience with that risk.',
    },
    {
      id: TREASURY_PARAMS.horizon,
      value: 26,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Treasury D4, D4.a: how far ahead the programme looks at what it must pay, so a wall is foreseeable and pre-funded rather than met on the day.',
    },
    {
      id: TREASURY_PARAMS.tenorMixShort,
      value: 0.35,
      unit: 'ratio of debt outstanding',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'model',
      why: 'Treasury E1, Sovereign A2.c: the maturity mix is a real choice with a real cost - short is cheaper when the curve slopes up and rolls more often. This is the share it manages towards.',
    },
    {
      id: TREASURY_PARAMS.auctionEvery,
      value: 4,
      unit: 'periods',
      dimension: 'periods',
      kind: 'policy',
      owner: 'model',
      why: 'Sovereign C1, C1.a: issuance is announced before it happens on a calendar the market can see. How often it comes is the issuer own choice.',
    },
    {
      id: TREASURY_PARAMS.transfers,
      value: 12,
      denominated: 'money',
      unit: 'of its own money, per member per period',
      dimension: 'amount',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury B1, B3: transfers to households. Until the polity exists this is the standing mandate declared at the seed, and the register prints parliament as its owner (Polity D5, XI-17).',
    },
    {
      id: TREASURY_PARAMS.publicService,
      value: 7000,
      denominated: 'time',
      unit: 'of the venue own time, per period',
      dimension: 'amount',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury B1, Labour F1: how big a public service the state keeps. It is a size, not a wage: the state posts these hours in the same venue everybody else posts in, at what the market has been paying, and what it ends up paying is what the venue cleared at. A state that stated the wage would be setting a price, which is not something a parliament does (Polity D5).',
    },
    {
      id: TREASURY_PARAMS.purchases,
      value: 30_000,
      denominated: 'money',
      unit: 'of its own money, per period',
      dimension: 'amount',
      kind: 'policy',
      owner: 'parliament',
      why: "Treasury B1: what the state puts aside to buy real things with. It is a budget and not a quantity: how much that buys is the market's business, and the state is rationed in it like any other buyer (Goods C4).",
    },
    {
      id: TREASURY_PARAMS.taxInterest,
      value: 0.2,
      unit: 'ratio of interest received',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1, C1.a: a rate on a real base with a named payer who remits it. Interest received is the only base that exists before firms and households earn anything (worklist 4).',
    },
    {
      id: TREASURY_PARAMS.taxIncome,
      value: 0.15,
      unit: 'ratio of what a household was paid',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1, C1.a: a tax on income, on the base the payer own statement gives — what actually reached a household from somebody other than the state. Its own transfers are not taxed back out of it, and interest is taxed where it is received by anybody, so no base carries two rates.',
    },
    {
      id: TREASURY_PARAMS.taxConsumption,
      value: 0.1,
      unit: 'ratio of what a household paid for goods',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1: a tax on consumption, on what a household actually paid a seller for real things. A household finds it on top of the price when it decides what to spend (Households C4), and it is remitted out of its own account.',
    },
    {
      id: TREASURY_PARAMS.concession,
      value: 0.004,
      unit: 'per annum over the curve',
      dimension: 'perAnnum',
      kind: 'preference',
      owner: 'model',
      why: 'Sovereign C5, C7: the issuer walk-away. How much worse than the curve it will still take before it withdraws the paper is its own patience, and it is what makes a failed auction reachable.',
    },
    {
      id: TREASURY_PARAMS.dealershipShare,
      value: 0.34,
      unit: 'ratio of the size offered',
      dimension: 'ratio',
      kind: 'policy',
      // The ISSUER's own term, and it is declared in the issuer's module and announced by it. The
      // owner enum has no treasury in it, and adding one is a kernel change this item may not make.
      owner: 'model',
      why: 'Sovereign C3, C3.a: primary dealers bid because they are obliged to, in exchange for privileges, and that obligation is what makes an auction hard to fail. It is a TERM OF THE DEALERSHIP and therefore the ISSUER own number — it announces it with the line, and a dealer reads it off the announcement rather than holding a copy. The issuer states a share such that its dealers between them cover what it brings: with three dealers in this world, a third each and a little over, so the announcement covers the size rather than falling just short of it. It does not make failure impossible, because a dealer bids out of the money it has.',
    },
    {
      id: TREASURY_PARAMS.buybackStale,
      value: 12,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Sovereign F5: the issuer manages its own curve by buying in an illiquid old line. How long a line must have gone without a trade before it counts as illiquid is its own judgement.',
    },
  ],
  phases: [
    {
      name: 'treasury.programme',
      spec: 'Treasury D1 Treasury D4 Sovereign A2 Sovereign C1',
      anchor: { after: 'corporateActions' },
      reads: [{ kind: 'event', name: 'treasury.receipts', of: 'anyPeriod' }],
      writes: [
        { kind: 'event', name: 'auction.announced' },
        { kind: 'event', name: 'treasury.programme' },
        { kind: 'event', name: 'treasury.excluded' },
        { kind: 'event', name: 'treasury.defaulted' },
      ],
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) runProgramme(ctx, t.id);
        }
      },
    },
    {
      name: 'treasury.outlays',
      spec: 'Treasury B1 Treasury A3.a Treasury D3',
      anchor: { after: 'treasury.programme' },
      // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
      // it — so what it reads is read off its module's source and not off a measurement, and
      // it is the module's whole read set rather than this phase's. It narrows the first time
      // the phase runs and the check can say which of these it actually wanted.
      reads: [
        { kind: 'event', name: 'auction.result', of: 'anyPeriod' },
        { kind: 'event', name: 'labour.print', of: 'anyPeriod' },
        { kind: 'event', name: 'treasury.programme', of: 'anyPeriod' },
        { kind: 'event', name: 'treasury.receipts', of: 'anyPeriod' },
      ],
      writes: [],
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) runOutlays(ctx, t.id);
        }
      },
    },
    {
      name: 'treasury.employment',
      spec: 'Treasury B1 Labour C5 Labour F1',
      // Before the jobs are struck: the state posts what it wants like any other employer, in the
      // same venue, and what it pays is what that venue cleared at.
      anchor: { before: 'labour.match' },
      reads: [{ kind: 'event', name: 'labour.print', of: 'anyPeriod' }],
      writes: [],
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) postPublicService(ctx, t.id);
        }
      },
    },
    {
      name: 'treasury.receipts',
      spec: 'Treasury C1 Treasury C1.a Treasury C3',
      anchor: { after: 'treasury.outlays' },
      reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
      writes: [
        { kind: 'event', name: 'credit.declined' },
        { kind: 'event', name: 'treasury.receipts' },
      ],
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) runReceipts(ctx, t.id);
        }
      },
    },
  ],
  participants: [
    {
      partyKind: TREASURY,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => [
        ...buyback(view, m),
        ...procure(view, m),
      ],
    },
  ],
  families: [allotmentReconciles()],
  // Sovereign G3 (12a.6): THERE IS NO ESTATE for a sovereign — nothing to seize. A treasury that
  // fails in a money it does not issue is resolved here: it is named in default, it is out of the
  // market while the row stands (G5), and what it owes stays owed. The exchange offer (G4) is 17.
  resolves: [TREASURY],
};

/**
 * Treasury D6: what the issuer said it placed and what the register issued are two records of one
 * fact, written by two different writers — the market's own report of the session, and settlement.
 * They reconcile to dust or somebody's debt is not what the auction said it was.
 */
function allotmentReconciles(): Family {
  return {
    name: 'flows',
    contributor: 'treasury',
    spec: 'Treasury D6 Sovereign C5 Sovereign C6',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const issued = new Map<string, Qty[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled' || r.instruction.cause !== 'issuance') continue;
        for (const d of r.deltas) {
          if (d.target !== 'issued') continue;
          const list = issued.get(d.instrument) ?? [];
          list.push(d.qty);
          issued.set(d.instrument, list);
        }
      }
      for (const e of view.journal.ofKind('auction.result')) {
        if (e.period !== view.period) continue;
        const line = String(e.data['line']);
        const allotted = e.data['allotted'];
        if (typeof allotted !== 'number') continue;
        const registered = sum(issued.get(line) ?? []);
        const claimed = sum([asAmount<'piece'>(allotted, 'what the auction placed')]);
        if (!withinDust(registered.value, claimed.value, combineDust(registered, claimed))) {
          out.push({
            family: 'flows',
            spec: 'Treasury D6',
            owner: line,
            size: minus(registered.value, claimed.value, 'allotment gap'),
            unit: 'units of par',
            period: view.period,
            message: `${line}: the auction placed ${allotted} but the register issued ${registered.value}`,
          });
        }
      }
      return out;
    },
  };
}

/** D1, D4: the need, then the announcement (C1). Everything here is a read. */
function runProgramme(ctx: MechanismContext, id: PartyId): void {
  const on = ctx.calendar.startOf(ctx.period);
  const horizonPeriods = ctx.params.periods(TREASURY_PARAMS.horizon);
  const horizon = period(ctx.period + horizonPeriods);
  const ccy = ctx.registry.currencyOf(ctx.parties.get(id).region);
  const service = debtService(ctx, id, on, horizon);
  const perPeriod = mandatePerPeriod(ctx, id);
  const mandate = scale(
    perPeriod,
    asRatio(horizonPeriods, 'the periods ahead'),
    'mandate over horizon',
  );
  const receipts = scale(
    lastReceipts(ctx, ccy),
    asRatio(horizonPeriods, 'the periods ahead'),
    'receipts over horizon',
  );
  const buffer = scale(
    perPeriod,
    asRatio(ctx.params.periods(TREASURY_PARAMS.bufferPeriods), 'the periods of buffer it keeps'),
    'buffer',
  );
  const cash = heldAsMoney(
    ctx.register.quantity(id, accountOf(ctx, id, ccy)),
    ccy,
    'what is in its account',
  );
  const need = minus(
    plus(plus(service, mandate, 'outlays'), buffer, 'with buffer'),
    plus(receipts, cash, 'resources'),
    'need',
  );
  const auctionEvery = ctx.params.periods(TREASURY_PARAMS.auctionEvery);
  const isAuctionPeriod = ctx.period % auctionEvery === 0;
  const auctions = Math.floor(horizonPeriods / auctionEvery);
  const size =
    auctions > 0 && need.pieces > 0
      ? over(need, asRatio(auctions, 'the auctions it will hold'), 'auction size')
      : noCash(ccy);

  // Sovereign G2, G3, G5 (12a.6): a treasury IN DEFAULT is EXCLUDED FROM THE MARKET, not
  // liquidated — there is no estate and nothing to seize. A default is a miss in a money it does
  // not issue (G2); in its own the miss is a shortfall (G1, Treasury D3) and it keeps funding
  // itself. While such an arrear stands it brings nothing: the rows it owes are read off the
  // register (Law 19), and what it still needs is on the programme for everybody to see. This
  // module is what resolves it — by saying so, and by standing out.
  const standing = arrearsStanding(ctx, id);
  const excluded = standing.length > 0;
  if (excluded) {
    ctx.record(
      'treasury.excluded',
      [id],
      { arrears: standing.map((a) => ({ instrument: a.id, ccy: a.ccy, owed: a.issued })) },
      true,
    );
    const why = failedWhy(ctx, ctx.participant(id));
    if (why !== undefined) ctx.record('treasury.defaulted', [id], { why }, true);
  }
  const planned =
    isAuctionPeriod && size.pieces > 0 && !excluded
      ? announce(ctx, id, ccy, size, on)
      : none<string>();
  // Law 15, 0e′.4: what it needs goes in this treasury's own working store, which is what its own
  // buy-in reads back this period. The event below is the PUBLIC programme (C1.a) — the whole point
  // of it is that everybody can see it — and it is written from the same number.
  const slot = ctx.workingOf(id, PROGRAMME, nothingNeeded);
  slot.at = ctx.period;
  slot.need = need;
  ctx.record(
    'treasury.programme',
    [id],
    {
      ccy,
      need: need.pieces,
      service: service.pieces,
      mandate: mandate.pieces,
      expectedReceipts: receipts.pieces,
      buffer: buffer.pieces,
      cash: cash.pieces,
      horizonPeriods,
      auctionEvery,
      plannedSize: planned.some ? size.pieces : 0,
      line: planned.some ? planned.value : null,
    },
    true,
  );
}

/** Money E1, Sovereign G2 (12a.6): the arrears this treasury has not paid in a money it does not issue. */
function arrearsStanding(ctx: MechanismContext, id: PartyId): readonly Instrument[] {
  return [...ctx.instruments.issuedBy(id)].filter(
    (i) => i.status.live && isArrear(i) && !sovereignIn(ctx, id, i.ccy),
  );
}

/**
 * Sovereign A4.b, B2, G2, Cross-Border C2 (16.4): WHICH MONEY IT BORROWS IN, and it is a choice.
 *
 * In its own money it opens at its own curve. In another's it has no curve of its own to open at,
 * so it opens at THAT money's sovereign benchmark (Sovereign D4: other credit is a spread to it, and
 * a debut has no spread yet) — and what that borrowing COSTS it is the benchmark yield plus what it
 * expects its own money to do against that one over a year, read off its own outlook of the pair's
 * print (§46 A2.a). A treasury with no view of a pair has no number to compare and stays at home
 * (App A). It borrows where it reads the cost lowest; C2 says why a state does, and A4.b says what it
 * has then taken on: a debt in a money it cannot create, which is the whole of its credit risk.
 */
function fundingCostIn(
  ctx: MechanismContext,
  id: PartyId,
  home: CurrencyCode,
  money: CurrencyCode,
  on: Civil,
  maturity: Civil,
): Option<{ readonly cost: Ratio; readonly opensAt: Ratio }> {
  if (money === home) {
    const reading = ctx.curve(curveFamilyOf(id, home)).at(yearFraction(dayCountOn(ctx, id, home), on, maturity));
    return reading.yield.some ? some({ cost: reading.yield.value, opensAt: reading.yield.value }) : none();
  }
  const benchmark = ctx.sovereignCurveIn(money);
  if (!benchmark.some) return none();
  const reading = ctx.curve(benchmark.value.id).at(yearFraction(benchmark.value.dayCount, on, maturity));
  if (!reading.yield.some) return none();
  const pair = fxPairId(home, money);
  const view = ctx.participant(id);
  const outlook = view.outlook(about({ on: 'price', instrument: pair }));
  if (!outlook.some || outlook.value.expected <= 0) return none();
  const now = ctx.valuation.rateInForce(home, money, ctx.period);
  // What one home buys of `money` now over what it expects next period, per year: a home expected to
  // weaken makes a debt in `money` dearer to service by that much (Cross-Border B3, A2.a).
  const ofAYear = yearFraction(benchmark.value.dayCount, on, ctx.calendar.startOf(period(ctx.period + 1)));
  if (ofAYear <= 0) return none();
  const move = div(now / outlook.value.expected - 1, ofAYear, 'what it expects its money to do against that one, a year');
  return some({
    cost: asRatio(reading.yield.value + move, `what borrowing in ${String(money)} costs it, a year`),
    opensAt: reading.yield.value,
  });
}

/** Sovereign C1: announce a size on a line, at a walk-away the curve gives it (C5). */
function announce(
  ctx: MechanismContext,
  id: PartyId,
  home: CurrencyCode,
  need: Cash,
  on: Civil,
): Option<InstrumentId> {
  const lines = linesOf(ctx, id, on);
  const shortOut = sum(lines.filter((l) => l.short).map((l) => l.issued)).value;
  const total = sum(lines.map((l) => l.issued)).value;
  const wantShort =
    total === 0 ||
    ratioOf(shortOut, total, 'short share') < ctx.params.ratio(TREASURY_PARAMS.tenorMixShort);
  const tenors = wantShort ? SHORT_TENORS : LONG_TENORS;
  // Within the bucket it brings the tenor it has least of, which is how a maturity profile stays
  // spread instead of piling into one date (D4.a).
  let target: number | undefined;
  let least: number | undefined;
  for (const tenor of tenors) {
    const outstanding = sum(
      lines.filter((l) => Math.abs(l.tenorYears - tenor) < TENOR_WINDOW_YEARS).map((l) => l.issued),
    ).value;
    if (least === undefined || outstanding < least) {
      least = outstanding;
      target = tenor;
    }
  }
  if (target === undefined) {
    throw new Impossible('Sovereign A2.c', 'the maturity mix names no tenor to bring');
  }
  const maturity = gridDate(addMonths(on, monthsOf(target)));
  // 16.4: the money it borrows in is the one it reads the cost lowest in — its own by default.
  let ccy: CurrencyCode = home;
  let chosen: { readonly cost: Ratio; readonly opensAt: Ratio } | undefined;
  for (const money of ctx.registry.currencies.keys()) {
    const cost = fundingCostIn(ctx, id, home, money, on, maturity);
    if (!cost.some) continue;
    if (chosen === undefined || cost.value.cost < chosen.cost) {
      chosen = cost.value;
      ccy = money;
    }
  }
  if (chosen === undefined) {
    // Nothing has printed anywhere on any curve it could open at, so there is no level to walk
    // away from. The treasury does not invent one: it brings nothing this period and says so.
    ctx.record('treasury.noCurve', [id], { reason: 'no point on the curve' }, true);
    return none<InstrumentId>();
  }
  const benchmark = ctx.sovereignCurveIn(ccy);
  const dayCount = ccy === home || !benchmark.some ? dayCountOn(ctx, id, home) : benchmark.value.dayCount;
  // Currency C4: the need is in its own money; what it must RAISE in another is that at the rate in
  // force — a size, and the proceeds land in its account in that money (A3, B2.a).
  const size = ctx.valuation.inMoney(need, ccy, ctx.period);
  const y = chosen.opensAt;
  const existing = lines.find((l) => compareCivil(l.maturity, maturity) === 0);
  const instrument =
    existing === undefined
      ? openLine(ctx, id, ccy, maturity, y, wantShort, dayCount)
      : instrumentId(existing.id);
  const inst = ctx.instruments.get(instrument);
  const flows = ctx.registry
    .instrumentKind(inst.kind)
    .cashFlows(inst, on, ctx.calendar, ctx.registry);
  const reservation = priceAt(
    flows,
    plus(
      asRatio(y, 'the yield it opens at'),
      ctx.params.perAnnum(TREASURY_PARAMS.concession),
      'walk-away yield',
    ),
    on,
    dayCount,
    'reservation',
  );
  // Law 8: it needs to raise a sum of money and it raises it by selling UNITS of a line, which are
  // indivisible — so what it brings is the whole units that sum comes to. Up, because the ask is
  // the money: an issue a fraction of a unit short of what the programme needs is short of it.
  const units = upTick(amountOf(size, reservation, 'units offered'));
  ctx.offer({
    market: marketOf(ctx, instrument),
    issuer: id,
    size: units,
    reservation: some(reservation),
    allotment: 'uniformPrice',
  });
  // C3: the announcement. What is brought, and what the dealership asks of the dealers who carry it
  // — the issuer's own term, published with the line so a dealer reads its obligation off the offer
  // rather than keeping a copy of a number that is not its to hold (Law 4).
  ctx.record(
    'auction.announced',
    [id, instrument],
    {
      line: instrument,
      size: units,
      dealershipShare: ctx.params.ratio(TREASURY_PARAMS.dealershipShare),
    },
    true,
  );
  return some<InstrumentId>(instrument);
}

/** A tenor in years placed on the calendar as months, so the grid date is found by date (G3.a). */
function monthsOf(tenorYears: number): number {
  return Math.round(mul(tenorYears, MONTHS_PER_YEAR, 'tenor in months'));
}

/**
 * A debut: a new line on the grid, with the coupon that makes it par at the curve (B3.a).
 *
 * Bond N5, N6: a coupon is a payment the ISSUER promises, so an issuer facing a curve below zero
 * brings a line that promises principal and nothing else rather than one on which its holders would
 * have to pay it. That is not a bound on the yield — the market still pays whatever it pays, and a
 * zero-coupon line above par IS a negative yield — it is what an issuer can actually promise.
 */
function openLine(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  maturity: Civil,
  y: number,
  short: boolean,
  // 16.4: the convention of the money it borrows in — its own curve's at home, the benchmark's abroad.
  dayCount: DayCount,
): InstrumentId {
  const on = ctx.calendar.startOf(ctx.period);
  const id = instrumentId(`${issuer}.${short ? 'bill.' : ''}${formatCivil(maturity)}`);
  const terms: SovereignBondTerms | SovereignBillTerms = short
    ? // A2.a (item 10b): on the same convention its coupon lines use, because it is the same
      // issuer quoting in the same money — and at this tenor the convention is part of the number.
      { kind: SOVEREIGN_BILL, issueDate: on, maturity, dayCount }
    : {
        kind: SOVEREIGN_BOND,
        /**
         * Sovereign A2, Law 6 (18a.4): THE COUPON IS WHAT THE CURVE SAYS, and it may be nothing.
         *
         * It reads as a floor — the thing Law 6 forbids — and it is standing for a real fact: an issuer does not promise to PAY somebody for lending to it, so a coupon below
         * zero is not a coupon. What happens where money is dear enough for the curve to go below
         * zero is that the issuer prints a ZERO coupon and the paper sells ABOVE PAR, which is the
         * lender paying for the privilege — a price, cleared in the auction, and not a promise.
         * So the zero is arithmetic about what a promise can BE and not a bound on an outcome —
         * which is why it stays where 18a.4 expected it to go, with its reason said properly.
         */
        coupon: rate(
          atLeast(y, 0, 'a promise to be PAID for borrowing is not a coupon; what it prints is a zero'),
          ANNUAL,
        ),
        couponPeriodicity: SEMI_ANNUAL,
        dayCount,
        issueDate: on,
        maturity,
      };
  const market = marketId(`mkt.${id}`);
  ctx.issue({
    id,
    kind: short ? SOVEREIGN_BILL : SOVEREIGN_BOND,
    issuer: some(issuer),
    ccy,
    terms,
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy,
    rationing: 'proRata',
  });
  return id;
}

function marketOf(ctx: MechanismContext, instrument: InstrumentId): MarketId {
  const inst = ctx.instruments.get(instrument);
  if (!inst.market.some) {
    throw new Missing('Clearing D1', `${instrument} names no market`, { instrument });
  }
  return inst.market.value;
}

/** A3: it has one account, and every payment leaves it. */
function accountOf(ctx: MechanismContext, party: PartyId, ccy: CurrencyCode): InstrumentId {
  return moneyInstrumentId(ctx.accountOf(party, ccy).issuer, ccy);
}

/**
 * A1, Polity A1, Currency A3: the households this state is the state OF — the ones booked in its
 * own region. A state's mandate reaches its own people and its taxes fall on them; a world with a
 * second sovereign in it is the place where that stops being a distinction without a difference,
 * because a treasury that paid every household everywhere would be paying the other one's people
 * in a money neither their bank nor they hold.
 */
function itsPeople(ctx: MechanismContext, id: PartyId): readonly CellParty[] {
  const region = ctx.parties.get(id).region;
  const its: CellParty[] = [];
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!p.status.alive || p.representation !== 'cell' || p.region !== region) continue;
    its.push(p);
  }
  return its;
}

/**
 * B1: each outlay reaches a named recipient's account, one instruction each (A1.b). The transfers
 * are here; the wage bill is not, because the people it employs are on employment rows and the
 * module that owns those rows is the one writer of what they are paid (Labour F1, Law 4).
 */
function runOutlays(ctx: MechanismContext, id: PartyId): void {
  const ccy = ctx.registry.currencyOf(ctx.parties.get(id).region);
  const transfers = ctx.params.amount(TREASURY_PARAMS.transfers, currencyUnit(ccy));
  let paid = noCash(ccy);
  let short = noCash(ccy);
  for (const p of itsPeople(ctx, id)) {
    // Law 8, XI-15: each member of the cell is paid a whole number of the smallest piece of the
    // money, so what the mandate actually costs is that times the weight — and the fraction below
    // a piece is not paid, because it is not money.
    const share = gridPerMember(ctx.registry, p, transfers);
    const perMember = share.perMember;
    if (perMember <= 0) continue;
    const total = share.total;
    const leg: Leg = {
      kind: 'money',
      from: ctx.accountOf(id, ccy),
      to: ctx.accountOf(p.id, ccy),
      // C1, B2: THE STATE DOES NOT TAX BACK THE TRANSFER IT JUST PAID. It was a comment above
      // `runReceipts` and a `from.holder !== id` test; it is now what the payment SAYS it is, so it
      // holds for any transfer from anywhere and not only for this treasury's own.
      receipt: { of: 'transfer' },
      ccy,
      amount: total,
    };
    const r = ctx.settle({
      legs: [leg],
      cause: 'transfer',
      reason: `standing mandate to ${p.id}`,
    });
    if (r.outcome === 'settled') paid = plus(paid, heldAsMoney(total, ccy, 'what it paid'), 'paid');
    else short = plus(short, heldAsMoney(total, ccy, 'what it could not pay'), 'short');
  }
  if (short.pieces > 0) {
    // A3.a, D5: the account was empty. Nothing advanced it (D3); the mandate simply went unpaid,
    // and the next programme sees a need that did not shrink.
    ctx.record('treasury.shortfall', [id], { unpaid: short.pieces, paid: paid.pieces, ccy }, true);
  }
}

/**
 * C1, C1.a, C2, C3: what it collected, from bases that are what the payers themselves did. Three
 * of them: interest anybody was paid, what a household was paid by anybody but the state, and what
 * a household paid for real things. Every one is read off last period's settled instructions — the
 * payer's own statement, not a proxy for it — and every one is remitted out of the payer's own
 * account, so receipts fall when income and spending fall because there is less there to read.
 *
 * A base carries one rate: the state does not tax back the transfer it just paid, and interest is
 * taxed where it is received rather than twice over as income as well.
 */
function runReceipts(ctx: MechanismContext, id: PartyId): void {
  const ccy = ctx.registry.currencyOf(ctx.parties.get(id).region);
  if (ctx.period === 0) return;
  const previous = period(ctx.period - 1);
  const onInterest = ctx.params.ratio(TREASURY_PARAMS.taxInterest);
  const onIncome = ctx.params.ratio(TREASURY_PARAMS.taxIncome);
  const onConsumption = ctx.params.ratio(TREASURY_PARAMS.taxConsumption);
  const cells = new Set(itsPeople(ctx, id).map((p) => p.id));
  const due = new Map<PartyId, number>();
  // Every base is MONEY — what was actually paid, in this treasury's own currency — so a rate can
  // never be added to one and a base can never be read as a rate (Law 8).
  const bases = {
    interest: noCash(ccy),
    income: noCash(ccy),
    consumption: noCash(ccy),
    unclassified: noCash(ccy),
  };
  for (const r of ctx.ledger.inPeriod(previous)) {
    if (r.outcome !== 'settled') continue;
    // C1: what a household bought in this instruction is what it paid for the real things in it.
    const buyers = new Set<PartyId>();
    for (const leg of r.instruction.legs) {
      if (!isAssetLeg(leg) || !cells.has(leg.to)) continue;
      const kind = ctx.registry.instrumentKind(ctx.instruments.get(leg.instrument).kind);
      if (kind.physical === true) buyers.add(leg.to);
    }
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg)) continue;
      /**
       * Law 8, Appendix B (13e): IN ITS OWN MONEY, and only in its own money. Every base here is a
       * sum of amounts and every amount carries a currency, so a leg in another one is not a
       * smaller or larger number — it is a different thing, and adding it is adding two currencies.
       *
       * Without this line every treasury walked EVERY settled leg in the world and billed the tax
       * in its own money: a dollar coupon paid in New York raised a euro assessment, a sterling one
       * and a yen one, all at once and all against a party that had never held any of them. Found
       * when a securitisation vehicle — the first party in this world that receives interest and
       * holds money in ONE currency — was billed four times, could not pay three of them, and died
       * of a cash failure it did not owe.
       */
      if (leg.ccy !== ccy) continue;
      if (buyers.has(leg.from.holder)) {
        bases.consumption = plus(
          bases.consumption,
          heldAsMoney(leg.amount, ccy, 'what moved'),
          'what households paid for goods',
        );
        addTo(
          due,
          leg.from.holder,
          scale(heldAsMoney(leg.amount, ccy, 'what it paid'), onConsumption, 'consumption tax')
            .pieces,
        );
      }
      if (leg.to.holder === id) continue;
      /**
       * C1, Law 15: THE BASE IS WHAT THE PAYER SAID THIS RECEIPT IS, dispatched, never inferred.
       *
       * It used to read `r.instruction.cause === 'coupon'` and treat EVERYTHING ELSE arriving at a
       * household as income — and `Cause` is nine SETTLEMENT labels for why bytes moved, not a tax
       * taxonomy. So a household paid income tax at the wage rate on gross share-sale proceeds, on a
       * maturing bill's principal, on a fund redemption, on a probate distribution and on a loan
       * drawdown. Borrowing was income (A-46, A-37).
       *
       * A receipt nobody classified is NOT TAXED and is counted, because taxing the unclassified at
       * the wage rate is the defect itself. `unclassified` is published in the receipts event, so
       * how much of this world's money movement still has no name is a number and not a silence.
       */
      const receipt = leg.receipt;
      if (receipt === undefined) {
        if (cells.has(leg.to.holder) && leg.from.holder !== id) {
          bases.unclassified = plus(
            bases.unclassified,
            heldAsMoney(leg.amount, ccy, 'what moved'),
            'received and unclassified',
          );
        }
        continue;
      }
      switch (receipt.of) {
        case 'interest': {
          bases.interest = plus(
            bases.interest,
            heldAsMoney(leg.amount, ccy, 'what moved'),
            'interest received',
          );
          addTo(
            due,
            leg.to.holder,
            scale(heldAsMoney(leg.amount, ccy, 'what it received'), onInterest, 'tax on interest')
              .pieces,
          );
          break;
        }
        case 'wage':
        case 'pension':
        case 'rent':
        case 'dividend': {
          if (!cells.has(leg.to.holder)) break;
          bases.income = plus(
            bases.income,
            heldAsMoney(leg.amount, ccy, 'what moved'),
            'what households were paid',
          );
          addTo(
            due,
            leg.to.holder,
            scale(heldAsMoney(leg.amount, ccy, 'what it was paid'), onIncome, 'income tax').pieces,
          );
          break;
        }
        case 'disposal':
          // Handled off `r.realised` below, because the gain needs the basis and the basis is
          // settlement's to publish, not the leg's (Law 19).
          break;
        case 'sale':
          // Revenue from selling what the seller MADE. What it nets to is a firm's profit, which is
          // a different tax on a different base, and this world has no mechanism for one: the
          // treasury taxes households (C1). Naming it is what keeps it out of `unclassified`
          // without pretending it is somebody's income (item 14 owns the corporate base).
          break;
        case 'returnOfCapital':
        case 'borrowing':
        case 'transfer':
        case 'tax':
        case 'claim':
        case 'contribution':
          // Its own money coming back, money it must repay, money the state itself moved, a tax
          // paid, an indemnity for what it lost (14.3), and what a payroll put into a pension fund
          // (14.6). None is income and the first three used to be taxed as one.
          break;
        default:
          assertNever(receipt, 'Treasury C1');
      }
    }
    /**
     * C1, Law 19: THE GAIN, from the pass that knows both halves. A disposal at or below what it
     * cost is not a gain, and there is no negative base here: what a realised LOSS does — carry
     * against other gains, or not — is a fiscal rule the polity owns (14), and inventing one to fill
     * the branch would be exactly the outcome-written-as-a-rule the method forbids.
     */
    for (const made of r.realised) {
      if (!cells.has(made.party)) continue;
      // Currency C4: a gain made in another money is taxed in this one at the rate in force.
      const gain = ctx.valuation.inMoney(
        minus(made.proceeds, made.basis, 'what it made on the sale'),
        ccy,
        ctx.period,
      );
      if (gain.pieces <= 0) continue;
      bases.income = plus(bases.income, gain, 'gains households realised');
      addTo(due, made.party, scale(gain, onIncome, 'tax on the gain').pieces);
    }
  }
  let collected = noCash(ccy);
  let unpaid = noCash(ccy);
  for (const [payer, total] of due) {
    const p = ctx.parties.get(payer);
    if (!p.status.alive || total <= 0) continue;
    // Law 8: what a payer can pay is a whole number of the smallest piece of the money, and for a
    // cell that is a whole number of pieces for each of its members. What the fraction below one
    // would have been is not collected — it is not money, so it was never owed.
    const share = gridPerMember(
      ctx.registry,
      p,
      eachMember(asTotal<'money:piece'>(total, 'what the cell owes'), weightOf(p), 'per member'),
    );
    if (share.total <= 0) continue;
    const leg: Leg = {
      kind: 'money',
      from: ctx.accountOf(payer, ccy),
      to: ctx.accountOf(id, ccy),
      // C1, Money E1 (12a.9): what this money IS — a tax — so the row settlement writes when it
      // does not arrive ranks where the law puts what the state is owed (XI-8).
      receipt: { of: 'tax' },
      ccy,
      amount: share.total,
    };
    const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `tax due from ${payer}` });
    // A payer that cannot pay its tax has not paid it: nothing advances it (Money E1, D3).
    if (r.outcome === 'settled') {
      collected = plus(collected, heldAsMoney(share.total, ccy, 'what was collected'), 'collected');
      continue;
    }
    // Money E1 (12a.3, 12a.9): the miss is the payer's arrear to the state, written by settlement in
    // the same pass; what is counted here is the measurement, not the claim.
    unpaid = plus(unpaid, heldAsMoney(share.total, ccy, 'what was not paid'), 'unpaid');
  }
  ctx.record(
    'treasury.receipts',
    [id],
    {
      ccy,
      total: collected.pieces,
      unpaid: unpaid.pieces,
      payers: due.size,
      bases: {
        interest: bases.interest.pieces,
        income: bases.income.pieces,
        consumption: bases.consumption.pieces,
        unclassified: bases.unclassified.pieces,
      },
    },
    true,
  );
}

/**
 * B1, Labour C5, F1: the state's own openings. It posts the hours its own policy says it keeps, at
 * what an hour has been costing it — or, when it has never employed anybody, at what the market
 * last printed for one. It never states the wage: a state that did would be setting a price, and
 * what it ends up paying is what the venue cleared at like everybody else.
 */
function postPublicService(ctx: MechanismContext, id: PartyId): void {
  const wage = wageItFaces(ctx, id);
  if (wage === undefined) return;
  const region = ctx.parties.get(id).region;
  const venue = findVenue(ctx.venues, { region, occupation: PUBLIC_OCCUPATION });
  if (venue === undefined) return;
  // Law 8: the hours it keeps, counted in the pieces the venue counts somebody's time in.
  const hours = ctx.params.amount(TREASURY_PARAMS.publicService, venue.unit);
  if (hours <= 0) return;
  // Labour C3, C5 (12b.2): the CHANGE against what it will have, like any employer; Law 8 (12b.5,
  // 12c.3): in whole people, like any employer's.
  const change = netChange(
    ctx.employment.by(id),
    PUBLIC_OCCUPATION,
    region,
    wholePeople(ctx, hours),
  );
  if (change === undefined) return;
  ctx.post(venue.id, {
    party: id,
    side: change.side,
    price: change.side === 'buy' ? wage : 'market',
    qty: change.qty,
  });
}

/**
 * B1, Goods C3: procurement. It puts a stated budget into the market for real things and takes
 * what that buys at the price the market makes — it is one buyer among the others, it is rationed
 * with them (C4), and it never states what a thing is worth.
 */
function procure(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const budget = heldAsMoney(
    view.params.amount(TREASURY_PARAMS.purchases, currencyUnit(m.ccy)),
    m.ccy,
    'the budget it buys real things with',
  );
  if (budget.pieces <= 0) return [];
  const terms = view.instruments.get(m.instrument).terms;
  const row = PROCUREMENT.find((p) => isGoodTerms(terms) && terms.subUnit === p.subUnit);
  if (row === undefined || !isGoodTerms(terms) || terms.region !== view.self.region) return [];
  const print = view.print(m.instrument);
  if (!print.some || print.value.price <= 0) return [];
  const spend = scale(budget, row.share, 'what it puts into this market');
  const ccy = view.registry.currencyOf(view.self.region);
  const cash = heldAsMoney(view.cash(ccy), ccy, 'what is in its account');
  // D1: it buys out of the balance it has, and an empty account buys nothing.
  const afford = atMostCash(spend, cash, 'it procures with the money in its account');
  // Law 8: a budget divided by a price is a fraction of a unit, and the state buys whole ones like
  // everybody else. Down: what it can afford never rounds up past the money it has.
  const qty = downTick(amountOf(afford, print.value.price, 'what the budget buys'));
  return qty > 0 ? [{ party: view.self.id, side: 'buy', price: print.value.price, qty }] : [];
}

/**
 * Sovereign F5: the issuer manages its own curve. A line nobody has traded for longer than its own
 * patience is one it will buy in, out of cash it holds above its buffer — and buying its own paper
 * is a redemption, which settlement already knows how to do.
 */
function buyback(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const inst = view.instruments.get(m.instrument);
  if (!issuedBy(inst, view.self.id)) return [];
  // It never bids in its own auction: the offer is the other side of this book (C1.b in spirit).
  if (view.offer(m.id).some) return [];
  const print = view.print(inst.id);
  if (!print.some) return [];
  const since = struckIn(print.value);
  if (
    sub(view.period, since, 'periods since a trade') <
    view.params.periods(TREASURY_PARAMS.buybackStale)
  )
    return [];
  // It buys in with money it does not need: the programme it published this period says whether it
  // has any. A treasury that is short does not buy its own paper back (A2, D4).
  const spare = sparePerProgramme(view);
  if (spare.pieces <= 0) return [];
  // Law 8: whole units of its own paper, out of money it does not need.
  const qty = downTick(amountOf(spare, print.value.price, 'units'));
  return qty > 0 ? [{ party: view.self.id, side: 'buy', price: print.value.price, qty }] : [];
}

/**
 * D1, D4.b: what it has over and above its own programme. The programme is published every period
 * (C1.a), so this is a read of its own announcement and not of anything private.
 */
function sparePerProgramme(view: ParticipantView): Cash {
  const ccy = view.registry.currencyOf(view.self.region);
  const nothing = noCash(ccy);
  const said = view.working(PROGRAMME, nothingNeeded);
  if (said.at !== view.period) return nothing;
  const need = said.need;
  // A need that is not negative is a treasury that needs what it has, and it buys nothing in.
  if (need === undefined || need.pieces >= 0) return nothing;
  const cash = heldAsMoney(view.cash(ccy), ccy, 'what is in its account');
  return atMostCash(
    negated(need, 'what it does not need'),
    cash,
    'it repays out of the money it has',
  );
}
