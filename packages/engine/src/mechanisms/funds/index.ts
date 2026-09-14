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
import { passiveOrders } from './passive.js';
import type { Family, Violation } from '../../audit/audit.js';
import { CENT_TICK } from '../../registry/grid.js';
import type { AuditView } from '../../audit/view.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import {
  type AgreementId,
  instrumentId,
  instrumentKindId,
  currencyUnit,
  marketId,
  moneyInstrumentId,
  partyKindId,
  venueId,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { admits } from '../../registry/blueprint.js';
import {
  amountOf,
  asCash,
  asPerPiece,
  asAmount,
  asRatio,
  type Ratio,
  type Cash,
  heldAsMoney,
  type PerPiece,
  minus,
  over,
  plus,
  pricedAt,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import { addTo, atMost, dustOf, material, sum, withinDust, zeroIfNone } from '../../core/num.js';
import {
  addQty,
  asQty,
  downTick,
  NO_QTY,
  scaleQty,
  subQty,
  upTick,
  type Qty,
} from '../../core/tick.js';

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
import { holdsThings, thingOrders } from './things.js';
import {
  costOfAPool,
  feeOn,
  type Launch,
  launchedPoolId,
  launchToMake,
  MANAGER_PARAMS,
  managerParam,
  type Notice,
  noticeToGive,
  staffOrders,
} from './manager.js';
import {
  FUND_PARAMS,
  inKindOf,
  MANAGER_NUMBERS,
  type ManagerDecl,
  nameOf,
  type FundDecl,
} from './data.js';
import { basketOf, basketValue, create, premiumOf, redeemInKind } from './inkind.js';
import {
  isMandate,
  MANDATE,
  type Mandate,
  mandateFor,
  type MandateTerms,
  openMandate,
  payingThisPeriod,
  livingPools,
  productOf,
  redeemable,
} from './mandate.js';
import { navOf } from './nav.js';

export * from './data.js';
export { fundChoosesBank, FUND_SWITCHING_COST } from './bank.js';
export { holdsThings, thingOrders } from './things.js';
export * from './manager.js';
export * from './inkind.js';
export * from './mandate.js';
export { navOf } from './nav.js';
export type { NavRead } from './nav.js';

/** E1: the venue an exchange-traded fund's shares are created and redeemed in, in kind (G1.a). */
/**
 * G1.a: the venue a pool's shares are created and redeemed IN KIND. Item 10e: named for the term —
 * what puts a pool here is `liquidity: 'listed'`, not what anybody calls it.
 */
export const inKindVenue = (fund: string): VenueId => venueId(`inKind.${fund}`);
export const listedMarketOf = (fund: string): MarketId => marketId(`mkt.${shareLineOf(fund)}`);

export const FUND = partyKindId('fund');
export const FUND_MANAGER = partyKindId('fundManager');
export const FUND_SHARE = instrumentKindId('fund.share');

/** A2, G1.b: a claim with a SHARE COUNT — which is what makes it something an investor can redeem. */
export interface FundShareTerms {
  readonly kind: typeof FUND_SHARE;
  readonly fund: PartyId;
}

export const shareLineOf = (fund: string): InstrumentId => instrumentId(`share.${fund}`);
/** A2: the claim a pool issues, and one writer of what its terms are (Law 4). */
const shareTerms = (fund: PartyId): FundShareTerms => ({ kind: FUND_SHARE, fund });
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
  /** item 15: what somebody else set it up to do, and it does not get to change it. */
  objective: 'itsMandate',
  id: FUND,
  representation: 'named',
  moneyIssuer: null,
  fails: ['solvency'],
  /**
   * F2, item 9.2: WHETHER A POOL MAY BE LEVERED IS ITS MANDATE'S, and `MandateTerms.leverage` is
   * where it is said. Every mandate this world draws says `false`, so no pool here borrows, and
   * this says the same thing while that is true.
   *
   * It is a SHAPE with a scheduled death and not a property of the category: as a fact about the
   * KIND it said no pool anywhere may ever be levered, which is what 13h claimed to have built
   * hedge funds on. What replaces it is the credit decision asking the pool's own mandate, and
   * that door is item 13.2's — it is not opened here because no mandate in this world answers
   * `true` yet, and a door nobody answers is `A-67` again.
   */
  borrows: false,
  // Money Market A1.c, E1: its cash is somebody's deposit and it is in the market all day — this
  // is the money that leaves first, and it leaves because it chose to (`bankChoices`, bank.ts).
  depositClass: 'wholesale',
};

/**
 * F3: the manager is a separate party. The fee is its income and the fund's cost.
 *
 * Item 15, item 9.2: ITS OBJECTIVE IS THE RESIDUAL AND NOT A MANDATE. `itsMandate` is what a party
 * somebody else set up and wrote the rules for is for — a pool, a central bank, an insurer — and
 * it was on the manager because the pool and the manager were one thing wearing two party ids.
 * They are two now: the POOL is run under a mandate (`funds.mandate`), and the MANAGER is a
 * business that competes to be given one and takes what is left after everybody else is paid.
 */
export const fundManagerKind: PartyKindProfile = {
  objective: 'theResidual',
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
  /**
   * A3, Register B3: THE CLAIM IS THE BOOK. What a fund owes its holders IS what its pool is worth,
   * so when the pool moves the liability moves with it and the fund's own equity stays at zero,
   * where a fund's equity belongs. This is the one kind in this world whose issuer's obligation
   * genuinely follows a price — every other liability here is owed at its face.
   */
  owes: 'value',
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
  readonly navStruck: PerPiece;
  readonly since: number;
}

interface Book {
  queued: Queued[];
  /** The NAV struck this period, so the orders and the settlement read one number (Law 4). */
  struck: Record<string, number>;
}

function emptyBook(): Book {
  return { queued: [], struck: {} };
}

const declOf = (decls: readonly FundDecl[], fund: string): FundDecl | undefined =>
  decls.find((f) => f.fund === fund);

/**
 * XI-14, item 10e.4: WHAT IS LEFT HERE IS THE WORLD'S NUMBERS, and the FUND'S ARE ON ITS MANDATE.
 *
 * Three per-fund parameters have gone — the buffer, the fee and what its investors require — and
 * they are terms of the agreement between the pool and its manager now, struck when the mandate was
 * written. That is where they belong: they are what two named parties agreed, the way a loan's rate
 * is on the loan, and a parameter register is for numbers the WORLD declares.
 *
 * ONE OF THEM WAS A PLACEHOLDER AND ITS DEATH IS THIS ITEM. The fee's reason said so in its own
 * words — *"no manager competes for the mandate, so the number stands where a competition should
 * be... the missing mechanism is a manager with a cost base"*. There is a manager with a cost base
 * now (`manager.ts`): it employs people out of the labour market, it knows what a pool costs it,
 * and what it charges for a pool it opens is what it takes to undercut whoever is already running
 * one. The fees this world OPENS with are drawn, which is an opening condition (Seed A3) and not a
 * placeholder — what makes the difference is that a mechanism now produces the number.
 *
 * And it is what made the roster an outcome: a parameter is declared at assembly, so a fund that
 * did not exist when the world was built could never have had one.
 */
function paramsOf(managers: readonly ManagerDecl[]): ParamDecl[] {
  return [
    {
      id: MANAGER_PARAMS.hoursPerPool,
      value: MANAGER_NUMBERS.hoursPerPool,
      unit: 'hours of the analysis trade one pool takes its manager in a period',
      dimension: 'count',
      kind: 'technology',
      owner: 'model',
      why: 'Fund Shares F3, Labour A2: what running one pool actually takes in people — the views, the dealing, the answering for it — and it is about the same for a small pool as for a large one. That is the whole economics of this industry, because the FEE is on the assets and the COST is on the product: a large pool carries a small one, and a house with one small pool cannot cover its own people. Nothing states that consequence; it falls out of a cost per product meeting a fee per pound.',
    },
    ...managers.flatMap((h): ParamDecl[] => [
      {
        id: managerParam(h.manager, 'undercut'),
        value: h.undercut,
        unit: 'share off the cheapest fee charged for the product it copies',
        dimension: 'ratio',
        kind: 'preference',
        owner: 'model',
        why: `Fund Shares F3, D2: how far ${h.name} comes in under the cheapest fee anybody already charges for a product it copies. An entrant has one lever and this is how hard it pulls it. What stops fees falling is not a floor: it is that the next entrant's fee would no longer cover what a pool costs it in people, so it does not open one (Law 6).`,
      },
      {
        id: managerParam(h.manager, 'patience'),
        value: h.patience,
        unit: 'periods',
        dimension: 'periods',
        kind: 'preference',
        owner: 'model',
        why: `Fund Shares F3: how long ${h.name} gives a pool it opened before it asks whether the fee covers the cost. A pool opened this week has been offered to nobody — the strike publishes it and a saver decides the week after — so a manager with no patience would close every fund it ever opened. Two managers with the same patience are one manager with two names (Ratings A4.b).`,
      },
    ]),
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
  ];
}

/**
 * B3, F3: what a fund owes its manager for the days this period actually has.
 *
 * ONE ACCRUAL, read by every fund there is (Law 4). It was written twice — once here and once
 * inline in `runInKind` — and the two copies did not agree about the case that matters, which is what
 * a second copy of a formula is for.
 *
 * Law 8: a per annum rate is not a per period one, and a fee is money, so what comes out is whole
 * pieces of it. Below one piece there is nothing to pay, and `payable` has already said so.
 */
function feeAccrued(ctx: MechanismContext, m: Mandate, share: Instrument): Qty {
  return ctx.registry.payable(
    feeOn(
      ctx,
      valueAt(ctx.valuation.markPerUnit(share.id, ctx.period), share.issued, 'net assets'),
      m.feePerAnnum,
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
function payFee(ctx: MechanismContext, m: Mandate, amount: Qty): void {
  if (amount <= 0) return;
  const fund = ctx.parties.get(m.pool);
  const manager = ctx.parties.get(m.manager);
  const leg: Leg = {
    kind: 'money',
    from: ctx.accountOf(fund.id, ctx.registry.currencyOf(fund.region)),
    to: ctx.accountOf(manager.id, ctx.registry.currencyOf(fund.region)),
    ccy: ctx.registry.currencyOf(fund.region),
    amount,
    fromCell: none(),
    toCell: none(),
  };
  const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `${fund.id} pays its manager` });
  ctx.record(
    'fund.fee',
    [fund.id, manager.id],
    { fund: fund.id, manager: manager.id, amount, paid: r.outcome === 'settled' },
    false,
  );
}

/** C1: cash in, shares out, one instruction. C3: the shares outstanding change, so a fund is not fixed-size. */
function subscribe(
  ctx: MechanismContext,
  m: Mandate,
  share: Instrument,
  holder: PartyId,
  sharesAsked: Qty,
  perShare: PerPiece,
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
  const fund = ctx.parties.get(m.pool);
  const ccy = ctx.registry.currencyOf(fund.region);
  // C1.d of the buyer's own budget: it subscribes with the money it has, and what it cannot pay
  // for it does not buy. That is a budget, not a bound on the decision.
  const cash = heldAsMoney(ctx.participant(holder).cash(ccy), 'what this holder has');
  const wanted = valueAt(perShare, sharesAsked, 'what it asked to put in');
  const budget = atMost(wanted, cash, 'it cannot spend money it does not hold');
  // Law 8: SHARES ARE ISSUED IN WHOLE PIECES and paid for in whole pieces of money, per member of a
  // cell (XI-15). The shares are struck first, because they are the thing being bought, and what
  // is paid is what they come to at the NAV — the nearest piece, so the fund is not shaved by a
  // fraction on every subscription it ever takes.
  let shares = downTick(amountOf(budget, perShare, 'shares it gets'));
  let paid = ctx.registry.cashFor(valueAt(perShare, shares, 'what it pays'));
  if (paid > cash) {
    shares = subQty(shares, ONE_PIECE, 'a piece less');
    paid = ctx.registry.cashFor(valueAt(perShare, shares, 'what it pays'));
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
  const r = ctx.settle({ legs, cause: 'issuance', reason: `${holder} subscribes to ${fund.id}` });
  ctx.record(
    'fund.subscribed',
    [fund.id, holder],
    { fund: fund.id, holder, sharesPerMember: shares, perShare, settled: r.outcome === 'settled' },
    false,
  );
}

/**
 * C2, C2.a: shares back, cash out, at the NAV struck when it asked. Returns how many shares per
 * member it could NOT pay for, which is what stays queued (C2.b: never dropped).
 */
function redeem(
  ctx: MechanismContext,
  m: Mandate,
  share: Instrument,
  holder: PartyId,
  sharesAsked: Qty,
  perShare: PerPiece,
): number {
  const party = ctx.parties.get(holder);
  const fund = ctx.parties.get(m.pool);
  const ccy = ctx.registry.currencyOf(fund.region);
  // XI-15: the register holds a cell's position PER MEMBER, which is the unit a request is in.
  const held = ctx.register.quantity(holder, share.id);
  const weight = weightOf(party);
  const asked = atMost(ctx.registry.deliverable(sharesAsked), held, 'it cannot hand back shares it does not hold');
  if (asked <= 0) return 0;
  // C2.a: from its buffer, or by selling. What it can pay now is what it holds now.
  const cash = ctx.register.quantity(
    fund.id,
    moneyInstrumentId(ctx.accountOf(fund.id, ccy).issuer, ccy),
  );
  const owedNow = ctx.registry.payable(valueAt(perShare, totalFor(party, asked), 'what it owes this holder'),
  );
  const paying = atMost(owedNow, cash, 'it pays out of the money there is');
  // Law 8: shares come back in whole pieces, per member, and the cash is what they come to at the
  // NAV — the nearest piece of money. What cannot be paid for stays in the queue (C2.b).
  const sharesNow = downTick(
    over(
      amountOf(heldAsMoney(paying, 'what it can pay with'), perShare, 'shares it can pay for'),
      asRatio(weight, 'the members of this cell'),
      'per member',
    ),
  );
  const perMemberCash = ctx.registry.cashFor(valueAt(perShare, sharesNow, 'what a member is paid'),
  );
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
    const r = ctx.settle({ legs, cause: 'maturity', reason: `${fund.id} redeems for ${holder}` });
    if (r.outcome === 'settled') {
      ctx.record(
        'fund.redeemed',
        [fund.id, holder],
        { fund: fund.id, holder, sharesPerMember: sharesNow, perShare },
        false,
      );
      return subQty(asked, sharesNow, 'what is left to pay');
    }
  }
  return asked;
}

/**
 * B1, B3, C1, C2: the day's work. Read what a share is worth, pay the manager, take in what was
 * subscribed and pay out what was asked for — and say what it could not pay, because that is what
 * it must sell for (C2.a) and what the market is about to see.
 */
/**
 * D2, D2.a, B1, Law 19 (item 9.9): WHAT A SHARE WAS WORTH LAST TIME, read off what this fund
 * PUBLISHED then.
 *
 * It was a second copy — `Book.previous`, written at the end of `strike` — of the `perShare` the
 * very same call had already recorded on `fund.struck`, a public event. That is the mirror Law 19
 * is about: two records of one fact, one of them private and invisible to everybody the number
 * exists for. And it exists FOR them: D2 says a fund competes with a deposit, and a competition
 * cannot happen against a number nobody can see.
 *
 * Nothing is stored now. The last strike is the last strike, whenever it was, so a fund that missed
 * a period returns over the period it actually last struck in rather than over a gap — which the
 * private copy said nothing about either way.
 */
function lastNav(ctx: MechanismContext, fund: string): Option<PerPiece> {
  const said = ctx.journal.lastOf('fund.struck', fund);
  if (said === undefined) return none<PerPiece>();
  const was = said.data['perShare'];
  // Item 16: a published number re-enters the type system here, through its dimension's own door.
  return typeof was === 'number' ? some(asPerPiece(was, 'what a share was worth then')) : none<PerPiece>();
}

/**
 * F3, C2.b, C4, item 10e.4: THE COMPULSORY REDEMPTION. Every holder is put on the queue for
 * everything it holds, at the NAV struck today, exactly as if each of them had asked.
 *
 * It is one queue and one payment convention (Appendix B), so a holder of a fund being wound up and
 * a holder who asked for its money back on an ordinary Tuesday are paid by the same code at the
 * same NAV in the same order. What it already asked for is already on the book under its own name
 * (C4, at the NAV of the day it asked), so only the REST is added — asking twice for one holding
 * would be two claims where there is one.
 */
function queueEverybody(
  ctx: MechanismContext,
  b: Book,
  m: Mandate,
  share: Instrument,
  perShare: PerPiece,
): void {
  const fundId = m.pool;
  for (const holder of ctx.register.holdersOf(share.id)) {
    if (holder === fundId) continue;
    const held = ctx.register.quantity(holder, share.id);
    if (held <= 0) continue;
    const already = sum(
      b.queued
        .filter((q) => q.fund === String(fundId) && q.holder === holder)
        .map((q) => q.sharesPerMember),
    ).value;
    const rest = subQty(held, already, 'shares of its it has not already asked back');
    if (rest <= 0) continue;
    b.queued.push({
      fund: String(fundId),
      holder,
      sharesPerMember: asQty(rest, 'shares redeemed out of it per member'),
      navStruck: perShare,
      since: ctx.period,
    });
    ctx.record(
      'fund.requested',
      [fundId, holder],
      { fund: fundId, holder, sharesPerMember: rest, perShare, why: 'windingUp' },
      false,
    );
  }
}

function strike(ctx: MechanismContext, b: Book, m: Mandate): void {
  const fundId = m.pool;
  // XI-3, Register F2: a fund that has ceased strikes nothing. Its investors' claims resolve
  // through its estate like anybody else's (Firm Birth D5).
  if (!ctx.parties.get(fundId).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(String(fundId)));
  const opening = ctx.params.price(FUND_PARAMS.openingShare);
  const ccy = ctx.registry.currencyOf(ctx.parties.get(fundId).region);
  const previous = lastNav(ctx, String(fundId));
  // B3: the fee is charged on what the book was worth before anybody transacted, and then the NAV
  // is read again — which is what "fees reduce NAV" means when the reduction is a real payment.
  if (share.issued > 0) payFee(ctx, m, feeAccrued(ctx, m, share));
  const perShare = share.issued > 0 ? ctx.valuation.markPerUnit(share.id, ctx.period) : opening;
  b.struck[String(fundId)] = perShare;
  // B2.a: how old the oldest mark behind it is. A stale mark makes a stale NAV and somebody
  // transacts on it: that is a real transfer between holders and it is said out loud.
  let oldest = ctx.period;
  for (const h of ctx.register.holdingsOf(fundId)) {
    if (h.instrument === share.id) continue;
    const worth = ctx.valuation.worthOf(fundId, h.instrument, ctx.period);
    if (worth.some && worth.value.from < oldest) oldest = worth.value.from;
  }
  if (oldest < ctx.period) {
    ctx.record(
      'fund.staleNav',
      [fundId],
      { fund: fundId, perShare, oldestMark: oldest, periodsStale: ctx.period - oldest },
      true,
    );
  }
  // F3, G1, C2.b, item 10e.4: NOTICE HAS BEEN GIVEN, so every holder goes on the queue at this
  // NAV whether they asked or not. That is what winding a fund up IS, and from here it is the
  // ordinary redemption path: the fund pays what its cash reaches, sells for the rest at whatever
  // the market gives (C2.a, XI-2), and the holders who are paid last get what the sales fetched.
  if (m.windingUp) queueEverybody(ctx, b, m, share, perShare);
  for (const o of ctx.posted(fundVenue(String(fundId)))) {
    if (o.qty <= 0) continue;
    // XI-15, Law 8: a posting is a total and a cell's decision is per member; this is the one place
    // the two meet, and it is the same conversion every other market makes. A SHARE IS INDIVISIBLE,
    // so what a member can ask for or hand back is a whole number of them — whoever posted it has
    // already decided in whole shares, and this is where a posting that did not is refused rather
    // than quietly queued for ever.
    const asked = ctx.registry.deliverable(over(o.qty, asRatio(weightOf(ctx.parties.get(o.party)), 'the members it has'), 'shares per member'),
    );
    if (asked <= 0) continue;
    if (o.side === 'buy') {
      // C1, F3: A FUND BEING WOUND UP TAKES NOBODY NEW. Selling a claim on a book that is being
      // sold off is selling somebody a share of a queue, and the refusal is recorded because a
      // saver whose subscription did not happen has a real fact about its own money (App A).
      if (m.windingUp) {
        ctx.record(
          'fund.notSubscribing',
          [fundId, o.party],
          { fund: fundId, holder: o.party, sharesPerMember: asked, why: 'windingUp' },
          true,
        );
        continue;
      }
      subscribe(ctx, m, share, o.party, asked, perShare);
    } else {
      // A holder cannot ask back what it does not have, and what it has already asked for is
      // already on the book — a second ask for the same shares is the same claim, not another one.
      // This is not C2.b's rationing: nothing that was ever a claim is dropped here.
      const held = ctx.register.quantity(o.party, share.id);
      const already = sum(
        b.queued.filter((q) => q.fund === String(fundId) && q.holder === o.party).map((q) => q.sharesPerMember),
      ).value;
      const room = subQty(held, already, 'shares it has not already asked back');
      const taking = atMost(asked, room, 'it cannot ask back shares it has already asked back');
      if (taking <= 0) continue;
      /**
       * G1, G1.a, G1.b (item 10e): AND WHETHER IT MAY ASK AT ALL, which is the terms it came in on.
       *
       * A CLOSED vehicle has no redemption: the money was committed for the life of the fund and
       * nobody can demand it back, which is the entire reason the structure exists and is what
       * makes it the one thing in this sector that can never be a forced seller. A LISTED one has
       * none either — you sell the share to a HOLDER, in a market, and creation and redemption are
       * in kind against the basket (G1.a).
       *
       * The refusal is RECORDED, because an investor asking for money it agreed it could not have
       * is a real event about that investor's own position — and Appendix A: a refusal is an answer.
       */
      if (!redeemable(m.liquidity)) {
        ctx.record(
          'fund.notRedeemable',
          [fundId, o.party],
          { fund: fundId, holder: o.party, sharesPerMember: taking, terms: m.liquidity.how },
          true,
        );
        continue;
      }
      // C2.b: the request goes on the book under its own name at the NAV of the day it asked. What
      // happens to it after that is a question of cash, never of whether it counts.
      b.queued.push({
        fund: fundId,
        holder: o.party,
        sharesPerMember: asQty(taking, 'shares it asked back per member'),
        navStruck: perShare,
        since: ctx.period,
      });
      ctx.record(
        'fund.requested',
        [fundId, o.party],
        { fund: fundId, holder: o.party, sharesPerMember: taking, perShare },
        false,
      );
    }
  }
  payQueue(ctx, b, m);
  const owed = owedOn(ctx, b, m);
  const cash = ctx.register.quantity(
    fundId,
    moneyInstrumentId(ctx.accountOf(fundId, ccy).issuer, ccy),
  );
  // C2.a, Law 8: WHAT IT KEEPS BACK IS MONEY, so it is a whole number of the smallest piece of it.
  // Up, because it is what the fund insists on holding: a buffer a cent short of what its own rule
  // asks for is a buffer it did not keep. What is left over after it — what the fund has spare —
  // is then a count too, and the schedule its orders are built from is on the grid by arithmetic
  // rather than by a rounding somewhere further down.
  const buffer = upTick(
    scale(
      valueAt(perShare, share.issued, 'net assets'),
      m.buffer,
      'the cash it keeps back',
    ),
  );
  const offer = offeredYield(ctx, m);
  ctx.record(
    'fund.struck',
    [fundId],
    {
      fund: fundId,
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
        !previous.some || previous.value <= 0
          ? null
          : ratioOf(
              minus(perShare, previous.value, 'what it made'),
              previous.value,
              'per share it returned',
            ),
      // C2.a: what it must find, and what it has spare. Its orders read these and nothing else,
      // so the decision and the schedule are one decision (Law 4).
      shortfall: owed,
      spare: subQty(subQty(cash, buffer, 'over its buffer'), owed, 'and after what it owes'),
      oldestMark: oldest,
      // D2, D2.a: what it offers a saver — what the paper its mandate lets it hold is yielding,
      // less what its manager takes. A fund with no curve to read publishes NO OFFER, and the key
      // is absent rather than zero: a saver reading the tape can tell "nothing to say" from "nothing
      // on offer", which a number cannot say (App A).
      ...(offer.some ? { offered: offer.value } : {}),
    },
    true,
  );
}

/**
 * D2.a: what the fund offers, read from the public curve at the longest thing its mandate lets it
 * hold, less its fee. It is not a forecast and not a promise: it is what that paper is fetching
 * today (Sovereign D3), which is the only thing anybody can compare a deposit against.
 */
function offeredYield(ctx: MechanismContext, m: Mandate): Option<number> {
  const region = ctx.registry.region(ctx.parties.get(m.pool).region);
  /**
   * D2.a, item 10e.4: THE LONGEST THING ITS MANDATE LETS IT HOLD, read off the DURATION BAND.
   *
   * It read `fundParam(fund, 'maxTenorPeriods')` — a parameter that item 10e.2 deleted when the
   * kind list and the tenor became one blueprint, and which nothing has declared since. A read of a
   * parameter that is not in the register THROWS, so this was a build-stopper in the strike of every
   * fund in the world, and the first period would not have finished (finding `E-20`; fixed where it
   * stands, as the rules for a violation that stops the build require).
   *
   * A blueprint states its duration in YEARS from today, which is the number a curve is asked at —
   * so the tenor in periods, the date it came to and the year fraction back out of that date are
   * all gone, and the read is the band itself (Law 19). A mandate that states no upper duration has
   * no longest thing and nothing to quote: an equity fund does not offer a yield, and saying so is
   * the honest answer rather than a point on a curve nobody asked for (App A).
   */
  const years = m.blueprint.duration?.to;
  if (years === undefined || years <= 0) return none<number>();
  /**
   * D2.a: THE BEST of the curves in its money, and `best` is what the word says.
   *
   * It was the LAST family the map happened to iterate that had a yield — an outcome of insertion
   * order, so a world that registered its curves in another order offered its savers a different
   * number for the same paper. And when none of them answered it fell to `best = 0`, so a fund with
   * no curve to read published a NEGATIVE offer of exactly its own fee, which is a saver being told
   * to pay for the privilege rather than a fund with nothing to say (App A: missing is missing).
   */
  let best: Option<Ratio> = none();
  for (const family of ctx.registry.curveFamilies.values()) {
    if (family.ccy !== ctx.registry.currencyOf(region.id)) continue;
    const read = ctx.curve(family.id).at(years);
    if (!read.yield.some) continue;
    if (!best.some || read.yield.value > best.value) best = some(read.yield.value);
  }
  if (!best.some) return none();
  return some(minus(best.value, m.feePerAnnum, 'what a saver gets after the manager'));
}

/** What this fund still owes its redeemers, at the NAV each of them struck (C4). */
function owedOn(ctx: MechanismContext, b: Book, m: Mandate): Qty {
  const terms: number[] = [];
  for (const q of b.queued) {
    if (q.fund !== String(m.pool)) continue;
    const holder = ctx.parties.get(q.holder);
    // C2, Law 8: what a redemption COMES TO is shares at a NAV, so it lands between two pieces of
    // money — and what the fund must find is the piece above, because paying all but a fraction of
    // a cent is not paying. It is the same rounding a subscription takes the other way (`cashFor`).
    terms.push(
      upTick(valueAt(q.navStruck, totalFor(holder, q.sharesPerMember), 'what it is owed')),
    );
  }
  return asQty(sum(terms).value, 'what its queue is owed');
}

/**
 * C2.a, C2.b, C4: pay what the cash reaches, oldest request first, at the NAV each one struck. What
 * it cannot pay stays on the book under its own name — never rationed away — and the fund is gated
 * until it is paid, which is public because a gate is information (C4.a: it is a cost to those who
 * stay, and they may want to leave too).
 */
function payQueue(ctx: MechanismContext, b: Book, m: Mandate): void {
  const fundId = m.pool;
  if (!ctx.parties.get(fundId).status.alive) return;
  /**
   * G1 (item 10e): A SEMI-LIQUID FUND PAYS WHEN ITS WINDOW IS OPEN and not otherwise.
   *
   * The queue is the mechanism, and what it costs is C4.a: a holder that asked in a closed period
   * waits, and the NAV it struck when it asked is the one it gets — so the difference between that
   * and what the fund realises when it does sell falls on the holders who stayed. That is the whole
   * reason the terms exist, and it is why a shock reaches this vehicle LATER and a liquid one now.
   *
   * It is not a gate anybody chose to close (Law 6): the window is a term of the mandate its
   * investors agreed to, and between windows there is simply no payment date.
   */
  // G1, item 10e.4: a pool being WOUND UP pays whenever it has the money. The window is a term
  // for a fund that has a next one; one that is being closed does not, and a queue that waited for
  // a window that will never open again is a holder who is never paid.
  if (!m.windingUp && !payingThisPeriod(m.liquidity, ctx.period)) return;
  const share = ctx.instruments.get(shareLineOf(String(fundId)));
  // Clearing C3: first come, first served is a stated rule, applied the same way every time.
  const mine = b.queued.filter((q) => q.fund === String(fundId)).sort((x, y) => x.since - y.since);
  if (mine.length === 0) return;
  const left: Queued[] = [];
  for (const q of mine) {
    const unpaid = redeem(ctx, m, share, q.holder, q.sharesPerMember, q.navStruck);
    if (unpaid > 0) left.push({ ...q, sharesPerMember: asQty(unpaid, 'shares still owed per member') });
  }
  b.queued = [...b.queued.filter((q) => q.fund !== String(fundId)), ...left];
  if (left.length === 0) return;
  ctx.record(
    'fund.gate',
    [fundId, ...left.map((q) => q.holder)],
    {
      fund: fundId,
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
function launchInKind(ctx: MechanismContext, e: FundDecl): void {
  const fund = e.fund as PartyId;
  if (ctx.parties.has(fund)) return;
  // Indices D5.a: before its own rule has answered there is no index, and a tracker on one is a
  // mandate with nothing in it.
  /**
   * Indices D5.a, C2: before its own rule has answered there is no index, and a tracker on one is a
   * mandate with nothing in it. A fund that tracks NOTHING is active and is not this path at all.
   */
  const index = e.tracks;
  if (index === undefined || !ctx.index(index).some) return;
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
    [fund, FUND, nameOf({ ...e, house: e.bank })],
  ] as const) {
    if (ctx.parties.has(id)) continue;
    ctx.enter({
      id,
      kind,
      region: region.id,
      name,
      bank,
      representation: 'named',
      status: { alive: true, standing: 'good' },
    });
  }
  /**
   * A4, E3, F3, XI-8, item 10e: THE MANDATE THIS POOL IS LAUNCHED UNDER — and a tracker's mandate
   * and its BASKET are two different things, which this used to conflate.
   *
   * It took the kinds its basket happened to name and called that the mandate. But an index fund's
   * mandate is the ASSET CLASS its investors bought — listed equity, or government paper — and the
   * INDEX is what says which lines and in what weights (E3). Keeping them apart is what lets the
   * index change its constituents without anybody rewriting the fund's mandate, which is what an
   * index doing its job looks like.
   *
   * So the blueprint is the classes the basket is made of, read off the classification rather than
   * off the kinds — and `currencies: [its own]`, because a tracker of a domestic index is a
   * single-currency vehicle and that is a term its investors agreed to.
   */
  const ccy = ctx.registry.currencyOf(region.id);
  openMandate(ctx, fund, manager, ccy, {
    ...productOf(e, ccy),
    blueprint: {
      classes: [
        ...new Set(Object.keys(inKindOf(e).basket).map((line) => ctx.classify(instrumentId(line)).what)),
      ],
      currencies: [ccy],
    },
  });
  const share = shareLineOf(e.fund);
  const market = listedMarketOf(e.fund);
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
    name: `${nameOf({ ...e, house: e.bank })} shares`,
    instrument: share,
    ccy: ctx.registry.currencyOf(region.id),
    rationing: 'proRata',
  });
  // G1.a: where it is created and redeemed IN KIND — not a market and it does not clear, so the
  // module that owns it runs it itself (Clearing B2).
  ctx.openVenue({
    id: inKindVenue(e.fund),
    name: `${nameOf({ ...e, house: e.bank })} creations and redemptions`,
    clearedBy: 'funds',
    unit: SHARES,
    ccy: ctx.registry.currencyOf(region.id),
    // Item 10e: the key says WHAT HAPPENS HERE — creation and redemption in kind — rather than
    // naming a kind of fund, so a household reading the door reads a fact and not a label.
    key: { kind: 'inKind', fund: e.fund, share },
  });
  const launched = firstCreation(ctx, e, share, size);
  ctx.record(
    'fund.launched',
    [e.fund, index, String(share)],
    { fund: e.fund, tracks: index, shares: launched },
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
function launchSize(ctx: MechanismContext, e: FundDecl): Qty {
  let full: Qty | undefined;
  for (const [holder, slice] of launchers(ctx, e)) {
    const reaches = over(
      couldCreate(ctx, e, holder),
      asRatio(slice, 'its declared slice'),
      'the launch its slice reaches',
    );
    if (full === undefined || reaches < full) full = reaches;
  }
  if (full === undefined) return NO_QTY;
  return full;
}

/** Seed A3, E3.a: who is putting the basket in, of those this world actually has. */
function launchers(ctx: MechanismContext, e: FundDecl): readonly (readonly [PartyId, number])[] {
  const out: (readonly [PartyId, number])[] = [];
  for (const [holder, slice] of Object.entries(inKindOf(e).by)) {
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
  e: FundDecl,
  share: InstrumentId,
  full: Qty,
): Qty {
  let made = NO_QTY;
  for (const [holder, slice] of launchers(ctx, e)) {
    const wanted = downTick(scale(full, asRatio(slice, "this holder's slice"), "this holder's slice of the launch"));
    if (wanted <= 0) continue;
    if (create(ctx, e.fund as PartyId, inKindOf(e).basket, share, holder, wanted)) {
      made = addQty(made, wanted, 'shares created');
    }
  }
  return made;
}

/** E3: the most shares this party could create out of what it actually holds, line by line. */
function couldCreate(ctx: MechanismContext, e: FundDecl, party: PartyId): Qty {
  const weight = weightOf(ctx.parties.get(party));
  let most: Qty | undefined;
  for (const [line, perShare] of Object.entries(inKindOf(e).basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const free = scaleQty(
      ctx.register.free(party, id),
      weight,
      'what it holds free of this line',
    );
    const backs = downTick(
      over(free, asRatio(perShare, 'what one share draws of it'), 'shares this line of its basket backs'),
    );
    if (most === undefined || backs < most) most = backs;
  }
  // A basket naming no line this world has is not a basket, and nothing backs a share of it. That
  // is a COUNT of shares it could create and not a missing number (Appendix A).
  if (most === undefined) return NO_QTY;
  return most;
}

function runInKind(ctx: MechanismContext, m: Mandate, d: FundDecl): void {
  const fund = m.pool;
  if (!ctx.parties.get(fund).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  if (!share.status.live) return;
  // B3, F3: the fee is a real payment out of what the fund holds, and it reduces the NAV because
  // the NAV is a read of the book and the book is smaller once it has left. The same accrual and
  // the same convention as every other fund (Law 4): what it owes goes to the wire whole, and a
  // fund without the money has a refused payment rather than a discount nobody granted it.
  const money = moneyInstrumentId(
    ctx.accountOf(fund, ctx.registry.currencyOf(ctx.parties.get(fund).region)).issuer,
    ctx.registry.currencyOf(ctx.parties.get(fund).region),
  );
  if (share.issued > 0) payFee(ctx, m, feeAccrued(ctx, m, share));
  // G1.a: in kind, against a pro-rata slice of its own book. Nothing is sold and no market is
  // touched, which is why this vehicle is not the forced seller (the money fund is, C2.b).
  for (const o of ctx.posted(inKindVenue(d.fund))) {
    if (o.qty <= 0) continue;
    if (o.side === 'buy') create(ctx, fund, inKindOf(d).basket, share.id, o.party, o.qty);
    else redeemInKind(ctx, fund, inKindOf(d).basket, share.id, o.party, o.qty);
  }
  distribute(ctx, m, share.id, money);
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
function placeSpareCash(ctx: MechanismContext, m: Mandate): void {
  const fund = ctx.parties.get(m.pool);
  if (!fund.status.alive) return;
  const ccy = ctx.registry.currencyOf(fund.region);
  const cash = ctx.register.quantity(
    fund.id,
    moneyInstrumentId(ctx.accountOf(fund.id, ccy).issuer, ccy),
  );
  // C2.a: it keeps its own buffer against the redemptions it expects and places the rest.
  const buffer = scale(cash, m.buffer, 'what it keeps liquid');
  const spare = ctx.registry.payable(heldAsMoney(minus(cash, buffer, 'what it can place'), 'what it can place'),
  );
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
function distribute(ctx: MechanismContext, m: Mandate, share: InstrumentId, money: InstrumentId): void {
  const fund = ctx.parties.get(m.pool);
  const issued = ctx.instruments.get(share).issued;
  const cash = ctx.register.quantity(fund.id, money);
  if (issued <= 0 || cash <= 0) return;
  const perShare = pricedAt(heldAsMoney(cash, 'what it has to pass on'), issued, 'what it passes on per share');
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
    const share2 = shareFor(
      ctx.registry,
      party,
      currencyUnit(ccy),
      valueAt(perShare, perMemberUnits, 'what a member is paid'),
    );
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
    [fund.id, share],
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
function readListed(ctx: MechanismContext, m: Mandate, d: FundDecl): void {
  const fund = m.pool;
  if (!ctx.parties.get(fund).status.alive) return;
  const share = ctx.instruments.get(shareLineOf(d.fund));
  if (!share.status.live) return;
  const basket = basketOf(ctx, fund, inKindOf(d).basket, share.id);
  const nav = share.issued > 0
    ? ctx.valuation.markPerUnit(share.id, ctx.period)
    : basketValue(basket);
  const print = ctx.prices.latest(share.id, ctx.period);
  const premium = premiumOf(print.some ? some(print.value.price) : none<PerPiece>(), nav);
  ctx.record(
    'fund.listedStruck',
    [fund, share.id],
    {
      fund,
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

/* --------------------------------------------------------------------------------------------
 * THE MANAGER'S PERIOD: what it closed, and what it opened
 * ------------------------------------------------------------------------------------------ */

/**
 * F3, item 10e.4: the manager's own period. Three things, in the order a business does them: what a
 * pool costs it, which of its pools no longer pay for themselves, and whether there is a product
 * worth opening. It decides nothing at all if it cannot yet price an hour of its own people's time,
 * which is the honest state of a business that has never met a wage (App A).
 */
function runManagers(ctx: MechanismContext): void {
  for (const manager of ctx.parties.ofKind(FUND_MANAGER)) {
    if (!manager.status.alive) continue;
    const view = ctx.participant(manager.id);
    const cost = costOfAPool(view);
    if (!cost.some) continue;
    for (const n of noticeToGive(ctx, view, cost.value)) giveNotice(ctx, n);
    const launch = launchToMake(ctx, view, livingPools(ctx), cost.value);
    if (launch.some) openPool(ctx, manager.id, launch.value);
  }
  // A pool that has paid its last holder and sold its last position is finished. It is asked of the
  // pools as they are AFTER the notices, so a product that never gathered a share is closed the
  // period its manager gave up on it — there is nothing to sell and nobody to pay — while one with
  // holders in it has shares outstanding and is not finished by this test for as long as that
  // takes.
  for (const m of livingPools(ctx)) if (m.windingUp) finishWindUp(ctx, m);
}

/**
 * F3, G1: NOTICE. The mandate is restated rather than ended — a pool with no mandate has nobody
 * deciding for it and its holders and its book would both still be there — and everything that
 * follows is the ordinary redemption path (`strike`, `payQueue`) running until there is nothing
 * left. It is public: a fund closing is information its holders and its market both need (C4.a).
 */
function giveNotice(ctx: MechanismContext, n: Notice): void {
  const a = ctx.agreements.get(mandateIdOf(ctx, n.pool));
  if (!isMandate(a.terms)) return;
  const noticed: MandateTerms = { ...a.terms, windingUp: true };
  ctx.restate(a.id, noticed);
  ctx.record(
    'fund.notice',
    [n.pool, a.creditor],
    { fund: n.pool, manager: a.creditor, earns: n.earns, costs: n.costs },
    true,
  );
}

/** A4: the mandate this pool is run under, as an id. A pool with none never reached this. */
function mandateIdOf(ctx: MechanismContext, pool: PartyId): AgreementId {
  for (const a of ctx.agreements.ofKind(MANDATE)) {
    if (a.state === 'performing' && a.debtor === pool && isMandate(a.terms)) return a.id;
  }
  throw new InvalidRegistry('Fund Shares A4', `${pool} is run under no mandate`);
}

/**
 * F3, Seed A3, item 10e.4: THE LAUNCH. A pool is a party, a mandate, a share line and a door its
 * investors come in at — exactly what the seed opens one with, opened by a manager instead.
 *
 * It opens with NOBODY IN IT and nothing in its account, which is the same opening every fund in
 * this world has (Seed E): the first subscription is a decision somebody takes at the unit its
 * shares are counted in, and a pool nobody subscribes to is a product that failed, which its
 * manager will close.
 */
function openPool(ctx: MechanismContext, manager: PartyId, launch: Launch): void {
  const house = ctx.parties.get(manager);
  const pool = launchedPoolId(manager, ctx.period);
  if (ctx.parties.has(pool)) return;
  // A1.c: a pool banks where its house banks, so a house whose bank has gone has nowhere to open
  // one until it has moved its own account (`fundChoosesBank`). That is a real wait, not a guard.
  if (!ctx.parties.has(house.bank) || !ctx.parties.get(house.bank).status.alive) return;
  const ccy = ctx.registry.currencyOf(house.region);
  const name = nameOf({ blueprint: launch.product.blueprint, house: house.name });
  ctx.enter({
    id: pool,
    kind: FUND,
    region: house.region,
    name,
    // A1.c: a pool banks where its manager banks, which is the relationship that already exists.
    bank: house.bank,
    representation: 'named',
    status: { alive: true, standing: 'good' },
  });
  openMandate(ctx, pool, manager, ccy, launch.product);
  const share = shareLineOf(String(pool));
  ctx.issue({
    id: share,
    kind: FUND_SHARE,
    issuer: some(pool),
    ccy,
    terms: shareTerms(pool),
    // A2: an open-ended fund's shares do not trade. What one is worth is what the book comes to.
    market: none(),
  });
  ctx.openVenue({
    id: fundVenue(String(pool)),
    name: `${name} subscriptions and redemptions`,
    clearedBy: 'funds',
    unit: SHARES,
    ccy,
    key: { kind: 'fund', fund: String(pool), share },
  });
  /**
   * A3, C1, F3, item 10e.5: THE SEED GOES IN THROUGH THE FRONT DOOR.
   *
   * A manager putting its own money into a pool it opened is an INVESTOR SUBSCRIBING: it posts into
   * the pool's own venue, at the NAV the strike is about to set, and takes the shares it pays for —
   * the same instruction, the same convention and the same refusal as a household (C1). Nothing is
   * endowed and nothing is written into the register (Appendix B: no seeded outcome), which is why
   * a pool a manager opened and a pool this world opened with are the same object.
   *
   * It is posted rather than settled here because the NAV is the strike's to set (Law 4), and this
   * phase runs before it: what the manager says is how many shares it wants, at the unit a pool
   * with no shares counts them in, and what it actually gets is what its money reached.
   */
  const opening = ctx.params.price(FUND_PARAMS.openingShare);
  const wants = opening > 0 ? downTick(amountOf(launch.seed, opening, 'shares its seed buys')) : NO_QTY;
  if (wants > 0) {
    ctx.post(fundVenue(String(pool)), {
      party: manager,
      side: 'buy',
      // C1: nobody names a price at this door. Everybody who asks transacts at the NAV.
      price: 'market',
      qty: wants,
    });
  }
  ctx.record(
    'fund.launched',
    [pool, manager],
    {
      fund: pool,
      manager,
      name,
      // A3: what the house put in of its own, which is the whole of its exposure to this pool.
      seed: launch.seed,
      seedShares: wants,
      // Law 3, F3: the two numbers the decision was taken on, both of them somebody else's — what a
      // rival of this product has actually gathered, and what this manager will charge to undercut
      // the cheapest of them.
      copying: launch.rival,
      expects: launch.expects,
      feePerAnnum: launch.product.feePerAnnum,
      earns: launch.earns,
    },
    true,
  );
}

/**
 * F3, Register F2, XI-8, item 10e.4: THE END OF A WIND-DOWN. Every share is back, every position is
 * sold, and what is left is dust the grid could not pay out.
 *
 * THE DUST HAS A HOLDER, which is the whole of why this is here: a pool that ceased holding money
 * would be a residual with nobody behind it (Law 2), and the holders it belonged to have all been
 * paid and gone. It goes to the manager as the last thing the pool pays for being run — one
 * two-sided instruction through the same door every other fee goes through (Law 4) — and the pool
 * ceases to the manager, so every reference to a closed fund resolves to the house that ran it.
 *
 * A pool that still holds a POSITION is not finished, however long it takes: it keeps striking,
 * keeps selling at whatever the market gives, and keeps paying its queue. Nothing here hurries it,
 * and a wind-down nobody will buy into is a real state this world can be in (XI-2).
 */
function finishWindUp(ctx: MechanismContext, m: Mandate): void {
  const pool = m.pool;
  const share = ctx.instruments.get(shareLineOf(String(pool)));
  if (share.issued > 0) return;
  const ccy = ctx.registry.currencyOf(ctx.parties.get(pool).region);
  const money = moneyInstrumentId(ctx.accountOf(pool, ccy).issuer, ccy);
  for (const h of ctx.register.holdingsOf(pool)) {
    if (h.instrument === money || h.instrument === share.id) continue;
    if (ctx.register.quantity(pool, h.instrument) > 0) return;
  }
  const left = ctx.register.quantity(pool, money);
  if (left > 0) payFee(ctx, m, left);
  ctx.endAgreement(m.id, `${pool} has been wound up`);
  ctx.cease(pool, m.manager);
  ctx.record('fund.woundUp', [pool, m.manager], { fund: pool, manager: m.manager, dust: left }, true);
}

/**
 * A4, C1.a, C2.b: what the fund takes to market. It has exactly two reasons to be there and they
 * are opposites: cash it must put to work per its mandate, and a redemption it must find the money
 * for. The second is the forced sale (XI-2): it names no price, because it has no choice.
 */
function ordersOf(
  view: ParticipantView,
  mandate: Mandate,
  m: MarketDecl,
): readonly Order[] {
  const own = view.lastOwnSince('fund.struck', view.period);
  if (!own.some) return [];
  const shortfall = own.value.data['shortfall'];
  const saidSpare = own.value.data['spare'];
  if (typeof shortfall !== 'number' || typeof saidSpare !== 'number') return [];
  // Item 16: money re-entering from what this fund published, at the read that knows what it is.
  const spare = asCash(saidSpare, 'what it published it has spare');
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
    const fraction = ratioOf(
      asCash(shortfall, 'what it published it is short of'),
      total,
      'the share of its book it must raise',
    );
    // Law 8: a share of a holding is a fraction of a unit, and a unit is what there is. It rounds
    // UP because this is a redemption it has to MEET — selling all but a fraction of what covers it
    // leaves the investor short — and it can never be more than what it holds.
    const wants = fraction >= 1 ? units : upTick(scale(units, fraction, 'units it must sell'));
    const qty = atMost(wants, units, 'there are no more units of it than there are');
    if (!material(qty, 2, units)) return [];
    // XI-2: at whatever the market gives. A forced seller that named a price would not be one.
    return [{ party: view.self.id, side: 'sell', price: 'market', qty }];
  }
  if (spare <= 0 || !eligible(view, mandate, i)) return [];
  // C1.a: it must buy something with the cash, and what it will pay is what makes the paper return
  // what its own investors require of it (D2). A price it will not pay does not fill.
  const on = view.calendar.startOf(view.period);
  const family = view.registry.curveFamily(curveFamilyOf(issuerOf(i), i.ccy));
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar, view.registry);
  if (flows.length === 0) return [];
  const price = priceAt(
    flows,
    mandate.requiredYieldPerAnnum,
    on,
    family.dayCount,
    `what ${i.id} is worth to ${mandate.pool}`,
  );
  if (price <= 0) return [];
  const lines = eligibleLines(view, mandate);
  if (lines === 0) return [];
  const each = over(
    spare,
    asRatio(lines, 'the lines it may hold'),
    'what it puts into each line it may hold',
  );
  const dirty = plus(price, view.accrued(i.id), 'what a unit costs it');
  // Law 8: whole units. Down, because it is what the cash it has spare actually reaches.
  const qty = downTick(amountOf(each, dirty, 'units it bids for'));
  // Law 7: this line's share against what the whole of what it has spare would have bought.
  const whole = amountOf(spare, dirty, 'what the whole of it would buy');
  if (qty <= 0 || !material(qty, lines + 1, whole)) {
    return [];
  }
  return [{ party: view.self.id, side: 'buy', price, qty }];
}

/**
 * What everything it holds is worth, at the last marks — the base a pro-rata sale is struck on.
 *
 * Currency C4.a, A-50: IN ITS OWN MONEY. This summed a print in whatever money the line is in, so
 * the denominator of the fraction a forced sale is struck on added dollars to euros and the fund
 * sold the wrong number of units of everything.
 */
function holdingsWorth(view: ParticipantView): Cash {
  const terms: Cash[] = [];
  for (const h of view.holdings()) {
    const print = view.print(h.instrument);
    if (!print.some) continue;
    terms.push(
      view.inOwnMoney(
        valueAt(print.value.price, sum(h.lots.map((l) => l.qty)).value, 'what it holds'),
        view.instruments.get(h.instrument).ccy,
      ),
    );
  }
  return sum(terms).value;
}

/**
 * A4, D1: what the mandate allows — the kind, and how long it may still have to run.
 *
 * XI-8 (item 9.2): WHAT IT MAY HOLD IS ITS OWN MANDATE'S, read off the pool's own commitment. It
 * used to be a field on a `FundDecl` row — a fact about this world's DATA rather than about these
 * two parties — which nothing outside this module could read and which no manager agreed to.
 */
function eligible(view: ParticipantView, m: Mandate, i: Instrument): boolean {
  if (!i.status.live) return false;
  /**
   * A4, item 10e: THE ONE QUESTION, ASKED ONCE. What this pool may hold is its blueprint, and the
   * ASSET ANSWERS FOR ITSELF out of the kernel's classification — so two funds cannot come to
   * different conclusions about the same paper, and nothing here enumerates the world.
   *
   * THREE HAND-WRITTEN CONSTRAINTS WENT INTO THE LANGUAGE and are gone from this function:
   *
   *  - the KIND LIST, which could not say "credit, three to seven years, senior" at all;
   *  - the CURRENCY test (A-47, A-50), which was hand-written and so a multi-currency mandate was
   *    inexpressible — a fund that may hold another money has an FX exposure its investors agreed
   *    to, and now its mandate is where that is agreed;
   *  - the TENOR test, which ran through `cashFlows` and therefore FAILED ANYTHING THAT PROMISES NO
   *    DATED PAYMENT by having no last flow. A share promises none, which is why no fund in this
   *    world could ever hold one whatever its mandate said (`docs/RECORD.md` item 4). A duration
   *    BAND asks where a duration exists, and a blueprint that wants shares does not state one.
   */
  if (!admits(m.blueprint, view.classify(i.id), () => undefined)) return false;
  /**
   * A4, Equity B1: AND WHETHER IT CAN PUT A NUMBER ON IT — a different question from whether the
   * mandate allows it, and the FUND's own refusal rather than its investors'.
   *
   * What it can value is asked of the kind's own valuation door, which every kind answers in its
   * own terms: a bond discounts its promise, a company capitalises what it published. A claim it
   * cannot value is one it does not buy.
   */
  return view.worth(i.id, m.requiredYieldPerAnnum).some;
}

/** How many lines the mandate lets it into, so what it has spare is spread over them and no more. */
function eligibleLines(view: ParticipantView, m: Mandate): number {
  let n = 0;
  for (const i of view.instruments.all()) {
    if (i.market.some && eligible(view, m, i)) n += 1;
  }
  return n;
}

/**
 * A3: a fund's equity is ZERO. The holders own the assets, so assets minus liabilities is nothing,
 * and a fund with equity has mislaid somebody's money.
 *
 * A-48: THE ZERO DOES NOT "FALL OUT OF THE WIRE", and this docstring said it did for a long time.
 * `fundShareKind` declares `owes: 'value'` and derives that value from `navOf().perShare`, so what
 * the fund owes its holders is what its book comes to BY CONSTRUCTION and `assets − liabilities = 0`
 * is an algebraic identity of the valuation, not an outcome of the instructions.
 *
 * What this family CAN catch, and why it is kept: a disagreement between two valuation PATHS. The
 * equity walk adds up what the register and the revaluation actually booked, period by period, and
 * the liability is `navOf` re-derived now. They are the same number only while every mark that was
 * booked is a mark `navOf` would still produce — so a rounding that was booked and not re-derived, a
 * currency conversion on one side and not the other (which is what stage 2d's `inOwnMoney` fixed in
 * `navOf`), or a stale mark on one path shows up here as a fund with equity.
 *
 * That is a narrower claim than the one it used to make, and it is the true one.
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
 * F3, A4, item 10e.4: EVERY POOL IS RUN BY SOMEBODY, and the manager it names is a party that is
 * still there.
 *
 * *"There is no fund without a manager"* (the owner). It used to be true by construction — the
 * roster was declared and every row named one — and it stopped being true by construction the
 * moment pools could be opened and closed while the world runs: a launch that entered the party and
 * failed to write the mandate, or a manager that ceased with pools still on its book, would leave a
 * pool with an account, holdings, and NOBODY DECIDING FOR IT. Nothing would throw; the pool would
 * simply never appear in `livingPools` again and its holders' money would sit there.
 *
 * A FORBID that holds is as valuable as a mechanism that works, and it breaks silently (Part II) —
 * which is exactly this. The size is the pool's own book, because that is what has no decider.
 */
function everyPoolIsRun(): Family {
  return {
    name: 'names',
    contributor: 'funds',
    spec: 'Fund Shares F3',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const p of view.parties.ofKind(FUND)) {
        if (!p.status.alive) continue;
        const run = view.agreements
          .ofKind(MANDATE)
          .find((a) => a.state === 'performing' && a.debtor === p.id);
        const gone =
          run !== undefined && (!view.parties.has(run.creditor) || !view.parties.get(run.creditor).status.alive);
        if (run !== undefined && !gone) continue;
        out.push({
          family: 'names',
          spec: 'Fund Shares F3',
          owner: p.id,
          // What has no decider is the pool's whole book, which is what the holders own (A3).
          size: view.register.holdingsOf(p.id).length,
          unit: 'positions with nobody deciding for them',
          period: view.period,
          message:
            run === undefined
              ? `${p.id} is a live pool under no mandate: there is no fund without a manager`
              : `${p.id} is run by ${run.creditor}, which has ceased`,
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
        const done = plus(
          asAmount<'piece'>(zeroIfNone(paid.get(k)), 'shares paid'),
          asAmount<'piece'>(zeroIfNone(queued.get(k)), 'shares still on the book'),
          'what was paid and what is queued',
        );
        // Law 7: both sides are sums of the same per-member numbers at the same magnitudes.
        if (withinDust(want, done, dustOf(asked.size + 2, Math.abs(want) + Math.abs(done)))) continue;
        out.push({
          family: 'flows',
          spec: 'Fund Shares C2.b',
          owner: k.split('|')[0] ?? k,
          size: minus(asAmount<'piece'>(want, 'shares it asked to redeem'), done, 'asked against paid and queued'),
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
function seedInKind(ctx: SeedContext, e: FundDecl): void {
  const bank = ctx.parties.get(e.bank as PartyId);
  const region = ctx.registry.region(bank.region);
  for (const [id, kind, name] of [
    [e.fund, FUND, nameOf({ ...e, house: e.bank })],
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
      status: { alive: true, standing: 'good' },
    });
  }
  const share = shareLineOf(e.fund);
  const market = listedMarketOf(e.fund);
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
    name: `${nameOf({ ...e, house: e.bank })} shares`,
    instrument: share,
    ccy: ctx.registry.currencyOf(region.id),
    rationing: 'proRata',
  });
  // G1.a: where it is created and redeemed IN KIND. It is not a market and it does not clear —
  // everybody who brings a basket gets the shares that basket is worth — so the module that owns
  // it runs it itself (Clearing B2).
  ctx.openVenue({
    id: inKindVenue(e.fund),
    name: `${nameOf({ ...e, house: e.bank })} creations and redemptions`,
    clearedBy: 'funds',
    unit: SHARES,
    ccy: ctx.registry.currencyOf(region.id),
    key: { kind: 'inKind', fund: e.fund, share },
  });
  // Seed A3: only what somebody who EXISTS actually took. A world without the banks that launch it
  // has a smaller fund, and its basket has to back the shares that were taken and no more — a
  // basket backing shares nobody holds would be a fund whose NAV was a multiple of what it owed.
  const holders = Object.entries(inKindOf(e).by).filter(
    ([holder, share]) => share > 0 && ctx.parties.has(holder as PartyId),
  );
  if (holders.length === 0) return;
  // Law 19, E3.a: HOW BIG THE LAUNCH IS, read off the lines it tracks rather than stated
  // (`FundDecl.launchShare`). A share of the fund is `perShare` of each line, so what each line can
  // back is its own float over that, and the SMALLEST of them is as far as all of them reach.
  const backs: Qty[] = [];
  for (const [line, perShare] of Object.entries(inKindOf(e).basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    backs.push(
      over(
        scale(
          ctx.instruments.get(id).issued,
          asRatio(inKindOf(e).share, 'the share of this line it holds'),
          'the share of this line it holds',
        ),
        asRatio(perShare, 'what one share draws of it'),
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
    downTick(scale(full, asRatio(share, "this holder's slice"), "this holder's slice of the launch")),
  );
  const launched = sum(taken).value;
  if (launched <= 0) return;
  // E3, Fund Shares A3: THE BASKET IS WHAT THE SHARES ARE A CLAIM ON, so the shares are decided
  // first and exactly that much is taken. A holder gives up whole pieces PER MEMBER (XI-15), so a
  // little LESS of a line may arrive than the arithmetic asked for — and that is the safe way for
  // it to be out. Taking first and issuing against what came left the fund holding more than its
  // shares claimed, and a fund holding more than its shares claim HAS EQUITY: the accounts family
  // said so at period zero, `etf.us has equity of 600`, which is somebody's money mislaid.
  const contributions: Cash[] = [];
  for (const [line, perShare] of Object.entries(inKindOf(e).basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const opening = ctx.prices.latest(id, ctx.period);
    if (!opening.some) continue;
    const units = outOfTheFloat(
      ctx,
      id,
      scale(launched, asRatio(perShare, 'what one share draws of it'), 'units of this line the launch takes'),
      e.fund as PartyId,
    );
    if (units <= 0) return;
    ctx.register.credit(e.fund as PartyId, id, units, opening.value.price, ctx.period);
    contributions.push(valueAt(opening.value.price, units, 'what this line put in'));
  }
  // Law 19, Fund Shares A3: WHAT ONE SHARE IS A CLAIM ON IS READ OFF THE BASKET THAT ARRIVED, never
  // off the one that was asked for. A holder gives up whole pieces per member, so a little less of
  // a line comes in — and a share issued at what a whole basket would have been worth is a claim on
  // more than the fund has: its holders carry it at that basis, the fund owes them that, and the
  // difference is equity a fund may not have (measured at -15,599,400 before this read).
  const perShare = pricedAt(sum(contributions).value, launched, 'what one share is a claim on');
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
  wanted: Qty,
  fund: PartyId,
): Qty {
  const outstanding = ctx.instruments.get(id).issued;
  if (outstanding <= 0 || wanted <= 0) return NO_QTY;
  const taken: Qty[] = [];
  for (const holder of ctx.register.holdersOf(id)) {
    if (holder === fund) continue;
    const weight = weightOf(ctx.parties.get(holder));
    const mine = ctx.register.quantity(holder, id);
    const perMember = downTick(
      over(
        scale(wanted, ratioOf(mine, outstanding, 'its share of the line'), 'its share of what the launch takes'),
        asRatio(1, 'and a holder holds per member already'),
        'per member',
      ),
    );
    if (perMember <= 0) continue;
    ctx.register.debit(holder, id, perMember);
    // XI-15, item 16: a count PER MEMBER times the cell's weight, through the one door that says
    // so — it refuses a fractional weight, and a total cannot be mistaken for a per-member count.
    taken.push(scaleQty(perMember, weight, 'units this holder gave up'));
  }
  return sum(taken).value;
}

export function funds(
  /**
   * Item 10e: ONE LIST. It was `(decls, etfs)` — two lists of two declaration types, which is the
   * module saying there are two kinds of thing here before a single line of behaviour runs. There
   * is one: a pool with a mandate, and what it DOES follows from that mandate's terms.
   *
   * Item 10e.4, Seed A3: AND IT IS AN OPENING CONDITION. These are the pools this world OPENS with,
   * the way the bank draw is the balance sheets it opens with. Nothing reads this list after the
   * seed has written the mandates: the pools that exist are the mandates that are performing, and
   * from period one that set is what managers opened and closed.
   */
  decls: readonly FundDecl[],
  /**
   * F3, item 10e.4: THE HOUSES. A manager is a business with its own preferences — what it will
   * give up to win a mandate, and how long it gives a product it opened — and they are declared
   * here rather than derived from a pool, because a house runs several and outlives any of them.
   */
  managers: readonly ManagerDecl[],
): SystemModule {
  /**
   * G1.a, E1: the ones whose shares TRADE and whose investors come and go in kind — read off the
   * LIQUIDITY TERM, which is the fact that decides it, and not off the launch data beside it.
   */
  const listed = decls.filter((d) => d.liquidity.how === 'listed');
  const state = emptyBook();
  // The observer sees the book as the data it is; the slot holds this very object (Law 4).
  const book = (ctx: MechanismContext): Book => ctx.state<Book>('funds', () => state);
  // E3: an exchange-traded fund's basket is lines somebody else registered and its launch names
  // parties somebody else created, so those modules have to have run first. It is a dependency of
  // THIS WORLD's funds and not of funds, which is why it is read off the data rather than written
  // into the module (Law 15): a world with no such fund in it needs none of them.
  const needs = [...new Set(listed.flatMap((e) => inKindOf(e).needs))];
  return {
    id: 'funds',
    nouns: [
      {
        name: 'funds',
        kind: 'working',
        holds:
          'the subscriptions and redemptions queued this cycle, and the NAV struck this period',
        why:
          'both are state one phase hands to a later phase WITHIN a period: the queue is what asked before the strike and the strike is what the orders and the settlement both read, so they are one number (Law 4). The PREVIOUS NAV was here too and was NOT this — it was a second copy of the `perShare` the same call had already published on `fund.struck`, which is what a fund\u2019s return is measured against and is public because D2\u2019s competition cannot happen against a number nobody can see. It is read off that event now (item 9.9).',
      },
    ],
    spec: 'Fund Shares, XI-2',
    // Its investors are households (D2), a fund that fails resolves through the same estate as
    // anything else (XI-3), and it banks somewhere — so all three are there before it opens.
    // Item 10e.4: AND LABOUR, because a manager employs people. The pools are the demand side of
    // this world's markets; the HOUSES that run them are on the buy side of its labour market, in a
    // trade that had a venue in every region and nobody bidding in it until now.
    requires: ['households', 'estate', 'labour', 'seed.foundation', ...needs],
    instrumentKinds: [fundShareKind],
    partyKinds: [fundKind, fundManagerKind],
    curveFamilies: [],
    // A2, G1.b: shares are what a fund is measured in. The unit itself is the world's — a share
    // count is what a claim on a book and a claim on a firm are both counted in (Equity A2), and
    // two modules cannot each introduce it — so it is registry data and this module only uses it.
    units: [],
    params: paramsOf(managers),
    phases: [
      {
        /**
         * F3, Labour A2, Seed A3, item 10e.4: THE MANAGER'S OWN PERIOD - what it closed, and what
         * it opened. BEFORE the strike, because a pool given notice today is redeemed at today's
         * NAV and a pool opened today publishes one, and both are things the strike does.
         */
        name: 'funds.manager',
        spec: 'Fund Shares A4 Fund Shares B3 Fund Shares F3 Fund Shares G1 Seed A3',
        cycle: 0,
        anchor: { before: 'funds.strike' },
        run: (ctx: MechanismContext) => {
          runManagers(ctx);
        },
      },
      {
        name: 'funds.strike',
        spec: 'Fund Shares B1 Fund Shares B3 Fund Shares C1 Fund Shares C2 Fund Shares C2.b',
        cycle: 0,
        // After the households have decided, because what they decided is posted into the venue
        // this phase reads; before the markets, because what it cannot pay is what it must sell.
        anchor: { after: 'households.decide' },
        run: (ctx: MechanismContext) => {
          const b = book(ctx);
          for (const m of livingPools(ctx)) strike(ctx, b, m);
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
          for (const m of livingPools(ctx)) placeSpareCash(ctx, m);
        },
      },
      {
        name: 'funds.settle',
        spec: 'Fund Shares C2.a Fund Shares C2.b Fund Shares C4 Fund Shares C4.a',
        cycle: 2,
        anchor: { after: 'markets' },
        run: (ctx: MechanismContext) => {
          const b = book(ctx);
          for (const m of livingPools(ctx)) payQueue(ctx, b, m);
        },
      },
      {
        name: 'funds.inKind',
        spec: 'Fund Shares B3 Fund Shares E3 Fund Shares G1.a',
        cycle: 0,
        // With the other fund's own strike, and before the session: what a dealing line brought in this
        // morning is shares it can offer this afternoon, and the fee is inside the period so the
        // revaluation that closes it carries the claim at what the book then comes to (A3).
        anchor: { after: 'funds.strike' },
        run: (ctx: MechanismContext) => {
          for (const d of listed) launchInKind(ctx, d);
          // E3.a: a vehicle whose index has not answered yet has not been launched, and there is
          // nothing of it to run. It is not absent — it is declared and waiting for its own rule.
          for (const m of livingPools(ctx)) {
            const d = declOf(listed, String(m.pool));
            if (d !== undefined) runInKind(ctx, m, d);
          }
        },
      },
      {
        name: 'funds.listed.read',
        spec: 'Fund Shares E1 Fund Shares E2 Fund Shares E4',
        cycle: 'anchor',
        // After the marks are in the books, because the NAV is a read of those marks and the
        // premium is a read of that NAV against what the session printed (Clearing F1.a).
        anchor: { after: 'revaluation' },
        run: (ctx: MechanismContext) => {
          for (const m of livingPools(ctx)) {
            const d = declOf(listed, String(m.pool));
            if (d !== undefined) readListed(ctx, m, d);
          }
        },
      },
    ],
    // Labour A1, A3, D1, item 10e.4: a manager wants the hours its pools take, in the `analysis`
    // trade, at what an hour is worth to it - and it is matched by the same rule as a bank or a
    // baker. Until this existed the trade had a venue in every region and NOBODY on the bid side.
    venueParticipants: [{ partyKind: FUND_MANAGER, orders: staffOrders }],
    participants: [
      {
        partyKind: FUND,
        /**
         * Clearing B2, A4, Law 15 (item 10e): ONE QUESTION — why is this fund in this book — and
         * the MANDATE answers it. Three reasons, and a fund has exactly one of them.
         *
         * It used to call all three and let each decide it was not theirs, which is three functions
         * carrying three copies of "which kind of fund am I for" — the separation this item exists
         * to delete. Now the mandate is read ONCE and the reason follows from its terms:
         *
         *  - it TRACKS, so it holds what the index says at whatever the book gives (Indices C2);
         *  - its blueprint is THINGS, so it buys what it expects to be paid more for than the wait
         *    costs it (Commodities Spot C3);
         *  - otherwise it CHOOSES, and what it will pay is what its investors require of it (D2).
         *
         * A pool with no mandate is not a fund and has no reason to be anywhere (App A).
         */
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const mandate = mandateFor(view);
          if (!mandate.some || !view.self.status.alive) return [];
          if (mandate.value.tracks.some) return passiveOrders(view, mandate.value, m);
          // Law 4: `holdsThings` is the one writer of "is this a mandate over things" — and it is
          // the one that gets the empty case right, because a blueprint that states NO class band
          // holds anything and `[].every(...)` would have called that a commodity fund.
          if (holdsThings(mandate.value.blueprint)) return thingOrders(view, mandate.value, m);
          return ordersOf(view, mandate.value, m);
        },
      },
    ],
    families: [equityIsZero(), noRequestVanishes(state), everyPoolIsRun()],
    // A1.c: both kinds are wholesale money and both leave for the same reasons (the manager runs
    // the money and banks like the money it runs), so one reason answers for both.
    agreementKinds: [
      {
        id: MANDATE,
        what: 'what a pool may hold and whether it may be levered, and which manager runs it',
      },
    ],
    // Fund Shares A3, `B-14` (item 9.7): WHAT A POOL MAY TAKE A POSITION IN IS ITS MANDATE'S. The
    // derivative layer speaks for a party in every contract book, so it asks before it speaks — a
    // pool with no mandate is not a fund and trades nothing, which is the same refusal `ordersOf`
    // makes in an ordinary market.
    tradingLimits: [
      {
        partyKind: FUND,
        mayTrade: (view: ParticipantView, kind): boolean => {
          const m = mandateFor(view);
          return m.some && m.value.mayWrite.includes(String(kind));
        },
      },
    ],
    bankChoices: [
      { partyKind: FUND, chooses: fundChoosesBank },
      { partyKind: FUND_MANAGER, chooses: fundChoosesBank },
    ],
    seed(ctx: SeedContext): void {
      for (const d of decls) {
        for (const [id, kind, name] of [
          [d.fund, FUND, nameOf({ ...d, house: d.bank })],
          [d.manager, FUND_MANAGER, d.managerName],
        ] as const) {
          ctx.parties.add({
            id: id as PartyId,
            kind,
            region: ctx.registry.region(ctx.parties.get(d.bank as PartyId).region).id,
            name,
            bank: d.bank as PartyId,
            representation: 'named',
            status: { alive: true, standing: 'good' },
          });
        }
        const ccy = ctx.registry.currencyOf(ctx.parties.get(d.fund as PartyId).region);
        // A4, F3, XI-8: THE MANDATE THIS POOL WAS SET UP UNDER. A fund that exists at period zero
        // was set up by somebody, on terms, and a seed states an opening STOCK (Seed A3) — so the
        // commitment is as much part of the opening as the share line is. The pool owes the manager
        // its fee; it owes nothing yet, because the fee falls due at the end of a period.
        openMandate(ctx, d.fund as PartyId, d.manager as PartyId, ccy, productOf(d, ccy));
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
          name: `${nameOf({ ...d, house: d.bank })} subscriptions and redemptions`,
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
      for (const e of listed) if (inKindOf(e).seeded) seedInKind(ctx, e);
    },
  };
}
