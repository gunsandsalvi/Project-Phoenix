/**
 * The manager as a business: many pools, a cost in people, and a roster that is an OUTCOME.
 *
 * @spec Fund Shares A4 Fund Shares B3 Fund Shares F3 Fund Shares G1 Labour A2 Labour A3 Seed A3 Law 2 Law 4 Law 6 Law 9
 *
 * What is worth asserting here is NOT that a particular fund opens or closes — that is a consequence
 * of what this world's savers happen to do, and a test that pinned it would be pinning an outcome
 * (Law 2, and a test never names a party). It is that the DECISION has the shape it claims to have:
 *
 *  - the product lives on the mandate, so a pool that did not exist at assembly can have one;
 *  - the set of pools is read from the mandates, not from the roster the world was drawn with;
 *  - what a manager charges to enter is UNDER what the incumbent charges, and it refuses to enter
 *    when that would not cover its cost — the refusal, not a floor, is what stops entry (Law 6);
 *  - a house runs several pools and takes a fee from each;
 *  - two products are the same product when their BLUEPRINTS say the same thing, never their names.
 */
import { describe, expect, it } from 'vitest';
import {
  drawFunds,
  drawManagers,
  drawStrategies,
  drawTrackers,
  nameOf,
} from '../src/mechanisms/funds/data.js';
import { sameProduct } from '../src/mechanisms/funds/manager.js';
import { holdsThings } from '../src/mechanisms/funds/things.js';
import { admits } from '../src/registry/blueprint.js';
import { USD } from '../src/index.js';
import { none } from '../src/core/option.js';
import type { Blueprint } from '../src/registry/blueprint.js';
import type { Grade } from '../src/registry/grades.js';
import type { PartyId } from '../src/core/ids.js';

const BANKS = [
  { bank: 'bank.a', size: 9 },
  { bank: 'bank.b', size: 7 },
  { bank: 'bank.c', size: 2 },
];

describe('the houses this world opens with', () => {
  it('gives one house every pool its bank sponsors, and the house has its own preferences', () => {
    const pools = drawFunds(BANKS, 'a-seed');
    const houses = drawManagers(pools, 'a-seed');
    // F3: a manager exists because a pool names it, and it is ONE party for all of that bank's.
    for (const house of houses) {
      const run = pools.filter((p) => p.manager === house.manager);
      expect(run.length).toBeGreaterThan(0);
      // Law 4: one name, however many pools name it. Two rows naming one manager two things would
      // give the party whichever the seed reached first, silently.
      expect(new Set(run.map((p) => p.managerName)).size).toBe(1);
    }
    // A house with more than one product is what makes it a business rather than a fund with a
    // second party id — and this world's largest sponsor has three.
    const biggest = houses
      .map((h) => pools.filter((p) => p.manager === h.manager).length)
      .sort((x, y) => y - x)[0];
    expect(biggest).toBeGreaterThan(1);
  });

  it('draws two houses unalike, because two with the same preferences are one with two names', () => {
    const houses = drawManagers(drawFunds(BANKS, 'a-seed'), 'a-seed');
    expect(houses.length).toBeGreaterThan(1);
    expect(new Set(houses.map((h) => h.undercut)).size).toBe(houses.length);
    for (const h of houses) {
      // A house with no patience closes every product it ever opens, the period after it opens it.
      expect(h.patience).toBeGreaterThan(0);
    }
  });

  it('draws every pool its own economics, so two pools are two bidders', () => {
    const pools = drawFunds(BANKS, 'a-seed');
    expect(new Set(pools.map((p) => p.fee)).size).toBe(pools.length);
    expect(new Set(pools.map((p) => p.requiredYield)).size).toBe(pools.length);
  });
});

describe('what makes two pools the same product', () => {
  const credit: Blueprint = {
    classes: ['corporate', 'structured'],
    currencies: [],
    duration: { from: 1, to: 30 },
  };

  it('is the blueprint and not the name', () => {
    // The same bands stated in another order are the same product: a manager copying a rival is
    // copying what the mandate SAYS, and the order somebody typed the classes in is not a fact.
    expect(sameProduct(credit, { ...credit, classes: ['structured', 'corporate'] })).toBe(true);
    // A different band is a different product, however alike the two read.
    expect(sameProduct(credit, { ...credit, duration: { from: 1, to: 7 } })).toBe(false);
    expect(sameProduct(credit, { ...credit, worstGrade: 'bbb' })).toBe(false);
    expect(sameProduct(credit, { ...credit, currencies: [USD] })).toBe(false);
  });

  it('separates a money fund from a credit fund by the band that actually differs', () => {
    const pools = drawFunds(BANKS, 'a-seed');
    const money = pools.find((p) => p.blueprint.duration?.to === 1);
    const longer = pools.find((p) => (p.blueprint.duration?.to ?? 0) > 1);
    expect(money).toBeDefined();
    expect(longer).toBeDefined();
    if (money === undefined || longer === undefined) return;
    expect(sameProduct(money.blueprint, longer.blueprint)).toBe(false);
  });
});

describe('a name is derived from what the pool does', () => {
  it('names the house first and what the mandate says second', () => {
    expect(nameOf({ blueprint: { classes: ['thing'], currencies: [] }, house: 'North' })).toBe(
      'North Real Asset Fund',
    );
    expect(
      nameOf({ blueprint: { classes: ['residual'], currencies: [] }, house: 'North' }),
    ).toBe('North Equity Fund');
    expect(nameOf({ blueprint: { classes: [], currencies: [] }, house: 'North' })).toBe(
      'North Multi-Asset Fund',
    );
    // Law 9: a tracker is named for the index it follows, which is what a market calls one.
    expect(
      nameOf({ blueprint: { classes: ['residual'], currencies: [] }, tracks: 'idx', house: 'North' }),
    ).toBe('North idx tracker');
  });
});

describe('what the seed is, and what it is not', () => {
  it('is a subscription and never an endowment', () => {
    // A3, C1, item 10e.5: the assertion that matters is structural and is made in the module rather
    // than here — `openPool` POSTS into the pool's own venue and the strike settles it like any
    // other subscription, so there is no path by which units reach a pool without an instruction.
    // What this test can hold is the shape of the decision: what a house puts in is bounded by what
    // it HOLDS, never by a share of anything, and a house with nothing to spare puts in nothing.
    const pools = drawFunds(BANKS, 'a-seed');
    // Every pool this world OPENS with has no seed at all: they are an opening condition, and the
    // seed is what a manager does when it opens one itself (Seed A3).
    expect(pools.every((p) => !('seed' in p))).toBe(true);
  });
});

describe('who may get in', () => {
  it('offers a deposit substitute and a listed vehicle to anybody, and asks of the rest', () => {
    const pools = drawFunds(BANKS, 'a-seed');
    // The owner's ladder: retail reaches a money fund and a listed one; a fund asks.
    for (const p of pools) {
      const isMoney = p.blueprint.duration?.to === 1;
      expect(p.offeredPublicly).toBe(isMoney);
    }
    for (const t of drawTrackers(['equity.firm.1'], ['bank.a'], 'a-seed', ['idx'])) {
      // E1, G1.a: you buy a listed share from a HOLDER, in a market anybody can trade in. A vehicle
      // whose shares are listed cannot ask anything of whoever ends up with one.
      expect(t.offeredPublicly).toBe(true);
    }
  });
});

describe('what makes a pool a hedge fund', () => {
  it('is four terms of its mandate and nothing else', () => {
    const strategies = drawStrategies(BANKS, 'a-seed');
    expect(strategies.length).toBeGreaterThan(1);
    for (const s of strategies) {
      // §28 A4: a WIDE mandate, said with a word rather than a list of every class this world
      // happens to have — a list goes stale the day somebody writes a tenth.
      expect(s.mayWrite).toBe('anything');
      // §28 B1: a PERMISSION. Nothing here supplies it; a prime broker does (13.3).
      expect(s.leverage).toBe(true);
      // §28 A3: the asymmetric second fee, and two houses would take different shares of a gain.
      expect(s.performanceFee).toBeGreaterThan(0);
      // §28 D5.a: a notice period, which is what makes a shock reach this vehicle LATER.
      expect(s.liquidity.how).toBe('semiLiquid');
      // §28 A1: never offered to the public.
      expect(s.offeredPublicly).toBe(false);
    }
    // All three under ONE house: a strategy house with one product has no book of business.
    expect(new Set(strategies.map((s) => s.manager)).size).toBe(1);
    // And every long-only pool this world opens with says the opposite of all four.
    for (const p of drawFunds(BANKS, 'a-seed')) {
      expect(p.mayWrite).toEqual([]);
      expect(p.leverage).toBe(false);
      expect(p.performanceFee).toBe(0);
    }
  });

  it('says a macro mandate by saying NOTHING, which is the test of the language', () => {
    const macro = drawStrategies(BANKS, 'a-seed').find((s) => s.blueprint.classes.length === 0);
    expect(macro).toBeDefined();
    if (macro === undefined) return;
    // A4: no class band, no currency band — unrestricted, and it stays unrestricted when this world
    // grows an asset class nobody has written yet. That is what a band-based language buys.
    expect(macro.blueprint.currencies).toEqual([]);
    expect(macro.blueprint.duration).toBeUndefined();
    expect(macro.ownCurrencyOnly).toBe(false);
  });
});

describe('the empty blueprint', () => {
  it('is a mandate over EVERYTHING and never a mandate over things', () => {
    // `[].every(...)` is true, so the case has to be said out loud rather than fallen into: a macro
    // mandate that states no class band holds anything, and routing it to the commodity path would
    // have had it bidding for grain and nothing else.
    expect(holdsThings({ classes: [], currencies: [] })).toBe(false);
    expect(holdsThings({ classes: ['thing'], currencies: [] })).toBe(true);
    expect(holdsThings({ classes: ['thing', 'corporate'], currencies: [] })).toBe(false);
  });

  it('admits an asset it says nothing about, and refuses one that cannot answer a band it states', () => {
    const anything: Blueprint = { classes: [], currencies: [] };
    const share = {
      what: 'residual' as const,
      ccy: USD,
      durationYears: none<number>(),
      seniority: none<number>(),
      secured: false,
      listed: true,
      grade: none<Grade>(),
      obligor: none<PartyId>(),
    };
    expect(admits(anything, share, () => undefined)).toBe(true);
    // A share has no duration. A blueprint that states one is not describing a share, and admitting
    // it by silence would be the `?? 0` this codebase refuses.
    expect(admits({ ...anything, duration: { to: 1 } }, share, () => undefined)).toBe(false);
  });
});
