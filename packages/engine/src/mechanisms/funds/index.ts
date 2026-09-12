/**
 * Funds: a named party whose liability is its shares, whose equity is zero, and whose investors can
 * ask for their money back — which is the second door into the forced seller.
 *
 * @spec Fund Shares A1 Fund Shares A2 Fund Shares A3 Fund Shares A4 Fund Shares B1 Fund Shares B2 Fund Shares B2.a Fund Shares B3 Fund Shares B4 Fund Shares C1 Fund Shares C1.a Fund Shares C2 Fund Shares C2.a Fund Shares C2.b Fund Shares C3 Fund Shares C4 Fund Shares C4.a Fund Shares C5 Fund Shares D1 Fund Shares D2 Fund Shares D3 Fund Shares D4 Fund Shares F1 Fund Shares F2 Fund Shares F3 Fund Shares G1 Fund Shares G1.b XI-2 XI-3 XI-6 Law 2 Law 4 Law 15
 *
 * WHY IT IS HERE (XI-2). A price falls; somebody must sell into the fall; the sale makes the fall
 * worse. Without a party that MUST sell, a price shock is absorbed by nobody and dissipates, and
 * every measurement of contagion measures that dissipation. A fund is the cleanest such party: its
 * investors can ask for cash at any time, the fund promised nothing about being able to pay, and
 * what it must do when its buffer runs out is sell into whatever market is there (C2.a, C2.b).
 *
 * WHAT A FUND IS (A1-A3). A party with an account and a register of holdings, whose liability is
 * its shares and whose equity is therefore ZERO: the holders own the assets, and a fund with equity
 * has mislaid somebody's money. Nothing here enforces that — it falls out of the wire. Cash in and
 * shares out is one instruction; a mark that moves the assets moves the share liability by the same
 * amount through the NAV; a fee out reduces both sides. The audit checks it and never repairs it.
 *
 * WHAT A GATE IS (C2.b). A redemption this fund cannot meet is NOT rationed away. It is paid as far
 * as the cash goes, the rest stays queued under the holder's name at the NAV struck when it asked
 * (C4), and the fund sells to meet it. The difference between that NAV and what the sales actually
 * fetched falls on the holders who stayed, which is why a redemption is a real cost to them and why
 * runs are a thing (C4.a). Dropping the unfilled part would delete the entire system.
 */
import type { Family, Violation } from '../../audit/audit.js';
import { CENT_TICK } from '../../registry/grid.js';
import type { AuditView } from '../../audit/view.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import {
  instrumentId,
  instrumentKindId,
  currencyUnit,
  marketId,
  partyKindId,
  venueId,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import {
  add,
  addTo,
  atMost,
  div,
  dustOf,
  material,
  mul,
  sub,
  sum,
  withinDust,
  zeroIfNone,
} from '../../core/num.js';
import { asQty, downTick, subQty, upTick, type Qty } from '../../core/tick.js';

/** Law 8: one piece — the smallest step there is, and the only literal a count of them can have. */
const ONE_PIECE = asQty(1);
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide, shareFor, totalFor } from '../../ledger/settlement.js';
import { curveFamilyOf, priceAt } from '../../prices/curve.js';
import { wasTraded } from '../../prices/price-store.js';
import { weightOf } from '../../parties/party.js';
import { issuerOf, type Instrument } from '../../register/instruments.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import type { InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import { SHARES } from '../../registry/profiles.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { fundChoosesBank, FUND_SWITCHING_COST } from './bank.js';
import { physicalOrders } from './physical.js';
import { FUND_PARAMS, fundParam, type EtfDecl, type FundDecl } from './data.js';
import { basketOf, basketValue, create, premiumOf, redeemInKind } from './etf.js';
import { trackerOrders } from './tracker.js';
import { navOf } from './nav.js';

export * from './data.js';
export { fundChoosesBank, FUND_SWITCHING_COST } from './bank.js';
export { holdsPhysical, physicalOrders } from './physical.js';
export * from './etf.js';
export { navOf } from './nav.js';
export type { NavRead } from './nav.js';

/** E1: the venue an exchange-traded fund's shares are created and redeemed in, in kind (G1.a). */
export const etfVenue = (fund: string): VenueId => venueId(`etf.${fund}`);
export const etfMarketOf = (fund: string): MarketId => marketId(`mkt.${shareLineOf(fund)}`);

export const FUND = partyKindId('fund');
export const FUND_MANAGER = partyKindId('fundManager');
export const FUND_SHARE = instrumentKindId('fund.share');

/** A2, G1.b: a claim with a SHARE COUNT — which is what makes it something an investor can redeem. */
export interface FundShareTerms {
  readonly kind: typeof FUND_SHARE;
  readonly fund: PartyId;
}

export const shareLineOf = (fund: string): InstrumentId => instrumentId(`share.${fund}`);
export const fundVenue = (fund: string): VenueId => venueId(`funds.${fund}`);

/**
 * A1, F2: a fund is a party like any other. It fails on solvency and on nothing else: its equity is
 * zero by construction (A3), so with no leverage there is nothing for it to be insolvent WITH — and
 * XI-3's fund row ("its equity is gone; its broker eats the shortfall") is the levered case, which
 * needs a prime broker (13a) and a hedge fund (13h). The trigger is declared so that it fires the
 * moment leverage makes negative equity reachable, rather than being remembered later.
 *
 * Nobody lends to it (F2: no leverage without a lender), so it cannot overdraw either: a fund short
 * of cash sells, which is the whole point of it being here.
 */
export const fundKind: PartyKindProfile = {
  id: FUND,
  representation: 'named',
  moneyIssuer: null,
  fails: ['solvency'],
  borrows: false,
  // Money Market A1.c, E1: its cash is somebody's deposit and it is in the market all day — this
  // is the money that leaves first, and it leaves because it chose to (`bankChoices`, bank.ts).
  depositClass: 'wholesale',
};

/** F3: the manager is a separate party. The fee is its income and the fund's cost. */
export const fundManagerKind: PartyKindProfile = {
  id: FUND_MANAGER,
  representation: 'named',
  moneyIssuer: null,
  fails: ['cash', 'solvency'],
  borrows: true,
  // A1.c: it runs the money and it banks like the money it runs.
  depositClass: 'wholesale',
};

/**
 * A2, B1: the share. Its value is DERIVED — a claim on a book is worth what the book comes to over
 * how many claims there are — so it needs no market of its own and prints no price (an open-ended
 * fund's shares do not trade; the ETF's do, and that is a different kind, E1).
 */
export const fundShareKind: InstrumentKindProfile = {
  id: FUND_SHARE,
  pricing: 'derived',
  // Law 8, E2: a cent, because an exchange-traded fund's shares TRADE and a share trades in cents.
  // It is the grid of the posted price and never of the net asset value: what the book comes to per
  // share is arithmetic and rounding it would leave the fund holding a residue of its own holders'
  // money (A3, and the audit says so within two periods).
  priceTick: CENT_TICK,
  // What a holder's book has recognised is the last value read, which revaluation re-marks each
  // period; a derived value has no stored history for a carrying rule to read back (prices/value.ts).
  carry: 'cost',
  // C4, D4: it moves both ways. A share is not inventory — its value follows the book up and down,
  // and a fund that could only be written down would be a fund with a hidden guarantor on the way up.
  fairValueThroughIncome: true,
  liabilityOfIssuer: true,
  unit: () => SHARES,
  // Bond N13.a: a share ranks behind anything else the fund owes and takes what is left. That is
  // what "the holders own the assets" means when there is not enough (A3).
  ranking: () => ({
    seniority: 1,
    secured: [],
    claim: 'a pro-rata share of what is left of the fund once anything else it owes is paid',
  }),
  validateTerms: () => undefined,
  // Law 9: a fund share is named by the fund, which is how anybody would name it.
  displayName: (_i, namer) => `${namer.issuer.some ? namer.issuer.value : 'a fund'} shares`,
  due: () => [],
  accrued: () => 0,
  cashFlows: () => [],
  carriedAt: (_i, _lot, marked) => marked,
  derive: (i, at, reads) => navOf(i, at, reads).perShare,
};

/** XI-2, C2.b: what a holder asked for and has not been paid, at the NAV struck when it asked. */
interface Queued {
  readonly fund: string;
  readonly holder: PartyId;
  /** Shares still to redeem, per member of the holder if it is a cell (XI-15). */
  /** Law 8: shares are indivisible, so what one member asked back is a whole number of them. */
  sharesPerMember: Qty;
  /** C4: the NAV of the day it asked, which is what it is owed at. */
  readonly navStruck: number;
  readonly since: number;
}

interface Book {
  queued: Queued[];
  /** The NAV struck this period, so the orders and the settlement read one number (Law 4). */
  struck: Record<string, number>;
  /** D2.a, B1: what a share was worth last time, so what it RETURNED is a read and not a series. */
  previous: Record<string, number>;
}

/** ACT/365F for a fee quoted per annum: a rate is not a number until its periodicity is (Law 8). */
const FEE_DAY_COUNT = 'ACT/365F' as const;

function emptyBook(): Book {
  return { queued: [], struck: {}, previous: {} };
}

const declOf = (decls: readonly FundDecl[], fund: string): FundDecl | undefined =>
  decls.find((f) => f.fund === fund);

function etfParamsOf(etfs: readonly EtfDecl[]): ParamDecl[] {
  return etfs.map((e) => ({
    id: fundParam(e.fund, 'fee'),
    value: e.fee,
    unit: 'per annum on net assets',
    dimension: 'perAnnum' as const,
    kind: 'placeholder' as const,
    owner: 'model' as const,
    why: `Fund Shares B3, F3: what ${e.managerName} charges for running ${e.name}. Nothing in this world produces it: no manager competes for the mandate, so the number stands where a competition should be.`,
    standsInFor: { mechanism: 'Fund Shares F3', worklistItem: '13h' },
  }));
}

function paramsOf(decls: readonly FundDecl[]): ParamDecl[] {
  return [
    {
      id: FUND_SWITCHING_COST,
      value: 900,
      denominated: true,
      unit: 'of the money the account is in, per move',
      dimension: 'amount',
      kind: 'preference',
      owner: 'model',
      why: 'Banks Funding A1.c, A1.d, E1: what it costs a fund to move its account, ONCE, as an amount of its own money. It is the largest of the three because a fund moves the most money at once and has the most to redirect — and it holds it back the least, because the balance it is weighed against is larger still. That is what "rate-sensitive" IS when the test is an amount against an amount: what a quarter point has already cost this depositor passes the cost in a week, where a household waits a year and never gets there.',
    },
    {
      id: FUND_PARAMS.openingShare,
      value: MONEY_PIECES,
      unit: 'pieces of money per share at the first subscription (one USD)',
      dimension: 'price',
      kind: 'resolution',
      owner: 'model',
      why: 'Fund Shares B1: a fund with no shares has nothing to divide by, so the first subscription fixes the unit its shares are counted in. Double it and every share count halves and no value, flow or decision moves — which is what makes it a resolution and not a price (Law 2).',
    },
    ...decls.flatMap((f): ParamDecl[] => [
      {
        id: fundParam(f.fund, 'buffer'),
        value: f.buffer,
        unit: 'share of net assets held as cash',
        dimension: 'ratio',
        kind: 'preference',
        owner: 'model',
        why: `Fund Shares C2.a: how much of ${f.fund} sits in cash so an ordinary redemption needs no sale. It is the whole of the difference between a redemption that is invisible and one that reaches a market, and a fund that held none would sell on every request.`,
      },
      {
        id: fundParam(f.fund, 'fee'),
        value: f.fee,
        unit: 'per annum on net assets',
        dimension: 'perAnnum',
        kind: 'placeholder',
        owner: 'model',
        why: `Fund Shares B3, F3: what ${f.managerName} charges. Nothing in this world produces it: no manager competes for the mandate, so the number stands where a competition should be.`,
        standsInFor: { mechanism: 'Fund Shares F3', worklistItem: '13h' },
      },
      {
        id: fundParam(f.fund, 'requiredYield'),
        value: f.requiredYield,
        unit: 'per annum over what a deposit returns',
        dimension: 'perAnnum',
        kind: 'preference',
        owner: 'model',
        why: `Fund Shares D2, D2.a: what ${f.fund}'s investors require of it over a deposit, and therefore what it will pay for paper. A deposit returns nothing until a bank decides to pay for one (Banks Funding B1, worklist 11), and this becomes a comparison rather than a level the period one does.`,
      },
      {
        id: fundParam(f.fund, 'maxTenorPeriods'),
        value: f.maxTenorPeriods,
        unit: 'periods',
        dimension: 'periods',
        kind: 'policy',
        owner: 'model',
        why: `Fund Shares A4, D1: the longest anything ${f.fund} holds may still have to run. A mandate is a rule somebody wrote in a prospectus, and it is a real constraint on what the fund buys rather than a label on it.`,
      },
    ]),
  ];
}

/**
 * B3, F3: what a fund owes its manager for the days this period actually has.
 *
 * ONE ACCRUAL, read by every fund there is (Law 4). It was written twice — once here and once
 * inline in `runEtf` — and the two copies did not agree about the case that matters, which is what
 * a second copy of a formula is for.
 *
 * Law 8: a per annum rate is not a per period one, and a fee is money, so what comes out is whole
 * pieces of it. Below one piece there is nothing to pay, and `payable` has already said so.
 */
function feeAccrued(ctx: MechanismContext, fund: string, share: Instrument): number {
  const party = ctx.parties.get(fund as PartyId);
  return ctx.registry.payable(
    ctx.registry.currencyOf(party.region),
    mul(
      mul(ctx.valuation.markPerUnit(share.id, ctx.period), share.issued, 'net assets'),
      mul(
        ctx.params.perAnnum(fundParam(fund, 'fee')),
        yearFraction(
          FEE_DAY_COUNT,
          ctx.calendar.startOf(ctx.period),
          ctx.calendar.endOf(ctx.period),
        ),
        'this period of a year',
      ),
      'the fee',
    ),
  );
}

/**
 * F3: the manager's income, paid out of the fund's own account — and ONE CONVENTION for a fund that
 * cannot cover it (Appendix B: one payment convention).
 *
 * What it owes is what it owes. A fund short of the money does not pay a smaller fee: the
 * instruction goes to the wire for the whole amount and the wire refuses it, which is what happens
 * to every other payment in this world that a payer cannot make (Money E1). The refusal is a
 * recorded state with two sides and a manager that can see it; the alternative this replaces —
 * paying whatever cash was lying there and writing off the rest — was income appearing at one end
 * with nothing to match it at the other, and a shortfall nobody held (Law 2, Law 5).
 */
function payFee(ctx: MechanismContext, d: { fund: string; manager: string }, amount: Qty): void {
  if (amount <= 0) return;
  const fund = ctx.parties.get(d.fund as PartyId);
  const manager = ctx.parties.get(d.manager as PartyId);
  const leg: Leg = {
    kind: 'money',
    from: ctx.accountOf(fund.id, ctx.registry.currencyOf(fund.region)),
    to: ctx.accountOf(manager.id, ctx.registry.currencyOf(fund.region)),
    ccy: ctx.registry.currencyOf(fund.region),
    amount,
    fromCell: none(),
    toCell: none(),
  };
  const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `${d.fund} pays its manager` });
  ctx.record(
    'fund.fee',
    [d.fund, d.manager],
    { fund: d.fund, manager: d.manager, amount, paid: r.outcome === 'settled' },
    false,
  );
}

/** C1: cash in, shares out, one instruction. C3: the shares outstanding change, so a fund is not fixed-size. */
function subscribe(
  ctx: MechanismContext,
  d: FundDecl,
  share: Instrument,
  holder: PartyId,
  sharesAsked: number,
  perShare: number,
): void {
  // Fund Shares B1, C1: A SUBSCRIPTION IS AT THE NAV, AND THERE HAS TO BE ONE. A fund whose share
  // is worth nothing has no price for anybody to buy a claim at — there is nothing to have a claim
  // ON — and dividing a budget by it is `0/0`, which is how this was found: a fund declared with a
  // mandate it opens holding none of arrives at its first session with no book and no shares, and
  // the first party to ask for one asked for NaN of them.
  //
  // Refused where it is, because a NaN stops the world (ARCHITECTURE 5) — and it is a REAL refusal
  // and not a guard: nobody can buy a share of nothing. WHY a fund's share can be worth nothing
  // while shares are outstanding is a different question and it is `13b-9`'s, positioned to 13h.
  if (perShare <= 0) return;
  const party = ctx.parties.get(holder);
  const fund = ctx.parties.get(d.fund as PartyId);
  const ccy = ctx.registry.currencyOf(fund.region);
  // C1.d of the buyer's own budget: it subscribes with the money it has, and what it cannot pay
  // for it does not buy. That is a budget, not a bound on the decision.
  const cash = ctx.participant(holder).cash(ccy);
  const wanted = mul(sharesAsked, perShare, 'what it asked to put in');
  const budget = atMost(wanted, cash, 'it cannot spend money it does not hold');
  // Law 8: SHARES ARE ISSUED IN WHOLE PIECES and paid for in whole pieces of money, per member of a
  // cell (XI-15). The shares are struck first, because they are the thing being bought, and what
  // is paid is what they come to at the NAV — the nearest piece, so the fund is not shaved by a
  // fraction on every subscription it ever takes.
  let shares = downTick(div(budget, perShare, 'shares it gets'));
  let paid = ctx.registry.cashFor(ccy, mul(shares, perShare, 'what it pays'));
  if (paid > cash) {
    shares = subQty(shares, ONE_PIECE, 'a piece less');
    paid = ctx.registry.cashFor(ccy, mul(shares, perShare, 'what it pays'));
  }
  if (shares <= 0 || paid <= 0 || !material(shares, 2, sharesAsked)) return;
  const side = cellSide(party, shares);
  const money = cellSide(party, paid);
  const legs: Leg[] = [
    {
      kind: 'money',
      from: ctx.accountOf(holder, ccy),
      to: ctx.accountOf(fund.id, ccy),
      ccy,
      amount: totalFor(party, paid),
      fromCell: money === undefined ? none() : some(money),
      toCell: none(),
    },
    {
      kind: 'asset',
      from: fund.id,
      to: holder,
      instrument: share.id,
      qty: totalFor(party, shares),
      pricePerUnit: some(perShare),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: side === undefined ? none() : some(side),
    },
  ];
  const r = ctx.settle({ legs, cause: 'issuance', reason: `${holder} subscribes to ${d.fund}` });
  ctx.record(
    'fund.subscribed',
    [d.fund, holder],
    { fund: d.fund, holder, sharesPerMember: shares, perShare, settled: r.outcome === 'settled' },
    false,
  );
}

/**
 * C2, C2.a: shares back, cash out, at the NAV struck when it asked. Returns how many shares per
 * member it could NOT pay for, which is what stays queued (C2.b: never dropped).
 */
function redeem(
  ctx: MechanismContext,
  d: FundDecl,
  share: Instrument,
  holder: PartyId,
  sharesAsked: number,
  perShare: number,
): number {
  const party = ctx.parties.get(holder);
  const fund = ctx.parties.get(d.fund as PartyId);
  const ccy = ctx.registry.currencyOf(fund.region);
  // XI-15: the register holds a cell's position PER MEMBER, which is the unit a request is in.
  const held = ctx.register.quantity(holder, share.id);
  const weight = weightOf(party);
  const asked = atMost(ctx.registry.deliverable(share.unit, sharesAsked), held, 'it cannot hand back shares it does not hold');
  if (asked <= 0) return 0;
  // C2.a: from its buffer, or by selling. What it can pay now is what it holds now.
  const cash = ctx.register.quantity(fund.id, moneyOf(ctx, fund.id, ccy));
  const owedNow = ctx.registry.payable(ccy, mul(totalFor(party, asked), perShare, 'what it owes this holder'));
  const paying = atMost(owedNow, cash, 'it pays out of the money there is');
  // Law 8: shares come back in whole pieces, per member, and the cash is what they come to at the
  // NAV — the nearest piece of money. What cannot be paid for stays in the queue (C2.b).
  const sharesNow = downTick(div(div(paying, perShare, 'shares it can pay for'), weight, 'per member'));
  const perMemberCash = ctx.registry.cashFor(ccy, mul(sharesNow, perShare, 'what a member is paid'));
  /**
   * Law 8, C2.b: A PAYMENT BELOW ONE PIECE OF MONEY IS NOT A PAYMENT.
   *
   * Shares come back in whole pieces and the cash goes out in whole pieces, and the two grids are
   * not the same grid: a share worth less than a cent gives a whole number of shares whose cash
   * value rounds to nothing. Handing the shares back for nothing would be a one-sided flow (Law 5),
   * and the wire says so — `[Money C1] money leg amount must be positive, got 0`, which stopped a
   * year-long run in `balance-identity.test.ts` (`13b.1-1`). What it cannot pay stays on the book
   * under its own name and the fund is gated, which is the answer C2.b already has for this.
   */
  if (material(sharesNow, 2, asked) && sharesNow > 0 && perMemberCash > 0) {
    const side = cellSide(party, sharesNow);
    const money = cellSide(party, perMemberCash);
    const legs: Leg[] = [
      {
        kind: 'asset',
        from: holder,
        to: fund.id,
        instrument: share.id,
        qty: totalFor(party, sharesNow),
        pricePerUnit: some(perShare),
        accruedPerUnit: none(),
        fromCell: side === undefined ? none() : some(side),
        toCell: none(),
      },
      {
        kind: 'money',
        from: ctx.accountOf(fund.id, ccy),
        to: ctx.accountOf(holder, ccy),
        ccy,
        amount: totalFor(party, perMemberCash),
        fromCell: none(),
        toCell: money === undefined ? none() : some(money),
      },
    ];
    const r = ctx.settle({ legs, cause: 'maturity', reason: `${d.fund} redeems for ${holder}` });
    if (r.outcome === 'settled') {
      ctx.record(
        'fund.redeemed',
        [d.fund, holder],
        { fund: d.fund, holder, sharesPerMember: sharesNow, perShare },
        false,
      );
      return sub(asked, sharesNow, 'what is left to pay');
    }
  }
  return asked;
}

/** The fund's own money account, which is where its buffer is (Money D2). */
function moneyOf(ctx: MechanismContext, party: PartyId, ccy: string): InstrumentId {
  const bank = ctx.parties.get(party).bank;
  return instrumentId(`money:${bank}:${ccy}`);
}

/**
 * B1, B3, C1, C2: the day's work. Read what a share is worth, pay the manager, take in what was
 * subscribed and pay out what was asked for — and say what it could not pay, because that is what
 * it must sell for (C2.a) and what the market is about to see.
 */
function strike(ctx: MechanismContext, b: Book, d: FundDecl): void {
  // XI-3, Register F2: a fund that has ceased strikes nothing. Its investors' claims resolve
  // through its estate like anybody else's (Firm Birth D5).
  if (!ctx.parties.get(d.fund as PartyId).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  const opening = ctx.params.price(FUND_PARAMS.openingShare);
  const ccy = ctx.registry.currencyOf(ctx.parties.get(d.fund as PartyId).region);
  const previous = b.previous[d.fund];
  // B3: the fee is charged on what the book was worth before anybody transacted, and then the NAV
  // is read again — which is what "fees reduce NAV" means when the reduction is a real payment.
  if (share.issued > 0) payFee(ctx, d, ctx.registry.payable(ccy, feeAccrued(ctx, d.fund, share)));
  const perShare = share.issued > 0 ? ctx.valuation.markPerUnit(share.id, ctx.period) : opening;
  b.struck[d.fund] = perShare;
  // B2.a: how old the oldest mark behind it is. A stale mark makes a stale NAV and somebody
  // transacts on it: that is a real transfer between holders and it is said out loud.
  let oldest = ctx.period;
  for (const h of ctx.register.holdingsOf(d.fund as PartyId)) {
    if (h.instrument === share.id) continue;
    const worth = ctx.valuation.worthOf(d.fund as PartyId, h.instrument, ctx.period);
    if (worth.some && worth.value.from < oldest) oldest = worth.value.from;
  }
  if (oldest < ctx.period) {
    ctx.record(
      'fund.staleNav',
      [d.fund],
      { fund: d.fund, perShare, oldestMark: oldest, periodsStale: ctx.period - oldest },
      true,
    );
  }
  for (const o of ctx.posted(fundVenue(d.fund))) {
    if (o.qty <= 0) continue;
    // XI-15, Law 8: a posting is a total and a cell's decision is per member; this is the one place
    // the two meet, and it is the same conversion every other market makes. A SHARE IS INDIVISIBLE,
    // so what a member can ask for or hand back is a whole number of them — whoever posted it has
    // already decided in whole shares, and this is where a posting that did not is refused rather
    // than quietly queued for ever.
    const asked = ctx.registry.deliverable(
      share.unit,
      div(o.qty, weightOf(ctx.parties.get(o.party)), 'shares per member'),
    );
    if (asked <= 0) continue;
    if (o.side === 'buy') {
      subscribe(ctx, d, share, o.party, asked, perShare);
    } else {
      // A holder cannot ask back what it does not have, and what it has already asked for is
      // already on the book — a second ask for the same shares is the same claim, not another one.
      // This is not C2.b's rationing: nothing that was ever a claim is dropped here.
      const held = ctx.register.quantity(o.party, share.id);
      const already = sum(
        b.queued.filter((q) => q.fund === d.fund && q.holder === o.party).map((q) => q.sharesPerMember),
      ).value;
      const room = sub(held, already, 'shares it has not already asked back');
      const taking = atMost(asked, room, 'it cannot ask back shares it has already asked back');
      if (taking <= 0) continue;
      // C2.b: the request goes on the book under its own name at the NAV of the day it asked. What
      // happens to it after that is a question of cash, never of whether it counts.
      b.queued.push({
        fund: d.fund,
        holder: o.party,
        sharesPerMember: asQty(taking, 'shares it asked back per member'),
        navStruck: perShare,
        since: ctx.period,
      });
      ctx.record(
        'fund.requested',
        [d.fund, o.party],
        { fund: d.fund, holder: o.party, sharesPerMember: taking, perShare },
        false,
      );
    }
  }
  b.previous[d.fund] = perShare;
  payQueue(ctx, b, d);
  const owed = owedOn(ctx, b, d);
  const cash = ctx.register.quantity(d.fund as PartyId, moneyOf(ctx, d.fund as PartyId, ccy));
  // C2.a, Law 8: WHAT IT KEEPS BACK IS MONEY, so it is a whole number of the smallest piece of it.
  // Up, because it is what the fund insists on holding: a buffer a cent short of what its own rule
  // asks for is a buffer it did not keep. What is left over after it — what the fund has spare —
  // is then a count too, and the schedule its orders are built from is on the grid by arithmetic
  // rather than by a rounding somewhere further down.
  const buffer = upTick(
    mul(
      ctx.params.ratio(fundParam(d.fund, 'buffer')),
      mul(perShare, share.issued, 'net assets'),
      'the cash it keeps back',
    ),
  );
  ctx.record(
    'fund.struck',
    [d.fund],
    {
      fund: d.fund,
      perShare,
      shares: share.issued,
      /**
       * D2, D2.a: what it actually returned over the period that closed — a read of two values it
       * published, not a series it keeps. It is what a saver compares against a deposit, and it is
       * published because the competition D2 names cannot happen against a number nobody can see.
       *
       * A RETURN ON NOTHING IS NOT A NUMBER (Appendix A). A fund in its first period has no
       * previous value to have returned over, and one whose shares were marked at nothing has no
       * denominator — and `0 / 0` is NaN, which stopped a year-long run dead at the wire (Law 7's
       * `finite`). Zero would have been worse than the throw: it says the fund made nothing, which
       * is a statement about a period that did not happen.
       */
      returned:
        previous === undefined || previous <= 0
          ? null
          : div(sub(perShare, previous, 'what it made'), previous, 'per share it returned'),
      // C2.a: what it must find, and what it has spare. Its orders read these and nothing else,
      // so the decision and the schedule are one decision (Law 4).
      shortfall: owed,
      spare: subQty(subQty(cash, buffer, 'over its buffer'), owed, 'and after what it owes'),
      oldestMark: oldest,
      // D2, D2.a: what it offers a saver — what the paper its mandate lets it hold is yielding,
      // less what its manager takes. Both halves are public reads (Sovereign D3, B3), it causes
      // nothing by itself (Observer A5), and it is the number that competes with a deposit rate.
      offered: offeredYield(ctx, d),
    },
    true,
  );
}

/**
 * D2.a: what the fund offers, read from the public curve at the longest thing its mandate lets it
 * hold, less its fee. It is not a forecast and not a promise: it is what that paper is fetching
 * today (Sovereign D3), which is the only thing anybody can compare a deposit against.
 */
function offeredYield(ctx: MechanismContext, d: FundDecl): number {
  const region = ctx.registry.region(ctx.parties.get(d.fund as PartyId).region);
  const fee = ctx.params.perAnnum(fundParam(d.fund, 'fee'));
  const tenor = ctx.params.periods(fundParam(d.fund, 'maxTenorPeriods'));
  const on = ctx.calendar.startOf(ctx.period);
  const by = ctx.calendar.startOf(periodOf(ctx.period + tenor));
  let best = 0;
  for (const family of ctx.registry.curveFamilies.values()) {
    if (family.ccy !== ctx.registry.currencyOf(region.id)) continue;
    const read = ctx.curve(family.id).at(yearFraction(family.dayCount, on, by));
    if (read.yield.some) best = read.yield.value;
  }
  return sub(best, fee, 'what a saver gets after the manager');
}

/** What this fund still owes its redeemers, at the NAV each of them struck (C4). */
function owedOn(ctx: MechanismContext, b: Book, d: FundDecl): Qty {
  const terms: number[] = [];
  for (const q of b.queued) {
    if (q.fund !== d.fund) continue;
    const holder = ctx.parties.get(q.holder);
    // C2, Law 8: what a redemption COMES TO is shares at a NAV, so it lands between two pieces of
    // money — and what the fund must find is the piece above, because paying all but a fraction of
    // a cent is not paying. It is the same rounding a subscription takes the other way (`cashFor`).
    terms.push(upTick(mul(totalFor(holder, q.sharesPerMember), q.navStruck, 'what it is owed')));
  }
  return asQty(sum(terms).value, 'what its queue is owed');
}

/**
 * C2.a, C2.b, C4: pay what the cash reaches, oldest request first, at the NAV each one struck. What
 * it cannot pay stays on the book under its own name — never rationed away — and the fund is gated
 * until it is paid, which is public because a gate is information (C4.a: it is a cost to those who
 * stay, and they may want to leave too).
 */
function payQueue(ctx: MechanismContext, b: Book, d: FundDecl): void {
  if (!ctx.parties.get(d.fund as PartyId).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  // Clearing C3: first come, first served is a stated rule, applied the same way every time.
  const mine = b.queued.filter((q) => q.fund === d.fund).sort((x, y) => x.since - y.since);
  if (mine.length === 0) return;
  const left: Queued[] = [];
  for (const q of mine) {
    const unpaid = redeem(ctx, d, share, q.holder, q.sharesPerMember, q.navStruck);
    if (unpaid > 0) left.push({ ...q, sharesPerMember: asQty(unpaid, 'shares still owed per member') });
  }
  b.queued = [...b.queued.filter((q) => q.fund !== d.fund), ...left];
  if (left.length === 0) return;
  ctx.record(
    'fund.gate',
    [d.fund, ...left.map((q) => q.holder)],
    {
      fund: d.fund,
      requests: left.length,
      sharesOwed: sum(left.map((q) => totalFor(ctx.parties.get(q.holder), q.sharesPerMember))).value,
      oldest: left.reduce<number>((at, q) => atMost(q.since, at, 'the oldest of them is as old as the oldest'), ctx.period),
    },
    true,
  );
}

/**
 * E1-E4, G1.a: the exchange-traded fund's period.
 *
 * It runs AFTER the marks are in the books, because the NAV is a read of those marks (B1, B2) and
 * because the premium is a read of that NAV against what the session actually printed. Then it
 * clears what anybody posted into its creation venue — in kind, against a pro-rata slice of its own
 * book — and says what both of its two values were.
 *
 * Nothing here closes the gap between them. What closes it is somebody creating or redeeming
 * because the difference is worth more than what the trade costs them (E3.a), and what happens when
 * nobody will is that the gap stays and this read says so (E4).
 */
/**
 * M9, Indices C2, Fund Shares E1, E3, E3.a: A VEHICLE IS LAUNCHED THE PERIOD ITS INDEX FIRST
 * ANSWERS, and what puts the basket in is the participants delivering it.
 *
 * A tracker on a size segment holds whatever that segment's rule says is in it, and the rule reads
 * its constituents' own prints (Indices A3) — which at period zero do not exist. So the seed cannot
 * launch one: it would be handing the fund a basket nobody could have said was right, and three
 * vehicles each endowed with their share of the same float is a market owned three times (measured:
 * `etf.us` and `etf.equity.large.us` both opened with NEGATIVE equity, `12d-4`). What can launch one
 * is the period the index first HAS a level.
 *
 * E3.a: AND THE FLOAT CANNOT BE OWNED TWICE, because nothing here is endowed. A launch is the
 * authorised participants handing in a basket they actually hold and taking the shares it is worth
 * — ordinary instructions, over the wire, refused if they do not hold it. So a second tracker's
 * basket comes out of somebody's book and not out of thin air, which is what the seed could not
 * promise and this does not have to.
 *
 * A world where nobody can deliver the basket launches nothing, and says so. That is a real answer:
 * a tracker needs somebody to put the market in, and until a party holds the market there is
 * nobody to do it.
 */
function launchTracker(ctx: MechanismContext, e: EtfDecl): void {
  const fund = e.fund as PartyId;
  if (ctx.parties.has(fund)) return;
  // Indices D5.a: before its own rule has answered there is no index, and a tracker on one is a
  // mandate with nothing in it.
  if (!ctx.index(e.tracks).some) return;
  const bank = e.bank as PartyId;
  if (!ctx.parties.has(bank) || !ctx.parties.get(bank).status.alive) return;
  /**
   * E3, A3: NOTHING IS OPENED UNTIL SOMEBODY CAN PUT THE BASKET IN. How big the launch would be is
   * read off what the participants actually hold, BEFORE there is a fund — and a world where none
   * of them can deliver it launches nothing at all rather than a share line with no shares behind
   * it, a market nobody can trade in and a venue nobody can create at. A fund's equity is zero by
   * construction (A3); an empty shell is not a fund with nothing in it, it is not a fund.
   */
  const size = launchSize(ctx, e);
  if (size <= 0) return;
  const region = ctx.registry.region(ctx.parties.get(bank).region);
  const manager = e.manager as PartyId;
  for (const [id, kind, name] of [
    [manager, FUND_MANAGER, e.managerName],
    [fund, FUND, e.name],
  ] as const) {
    if (ctx.parties.has(id)) continue;
    ctx.enter({
      id,
      kind,
      region: region.id,
      name,
      bank,
      representation: 'named',
      status: { alive: true },
    });
  }
  const share = shareLineOf(e.fund);
  const market = etfMarketOf(e.fund);
  const terms: FundShareTerms = { kind: FUND_SHARE, fund };
  ctx.issue({
    id: share,
    kind: FUND_SHARE,
    issuer: some(fund),
    ccy: ctx.registry.currencyOf(region.id),
    terms,
    // E1: its shares TRADE, which is what makes it an exchange-traded fund — the same claim on the
    // same kind of book as any other fund's, with a session in it as well (E2).
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: `${e.name} shares`,
    instrument: share,
    ccy: ctx.registry.currencyOf(region.id),
    rationing: 'proRata',
  });
  // G1.a: where it is created and redeemed IN KIND — not a market and it does not clear, so the
  // module that owns it runs it itself (Clearing B2).
  ctx.openVenue({
    id: etfVenue(e.fund),
    name: `${e.name} creations and redemptions`,
    clearedBy: 'funds',
    unit: SHARES,
    ccy: ctx.registry.currencyOf(region.id),
    key: { kind: 'etf', fund: e.fund, share },
  });
  const launched = firstCreation(ctx, e, share, size);
  ctx.record(
    'etf.launched',
    [e.fund, e.tracks, String(share)],
    { fund: e.fund, tracks: e.tracks, shares: launched },
    true,
  );
}

/**
 * E3.a: HOW BIG A LAUNCH WOULD BE — the smallest of what each launching party's own slice reaches,
 * out of what it actually holds. It is asked before anything is opened, so a launch nobody can
 * back does not leave a fund behind it.
 *
 * Law 6: the smallest of them is how far all of them reach. It is arithmetic on what parties hold,
 * not a size anybody chose, and the proportions are what makes them the holders they said they
 * would be.
 */
function launchSize(ctx: MechanismContext, e: EtfDecl): number {
  let full: number | undefined;
  for (const [holder, slice] of launchers(ctx, e)) {
    const reaches = div(couldCreate(ctx, e, holder), slice, 'the launch its slice reaches');
    if (full === undefined || reaches < full) full = reaches;
  }
  if (full === undefined) return 0;
  return full;
}

/** Seed A3, E3.a: who is putting the basket in, of those this world actually has. */
function launchers(ctx: MechanismContext, e: EtfDecl): readonly (readonly [PartyId, number])[] {
  const out: (readonly [PartyId, number])[] = [];
  for (const [holder, slice] of Object.entries(e.launchedBy)) {
    if (slice > 0 && ctx.parties.has(holder as PartyId)) out.push([holder as PartyId, slice]);
  }
  return out;
}

/**
 * E3, E3.a: THE FIRST CREATION, and it is the ordinary one. Each launching party hands in the
 * basket and takes the shares it is worth, in its declared share of the launch — and how big the
 * launch is, is read off whoever can deliver LEAST of it, because the proportions are what makes
 * them the holders they said they would be. Nothing is endowed and nothing is bounded: a party that
 * cannot deliver its slice launches a smaller fund, and a world where none of them can launches
 * none.
 */
function firstCreation(
  ctx: MechanismContext,
  e: EtfDecl,
  share: InstrumentId,
  full: number,
): number {
  let made = 0;
  for (const [holder, slice] of launchers(ctx, e)) {
    const wanted = downTick(mul(full, slice, "this holder's slice of the launch"));
    if (wanted <= 0) continue;
    if (create(ctx, e, share, holder, wanted)) made = add(made, wanted, 'shares created');
  }
  return made;
}

/** E3: the most shares this party could create out of what it actually holds, line by line. */
function couldCreate(ctx: MechanismContext, e: EtfDecl, party: PartyId): number {
  const weight = weightOf(ctx.parties.get(party));
  let most: number | undefined;
  for (const [line, perShare] of Object.entries(e.basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const free = mul(ctx.register.free(party, id), weight, 'what it holds free of this line');
    const backs = downTick(div(free, perShare, 'shares this line of its basket backs'));
    if (most === undefined || backs < most) most = backs;
  }
  // A basket naming no line this world has is not a basket, and nothing backs a share of it. That
  // is a COUNT of shares it could create and not a missing number (Appendix A).
  if (most === undefined) return 0;
  return most;
}

function runEtf(ctx: MechanismContext, d: EtfDecl): void {
  const fund = d.fund as PartyId;
  if (!ctx.parties.get(fund).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  if (!share.status.live) return;
  // B3, F3: the fee is a real payment out of what the fund holds, and it reduces the NAV because
  // the NAV is a read of the book and the book is smaller once it has left. The same accrual and
  // the same convention as every other fund (Law 4): what it owes goes to the wire whole, and a
  // fund without the money has a refused payment rather than a discount nobody granted it.
  const money = moneyOf(ctx, fund, ctx.registry.currencyOf(ctx.parties.get(fund).region));
  const ccy = ctx.registry.currencyOf(ctx.parties.get(fund).region);
  if (share.issued > 0) payFee(ctx, d, ctx.registry.payable(ccy, feeAccrued(ctx, d.fund, share)));
  // G1.a: in kind, against a pro-rata slice of its own book. Nothing is sold and no market is
  // touched, which is why this vehicle is not the forced seller (the money fund is, C2.b).
  for (const o of ctx.posted(etfVenue(d.fund))) {
    if (o.qty <= 0) continue;
    if (o.side === 'buy') create(ctx, d, share.id, o.party, o.qty);
    else redeemInKind(ctx, d, share.id, o.party, o.qty);
  }
  distribute(ctx, d, share.id, money);
}

/**
 * Money Market B5, B5.a, Fund Shares C2.a: the fund places what it is not holding as a buffer.
 *
 * It posts into every borrowing name's overnight secured book -- secured, because a money fund
 * lends against collateral and takes nobody's name unsecured -- at the floor, which is what the
 * central bank pays for cash it takes in and therefore the least anybody should accept. What it can
 * actually lend is capped by the session itself (a lender cannot lend the same money twice), and
 * what nobody takes stays in its account, which is where it was.
 *
 * It finds the books by their public key rather than by knowing who runs them (Clearing B2): a
 * world without a money market has no such venue and the fund places nothing.
 */
function placeSpareCash(ctx: MechanismContext, d: FundDecl): void {
  const fund = ctx.parties.get(d.fund as PartyId);
  if (!fund.status.alive) return;
  const ccy = ctx.registry.currencyOf(fund.region);
  const cash = ctx.register.quantity(fund.id, moneyOf(ctx, fund.id, ccy));
  // C2.a: it keeps its own buffer against the redemptions it expects and places the rest.
  const buffer = mul(cash, ctx.params.ratio(fundParam(d.fund, 'buffer')), 'what it keeps liquid');
  const spare = ctx.registry.payable(ccy, sub(cash, buffer, 'what it can place'));
  if (spare <= 0) return;
  const floor = floorRate(ctx);
  if (!floor.some) return;
  for (const v of ctx.venues) {
    if (v.key['market'] !== 'money') continue;
    if (v.key['secured'] !== 'true' || v.key['tenor'] !== 'overnight') continue;
    const borrower = v.key['borrower'];
    if (borrower === undefined || borrower === String(fund.id)) continue;
    ctx.post(v.id, { party: fund.id, side: 'sell', price: floor.value, qty: spare });
  }
}

/** B5.a: what the central bank pays for cash it takes in, read off what it declared (C1). */
function floorRate(ctx: MechanismContext): Option<number> {
  const said = ctx.journal.ofKind('centralBank.corridor');
  const last = said[said.length - 1];
  if (last === undefined) return none<number>();
  const floor = last.data['floor'];
  return typeof floor === 'number' ? some(floor) : none<number>();
}

/**
 * B3, Equity F1, C4: what the fund received it passes on.
 *
 * An index fund owns the firms in its basket and the dividends they declare arrive in its account
 * (Equity D3.a). It is a claim on a book and not a box money goes into: what came in goes out, per
 * share, to whoever the register says holds one — which is what makes an exchange-traded fund a
 * thing a saver can value at all, because a claim that has never paid anything gives an outsider
 * nothing to go on (Equity B3).
 *
 * It is declared under the one name every issuer that pays anything declares under, so that a saver
 * reads one public fact and does not have to know which system it came out of (Law 4).
 */
function distribute(ctx: MechanismContext, d: EtfDecl, share: InstrumentId, money: InstrumentId): void {
  const fund = ctx.parties.get(d.fund as PartyId);
  const issued = ctx.instruments.get(share).issued;
  const cash = ctx.register.quantity(fund.id, money);
  if (issued <= 0 || cash <= 0) return;
  const perShare = div(cash, issued, 'what it passes on per share');
  if (!material(perShare, 2, perShare)) return;
  const ccy = ctx.registry.currencyOf(fund.region);
  const paid: number[] = [];
  let failed = 0;
  for (const holder of ctx.register.holdersOf(share)) {
    if (holder === fund.id) continue;
    const party = ctx.parties.get(holder);
    const perMemberUnits = ctx.register.quantity(holder, share);
    if (perMemberUnits <= 0) continue;
    // Law 8: what reaches a holder is whole pieces of money, per member. A holding whose share of
    // the pass-through is less than one piece is paid nothing this period, and the cash stays in
    // the fund for the next one — which is where it was anyway.
    const share2 = shareFor(ctx.registry, party, currencyUnit(ccy), mul(perMemberUnits, perShare, 'what a member is paid'));
    const perMemberCash = share2.perMember;
    const total = share2.total;
    if (!material(total, 2, total) || total <= 0) continue;
    const side = cellSide(party, perMemberCash);
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(fund.id, ccy),
          to: ctx.accountOf(holder, ccy),
          ccy,
          amount: total,
          fromCell: none(),
          toCell: side === undefined ? none() : some(side),
        },
      ],
      cause: 'corporateAction',
      reason: `payout on ${share} to ${holder}`,
    });
    if (r.outcome === 'settled') paid.push(total);
    else failed += 1;
  }
  if (paid.length === 0 && failed === 0) return;
  ctx.record(
    'payout.declared',
    [d.fund, share],
    {
      line: share,
      perShare,
      shares: issued,
      paid: sum(paid).value,
      failedPayments: failed,
    },
    true,
  );
}

/**
 * E2, E4: the two values, side by side, after the marks are in the books.
 *
 * The NAV is a read of what the fund holds over the claims on it (B1); the price is what a third
 * party paid for one this session. Nothing here brings them together and nothing anywhere clamps
 * the difference: what closes a gap is somebody creating or redeeming because it is worth their
 * while (E3.a), and a gap that stays is a finding about liquidity (E4).
 */
function readEtf(ctx: MechanismContext, d: EtfDecl): void {
  const fund = d.fund as PartyId;
  if (!ctx.parties.get(fund).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  if (!share.status.live) return;
  const basket = basketOf(ctx, d, share.id);
  const nav = share.issued > 0
    ? ctx.valuation.markPerUnit(share.id, ctx.period)
    : basketValue(basket);
  const print = ctx.prices.latest(share.id, ctx.period);
  const premium = premiumOf(print.some ? some(print.value.price) : none<number>(), nav);
  ctx.record(
    'etf.struck',
    [d.fund, share.id],
    {
      fund: d.fund,
      share: share.id,
      shares: share.issued,
      // E2: the two of them, because that they are different numbers is the point.
      perShare: nav,
      price: print.some ? print.value.price : null,
      // E4: a read of the two. Nothing anywhere clamps it, and a persistently large one is a
      // finding about liquidity rather than a defect in the arithmetic.
      premium: premium.some ? premium.value : null,
      // Clearing E4: whether the price half of it is a trade or a mark carried forward.
      stale: print.some ? !wasTraded(print.value) : true,
      basket: Object.fromEntries(basket.map((b) => [b.instrument, b.perShare])),
    },
    true,
  );
}

/**
 * A4, C1.a, C2.b: what the fund takes to market. It has exactly two reasons to be there and they
 * are opposites: cash it must put to work per its mandate, and a redemption it must find the money
 * for. The second is the forced sale (XI-2): it names no price, because it has no choice.
 */
function ordersOf(
  decls: readonly FundDecl[],
  view: ParticipantView,
  m: MarketDecl,
): readonly Order[] {
  const d = declOf(decls, view.self.id);
  if (d === undefined) return [];
  const own = view.lastOwn('fund.struck');
  if (!own.some || own.value.period !== view.period) return [];
  const shortfall = own.value.data['shortfall'];
  const spare = own.value.data['spare'];
  if (typeof shortfall !== 'number' || typeof spare !== 'number') return [];
  const i = view.instruments.get(m.instrument);
  if (shortfall > 0) {
    // Clearing C1.b, Treasury D3.a: in a primary market the seller is the ISSUER. A holder with
    // paper to sell waits for the secondary session; it does not stand beside the issuer in its
    // own auction.
    if (view.offer(m.id).some) return [];
    const units = view.free(m.instrument);
    if (units <= 0) return [];
    // It sells across what it holds in proportion to what each is worth: nothing tells it to
    // prefer one line over another, so the rule is stated once and applied the same way (C3).
    const worth = view.print(m.instrument);
    if (!worth.some) return [];
    const total = holdingsWorth(view);
    if (total <= 0) return [];
    const fraction = div(shortfall, total, 'the share of its book it must raise');
    // Law 8: a share of a holding is a fraction of a unit, and a unit is what there is. It rounds
    // UP because this is a redemption it has to MEET — selling all but a fraction of what covers it
    // leaves the investor short — and it can never be more than what it holds.
    const wants = fraction >= 1 ? units : upTick(mul(units, fraction, 'units it must sell'));
    const qty = atMost(wants, units, 'there are no more units of it than there are');
    if (!material(qty, 2, units)) return [];
    // XI-2: at whatever the market gives. A forced seller that named a price would not be one.
    return [{ party: view.self.id, side: 'sell', price: 'market', qty }];
  }
  if (spare <= 0 || !eligible(view, d, i)) return [];
  // C1.a: it must buy something with the cash, and what it will pay is what makes the paper return
  // what its own investors require of it (D2). A price it will not pay does not fill.
  const on = view.calendar.startOf(view.period);
  const family = view.registry.curveFamily(curveFamilyOf(issuerOf(i), i.ccy));
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  if (flows.length === 0) return [];
  const required = view.params.perAnnum(fundParam(d.fund, 'requiredYield'));
  const price = priceAt(flows, required, on, family.dayCount, `what ${i.id} is worth to ${d.fund}`);
  if (price <= 0) return [];
  const lines = eligibleLines(view, d);
  if (lines === 0) return [];
  const each = div(spare, lines, 'what it puts into each line it may hold');
  const dirty = add(price, view.accrued(i.id), 'what a unit costs it');
  // Law 8: whole units. Down, because it is what the cash it has spare actually reaches.
  const qty = downTick(div(each, dirty, 'units it bids for'));
  // Law 7: this line's share against what the whole of what it has spare would have bought.
  if (qty <= 0 || !material(qty, lines + 1, div(spare, dirty, 'what the whole of it would buy'))) return [];
  return [{ party: view.self.id, side: 'buy', price, qty }];
}

/** What everything it holds is worth, at the last marks — the base a pro-rata sale is struck on. */
function holdingsWorth(view: ParticipantView): number {
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const print = view.print(h.instrument);
    if (!print.some) continue;
    terms.push(mul(sum(h.lots.map((l) => l.qty)).value, print.value.price, 'what it holds'));
  }
  return sum(terms).value;
}

/** A4, D1: what the mandate allows — the kind, and how long it may still have to run. */
function eligible(view: ParticipantView, d: FundDecl, i: Instrument): boolean {
  if (!i.status.live || !d.eligible.includes(i.kind)) return false;
  const on = view.calendar.startOf(view.period);
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  const last = flows[flows.length - 1];
  if (last === undefined) return false;
  const by = view.calendar.startOf(
    periodOf(view.period + view.params.periods(fundParam(d.fund, 'maxTenorPeriods'))),
  );
  return compareCivil(last.date, by) <= 0;
}

/** How many lines the mandate lets it into, so what it has spare is spread over them and no more. */
function eligibleLines(view: ParticipantView, d: FundDecl): number {
  let n = 0;
  for (const i of view.instruments.all()) {
    if (i.market.some && eligible(view, d, i)) n += 1;
  }
  return n;
}

/**
 * A3: a fund's equity is ZERO. The holders own the assets, so assets minus liabilities is nothing —
 * and a fund with equity has mislaid somebody's money. Nothing in the module enforces it: it falls
 * out of the wire, and this is the check that says whether the wire actually did it.
 */
function equityIsZero(): Family {
  return {
    name: 'accounts',
    contributor: 'funds',
    spec: 'Fund Shares A3',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const p of view.parties.ofKind(FUND)) {
        if (!p.status.alive || !view.register.hasEquityAccount(p.id)) continue;
        const walk = view.register.equityWalk(p.id);
        // Law 7: the same derivation the failure test uses, because it is the same fact — whether
        // this account is at zero or not (Law 4). A fund lives on that difference every period.
        if (withinDust(walk.value, 0, view.valuation.equityDust(p.id, walk, view.period))) continue;
        out.push({
          family: 'accounts',
          spec: 'Fund Shares A3',
          owner: p.id,
          size: walk.value,
          unit: view.registry.currencyOf(p.region),
          period: view.period,
          message: `${p.id} has equity of ${walk.value}: a fund with equity has mislaid somebody's money`,
        });
      }
      return out;
    },
  };
}

/**
 * C2.b: no redemption request disappears. Everything ever asked for is either paid or still on the
 * book — a request rationed away with the unfilled part dropped deletes the entire system, and it
 * would break in silence, because the number that vanished is the number nobody is looking at.
 */
function noRequestVanishes(b: Book): Family {
  return {
    name: 'flows',
    contributor: 'funds',
    spec: 'Fund Shares C2.b Fund Shares C5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const asked = new Map<string, number>();
      const paid = new Map<string, number>();
      for (const e of view.journal.ofKind('fund.requested')) {
        addTo(asked, key(e.data), totalAsked(view, e.data));
      }
      for (const e of view.journal.ofKind('fund.redeemed')) {
        addTo(paid, key(e.data), totalAsked(view, e.data));
      }
      const queued = new Map<string, number>();
      for (const q of b.queued) {
        addTo(queued, `${q.fund}|${q.holder}`, totalFor(view.parties.get(q.holder), q.sharesPerMember));
      }
      for (const [k, want] of asked) {
        const done = zeroIfNone(paid.get(k)) + zeroIfNone(queued.get(k));
        // Law 7: both sides are sums of the same per-member numbers at the same magnitudes.
        if (withinDust(want, done, dustOf(asked.size + 2, Math.abs(want) + Math.abs(done)))) continue;
        out.push({
          family: 'flows',
          spec: 'Fund Shares C2.b',
          owner: k.split('|')[0] ?? k,
          size: sub(want, done, 'asked against paid and queued'),
          unit: 'shares',
          period: view.period,
          message: `${k} asked to redeem ${want} shares and only ${done} were paid or are still on the book`,
        });
      }
      return out;
    },
  };
}

const key = (data: Record<string, unknown>): string => `${String(data['fund'])}|${String(data['holder'])}`;

function totalAsked(view: AuditView, data: Record<string, unknown>): number {
  const per = data['sharesPerMember'];
  const holder = data['holder'];
  if (typeof per !== 'number' || typeof holder !== 'string') return 0;
  return totalFor(view.parties.get(holder as PartyId), asQty(per, 'shares asked back per member'));
}

/**
 * E1, Seed A3: an exchange-traded fund opens LAUNCHED — a sponsor put a basket in and holds the
 * shares that came out. That is what a fund launch is, and it is endowment state like anything else
 * the seed states (A3): what nobody may state is what its shares are worth from then on, which is
 * why its market opens at the basket it holds and is repriced by its first session (C4.a).
 */
function seedEtf(ctx: SeedContext, e: EtfDecl): void {
  const bank = ctx.parties.get(e.bank as PartyId);
  const region = ctx.registry.region(bank.region);
  for (const [id, kind, name] of [
    [e.fund, FUND, e.name],
    [e.manager, FUND_MANAGER, e.managerName],
  ] as const) {
    // Seed B2, Law 4: ONE PARTY, NAMED ONCE. A manager runs more than one fund — that is what a
    // fund manager IS — so the second vehicle it launches finds it already here and does not make
    // a second one. The FUND is its own party either way: two funds are two balance sheets.
    if (ctx.parties.has(id as PartyId)) continue;
    ctx.parties.add({
      id: id as PartyId,
      kind,
      region: region.id,
      name,
      bank: bank.id,
      representation: 'named',
      status: { alive: true },
    });
  }
  const share = shareLineOf(e.fund);
  const market = etfMarketOf(e.fund);
  const terms: FundShareTerms = { kind: FUND_SHARE, fund: e.fund as PartyId };
  ctx.instruments.add({
    id: share,
    kind: FUND_SHARE,
    issuer: some(e.fund as PartyId),
    ccy: ctx.registry.currencyOf(region.id),
    terms,
    // E1: its shares TRADE, which is the whole of what makes it an exchange-traded fund. It is the
    // same claim on the same kind of book as any other fund's; what is different is that there is
    // a session in it, so it has a cleared price as well as a book value (E2).
    market: some(market),
  });
  ctx.openMarket({
    id: market,
    name: `${e.name} shares`,
    instrument: share,
    ccy: ctx.registry.currencyOf(region.id),
    rationing: 'proRata',
  });
  // G1.a: where it is created and redeemed IN KIND. It is not a market and it does not clear —
  // everybody who brings a basket gets the shares that basket is worth — so the module that owns
  // it runs it itself (Clearing B2).
  ctx.openVenue({
    id: etfVenue(e.fund),
    name: `${e.name} creations and redemptions`,
    clearedBy: 'funds',
    unit: SHARES,
    ccy: ctx.registry.currencyOf(region.id),
    key: { kind: 'etf', fund: e.fund, share },
  });
  // Seed A3: only what somebody who EXISTS actually took. A world without the banks that launch it
  // has a smaller fund, and its basket has to back the shares that were taken and no more — a
  // basket backing shares nobody holds would be a fund whose NAV was a multiple of what it owed.
  const holders = Object.entries(e.launchedBy).filter(
    ([holder, share]) => share > 0 && ctx.parties.has(holder as PartyId),
  );
  if (holders.length === 0) return;
  // Law 19, E3.a: HOW BIG THE LAUNCH IS, read off the lines it tracks rather than stated
  // (`EtfDecl.launchShare`). A share of the fund is `perShare` of each line, so what each line can
  // back is its own float over that, and the SMALLEST of them is as far as all of them reach.
  const backs: number[] = [];
  for (const [line, perShare] of Object.entries(e.basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    backs.push(
      div(
        mul(ctx.instruments.get(id).issued, e.launchShare, 'the share of this line it holds'),
        perShare,
        'the shares of the fund this line backs',
      ),
    );
  }
  if (backs.length === 0) return;
  // Law 8: whole shares. Each holder puts in ITS OWN slice, so a world missing the desks that would
  // have made its market launches a fund short by exactly their slices — which is Seed A3's "only
  // what somebody who EXISTS actually took" and is why this is not one split of one total.
  const full = downTick(backs.reduce((a, b) => atMost(b, a, 'the launch reaches as far as the shortest line backs it')));
  if (full <= 0) return;
  const taken = holders.map(([, share]) =>
    downTick(mul(full, share, "this holder's slice of the launch")),
  );
  const launched = sum(taken).value;
  if (launched <= 0) return;
  // E3, Fund Shares A3: THE BASKET IS WHAT THE SHARES ARE A CLAIM ON, so the shares are decided
  // first and exactly that much is taken. A holder gives up whole pieces PER MEMBER (XI-15), so a
  // little LESS of a line may arrive than the arithmetic asked for — and that is the safe way for
  // it to be out. Taking first and issuing against what came left the fund holding more than its
  // shares claimed, and a fund holding more than its shares claim HAS EQUITY: the accounts family
  // said so at period zero, `etf.us has equity of 600`, which is somebody's money mislaid.
  const contributions: number[] = [];
  for (const [line, perShare] of Object.entries(e.basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const opening = ctx.prices.latest(id, ctx.period);
    if (!opening.some) continue;
    const units = outOfTheFloat(
      ctx,
      id,
      mul(perShare, launched, 'units of this line the launch takes'),
      e.fund as PartyId,
    );
    if (units <= 0) return;
    ctx.register.credit(e.fund as PartyId, id, units, opening.value.price, ctx.period);
    contributions.push(mul(units, opening.value.price, 'what this line put in'));
  }
  // Law 19, Fund Shares A3: WHAT ONE SHARE IS A CLAIM ON IS READ OFF THE BASKET THAT ARRIVED, never
  // off the one that was asked for. A holder gives up whole pieces per member, so a little less of
  // a line comes in — and a share issued at what a whole basket would have been worth is a claim on
  // more than the fund has: its holders carry it at that basis, the fund owes them that, and the
  // difference is equity a fund may not have (measured at -15,599,400 before this read).
  const perShare = div(sum(contributions).value, launched, 'what one share is a claim on');
  if (perShare <= 0) return;
  for (const [at, [holder]] of holders.entries()) {
    const mine = taken[at];
    if (mine === undefined || mine <= 0) continue;
    ctx.endowUnits(holder as PartyId, share, mine, perShare);
  }
  // Seed C4: the level its market opens at, which is what its book was worth when it opened. The
  // first session reprices it, and the two have been able to differ ever since (E2).
  ctx.prices.write({
    instrument: share,
    market,
    period: ctx.period,
    // Law 8, 12b.1: A LEVEL IS ON ITS MARKET'S OWN GRID, opening print included. What one share is
    // a claim on is a division and lands wherever it lands (`perShare` above); what a market SHOWS
    // is a tick, and a print off the grid is a level nobody could have posted.
    price: ctx.registry.onQuoteGrid(FUND_SHARE, ctx.registry.currencyOf(region.id), perShare),
    ccy: ctx.registry.currencyOf(region.id),
    provenance: { kind: 'opening' },
  });
}

/**
 * Seed A3, Equity A1, C1.a: A LAUNCH TAKES ITS BASKET OUT OF THE FLOAT, and never adds to it.
 *
 * It used to CREATE the units — `credit` to the fund and `adjustIssued` on the line — which is
 * twenty thousand shares of a company nobody subscribed for, handed to a fund that paid nothing.
 * At a stated launch of twenty thousand against a line of a hundred and eighty million that was a
 * hundredth of a basis point and it never showed. Sized to the world it is five per cent, and five
 * per cent is a company issuing a twentieth of itself and receiving nothing for it: `issued` stops
 * being `book / price`, so every holder's claim on the residual is diluted by exactly the fund's
 * basket and the book value a saver reads off the accounts is short by it (Equity A1, B3).
 *
 * So the launch takes what it holds from the holders that have it, pro rata, and the line's count
 * does not move (D4: it changes only by a named event, and a launch is not one). What each of them
 * gives up is per member and whole (Law 8, XI-15), so a cell too small to give up one piece gives up
 * none and the fund holds a little less than the arithmetic asked for — which is what its basket per
 * share is READ as anyway (`basketOf`), never stated.
 */
function outOfTheFloat(
  ctx: SeedContext,
  id: InstrumentId,
  wanted: number,
  fund: PartyId,
): number {
  const outstanding = ctx.instruments.get(id).issued;
  if (outstanding <= 0 || wanted <= 0) return 0;
  const taken: number[] = [];
  for (const holder of ctx.register.holdersOf(id)) {
    if (holder === fund) continue;
    const weight = weightOf(ctx.parties.get(holder));
    const mine = ctx.register.quantity(holder, id);
    const perMember = downTick(
      div(mul(mine, wanted, 'its share of what the launch takes'), outstanding, 'per member'),
    );
    if (perMember <= 0) continue;
    ctx.register.debit(holder, id, perMember);
    taken.push(mul(perMember, weight, 'units this holder gave up'));
  }
  return sum(taken).value;
}

export function funds(
  decls: readonly FundDecl[],
  etfs: readonly EtfDecl[],
): SystemModule {
  const state = emptyBook();
  // The observer sees the book as the data it is; the slot holds this very object (Law 4).
  const book = (ctx: MechanismContext): Book => ctx.state<Book>('funds', () => state);
  // E3: an exchange-traded fund's basket is lines somebody else registered and its launch names
  // parties somebody else created, so those modules have to have run first. It is a dependency of
  // THIS WORLD's funds and not of funds, which is why it is read off the data rather than written
  // into the module (Law 15): a world with no such fund in it needs none of them.
  const needs = [...new Set(etfs.flatMap((e) => e.needs))];
  return {
    id: 'funds',
    spec: 'Fund Shares, XI-2',
    // Its investors are households (D2), a fund that fails resolves through the same estate as
    // anything else (XI-3), and it banks somewhere — so all three are there before it opens.
    requires: ['households', 'estate', 'seed.foundation', ...needs],
    instrumentKinds: [fundShareKind],
    partyKinds: [fundKind, fundManagerKind],
    curveFamilies: [],
    // A2, G1.b: shares are what a fund is measured in. The unit itself is the world's — a share
    // count is what a claim on a book and a claim on a firm are both counted in (Equity A2), and
    // two modules cannot each introduce it — so it is registry data and this module only uses it.
    units: [],
    params: [...paramsOf(decls), ...etfParamsOf(etfs)],
    phases: [
      {
        name: 'funds.strike',
        spec: 'Fund Shares B1 Fund Shares B3 Fund Shares C1 Fund Shares C2 Fund Shares C2.b',
        cycle: 0,
        // After the households have decided, because what they decided is posted into the venue
        // this phase reads; before the markets, because what it cannot pay is what it must sell.
        anchor: { after: 'households.decide' },
        run: (ctx: MechanismContext) => {
          const b = book(ctx);
          for (const d of decls) strike(ctx, b, d);
        },
      },
      {
        name: 'funds.place',
        spec: 'Fund Shares C2.a Money Market B5 Money Market B5.a',
        cycle: 0,
        // Money Market B5: NON-BANK CASH IS IN THE SAME MARKET. A money fund's spare cash is the
        // largest single pool of it in this world, and a fund that left it sitting as a deposit
        // would be a money fund that does not use the money market -- which is most of what a money
        // fund is. It places in the morning, out of what it holds after the day's subscriptions and
        // redemptions have been struck, and what it will take for it is the floor (B5.a): the
        // central bank pays that for cash it takes in, and nothing it lends should earn less.
        anchor: { after: 'funds.strike' },
        run: (ctx: MechanismContext) => {
          for (const d of decls) placeSpareCash(ctx, d);
        },
      },
      {
        name: 'funds.settle',
        spec: 'Fund Shares C2.a Fund Shares C2.b Fund Shares C4 Fund Shares C4.a',
        cycle: 2,
        anchor: { after: 'markets' },
        run: (ctx: MechanismContext) => {
          const b = book(ctx);
          for (const d of decls) payQueue(ctx, b, d);
        },
      },
      {
        name: 'funds.etf',
        spec: 'Fund Shares B3 Fund Shares E3 Fund Shares G1.a',
        cycle: 0,
        // With the other fund's own strike, and before the session: what a dealing line brought in this
        // morning is shares it can offer this afternoon, and the fee is inside the period so the
        // revaluation that closes it carries the claim at what the book then comes to (A3).
        anchor: { after: 'funds.strike' },
        run: (ctx: MechanismContext) => {
          for (const d of etfs) launchTracker(ctx, d);
          // E3.a: a vehicle whose index has not answered yet has not been launched, and there is
          // nothing of it to run. It is not absent — it is declared and waiting for its own rule.
          for (const d of etfs) if (ctx.parties.has(d.fund as PartyId)) runEtf(ctx, d);
        },
      },
      {
        name: 'funds.etf.read',
        spec: 'Fund Shares E1 Fund Shares E2 Fund Shares E4',
        cycle: 'anchor',
        // After the marks are in the books, because the NAV is a read of those marks and the
        // premium is a read of that NAV against what the session printed (Clearing F1.a).
        anchor: { after: 'revaluation' },
        run: (ctx: MechanismContext) => {
          for (const d of etfs) if (ctx.parties.has(d.fund as PartyId)) readEtf(ctx, d);
        },
      },
    ],
    participants: [
      {
        partyKind: FUND,
        // Three mandates, three reasons, and a fund has exactly one of them: a money fund puts its
        // spare cash to work at a yield it requires (D2), a tracker holds the index whatever it
        // costs (Indices C2), and a commodity fund holds the thing itself because it expects to be
        // paid more for it than the wait costs (Commodities Spot C3). None answers for a fund that
        // is not its own, so no two ever speak for one party (Law 4).
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => [
          ...ordersOf(decls, view, m),
          ...trackerOrders(etfs, view, m),
          ...physicalOrders(decls, view, m),
        ],
      },
    ],
    families: [equityIsZero(), noRequestVanishes(state)],
    // A1.c: both kinds are wholesale money and both leave for the same reasons (the manager runs
    // the money and banks like the money it runs), so one reason answers for both.
    bankChoices: [
      { partyKind: FUND, chooses: fundChoosesBank },
      { partyKind: FUND_MANAGER, chooses: fundChoosesBank },
    ],
    seed(ctx: SeedContext): void {
      for (const d of decls) {
        for (const [id, kind, name] of [
          [d.fund, FUND, d.name],
          [d.manager, FUND_MANAGER, d.managerName],
        ] as const) {
          ctx.parties.add({
            id: id as PartyId,
            kind,
            region: ctx.registry.region(ctx.parties.get(d.bank as PartyId).region).id,
            name,
            bank: d.bank as PartyId,
            representation: 'named',
            status: { alive: true },
          });
        }
        const ccy = ctx.registry.currencyOf(ctx.parties.get(d.fund as PartyId).region);
        const terms: FundShareTerms = { kind: FUND_SHARE, fund: d.fund as PartyId };
        ctx.instruments.add({
          id: shareLineOf(d.fund),
          kind: FUND_SHARE,
          issuer: some(d.fund as PartyId),
          ccy,
          terms,
          market: none(),
        });
        // Clearing B2: the venue where its investors ask. It is not a market and it does not clear
        // — everybody who asks transacts at the same NAV (C1, C2) — which is exactly why the module
        // that owns it runs it itself rather than the solver.
        const venue: VenueDecl = {
          id: fundVenue(d.fund),
          name: `${d.name} subscriptions and redemptions`,
          clearedBy: 'funds',
          unit: SHARES,
          ccy,
          key: { kind: 'fund', fund: d.fund, share: shareLineOf(d.fund) },
        };
        // Seed E: the fund opens with NOBODY in it and nothing in its account. Everything anybody
        // has in this world is something they were paid or something they decided to buy, and that
        // is as true of a fund share as of a deposit — so the first subscription is a decision a
        // household takes, at the unit its shares are counted in (FUND_PARAMS.openingShare).
        ctx.openVenue(venue);
      }
      // E3.a: only the vehicle whose index the seed can see; the rest are a phase's.
      for (const e of etfs) if (e.seeded) seedEtf(ctx, e);
    },
  };
}
