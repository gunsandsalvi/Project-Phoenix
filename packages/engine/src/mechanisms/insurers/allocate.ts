/**
 * An institution does not invest. It hands its assets to somebody whose business that is.
 *
 * @spec Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers A4.c Fund Shares A1 Fund Shares A4 Fund Shares C2 Fund Shares D2 Fund Shares D5 Private Equity A2 Private Equity A2.a Private Equity A2.b XI-2 Law 2 Law 3 Law 4 Law 6 Law 19
 *
 * ITEM 14.0, AND IT IS THE OWNER'S LARGEST SIMPLIFICATION: *"insurance companies and pension funds
 * don't invest themselves. Their assets are always third party managed."* So there is no portfolio
 * mechanism here, no allocation rule, and no decision about which bond to buy. There is a LIABILITY
 * SCHEDULE and a choice of manager, and B2.b's duration matching is the whole of the choice.
 *
 * *"Nothing in this world invests except a fund, and every fund has a manager"* — so an insurer
 * reaches the markets through the same door a household does, at the same venue, under the same
 * refusals. What is different is only WHICH door it picks and WHY, and both of those are §27's.
 *
 * WHY IT IS INSERTED HERE, AHEAD OF THE REST OF ITEM 13 (Law 10). Items 13.2 and 13.3 built a
 * strategy house and a prime broker, and item 10e built a credit fund — and none of them is offered
 * to the public, so the only investors this world has (household cells) cannot reach any of them.
 * A private-equity fund (13.5) is the same shape and would have been the third. **The money those
 * sectors run on is institutional, and nothing in this world allocated any**: an insurer wrote
 * cover, took premiums, and sat on the cash for ever. This is where that money comes from.
 *
 * WHAT IT DOES NOT DO. It does not pick a bond, a tenor or a name; it does not rebalance; it holds
 * no view about a price. Its whole decision is: what can I put to work, and whose mandate it can
 * accept. Everything after that is the manager's (Fund Shares A4).
 *
 * ITEM 10f.5, AND IT IS THE OWNER'S CORRECTION TO 14.0: *"insurance and pension don't only go for
 * duration. They invest in tons of different strategies."* What stood here matched its longest
 * promise to the pool whose stated duration was NEAREST it, took that one, and REFUSED every pool
 * that stated no duration at all — which is every strategy pool, every equity pool and every
 * private-equity pool in this world. One institution, one manager, one asset class, for ever.
 *
 * Both halves of the correction are right and the fix keeps both:
 *
 *  - **What it requires is what its own promises are discounted at** (B2). Its liabilities have a
 *    present value at the curve, and the rate they are discounted at is the rate its assets must
 *    earn to cover them. It is a READ of a published curve and never a preference anybody declared.
 *  - **What it will not take is duration it did not promise.** A pool whose stated duration runs
 *    PAST its longest promise is refused, because holding it is a rate risk nobody asked it to take.
 *    A pool that states NO duration is not refused: equity has no duration to mismatch, which is how
 *    strategies, credit and equity all become eligible with one test rather than three.
 *  - **A pool that OFFERS LESS than what its promises require is refused**, and one that offers
 *    nothing at all is not — because nothing was said, and an absence of evidence is returned as one
 *    (App A). That is the whole of why an institution reaches past bonds: a pool with a yield has
 *    told it whether it covers the promises, and a pool without one has not claimed anything to
 *    fail against.
 *  - **How it spreads is by FEEDING THE SMALLEST**: this period's money goes to whichever of the
 *    doors it can accept it holds least of. Diversification is then an OUTCOME and there is no
 *    allocation rule anywhere — no percentages, no target weights, no optimiser (Law 2, Law 6).
 */
import type { CurrencyCode, PartyId, VenueId } from '../../core/ids.js';
import { moneyInstrumentId, partyId } from '../../core/ids.js';
import {
  amountOf,
  asCash,
  asPerPiece,
  type Cash,
  heldAsMoney,
  minus,
  type PerPiece,
  plus,
  valueAt,
} from '../../core/measure.js';
import { atMost, largest, sum } from '../../core/num.js';
import { asQty, downTick } from '../../core/tick.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { MechanismContext } from '../../world/context.js';
import { endOfMortalityTable, isPensionTerms, isPolicyTerms } from '../../registry/insurance.js';
import { callsOn, strikeOf } from '../../registry/funding.js';
import { about } from '../../world/context.js';
import { weightOf } from '../../parties/party.js';
import { pensionPerMember, type PensionReads } from './pensions.js';

/** What a pool published about itself, as an allocator reads it off the tape (Observer A3). */
interface Door {
  readonly venue: VenueId;
  readonly fund: string;
  readonly perShare: PerPiece;
  /** B2.b: what its mandate says about duration, and none where it says nothing. */
  readonly years: number | undefined;
  /** D2, D2.a (10f.5): what it OFFERS a saver, and none where it has never claimed anything. */
  readonly offered: number | undefined;
  /** Item 10e.6: the policy an entrant must clear, and none where it is offered to the public. */
  readonly asks: string | undefined;
}

/**
 * Observer A3, Law 19: every pool this institution could put money into, read off the venues and
 * what each of them last struck. It knows nothing private about any of them — the same two public
 * facts a household reads, and the door's own key.
 */
function doors(ctx: MechanismContext): readonly Door[] {
  const out: Door[] = [];
  for (const v of ctx.venues) {
    if (v.key['kind'] !== 'fund') continue;
    const fund = v.key['fund'];
    if (fund === undefined) continue;
    // 14.7, XI-3: A DOOR IS LIVE ONLY WHILE ITS FUND IS. A pool that has ceased leaves its venue and
    // its last strike on the record, and money posted at that door is money posted to nobody.
    if (!ctx.parties.has(partyId(fund)) || !ctx.parties.get(partyId(fund)).status.alive) continue;
    const said = strikeOf(ctx.journal, fund);
    if (!said.some) continue;
    const years = said.value.durationYears.some ? said.value.durationYears.value : undefined;
    const offered = said.value.offered;
    out.push({
      venue: v.id,
      fund,
      perShare: said.value.perShare,
      years: typeof years === 'number' ? years : undefined,
      offered: typeof offered === 'number' ? offered : undefined,
      asks: v.key['asks'],
    });
  }
  return out;
}

/**
 * B2, B2.b, Law 19: THE LONGEST PROMISE IT HAS MADE, in years from today.
 *
 * It is a SELECTION over real dates and never a weighted average of them: an average duration is a
 * number no promise in the book actually has, and matching to it would be deciding at an average
 * (Appendix B). The furthest date it has promised anything on is a fact about one real schedule,
 * and it is the one that decides whether this institution can hold a long asset at all.
 *
 * An institution that has promised nothing yet has no duration to match, and gets none rather than
 * a zero — a life company with no policies is not a company that needs its money tomorrow.
 */
export function longestPromise(ctx: MechanismContext, insurer: PartyId): number | undefined {
  const now = ctx.calendar.startOf(ctx.period);
  let furthest: typeof now | undefined;
  let years: number | undefined;
  // 14.5: its promises are rows, and the furthest is the last day any of its cover runs to.
  for (const a of ctx.agreements.owedBy(insurer)) {
    if (a.state !== 'performing') continue;
    if (isPolicyTerms(a.terms) && a.terms.cover > 0) {
      if (compareCivil(a.terms.to, now) <= 0) continue;
      if (furthest === undefined || compareCivil(a.terms.to, furthest) > 0) furthest = a.terms.to;
    }
    // 14.7, Insurers B1, C2: a pension runs as long as the youngest member could live — to the end
    // of the mortality table from the age the cohort enters at — which is the long promise C2.a
    // says this sector holds against, and what a fund's duration is a read of.
    if (isPensionTerms(a.terms)) {
      const end = endOfMortalityTable(ctx.params);
      if (end === undefined) continue;
      const left = end - a.terms.entersAt;
      if (left > 0 && (years === undefined || left > years)) years = left;
    }
  }
  const cover = furthest === undefined ? undefined : yearFraction('ACT/365F', now, furthest);
  if (cover === undefined) return years;
  if (years === undefined) return cover;
  // A SELECTION over two real promises — the furthest of them is the one that decides (App B: not a bound).
  return largest([cover, years], 'the furthest of its promises');
}

/** 14.7: what an institution keeps back this period, and why — each a read of its own book. */
export interface Buffers {
  /** A4.c: the claims it expects a period on the cover it has out — its own outlook, on its own book. */
  readonly forClaims: Cash;
  /** Insurers B1 (14.6): the pensions its rows say it pays next period — a read of the schedule. */
  readonly forPensions: Cash;
  /** §29 A2.a: what it expects the pools to call of it a period — its own outlook of its own calls. */
  readonly forCalls: Cash;
}

/**
 * B2, A4.c, §29 A2.a, §46 (14.7): WHAT IT KEEPS BACK — three reads of its own book, none a ratio
 * anybody stated, and none the LAST thing that happened to it. What stood here kept back its last
 * claim and its last call: one storm and it sat on that much for ever, no storm and it kept
 * nothing. What it keeps now is what it EXPECTS a period — its `claims` outlook on the cover it
 * has out, its `called` outlook on the commitments it has made, both formed from every period of
 * its own history and corrected at its own memory (§46) — and what its pension rows say it pays
 * next period, which is a schedule and needs no outlook. An institution that has never had cover
 * out, never promised a pension and never been called keeps nothing back, which is right: it has
 * nothing to keep it against.
 */
export function buffersOf(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): Buffers {
  const view = ctx.participant(insurer);
  let coverOut = 0;
  const pensions: Cash[] = [];
  const reads: PensionReads = { goingRate: (o, r) => ctx.employment.goingRate(o, r), params: ctx.params };
  for (const a of ctx.agreements.owedBy(insurer)) {
    if (a.state !== 'performing' || a.ccy !== ccy) continue;
    if (isPolicyTerms(a.terms)) coverOut += a.terms.cover;
    if (isPensionTerms(a.terms)) {
      const perMember = pensionPerMember(reads, a.terms);
      if (perMember.some) pensions.push(valueAt(asPerPiece(perMember.value, 'a member’s pension'), asQty(weightOf(ctx.parties.resolve(a.creditor)), 'the members promised'), 'what the row pays next period'));
    }
  }
  const claims = view.outlook(about({ on: 'claims' }));
  const forClaims = claims.some && coverOut > 0
    ? valueAt(asPerPiece(claims.value.expected, 'what a unit of its cover costs it a period'), asQty(coverOut, 'the cover it has out'), 'the claims it expects a period')
    : asCash(0, 'nothing out, or nothing expected of it');
  const called = view.outlook(about({ on: 'called' }));
  const forCalls = called.some ? asCash(called.value.expected, 'what it expects to be called a period') : asCash(0, 'nobody has called it');
  return { forClaims, forPensions: sum(pensions).value, forCalls };
}

/**
 * B2, A4.c: WHAT IT CAN PUT TO WORK — its account, less what it keeps back (`buffersOf`). What is
 * left is money that should be earning, and an insurer sitting on it is an insurer whose promises
 * are unfunded — which is what B2 is about.
 */
export function investable(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): Cash {
  const account = ctx.accountOf(insurer, ccy);
  const cash = heldAsMoney(
    ctx.register.quantity(insurer, moneyInstrumentId(account.issuer, ccy)),
    'what is in its account',
  );
  const kept = buffersOf(ctx, insurer, ccy);
  return minus(cash, plus(plus(kept.forClaims, kept.forPensions, 'claims and pensions'), kept.forCalls, 'what it keeps back'), 'what it can put to work');
}

/**
 * §29 A2.a, A2.b, XI-2, Fund Shares C2 (item 13.5c): A CALL IT COULD NOT MEET, AND WHAT IT DOES
 * ABOUT IT — *"the investor funds it from its own liquidity ladder, selling if it must, or it
 * defaults on the call"*.
 *
 * 13.5 built the OBLIGATION and the DEFAULT: the call goes to the wire for the whole amount and an
 * investor that cannot pay gets a REFUSED instruction, which is a recorded state and nothing is
 * trimmed to fit (the fifth silent FORBID). **What was missing is the rest of the sentence.** An
 * investor that defaults and does nothing about it defaults again next time, and the ladder A2.a
 * describes — hold liquidity, sell if you must — had only its first rung.
 *
 * So it asks for its money back, out of the pools it is in, for what the call took and it did not
 * have. It names no price (XI-2): a forced seller that named one would not be one, and what it gets
 * is what the queue gives it at the NAV of the day it asked (Fund Shares C2). It is the SAME channel
 * a household short of its own cushion uses (`households/portfolio.ts`) — a holder that needs money
 * asks the pools it is in — and there is one of it in this world rather than one per holder (Law 4).
 *
 * THE LAG IS REAL AND IS THE CLAUSE. A call arrives and settles in one pass; the response is the
 * next period. *"In a stress the calls and its own troubles arrive together"* (A2.a), and an
 * investor selling into the market a week after it was called is what that looks like from inside.
 */
export function meetCalls(ctx: MechanismContext, insurer: PartyId): void {
  // 14.7, Law 8: THE PERIOD IS IN THE ASK. It walked every call ever made of it and tested the
  // period itself; it asks the registry for last period's calls, and a stale one cannot be read as
  // current. It is every call of the period and not the last, because two pools may call at once.
  let missed = asCash(0, 'nothing has been called of it');
  for (const c of callsOn(ctx.journal, insurer, periodOf(Number(ctx.period) - 1))) {
    if (c.paid || c.called <= 0) continue;
    missed = plus(missed, c.called, 'what it owes');
  }
  if (missed <= 0) return;
  for (const d of doors(ctx)) {
    const held = heldIn(ctx, insurer, d);
    if (held <= 0) continue;
    // Law 8: a share is indivisible, so what it hands back is a whole number of them, and it is the
    // number DOWN — what it can actually give back, never a fraction of a claim.
    const want = downTick(amountOf(missed, d.perShare, 'shares it must give back'));
    const units = downTick(amountOf(held, d.perShare, 'the shares it holds of this one'));
    if (want <= 0 || units <= 0) continue;
    // Law 6: not a bound on an outcome — it cannot hand back more of a pool than it holds of it,
    // which is arithmetic impossibility and says so.
    const asked = atMost(want, units, 'it cannot give back more of a pool than it holds');
    ctx.post(d.venue, {
      party: insurer,
      side: 'sell',
      // XI-2: at whatever the queue gives. A forced seller that named a price would not be one.
      price: 'market',
      qty: asQty(asked, 'what it asks back of this pool'),
    });
    ctx.record(
      'insurer.raised',
      [insurer, d.fund],
      { insurer, fund: d.fund, missed, asked },
      false,
    );
  }
}

/**
 * B2, B2.b (item 10f.5): WHETHER THIS INSTITUTION CAN ACCEPT THIS POOL — two refusals and nothing
 * else. It is an ACCEPTANCE and not a preference: what is left over is a set and not a winner.
 *
 *  - **Not longer than it promised.** A pool whose stated duration runs past the furthest thing this
 *    institution owes is a rate risk nobody asked it to take. A pool that states NO duration is not
 *    refused, because there is nothing to mismatch — which is how strategies, credit and equity
 *    become eligible under one test rather than three.
 *  - **Not less than its promises require.** What it requires is what those promises are discounted
 *    at; a pool that has told it it earns less than that has told it it does not cover them. A pool
 *    that has said nothing has not claimed anything to fail against, and nothing is returned as
 *    nothing rather than as a zero (App A).
 *
 * An institution that has promised nothing has neither test to apply, which is right: it has capital
 * and no liabilities, and there is nothing for an asset to be mismatched against.
 */
export function acceptable(
  d: Door,
  years: number | undefined,
  requires: number | undefined,
): boolean {
  if (years !== undefined && d.years !== undefined && d.years > years) return false;
  if (requires !== undefined && d.offered !== undefined && d.offered < requires) return false;
  return true;
}

/**
 * Fund Shares D2 (item 10f.5): WHERE THIS PERIOD'S MONEY GOES — whichever of the doors it can accept
 * it holds LEAST of, valued at what that pool published a share is worth.
 *
 * It is the whole of how an institution spreads, and **diversification is the OUTCOME of it rather
 * than a rule** (Law 2): nothing states a weight, a target or a maximum, and a pool it has never
 * bought is worth nothing to it and is therefore the smallest. Feed it every period and the book
 * fills out on its own; stop feeding one and it falls behind and is fed again.
 *
 * Ties break on the fund's own name, so two pools it holds none of are not separated by the order a
 * venue list happened to be built in (Audit D3).
 */
export function feedTheSmallest(
  ctx: MechanismContext,
  insurer: PartyId,
  open: readonly Door[],
): Door | undefined {
  let best: Door | undefined;
  let smallest = 0;
  for (const d of open) {
    const held = heldIn(ctx, insurer, d);
    if (best === undefined || held < smallest || (held === smallest && d.fund < best.fund)) {
      best = d;
      smallest = held;
    }
  }
  return best;
}

/** Law 19: what this institution holds of one pool, at what that pool published (Observer A3). */
function heldIn(ctx: MechanismContext, insurer: PartyId, d: Door): Cash {
  const terms: Cash[] = [];
  for (const i of ctx.instruments.issuedBy(partyId(d.fund))) {
    if (!i.status.live) continue;
    const units = ctx.register.quantity(insurer, i.id);
    if (units <= 0) continue;
    terms.push(valueAt(d.perShare, units, 'what it holds of this pool'));
  }
  return sum(terms).value;
}

/**
 * B2, Fund Shares C1: the institution's whole period. It reads what it owes, reads what it has,
 * picks whose mandate matches, and subscribes — one posting, at the same door a household uses.
 *
 * It CERTIFIES first where the door asks (item 10e.6). A named party is an institution and needs no
 * wealth test, so this is only for a door that asks something a named party could fail — and saying
 * so is cheaper than knowing which doors those are.
 */
export function allocate(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): void {
  const kept = buffersOf(ctx, insurer, ccy);
  const spare = investable(ctx, insurer, ccy);
  if (spare <= 0) return;
  const years = longestPromise(ctx, insurer);
  const requires = requiredOf(ctx, ccy, years);
  const open = doors(ctx).filter((d) => acceptable(d, years, requires));
  const door = feedTheSmallest(ctx, insurer, open);
  if (door === undefined) return;
  // Law 8: whole shares, and DOWN — what its money buys, never a share it cannot pay for.
  const shares = downTick(amountOf(spare, door.perShare, 'shares its money buys'));
  if (shares <= 0) return;
  ctx.post(door.venue, { party: insurer, side: 'buy', price: 'market', qty: shares });
  ctx.record(
    'insurer.allocated',
    [insurer, door.fund],
    {
      insurer,
      fund: door.fund,
      // B2, B2.b: the numbers the acceptance was made on, all of them public.
      promisedYears: years ?? null,
      mandateYears: door.years ?? null,
      requires: requires ?? null,
      offered: door.offered ?? null,
      // 10f.5: how many doors it could accept, which is what its book spreads over as the periods
      // go by. One is a real answer and it is visible as one.
      doors: open.length,
      shares,
      putToWork: spare,
      // 14.7: and what it kept back, each read named, so the whole of its account is on the record.
      keptForClaims: kept.forClaims,
      keptForPensions: kept.forPensions,
      keptForCalls: kept.forCalls,
    },
    false,
  );
}


/**
 * B2 (item 10f.5): WHAT ITS PROMISES REQUIRE — the rate they are discounted at, read off the
 * sovereign curve of the money they are promised in, at the tenor of the furthest of them.
 *
 * A liability payable in ten years has a present value, and the rate it is discounted at is the
 * rate the assets against it must earn to cover it. That is B2's actual economics and it is a READ:
 * no preference is declared, nothing is a spread over anything, and an institution in a world whose
 * sovereign has published no curve requires NOTHING rather than zero (App A) — it cannot tell
 * whether a pool covers its promises, so it refuses nothing on that ground.
 */
function requiredOf(
  ctx: MechanismContext,
  ccy: CurrencyCode,
  years: number | undefined,
): number | undefined {
  if (years === undefined) return undefined;
  const family = ctx.sovereignCurveIn(ccy);
  if (!family.some) return undefined;
  const at = ctx.curve(family.value.id).at(years);
  return at.yield.some ? at.yield.value : undefined;
}
