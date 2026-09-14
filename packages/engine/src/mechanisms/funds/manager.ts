/**
 * THE MANAGER IS A BUSINESS: it runs several pools, it employs people to run them, and the set of
 * pools that exist is what managers decided rather than what the world declared.
 *
 * @spec Fund Shares A4 Fund Shares B3 Fund Shares F3 Labour A1 Labour A2 Labour A3 Labour D1 Seed A3 XI-2 Law 2 Law 3 Law 6 Law 12 Law 19
 *
 * Item 10e.4, and the owner's words it implements: *"There are fund blueprints that each manager can
 * create or winddown depending on competition and cost Vs fees"*, and *"an HF doesn't go bankrupt
 * because a fund does bad by itself"*.
 *
 * WHAT WAS HERE BEFORE. A manager was a party with one pool, a fee that was a PLACEHOLDER — the
 * number stood where a competition should have been, and its own reason said so — and no cost of
 * any kind. So there was no answer to why a fee is what it is, no reason for a fund to close, and
 * the roster of funds at period fifty was the roster `drawFunds` returned before period zero. That
 * is a declared equilibrium of an industry's structure (Law 2), and it is the largest one left in
 * this sector.
 *
 * WHAT A MANAGER NOW DOES, and all three are decisions with real numbers on both sides:
 *
 *  - IT EMPLOYS. Running a pool takes people, and this world has a labour market with a trade in it
 *    (`analysis`, Labour A3) that until now NOBODY EMPLOYED. It posts openings in the same venue
 *    every other employer does, at what an hour is worth to it — its fee income over the hours its
 *    pools take — and it is matched, or not, by the same rule as a bank or a baker.
 *  - IT WINDS A POOL UP when the fee that pool pays it stops covering what running it costs. That is
 *    a FORCED SALE OF A WHOLE BOOK into whatever the market gives (C2.a, XI-2), which is a channel
 *    this sector did not have.
 *  - IT LAUNCHES one when a product somebody else is running would cover its cost at a fee that
 *    undercuts them. Fees fall where several managers run the same blueprint, because that is the
 *    only lever an entrant has, and entry stops where the fee stops covering the cost. Nothing is
 *    bounded: the refusal to launch IS the mechanism (Law 6).
 *
 * AND IT DOES NOT FAIL BECAUSE A POOL DOES. Its exposure to a pool going wrong is exactly the fee
 * income it stops earning — the holders own the assets and bear the losses (A3) — and at 10e.5 the
 * seed it chose to put in. Both are things it actually did, and neither is a balance sheet it never
 * agreed to carry.
 */
import { findVenue, type VenueDecl } from '../../clearing/venue.js';
import type { Order } from '../../clearing/solver.js';
import { yearFraction } from '../../calendar/daycount.js';
import {
  paramId,
  partyId,
  type ParamId,
  type PartyId,
} from '../../core/ids.js';
import {
  asAmount,
  asCash,
  asRatio,
  type Cash,
  minus,
  type PerPiece,
  pricedAt,
  type Ratio,
  scale,
  valueAt,
} from '../../core/measure.js';
import { atMost } from '../../core/num.js';
import { none, type Option, some } from '../../core/option.js';
import { asQty, type Qty } from '../../core/tick.js';
import type { Blueprint } from '../../registry/blueprint.js';
import { wageFacing } from '../../registry/wages.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { isMandate, type Mandate, mandateOf, type Product } from './mandate.js';

/**
 * Labour A3, item 10e.4: THE TRADE AN ASSET MANAGER EMPLOYS, and it is one this world already has.
 *
 * `analysis` is *"research and credit analysis"* in the finance sector, declared at 13d for a bank's
 * credit desk and EMPLOYED BY NOBODY since — a venue in every region with nothing on the bid side,
 * which is a mechanism that never runs. It is the same work: somebody forms a view on a name and
 * somebody is paid for having formed it. So a bank's analysts and a manager's compete for the same
 * people in the same book, which is what makes a hiring bank dearer for a manager and the other way
 * round — a real link between the two halves of this world's finance industry, out of one trade.
 */
export const INVESTING = 'analysis';

export const MANAGER_PARAMS = {
  hoursPerPool: paramId('funds.hoursPerPoolPeriod'),
} as const satisfies Record<string, ParamId>;

export const managerParam = (manager: string, what: string): ParamId =>
  paramId(`manager.${what}.${manager}`);

/** A fee is per annum, so what a period of it is worth needs the period in years (Law 8). */
const FEE_DAY_COUNT = 'ACT/365F' as const;

/**
 * F3, Law 19: THE POOLS THIS MANAGER RUNS, asked of its own commitments. A manager is the CREDITOR
 * of every mandate it holds — the pool owes it the fee, it owes the pool its judgement — so its
 * book of business is a read of the same agreements the pools are run under, and there is no second
 * list of who manages what anywhere.
 */
export function poolsRun(view: ParticipantView): readonly Mandate[] {
  const out: Mandate[] = [];
  for (const a of view.commitments()) {
    if (a.state === 'performing' && a.creditor === view.self.id && isMandate(a.terms)) {
      out.push(mandateOf(a));
    }
  }
  return out;
}

/**
 * Labour A2: THE HOURS ITS BUSINESS TAKES — the pools it runs at what running one takes. The hours
 * one pool takes is the TECHNOLOGY and the pools are a read of its own agreements, so this is that
 * technology scaled by a count and never a second unit.
 *
 * A POOL COSTS ABOUT THE SAME WHATEVER IT IS WORTH, which is the whole economics of this industry:
 * the fee is on the assets and the cost is on the product, so a large pool carries a small one and
 * a manager with one small pool cannot cover its own people. Nothing states that; it falls out of a
 * cost per PRODUCT meeting a fee per POUND.
 */
export function hoursNeeded(view: ParticipantView): Qty {
  const perPool = asAmount<'piece'>(
    view.params.count(MANAGER_PARAMS.hoursPerPool),
    'the hours one pool takes to run',
  );
  return scale(perPool, asRatio(poolsRun(view).length, 'the pools it runs'), 'the hours it needs');
}

/** The venue this manager's people are hired in, or none because this world has no such trade. */
export const staffVenue = (view: ParticipantView): VenueDecl | undefined =>
  findVenue(view.venues, { region: String(view.self.region), occupation: INVESTING });

/**
 * Labour A1, A2, D1: THE OPENING. It wants the hours its pools take and it bids what an hour is
 * worth to it — what its business took in over the last period, over those hours. A manager whose
 * pools earn nothing bids nothing and hires nobody, and one that loses a pool wants fewer hours and
 * sheds the difference at its own cost (Labour C3), which is how an asset manager shrinks without
 * anybody writing a rule for it.
 */
export function staffOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.key['occupation'] !== INVESTING) return [];
  if (venue.key['region'] !== String(view.self.region)) return [];
  const hours = hoursNeeded(view);
  if (hours <= 0) return [];
  const took = view.earned(1);
  if (took <= 0) return [];
  const worth = pricedAt(took, hours, 'what an hour of this is worth to it');
  if (worth <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth, qty: asQty(hours) }];
}

/**
 * F3, Labour D1.c: WHAT ONE POOL COSTS IT IN A PERIOD — the hours a pool takes at what an hour
 * costs it where it is. A manager that has never met a wage and faces no published rate cannot
 * price an hour and therefore cannot price a pool: it decides nothing this period, which is the
 * honest answer for a business that does not yet know what it costs (App A).
 */
export function costOfAPool(view: ParticipantView): Option<Cash> {
  const venue = staffVenue(view);
  if (venue === undefined) return none<Cash>();
  const hour = wageFacing(view, view.period, venue.id);
  if (!hour.some) return none<Cash>();
  const perPool = asAmount<'piece'>(
    view.params.count(MANAGER_PARAMS.hoursPerPool),
    'the hours one pool takes to run',
  );
  return some(valueAt(hour.value, perPool, 'what one pool costs it a period'));
}

/**
 * B1, B3, Observer A3: WHAT A POOL IS WORTH, as anybody outside it can see — off what the pool
 * itself published (`fund.struck`), which carries what a share is worth and how many there are.
 *
 * It is read from the PUBLIC record and not from the register, because a manager looking at a
 * rival's product may see exactly what a saver sees and nothing more. A pool that has never struck
 * has published nothing and there is nothing to read (App A).
 */
function netAssetsOf(ctx: MechanismContext, pool: PartyId): Option<Cash> {
  const said = ctx.journal.lastOf('fund.struck', String(pool));
  if (said === undefined) return none<Cash>();
  const perShare = said.data['perShare'];
  const shares = said.data['shares'];
  if (typeof perShare !== 'number' || typeof shares !== 'number') return none<Cash>();
  if (perShare <= 0 || shares <= 0) return none<Cash>();
  // Item 16: two published numbers re-entering the type system through their own doors.
  return some(
    valueAt(
      perShare as PerPiece,
      asAmount<'piece'>(shares, 'the shares it has outstanding'),
      'what its book comes to',
    ),
  );
}

/**
 * B3, Law 4, Law 8: WHAT A BOOK OF THIS SIZE PAYS A MANAGER IN A PERIOD — one formula, and the
 * only one there is.
 *
 * A fee is quoted per annum, so it is not a number until the period is (Law 8): what this period is
 * of a year, at the day count a rate on money is quoted at. Two callers ask it, from two sources
 * that must differ — the POOL asks with what its own register says its book is worth, because that
 * is what it actually pays, and a MANAGER asks with what a rival PUBLISHED, because nobody may see
 * another party's register (Observer A4). Two sources, one arithmetic: the alternative is the same
 * fee computed two ways, which is what Law 4 hunts.
 */
export function feeOn(ctx: { readonly calendar: MechanismContext['calendar']; readonly period: MechanismContext['period'] }, assets: Cash, perAnnum: Ratio): Cash {
  return scale(
    assets,
    scale(
      perAnnum,
      asRatio(
        yearFraction(
          FEE_DAY_COUNT,
          ctx.calendar.startOf(ctx.period),
          ctx.calendar.endOf(ctx.period),
        ),
        'this period of a year',
      ),
      'this period of a year',
    ),
    'the fee a book of this size pays',
  );
}

/**
 * A4, Law 4: ARE THESE THE SAME PRODUCT? Two blueprints are the same product when they say the same
 * thing about every dimension of the investment universe, which is what a blueprint IS — so this is
 * a comparison of the language and never of a name, a kind or a label (item 10e).
 *
 * It is what makes competition mean anything: a manager undercuts the people running the product it
 * is copying, and "the product" has to be a fact about the mandate rather than about what anybody
 * decided to call it.
 */
export function sameProduct(a: Blueprint, b: Blueprint): boolean {
  const band = (x?: { from?: number; to?: number }, y?: { from?: number; to?: number }): boolean =>
    x?.from === y?.from && x?.to === y?.to;
  return (
    [...a.classes].sort().join('|') === [...b.classes].sort().join('|') &&
    [...a.currencies].sort().join('|') === [...b.currencies].sort().join('|') &&
    band(a.duration, b.duration) &&
    band(a.seniority, b.seniority) &&
    band(a.size, b.size) &&
    a.secured === b.secured &&
    a.listed === b.listed &&
    a.worstGrade === b.worstGrade &&
    a.unratedAllowed === b.unratedAllowed
  );
}

/** What a manager decided to do about one pool, and why — the record of the decision (Law 14). */
export interface Notice {
  readonly pool: PartyId;
  readonly earns: Cash;
  readonly costs: Cash;
}

/**
 * F3, item 10e.4: THE WIND-DOWN DECISION. A pool whose fee stops covering what running it costs is
 * a product this manager is losing money on, and it closes it.
 *
 * Two reads and a comparison, and no bound anywhere: what the pool pays it this period at its own
 * mandate's fee, against what a pool costs it in people. The pool is not ended here — notice is
 * given, and the wind-down is the ordinary redemption path running until the last share is gone
 * (`strike`, `payQueue`), which is what makes it a forced sale rather than a line disappearing.
 *
 * A POOL IS GIVEN TIME. A product launched this week has been offered to nobody: the first strike
 * is what publishes it, the saver reads that and decides the week after, and its subscription is
 * struck the week after that. Judging it before then would close every fund in this world the
 * period after it opened. How long a manager waits is its own PATIENCE and it is drawn — two
 * managers with the same patience are one manager with two names (Ratings A4.b, and it is why the
 * assessors are drawn unalike).
 */
export function noticeToGive(
  ctx: MechanismContext,
  view: ParticipantView,
  cost: Cash,
): readonly Notice[] {
  const out: Notice[] = [];
  const patience = ctx.params.periods(managerParam(String(view.self.id), 'patience'));
  for (const m of poolsRun(view)) {
    if (m.windingUp) continue;
    if (ctx.period < m.since + patience) continue;
    const assets = netAssetsOf(ctx, m.pool);
    const earns = assets.some
      ? feeOn(ctx, assets.value, m.feePerAnnum)
      : asCash(0, 'a pool with no book pays its manager nothing');
    if (earns >= cost) continue;
    out.push({ pool: m.pool, earns, costs: cost });
  }
  return out;
}

/**
 * F3, Law 3, item 10e.4: THE LAUNCH DECISION, and what makes it a decision is that both sides of it
 * are numbers somebody else produced.
 *
 * A manager copies a PRODUCT IT CAN SEE WORKING: a blueprint somebody else is running, at a book
 * somebody else has actually gathered, for a fee it can see published. It expects to be the
 * SMALLEST of them, because it is the newest — a selection over real books and never a blend of
 * them (no decision at an average) — and it charges less than the cheapest of them, because price
 * is the only lever an entrant has. If that fee on that book covers what a pool costs it, it opens
 * one; if it does not, it does not, and that refusal is what stops entry (Law 6: no bound, and
 * none needed).
 *
 * IT CANNOT INVENT A PRODUCT NOBODY RUNS. Nothing in this world would tell it what such a thing
 * would gather, and a manager that guessed would be a forecast with no falsification (Law 17). The
 * products this world STARTS with are the seed's (Seed A3) and everything after that is a copy of
 * one of them, wound down or undercut — which is an industry with an opening condition rather than
 * an industry with a permanent structure.
 */
export interface Launch {
  readonly product: Product;
  readonly rival: PartyId;
  readonly expects: Cash;
  readonly earns: Cash;
}

export function launchToMake(
  ctx: MechanismContext,
  view: ParticipantView,
  pools: readonly Mandate[],
  cost: Cash,
): Option<Launch> {
  const mine = poolsRun(view);
  const undercut = ctx.params.ratio(managerParam(String(view.self.id), 'undercut'));
  let best: Launch | undefined;
  for (const rival of pools) {
    if (rival.manager === view.self.id || rival.windingUp) continue;
    // A manager does not run one product twice: a second pool with the same mandate at the same
    // house is one product with two names, and it would compete with itself for the same saver.
    if (mine.some((m) => sameProduct(m.blueprint, rival.blueprint))) continue;
    // C2, Indices C2: a TRACKER is not copied here. Its investors come and go in kind against a
    // basket, which needs somebody who holds the basket to put it in (E3), and that is a launch
    // with a different mechanism (`launchInKind`). This is the cash-subscription door.
    if (rival.tracks.some || rival.liquidity.how === 'listed') continue;
    const running = pools.filter((p) => !p.windingUp && sameProduct(p.blueprint, rival.blueprint));
    // What the SMALLEST of them has gathered, and what the CHEAPEST of them charges. Both are the
    // conservative read: it will be the newest and it has to be the cheapest to be chosen at all.
    let smallest: Cash | undefined;
    let cheapest: Ratio | undefined;
    for (const p of running) {
      const assets = netAssetsOf(ctx, p.pool);
      if (!assets.some) continue;
      smallest =
        smallest === undefined
          ? assets.value
          : atMost(assets.value, smallest, 'the smallest book anybody running this has');
      cheapest =
        cheapest === undefined
          ? p.feePerAnnum
          : atMost(p.feePerAnnum, cheapest, 'the least anybody charges for this');
    }
    if (smallest === undefined || cheapest === undefined) continue;
    const fee = minus(
      cheapest,
      scale(cheapest, undercut, 'what it gives up to be chosen'),
      'what it will charge',
    );
    if (fee <= 0) continue;
    const earns = feeOn(ctx, smallest, fee);
    if (earns < cost) continue;
    // One decision, and the best of them: a manager that opened five products in a week is not a
    // business taking a decision, it is a list being walked.
    if (best !== undefined && earns <= best.earns) continue;
    best = {
      product: {
        blueprint: rival.blueprint,
        liquidity: rival.liquidity,
        tracks: none<string>(),
        feePerAnnum: fee,
        // A manager copying a product copies the product: what the pool keeps in cash and what its
        // investors require of it are terms of the thing it is copying, not of the copier.
        buffer: rival.buffer,
        requiredYieldPerAnnum: rival.requiredYieldPerAnnum,
      },
      rival: rival.pool,
      expects: smallest,
      earns,
    };
  }
  return best === undefined ? none<Launch>() : some(best);
}

/** A1, Law 9: the pool's internal id, which is never its display name. Its name is `nameOf`. */
export const launchedPoolId = (manager: PartyId, at: number): PartyId =>
  partyId(`pool.${String(manager)}.${at}`);
