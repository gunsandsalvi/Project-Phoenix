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
import type { Civil } from '../../calendar/civil.js';
import { addMonths, compareCivil, formatCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period, type Period } from '../../calendar/calendar.js';
import { Impossible, Missing } from '../../core/errors.js';
import {
  instrumentId,
  marketId,
  moneyInstrumentId,
  paramId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
} from '../../core/ids.js';
import { add, addTo, combineDust, div, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../../core/rate.js';
import { curveFamilyOf, priceAt } from '../../prices/curve.js';
import { struckIn } from '../../prices/price-store.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import { isAssetLeg, isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import { displayName } from '../../registry/naming.js';
import { HOUSEHOLD, TREASURY } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { Family, Violation } from '../../audit/audit.js';
import { issuedBy } from '../../register/instruments.js';
import type { SystemModule } from '../../world/module.js';
import type { Order } from '../../clearing/solver.js';
import type { MarketDecl } from '../../clearing/market.js';
import {
  SOVEREIGN_BILL,
  SOVEREIGN_BOND,
  type SovereignBillTerms,
  type SovereignBondTerms,
} from '../sovereign-instruments/index.js';
import { CURVE_DAY_COUNT } from '../sovereign-curve/index.js';
import {
  GRID_DAY,
  GRID_MONTHS,
  LONG_TENORS,
  MONTHS_PER_YEAR,
  SHORT_TENORS,
  TENOR_WINDOW_YEARS,
} from './data.js';

export const TREASURY_PARAMS = {
  bufferPeriods: paramId('treasury.buffer.periods'),
  horizon: paramId('treasury.programme.horizon'),
  tenorMixShort: paramId('treasury.tenorMix.short'),
  auctionEvery: paramId('treasury.auction.everyPeriods'),
  transfers: paramId('treasury.outlays.transfers.perMember'),
  publicWages: paramId('treasury.outlays.publicWages.perMember'),
  taxInterest: paramId('treasury.tax.interestIncome'),
  taxIncome: paramId('treasury.tax.income'),
  taxConsumption: paramId('treasury.tax.consumption'),
  concession: paramId('treasury.walkAway.concession'),
  buybackStale: paramId('treasury.buyback.stalePeriods'),
} as const;

interface Line {
  readonly id: string;
  readonly maturity: Civil;
  readonly issued: number;
  readonly tenorYears: number;
  readonly short: boolean;
}

/** Every live line this issuer has out, with what it owes on it (E1: read from the register). */
function linesOf(ctx: MechanismContext, issuer: PartyId, on: Civil): Line[] {
  const out: Line[] = [];
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, issuer) || !i.status.live) continue;
    const flows = ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar);
    const last = flows[flows.length - 1];
    if (last === undefined) continue;
    const tenorYears = yearFraction(CURVE_DAY_COUNT, on, last.date);
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
function debtService(ctx: MechanismContext, issuer: PartyId, on: Civil, horizon: Period): number {
  const terms: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, issuer) || !i.status.live) continue;
    for (const f of ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar)) {
      if (ctx.calendar.place(f.date) <= horizon) terms.push(mul(f.perUnit, i.issued, 'service'));
    }
  }
  return sum(terms).value;
}

/** The standing mandate's outlay per period: every member of every household cell (B1, B3). */
function mandatePerPeriod(ctx: MechanismContext): number {
  const transfers = ctx.params.get(TREASURY_PARAMS.transfers);
  const wages = ctx.params.get(TREASURY_PARAMS.publicWages);
  const terms: number[] = [];
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!p.status.alive || p.representation !== 'cell') continue;
    terms.push(mul(p.weight, add(transfers, working(p.key.cohort) ? wages : 0, 'per member'), 'outlay'));
  }
  return sum(terms).value;
}

/** Public wages reach the working cohort; the retired draw a transfer (B1). A cohort is data. */
function working(cohort: string): boolean {
  return cohort === 'working';
}

/** What it collected last period, which is what it has to go on until it has an outlook (§46 C5). */
function lastReceipts(ctx: MechanismContext): number {
  const events = ctx.journal.ofKind('treasury.receipts');
  const last = events[events.length - 1];
  if (last === undefined) return 0;
  const total = last.data['total'];
  return typeof total === 'number' ? total : 0;
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
  id: 'treasury',
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
      kind: 'preference',
      owner: 'model',
      why: 'Treasury D4.b: it holds a cash buffer because the alternative is dependence on every single auction clearing. How many periods of known outlays it wants in hand is its own patience with that risk.',
    },
    {
      id: TREASURY_PARAMS.horizon,
      value: 26,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Treasury D4, D4.a: how far ahead the programme looks at what it must pay, so a wall is foreseeable and pre-funded rather than met on the day.',
    },
    {
      id: TREASURY_PARAMS.tenorMixShort,
      value: 0.35,
      unit: 'ratio of debt outstanding',
      kind: 'policy',
      owner: 'model',
      why: 'Treasury E1, Sovereign A2.c: the maturity mix is a real choice with a real cost - short is cheaper when the curve slopes up and rolls more often. This is the share it manages towards.',
    },
    {
      id: TREASURY_PARAMS.auctionEvery,
      value: 4,
      unit: 'periods',
      kind: 'policy',
      owner: 'model',
      why: 'Sovereign C1, C1.a: issuance is announced before it happens on a calendar the market can see. How often it comes is the issuer own choice.',
    },
    {
      id: TREASURY_PARAMS.transfers,
      value: 0.012,
      unit: 'PHX per member per period',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury B1, B3: transfers to households. Until the polity exists this is the standing mandate declared at the seed, and the register prints parliament as its owner (Polity D5, XI-17).',
    },
    {
      id: TREASURY_PARAMS.publicWages,
      value: 0.02,
      unit: 'PHX per working-age member per period',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury B1: public wages. Until labour exists (worklist 4) they reach the working cohort directly rather than through an employment relationship, which is why they are a mandate number and not a wage bill.',
    },
    {
      id: TREASURY_PARAMS.taxInterest,
      value: 0.2,
      unit: 'ratio of interest received',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1, C1.a: a rate on a real base with a named payer who remits it. Interest received is the only base that exists before firms and households earn anything (worklist 4).',
    },
    {
      id: TREASURY_PARAMS.taxIncome,
      value: 0.15,
      unit: 'ratio of what a household was paid',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1, C1.a: a tax on income, on the base the payer own statement gives — what actually reached a household from somebody other than the state. Its own transfers are not taxed back out of it, and interest is taxed where it is received by anybody, so no base carries two rates.',
    },
    {
      id: TREASURY_PARAMS.taxConsumption,
      value: 0.1,
      unit: 'ratio of what a household paid for goods',
      kind: 'policy',
      owner: 'parliament',
      why: 'Treasury C1: a tax on consumption, on what a household actually paid a seller for real things. A household finds it on top of the price when it decides what to spend (Households C4), and it is remitted out of its own account.',
    },
    {
      id: TREASURY_PARAMS.concession,
      value: 0.004,
      unit: 'per annum over the curve',
      kind: 'preference',
      owner: 'model',
      why: 'Sovereign C5, C7: the issuer walk-away. How much worse than the curve it will still take before it withdraws the paper is its own patience, and it is what makes a failed auction reachable.',
    },
    {
      id: TREASURY_PARAMS.buybackStale,
      value: 12,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Sovereign F5: the issuer manages its own curve by buying in an illiquid old line. How long a line must have gone without a trade before it counts as illiquid is its own judgement.',
    },
  ],
  phases: [
    {
      name: 'treasury.programme',
      spec: 'Treasury D1 Treasury D4 Sovereign A2 Sovereign C1',
      cycle: 0,
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) runProgramme(ctx, t.id);
        }
      },
    },
    {
      name: 'treasury.outlays',
      spec: 'Treasury B1 Treasury A3.a Treasury D3',
      cycle: 0,
      anchor: { after: 'treasury.programme' },
      run: (ctx: MechanismContext): void => {
        for (const t of ctx.parties.ofKind(TREASURY)) {
          if (t.status.alive) runOutlays(ctx, t.id);
        }
      },
    },
    {
      name: 'treasury.receipts',
      spec: 'Treasury C1 Treasury C1.a Treasury C3',
      cycle: 0,
      anchor: { after: 'treasury.outlays' },
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
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => buyback(view, m),
    },
  ],
  families: [allotmentReconciles()],
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
      const issued = new Map<string, number[]>();
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
        const claimed = sum([allotted]);
        if (!withinDust(registered.value, claimed.value, combineDust(registered, claimed))) {
          out.push({
            family: 'flows',
            spec: 'Treasury D6',
            owner: line,
            size: sub(registered.value, claimed.value, 'allotment gap'),
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
  const horizonPeriods = ctx.params.get(TREASURY_PARAMS.horizon);
  const horizon = period(ctx.period + horizonPeriods);
  const ccy = ctx.registry.region(ctx.parties.get(id).region).ccy;
  const service = debtService(ctx, id, on, horizon);
  const perPeriod = mandatePerPeriod(ctx);
  const mandate = mul(perPeriod, horizonPeriods, 'mandate over horizon');
  const receipts = mul(lastReceipts(ctx), horizonPeriods, 'receipts over horizon');
  const buffer = mul(perPeriod, ctx.params.get(TREASURY_PARAMS.bufferPeriods), 'buffer');
  const cash = ctx.register.quantity(id, accountOf(ctx, id, ccy));
  const need = sub(add(add(service, mandate, 'outlays'), buffer, 'with buffer'), add(receipts, cash, 'resources'), 'need');
  const auctionEvery = ctx.params.get(TREASURY_PARAMS.auctionEvery);
  const isAuctionPeriod = ctx.period % auctionEvery === 0;
  const auctions = Math.floor(horizonPeriods / auctionEvery);
  const size = auctions > 0 && need > 0 ? div(need, auctions, 'auction size') : 0;

  const planned = isAuctionPeriod && size > 0 ? announce(ctx, id, ccy, size, on) : none<string>();
  ctx.record(
    'treasury.programme',
    [id],
    {
      need,
      service,
      mandate,
      expectedReceipts: receipts,
      buffer,
      cash,
      horizonPeriods,
      auctionEvery,
      plannedSize: planned.some ? size : 0,
      line: planned.some ? planned.value : null,
    },
    true,
  );
}

/** Sovereign C1: announce a size on a line, at a walk-away the curve gives it (C5). */
function announce(
  ctx: MechanismContext,
  id: PartyId,
  ccy: CurrencyCode,
  size: number,
  on: Civil,
): Option<InstrumentId> {
  const lines = linesOf(ctx, id, on);
  const shortOut = sum(lines.filter((l) => l.short).map((l) => l.issued)).value;
  const total = sum(lines.map((l) => l.issued)).value;
  const wantShort = total === 0 || div(shortOut, total, 'short share') < ctx.params.get(TREASURY_PARAMS.tenorMixShort);
  const tenors = wantShort ? SHORT_TENORS : LONG_TENORS;
  // Within the bucket it brings the tenor it has least of, which is how a maturity profile stays
  // spread instead of piling into one date (D4.a).
  let target: number | undefined;
  let least: number | undefined;
  for (const tenor of tenors) {
    const outstanding = sum(
      lines
        .filter((l) => Math.abs(l.tenorYears - tenor) < TENOR_WINDOW_YEARS)
        .map((l) => l.issued),
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
  const curve = ctx.curve(curveFamilyOf(id, ccy));
  const reading = curve.at(yearFraction(CURVE_DAY_COUNT, on, maturity));
  if (!reading.yield.some) {
    // Nothing has printed anywhere on this curve, so there is no level to walk away from. The
    // treasury does not invent one: it brings nothing this period and says so.
    ctx.record('treasury.noCurve', [id], { reason: 'no point on the curve' }, true);
    return none<InstrumentId>();
  }
  const y = reading.yield.value;
  const existing = lines.find((l) => compareCivil(l.maturity, maturity) === 0);
  const instrument =
    existing === undefined
      ? openLine(ctx, id, ccy, maturity, y, wantShort)
      : instrumentId(existing.id);
  const inst = ctx.instruments.get(instrument);
  const flows = ctx.registry.instrumentKind(inst.kind).cashFlows(inst, on, ctx.calendar);
  const reservation = priceAt(
    flows,
    add(y, ctx.params.get(TREASURY_PARAMS.concession), 'walk-away yield'),
    on,
    CURVE_DAY_COUNT,
    'reservation',
  );
  ctx.offer({
    market: marketOf(ctx, instrument),
    issuer: id,
    size: div(size, reservation, 'units offered'),
    reservation,
    allotment: 'uniformPrice',
  });
  return some<InstrumentId>(instrument);
}

/** A tenor in years placed on the calendar as months, so the grid date is found by date (G3.a). */
function monthsOf(tenorYears: number): number {
  return Math.round(mul(tenorYears, MONTHS_PER_YEAR, 'tenor in months'));
}

/** A debut: a new line on the grid, with the coupon that makes it par at the curve (B3.a). */
function openLine(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  maturity: Civil,
  y: number,
  short: boolean,
): InstrumentId {
  const on = ctx.calendar.startOf(ctx.period);
  const id = instrumentId(`${issuer}.${short ? 'bill.' : ''}${formatCivil(maturity)}`);
  const terms: SovereignBondTerms | SovereignBillTerms = short
    ? { kind: SOVEREIGN_BILL, issueDate: on, maturity }
    : {
        kind: SOVEREIGN_BOND,
        coupon: rate(y, ANNUAL),
        couponPeriodicity: SEMI_ANNUAL,
        dayCount: CURVE_DAY_COUNT,
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
  const p = ctx.parties.get(party);
  return moneyInstrumentId(p.bank, ccy);
}

/** B1: each outlay reaches a named recipient's account, one instruction each (A1.b). */
function runOutlays(ctx: MechanismContext, id: PartyId): void {
  const ccy = ctx.registry.region(ctx.parties.get(id).region).ccy;
  const transfers = ctx.params.get(TREASURY_PARAMS.transfers);
  const wages = ctx.params.get(TREASURY_PARAMS.publicWages);
  let paid = 0;
  let short = 0;
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!p.status.alive || p.representation !== 'cell') continue;
    const perMember = add(transfers, working(p.key.cohort) ? wages : 0, 'per member');
    if (perMember <= 0) continue;
    const total = totalFor(p, perMember);
    const leg: Leg = {
      kind: 'money',
      from: { holder: id, issuer: ctx.parties.get(id).bank },
      to: { holder: p.id, issuer: p.bank },
      ccy,
      amount: total,
      fromCell: none(),
      toCell: some(cellSide(p, perMember) ?? { perMember, weight: p.weight }),
    };
    const r = ctx.settle({
      legs: [leg],
      cause: 'transfer',
      reason: `standing mandate to ${p.id}`,
    });
    if (r.outcome === 'settled') paid = add(paid, total, 'paid');
    else short = add(short, total, 'short');
  }
  if (short > 0) {
    // A3.a, D5: the account was empty. Nothing advanced it (D3); the mandate simply went unpaid,
    // and the next programme sees a need that did not shrink.
    ctx.record('treasury.shortfall', [id], { unpaid: short, paid }, true);
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
  const ccy = ctx.registry.region(ctx.parties.get(id).region).ccy;
  if (ctx.period === 0) return;
  const previous = period(ctx.period - 1);
  const onInterest = ctx.params.get(TREASURY_PARAMS.taxInterest);
  const onIncome = ctx.params.get(TREASURY_PARAMS.taxIncome);
  const onConsumption = ctx.params.get(TREASURY_PARAMS.taxConsumption);
  const cells = new Set(ctx.parties.ofKind(HOUSEHOLD).map((p) => p.id));
  const due = new Map<PartyId, number>();
  const bases = { interest: 0, income: 0, consumption: 0 };
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
      if (buyers.has(leg.from.holder)) {
        bases.consumption = add(bases.consumption, leg.amount, 'what households paid for goods');
        addTo(due, leg.from.holder, mul(leg.amount, onConsumption, 'consumption tax'));
      }
      if (leg.to.holder === id) continue;
      if (r.instruction.cause === 'coupon') {
        bases.interest = add(bases.interest, leg.amount, 'interest received');
        addTo(due, leg.to.holder, mul(leg.amount, onInterest, 'tax on interest'));
      } else if (cells.has(leg.to.holder) && leg.from.holder !== id) {
        bases.income = add(bases.income, leg.amount, 'what households were paid');
        addTo(due, leg.to.holder, mul(leg.amount, onIncome, 'income tax'));
      }
    }
  }
  let collected = 0;
  let unpaid = 0;
  for (const [payer, total] of due) {
    const p = ctx.parties.get(payer);
    if (!p.status.alive || total <= 0) continue;
    const perMember = p.representation === 'cell' ? div(total, p.weight, 'per member') : total;
    const leg: Leg = {
      kind: 'money',
      from: { holder: payer, issuer: p.bank },
      to: { holder: id, issuer: ctx.parties.get(id).bank },
      ccy,
      amount: total,
      fromCell: p.representation === 'cell' ? some({ perMember, weight: p.weight }) : none(),
      toCell: none(),
    };
    const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `tax due from ${payer}` });
    // A payer that cannot pay its tax has not paid it: nothing advances it (Money E1, D3).
    if (r.outcome === 'settled') collected = add(collected, total, 'collected');
    else unpaid = add(unpaid, total, 'unpaid');
  }
  ctx.record(
    'treasury.receipts',
    [id],
    { total: collected, unpaid, payers: due.size, bases },
    true,
  );
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
  if (sub(view.period, since, 'periods since a trade') < view.params.get(TREASURY_PARAMS.buybackStale)) return [];
  // It buys in with money it does not need: the programme it published this period says whether it
  // has any. A treasury that is short does not buy its own paper back (A2, D4).
  const spare = sparePerProgramme(view);
  if (spare <= 0) return [];
  const qty = div(spare, print.value.price, 'units');
  return qty > 0 ? [{ party: view.self.id, side: 'buy', price: print.value.price, qty }] : [];
}

/**
 * D1, D4.b: what it has over and above its own programme. The programme is published every period
 * (C1.a), so this is a read of its own announcement and not of anything private.
 */
function sparePerProgramme(view: ParticipantView): number {
  const published = view.lastPublic('treasury.programme');
  if (!published.some || published.value.period !== view.period) return 0;
  if (!published.value.subjects.includes(view.self.id)) return 0;
  const need = published.value.data['need'];
  if (typeof need !== 'number' || need >= 0) return 0;
  const ccy = view.registry.region(view.self.region).ccy;
  const cash = view.cash(ccy);
  return -need < cash ? -need : cash;
}
