/**
 * The funds this world has, and what each one may hold.
 *
 * @spec Fund Shares A1 Fund Shares A4 Fund Shares D1 Fund Shares E1 Fund Shares E2 Fund Shares E3 Fund Shares F3 Hedge Funds A1 Hedge Funds A3 Hedge Funds A4 Hedge Funds B1 Hedge Funds C1 Hedge Funds D5 Hedge Funds D5.a Private Equity A1 Private Equity A2 Private Equity A3 Private Equity A5 Private Equity B2.a Equity C2.c Seed A3 Seed B1 Law 2 Law 15
 *
 * Data only (Law 15). A MANDATE is what a fund may hold and how long it may hold it for, and it is
 * a real constraint on what the fund buys rather than a label on it (A4): the module never posts an
 * order for something outside it, which is why a flow into a fund becomes a purchase of exactly
 * what the mandate allows and nothing else. That is what makes a fund a transmission channel.
 */
import { paramId } from '../../core/ids.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { InvalidRegistry } from '../../core/errors.js';
import type { Blueprint } from '../../registry/blueprint.js';
import type { AssetClass } from '../../registry/universe.js';
import type { Liquidity, MayWrite } from './mandate.js';
import { prng } from '../../rng/prng.js';
import { between, type Spread } from '../../rng/spread.js';

export interface FundDecl {
  /** A1: the named party. It has an account and a register of holdings like anything else. */
  readonly fund: string;
  /** F3: the manager is a SEPARATE party. The fee is its income and the fund's cost. */
  readonly manager: string;
  readonly managerName: string;
  readonly bank: string;

  /* ---- THE MANDATE: what it may hold, how you get out, and whether it chooses ---- */

  /**
   * A4, item 10e: WHAT IT MAY HOLD, in the one language every vehicle is described by. It was a
   * list of instrument kind ids beside a tenor counted in periods; both are bands now, and the
   * duration one is in YEARS measured from today, so a line ages out of the mandate by itself.
   */
  readonly blueprint: Blueprint;
  /**
   * Item 10e: SINGLE OR MULTI CURRENCY, which is a real term of a mandate and not a default. A
   * fund that may hold another money has an FX exposure its investors agreed to; one that may not
   * has its own money written into its blueprint when it is launched, from where the fund IS.
   */
  readonly ownCurrencyOnly: boolean;
  /** G1: how its investors get in and out, which is what decides if it can be forced to sell. */
  readonly liquidity: Liquidity;
  /**
   * A3, §28 A4 (item 13.2): WHAT IT MAY TAKE A POSITION IN. `[]` is a real term — a money fund does
   * not write derivatives — and `'anything'` is §28's wide mandate.
   */
  readonly mayWrite: MayWrite;
  /**
   * B1, F2, XI-3, §28 B1 (item 13.2): WHETHER IT MAY BE LEVERED. `false` is a real term of a
   * mandate and not an absence; `true` is what makes one a hedge fund's. A levered pool borrows
   * from a NAMED lender, which is what makes its leverage a fact about a loan and never a property
   * of the pool — so this permits, and it never supplies.
   */
  readonly leverage: boolean;
  /**
   * §28 A3 (item 13.2): THE SHARE OF A GAIN THE MANAGER TAKES, over the highest value a share of
   * this pool has ever been worth at a charge. Zero for everything that is not a hedge fund, and
   * *"the asymmetry of that second fee is a reason for risk-taking"* — it is paid on the way up and
   * never refunded on the way down.
   */
  readonly performanceFee: number;
  /**
   * Hedge Funds B1, B5 (item 13.3): the multiple of its investors' money it MEANS to run its
   * securities book at. `1` is unlevered, which is a real term. What it actually runs at is the
   * lesser of this and what its broker will finance — the lender's decision, not the pool's.
   */
  readonly targetLeverage: number;
  /**
   * Item 10e.6: WHO MAY GET IN AT ALL. `true` is offered to the public and anybody may subscribe;
   * `false` asks an entrant to clear the accredited-investor line, which is a POLICY a regulator
   * sets (`FUND_PARAMS.accreditedWealth`) and not a number about this fund.
   */
  readonly offeredPublicly: boolean;
  /**
   * Indices C2, C2.a: WHETHER IT CHOOSES. Absent is ACTIVE — it picks within its blueprint on its
   * own view. Present is PASSIVE: it holds whatever that index says is in it at whatever the index
   * weighs it, and a rebalance is the index answering differently, which forces a trade in the same
   * session at whatever the book gives (C2.a). A tracker is not an investor, and that is the whole
   * difference between the two businesses.
   */
  readonly tracks?: string;

  /* ---- ITS ECONOMICS: one block, one spread table, every vehicle ---- */

  /** C2.a: the share of its net assets it keeps in cash, so an ordinary redemption needs no sale. */
  readonly buffer: number;
  /**
   * B3, F3: what the manager charges, per annum on net assets — its income and the fund's cost.
   *
   * Item 10e: IN THE SAME BLOCK AS THE OTHER TWO, AND DRAWN THE SAME WAY. The tracker declared a
   * literal `0.001` inline while every other vehicle drew from `FUND_SPREAD.fee`: two writers of
   * one fact (Law 4), and the consequence was that a tracker's fee could not DISPERSE — so it could
   * never be dearer or cheaper than a rival, never lose money to one, and never be wound up for
   * costing more than it earned. Half of what a manager's launch decision turns on was missing for
   * one kind of vehicle because its fee lived somewhere else.
   */
  readonly fee: number;
  /**
   * D2, D2.a: what its investors require of it over what a deposit pays them, per annum. It is what
   * the fund will pay for paper: what a saver demands of it is what it demands of what it holds.
   */
  readonly requiredYield: number;

  /* ---- HOW THIS WORLD OPENS IT ---- */

  /**
   * Private Equity A1, A2, Seed A3 (item 13.5): WHO RAISED THIS FUND AND FOR HOW MUCH, present only
   * for a closed-end vehicle whose capital is COMMITTED rather than paid.
   *
   * A closed-end fund was raised before it existed — that is what a vintage is — so the commitments
   * behind the one this world opens with are an opening condition, the way a bank's balance sheet
   * is. What is NOT modelled, and is not pretended to be, is WHY each institution committed: that
   * decision wants a pension with a very long liability (14.1) or a deal pipeline worth funding
   * (13.5b), and neither is built. What IS built is everything that happens after it committed.
   *
   * 14.1: the value is the SHARE of the money the investor holds when its promise is opened, not an
   * amount — the amount is read off the investor's own account at that moment (`openCommitments`).
   */
  readonly commitments?: Readonly<Record<string, number>>;
  /**
   * Seed A3, G1.a: present when this vehicle's investors come and go IN KIND against a basket —
   * which is a consequence of `liquidity: 'listed'` and never a separate kind of thing.
   *
   * A vehicle whose investors subscribe with CASH is launched by the first of them deciding to; one
   * whose investors come in kind cannot be, because the first share has to be paid for with the
   * things themselves. So this is what the seed needs to open one, and it is absent for everything
   * that opens the ordinary way.
   */
  readonly inKind?: InKindLaunch;
  readonly why: string;
}

/**
 * G1.a, item 10e: THE LAUNCH TERMS OF A VEHICLE WHOSE INVESTORS COME IN KIND, or a refusal.
 *
 * A vehicle that subscribes for CASH is launched by the first investor deciding to; one that comes
 * in kind cannot be, because the first share has to be paid for with the things themselves. Asking
 * for terms a fund does not have is a question about the wrong fund, so it throws rather than
 * answering with an empty basket (App A: missing is missing).
 */
export function inKindOf(d: FundDecl): InKindLaunch {
  if (d.inKind === undefined) {
    throw new InvalidRegistry('Fund Shares G1.a', `${d.fund} is not launched in kind`);
  }
  return d.inKind;
}

/** E3, E3.a, Seed A3: what it takes to open a vehicle whose investors come and go in kind. */
export interface InKindLaunch {
  /** E3: what a creation unit is made of — the lines, and how much of each. */
  readonly basket: Readonly<Record<string, number>>;
  /** What share of the market its launch takes, so how many shares that is can be read off. */
  readonly share: number;
  /** Seed A3: who takes the first shares, and what share of the launch each of them took. */
  readonly by: Readonly<Record<string, number>>;
  /**
   * E3.a: whether the SEED opens it, or a phase does when its index first answers. A tracker on a
   * rule that reads its constituents' own prints cannot be handed a basket at period zero —
   * nobody could have said it was right — and two vehicles endowed with the same float is a market
   * owned twice.
   */
  readonly seeded: boolean;
  /** The modules that must be assembled before it, because its basket is made of what they issue. */
  readonly needs: readonly string[];
}

/**
 * Law 9, item 10e: WHAT THIS VEHICLE IS CALLED, DERIVED FROM WHAT IT IS.
 *
 * A market names a thing by what it is, and so does this: the names were typed — "Money Fund",
 * "Credit Fund", "{index} tracker" — which meant a vehicle could be called one thing and hold
 * another, and that the name was a THIRD place a fund's type was written down after its
 * declaration and its behaviour.
 *
 * It reads the mandate and nothing else, so a fund whose blueprint changes is renamed by the same
 * change and cannot drift from what it does.
 */
export interface Named {
  readonly blueprint: Blueprint;
  readonly tracks?: string;
  /**
   * Item 10e.4: WHOSE IT IS. A pool this world OPENS with is named for the bank that sponsored it,
   * which is how a bank-sponsored fund is named; one a manager LAUNCHES is named for the manager,
   * because there is nobody else it belongs to. Either way it is a house's name in front of what
   * the mandate says the fund does, which is how a market names a fund (Law 9).
   */
  readonly house: string;
}

export function nameOf(d: Named): string {
  // Law 9, item 13.2: THE HOUSE IS THE MANAGER, not the bank the pool banks at. A market names a
  // fund for whoever runs it; the bank is where its account is, which is a fact about its cash and
  // not about the product. It read `d.bank` while every pool in this world was sponsored by one —
  // and a hedge fund is sponsored by nobody, so the field was about to name the wrong party.
  if (d.tracks !== undefined) return `${d.house} ${d.tracks} tracker`;
  const what = d.blueprint.classes;
  const short = d.blueprint.duration?.to !== undefined && d.blueprint.duration.to <= 1;
  const of =
    what.length === 0
      ? 'Multi-Asset'
      : what.includes('thing')
        ? 'Real Asset'
        : what.includes('residual')
          ? 'Equity'
          : short
            ? 'Money'
            : 'Credit';
  return `${d.house} ${of} Fund`;
}

export const FUND_PARAMS = {
  openingShare: paramId('fund.openingSharePrice'),
  /**
   * Item 10e.6, Law 2: THE ACCREDITED-INVESTOR LINE — what an entrant must be worth PER MEMBER to
   * get into a vehicle that is not offered to the public. It is a POLICY: a number a regulator sets
   * and changes, owned by `parliament`, and it is the first real channel item 19 has into this
   * sector. Nothing in this world produces it and nothing should: it is not an outcome.
   */
  accreditedWealth: paramId('funds.accreditedWealthPerMember'),
} as const;

/**
 * Item 10e.6: the line, in money per member.
 *
 * WHY IT IS A ROUND NUMBER AND WHY THAT IS RIGHT. A regulator's threshold is a round number chosen
 * by somebody, not a quantile of a distribution — and if it were a quantile it would be an OUTCOME
 * dressed as a rule, moving with the wealth it is supposed to sort. A million dollars is the figure
 * the real accredited-investor and professional-client tests are built on, net of a home, which is
 * what this world's `wealthOf` already excludes (a home is a thing its people live in, not a claim).
 */
export const ACCREDITED_WEALTH = 1_000_000 * MONEY_PIECES;

/**
 * E1-E3: a fund whose shares are LISTED. It is not another kind of thing — its shares are the same
 * claim on the same kind of book (A2, B1) — and what makes it an exchange-traded fund is two facts
 * about it: its shares trade, so it has a market and a cleared price beside its NAV (E2), and its
 * investors come and go IN KIND against a basket rather than for cash (G1.a), which is why it is
 * not a forced seller and why some other vehicle has to carry that.
 */
/**
 * E3, Indices C1, C2: how big a slice of its lines a vehicle opens holding, and who takes the
 * first shares. Both are facts about the OPENING (Seed A3) and neither says what kind of thing it
 * is — a vehicle whose investors come in kind needs somebody to hand it the things.
 */
export const IN_KIND_LAUNCH_SHARE = 0.05;
export const IN_KIND_SPONSOR_SHARE = 0.4;
export const IN_KIND_PARTICIPANTS = 4;

/**
 * M9, Indices C1, C2, item 10e: THE VEHICLES THIS WORLD OPENS ON AN INDEX — and they are FUNDS.
 *
 * There is no `EtfDecl` any more. What used to be a second declaration type, with its own draw, its
 * own params, its own phase and its own FEE written as a literal, is a fund whose mandate says two
 * things: its investors come and go IN KIND (`liquidity: 'listed'`), and it does not choose
 * (`tracks`). Everything else about it — a blueprint, a buffer, a fee, what its investors require —
 * is the same block every other vehicle has, drawn from the same spreads, so a tracker can be
 * dearer than a rival and lose money to one.
 */
export function drawTrackers(
  lines: readonly string[],
  banks: readonly string[],
  seed: string,
  /**
   * M9, Indices C1, C2: one vehicle per index, because a tracker is how an index reaches a market
   * at all. With one vehicle on one line, C2's simultaneity is a market of one and a firm crossing
   * a size boundary is a rebalance nobody has to trade.
   */
  tracks: readonly string[],
  /**
   * A4, Indices C1 (17.10b): WHAT THESE VEHICLES MAY HOLD, which is not what their index says they
   * MUST. An equity tracker's mandate is listed equity — the class a share is — and a CREDIT
   * tracker's is the claims a company issued; the index picks the lines inside that, and the
   * mandate is what bounds it. Two words apart, and without them a credit tracker would be an
   * equity mandate holding bonds.
   */
  holds: readonly AssetClass[],
  /** Law 9: what this house's vehicles are called, so two sets of them are two names. */
  named: string,
  /**
   * E3.a, Indices A3, D5.a: whether the SEED may open the first of them. It may where the index
   * answers at period zero — a broad equity line is its listed constituents and they are listed —
   * and it may not where the index holds paper that does not exist yet, which is every credit line.
   */
  seeds: boolean,
): readonly FundDecl[] {
  if (tracks.length === 0) return [];
  // E3.a: a SEEDED vehicle needs the basket named in advance; one that launches in kind reads its
  // own index for it (`funds/index.ts launchBasket`), so it needs no list here at all.
  if (seeds && lines.length === 0) return [];
  const rng = prng(seed, 'trackers');
  const basket: Record<string, number> = {};
  for (const line of lines) basket[line] = 1;
  const manager = `manager.${named}`;
  const by: Record<string, number> = {};
  by[manager] = IN_KIND_SPONSOR_SHARE;
  // The participants, drawn without repeating a name: a bank cannot be two of them.
  const pool = [...banks];
  const chosen: string[] = [];
  while (chosen.length < IN_KIND_PARTICIPANTS && pool.length > 0) {
    const at = rng.int(pool.length);
    const taken = pool[at];
    pool.splice(at, 1);
    if (taken !== undefined) chosen.push(taken);
  }
  // E3: what is left for the participants once the sponsor has its slice. These are SHARES OF THE
  // LAUNCH and not counts: how many shares that comes to is the seed's, because it is the one that
  // can see how big the lines turned out.
  const rest = 1 - IN_KIND_SPONSOR_SHARE;
  for (const bank of [...chosen].sort()) by[bank] = rest / chosen.length;
  const home = chosen[0];
  return tracks.map((index, n) => ({
    // Law 9: the index it follows IS its name. The one the seed opens is the house's broad
    // vehicle and is named for the house; every other is named for its line, so a reader can see
    // which market a vehicle is in without looking anything up.
    fund: seeds && n === 0 ? `${named}.us` : `${named}.${index}`,
    manager,
    managerName: 'American Index Managers',
    bank: home ?? manager,
    /**
     * A4: what it MAY hold, which is not the same as what the index says it MUST. Listed equity —
     * the class a share is, and the one read that separates a public company from a private one.
     * The index picks the lines inside that; the mandate is what bounds it.
     */
    /**
     * A3, B1, §28 A3, A4, B1 (item 13.2): WHAT EVERY LONG-ONLY POOL THIS WORLD OPENS WITH SAYS.
     * It writes no contracts, it may not be levered, and its manager takes no share of a gain.
     * All three are TERMS its investors agreed to, and a hedge fund is the mandate that says
     * otherwise — which is the whole of what makes one (§28, and there is no hedge-fund party kind).
     */
    mayWrite: [],
    leverage: false,
    performanceFee: 0,
    targetLeverage: 1,
    blueprint: { classes: holds, currencies: [], listed: true },
    ownCurrencyOnly: true,
    // E1, G1.a: its shares TRADE and its investors come and go IN KIND against the basket, which is
    // why it is not a forced seller and why some other vehicle has to carry that.
    liquidity: { how: 'listed' },
    // Item 10e.6: you buy the share from a HOLDER, in a market anybody can trade in. A vehicle
    // whose shares are listed cannot ask anything of whoever ends up holding one, and that is the
    // first rung of the owner's ladder: retail reaches listed funds and money funds.
    offeredPublicly: true,
    tracks: index,
    buffer: between(rng, FUND_SPREAD.buffer),
    fee: between(rng, FUND_SPREAD.fee),
    requiredYield: between(rng, FUND_SPREAD.requiredYield),
    inKind: {
      basket,
      share: IN_KIND_LAUNCH_SHARE,
      by,
      /**
       * E3.a, Indices A3, D5.a: the seed can open the one whose index answers at period zero. A
       * tracker on a size segment holds whatever that segment's rule says is in it, and the rule
       * reads its constituents' own prints — so a seed that launched one would be handing it a
       * basket nobody could have said was right.
       */
      seeded: seeds && n === 0,
      needs: ['equity', 'banks'],
    },
    why: 'Fund Shares E1-E4: the vehicle that has TWO values. Its shares trade, so a session prices them; its book is the index it tracks, so a read prices them too; and the gap between the two is what somebody has to want to close for it to close at all (E3.a).',
  }));
}
export const SPONSOR_SIZE = 4;

/**
 * Fund Shares A3, A4, C1.a: THE ASSET MANAGER THAT HOLDS CREDIT, and it is the demand side of every
 * corporate market this world has.
 *
 * A4 says a mandate is *"a real constraint on what it buys, not a label"*, and A3 and A4 together are
 * why a fund is a TRANSMISSION CHANNEL: a flow into the fund becomes a purchase of what the mandate
 * allows. Until this existed, **no mandate in this world allowed credit** — every fund drawn was a
 * money fund holding bills, one commodity fund holding grain, or a tracker holding an index — so a
 * corporate bond, a piece of commercial paper and a securitisation note could all be brought to a
 * book whose only bidders were bank dealing desks putting their own capital behind an inventory.
 *
 * A DEALING DESK IS NOT DEMAND. It makes a market, carries what it cannot place and is bounded by
 * its own balance sheet (Dealer Desks D3.b) — it is the intermediary, not the holder. What actually
 * absorbs a corporate issue is somebody's SAVINGS, managed by somebody paid to manage them, and
 * that is this: a long-only manager whose investors gave it money to hold credit, whose appetite is
 * what its investors require of it, and which must find the cash by SELLING when they want it back
 * (C2.b — the forced-seller channel, and the point).
 *
 * It is not a hedge fund (§28) and must not be confused with one: nothing here is levered, nothing
 * is short, and its reason is not a view about a mispricing. It is the bulk of the buy side, and
 * the leveraged, speculative slice is a different sector with a different item.
 */
export const WIDE_SPONSOR_SIZE = 3;

/**
 * Fund Shares D2.a, Corporate Credit E5: WHAT A CREDIT FUND'S INVESTORS REQUIRE OF IT, per annum.
 *
 * A money fund's investor is choosing against a DEPOSIT and requires a little over it (0.2–0.9% in
 * `FUND_SPREAD`). A credit fund's investor is taking a company's credit and years of duration, and
 * requires a great deal more for it — which is why this is its own spread and not the same number
 * read twice. The dispersion between two funds is what gives a corporate book two bidders at
 * different levels rather than one price everybody agrees on (§46 A3, Corporate Credit A4.b).
 */
export const CREDIT_SPREAD: Spread = {
  low: 0.012,
  high: 0.055,
  why: 'Fund Shares D2.a: what a credit fund\u2019s investors require of it per annum, which is what it will pay for a company\u2019s paper. Its investor is taking credit and duration rather than choosing against a deposit, so it requires several times what a money fund\u2019s investor does \u2014 and the SPREAD between two of them is what makes a corporate book have two bidders at different levels instead of one number everybody shares.',
};

/**
 * §28 A1, A3, A4, B1, C1, D5.a (item 13.2): THE STRATEGIES — and there is no hedge-fund party kind.
 *
 * *"Everything is a fund. An HF runs funds, same as PE"* (the owner). So a hedge fund house is a
 * MANAGER like any other, and what makes its pools hedge funds is four terms of their mandates and
 * nothing else:
 *
 *  - a WIDE blueprint — what the strategy is about, and a macro one states no class band at all;
 *  - `mayWrite: 'anything'` — §28 A4's wide mandate, which is what makes it *"the natural home of
 *    the speculative side of every derivative book"* (C1);
 *  - `leverage: true` — a PERMISSION, and it never supplies: what lends to it is a prime broker
 *    (13.3), and until there is one this is a permission nobody has acted on (B1);
 *  - a PERFORMANCE FEE — the asymmetric second fee (A3), which is the reason for risk-taking.
 *
 * AND ITS INVESTORS WAIT. `semiLiquid` with a quarterly window is D5.a's notice period, *"a real
 * contractual term with real consequences for who gets out"* — the queue is the mechanism, what it
 * costs the holders who stayed is C4.a, and it is the whole reason a shock reaches this vehicle
 * LATER than it reaches a money fund.
 *
 * THREE STRATEGIES UNDER ONE HOUSE, because a house with one product has no book of business to
 * spread its people over (item 10e.4) and because *"HFs exist in multiple strategies"* — and each is
 * a BLUEPRINT rather than a mechanism, which is the whole of what 10e's language bought here.
 */
export interface StrategyDecl {
  readonly id: string;
  readonly what: Blueprint;
  readonly why: string;
}

export const STRATEGIES: readonly StrategyDecl[] = [
  {
    id: 'equity',
    // C1, C2: listed shares, which is the one class whose price this world clears in a session
    // every period — so a view about one can be wrong in public, every week.
    what: { classes: ['residual'], currencies: [], listed: true },
    why: '§28 C1, C2: long and short listed equity. Its reason is a view about what a line is worth against what the book says, and the SHORT half needs a borrow (item 9.4), which is what makes it a position somebody can lose rather than a free arbitrage (Appendix B).',
  },
  {
    id: 'credit',
    what: { classes: ['corporate', 'structured'], currencies: [] },
    why: '§28 C1, C3: long and short corporate credit. It is the other side of every issue items 10, 10b and 10c brought to market, and the liquidity premium C3 says it is PAID to hold is the one a forced seller pays (XI-2).',
  },
  {
    id: 'macro',
    // A4: a wide mandate states NO class band and NO currency — which in the blueprint language is
    // silence, and silence is what "unrestricted" has to be if it is to stay true when this world
    // grows an asset class nobody has written yet.
    what: { classes: [], currencies: [] },
    why: '§28 A4, C1: macro — any class, any money, and its reason is a view about a rate, a currency or a level rather than about a name. It is the mandate the blueprint language describes by saying NOTHING, which is what makes it the test of that language.',
  },
];

/** D5.a: periods between the windows a strategy's investors may get out at. A quarter. */
export const STRATEGY_WINDOW = 13;

export const STRATEGY_SPREAD: Readonly<
  Record<'fee' | 'performance' | 'buffer' | 'leverage', Spread>
> = {
  leverage: {
    low: 1.5,
    high: 4,
    why: '\u00a728 B1, B5: the multiple of its investors\u2019 money a strategy MEANS to run its book at. It is a preference and never what it achieves: a broker finances what its own view of the risk leaves room for (Prime Brokerage C1), so a house that wants four and is offered two runs at two \u2014 and the gap between the two numbers is what B3 means by *"the amount available is the lender\u2019s decision and it changes"*. The SPREAD is what makes one house fail a call that another survives.',
  },
  fee: {
    low: 0.01,
    high: 0.022,
    why: '§28 A3: what a strategy house charges on assets per annum. Several times a long-only manager’s, because what it is selling is not access to a market but a view — and it is the half of its income that arrives whether the view was right or not, which is why the OTHER half is what the clause is about.',
  },
  performance: {
    low: 0.1,
    high: 0.25,
    why: '§28 A3: the share of a gain over the high-water mark. *"The asymmetry of that second fee is a reason for risk-taking"* — it is paid on the way up and never refunded on the way down, so a manager that has just lost money earns nothing until it is back above where it last charged. The SPREAD is what makes two houses two businesses: one that takes a quarter of every gain has a reason to swing that one taking a tenth does not.',
  },
  buffer: {
    low: 0.02,
    high: 0.1,
    why: '§28 D5, Fund Shares C2.a: what a strategy keeps in cash. LESS than a long-only pool’s, because its investors cannot ask for their money back until a window opens (D5.a) — so it can afford to be more fully invested, which is also why a redemption window arriving in a bad market is the second forced-seller channel D5 names.',
  },
};

/**
 * §28 A1, A3, F3 (item 13.2): the strategy house this world opens with, and its three pools.
 *
 * It banks at the largest bank that sponsors anything, which is where a fund of this size would
 * bank — and that is ALL the bank is to it: it sponsors nothing, names nothing, and the pools are
 * named for the HOUSE that runs them (Law 9).
 */
export function drawStrategies(
  banks: readonly { readonly bank: string; readonly size: number }[],
  seed: string,
): readonly FundDecl[] {
  const at = banks.find((b) => b.size >= SPONSOR_SIZE);
  if (at === undefined) return [];
  const rng = prng(seed, 'strategies');
  return STRATEGIES.map((strategy) => ({
    fund: `fund.strategy.${strategy.id}`,
    manager: 'manager.strategy',
    managerName: 'Meridian Partners',
    bank: at.bank,
    blueprint: strategy.what,
    // A4: a strategy takes a view wherever it finds one, and a macro one says so by stating no
    // currency band at all. A single-currency hedge fund would be a hedge fund with a term its
    // investors did not agree to.
    ownCurrencyOnly: false,
    // A4, C1: THE WIDE MANDATE. It is what makes this the speculative side of every contract book.
    mayWrite: 'anything',
    // B1: a permission, and nothing here supplies it. A prime broker does (13.3).
    leverage: true,
    performanceFee: between(rng, STRATEGY_SPREAD.performance),
    // B1, B5: what it MEANS to run at. What it gets is its broker's decision, and the difference
    // between the two is what B3 is about.
    targetLeverage: between(rng, STRATEGY_SPREAD.leverage),
    // D5.a: a notice period, which is a real contractual term with real consequences for who gets
    // out — and the reason a shock reaches this vehicle later than it reaches a money fund.
    liquidity: { how: 'semiLiquid', everyPeriods: STRATEGY_WINDOW },
    // A1: its investors are institutions and the wealthiest households, never the public.
    offeredPublicly: false,
    buffer: between(rng, STRATEGY_SPREAD.buffer),
    fee: between(rng, STRATEGY_SPREAD.fee),
    requiredYield: between(rng, CREDIT_SPREAD),
    why: strategy.why,
  }));
}

/**
 * §29 A1-A5 (item 13.5): THE PRIVATE-EQUITY HOUSE, and it is the same object again.
 *
 * *"Everything is a fund. An HF runs funds, same as PE"* (the owner). So §29 is not a sector either:
 * it is a manager whose pools are CLOSED-END over UNLISTED equity, and the four terms that make one
 * are the same four slots every other vehicle fills in.
 *
 *  - `liquidity: 'closed'` — A2: capital is COMMITTED and called, and nobody can demand money back.
 *    It is the one vehicle in this world that can never be a forced seller, in either direction:
 *    nothing can be redeemed out of it and nothing can be subscribed into it.
 *  - a blueprint of UNLISTED equity — `listed: false`, which is the read that separates a private
 *    company from a public one (item 10e). **This world has no unlisted equity yet**: every share
 *    here trades, so what a fund with this mandate can buy is nothing, and it holds the money it
 *    called until §29 B gives it a company to buy (13.5b). That is stated rather than fixed by
 *    widening the mandate, because a closed-end fund over LISTED equity is not private equity.
 *  - a PERFORMANCE FEE, which for this vehicle is carry (A3);
 *  - and it is not levered. B2.a: the debt in a buyout is the TARGET's liability, *"which is why a
 *    failed buyout kills the firm and not the fund"* — so the fund itself borrows nothing, and
 *    `leverage: false` is what says so.
 */
export const PRIVATE_EQUITY_SPREAD: Readonly<Record<'fee' | 'carry' | 'committed', Spread>> = {
  fee: {
    low: 0.015,
    high: 0.025,
    why: '\u00a729 A3: what a private-equity manager charges on committed capital per annum. It is charged on what was PROMISED rather than on what is invested, which is why a fund that has not deployed still costs its investors something \u2014 and why the pressure to deploy is real.',
  },
  carry: {
    low: 0.15,
    high: 0.25,
    why: '\u00a729 A3, \u00a728 A3: the carry \u2014 the share of a gain the manager takes over the highest value a share has been worth at a charge. The same asymmetric fee a strategy house charges and for the same reason: it is paid on the way up and never refunded on the way down, so a manager that has lost money earns nothing until it is back above where it last charged.',
  },
  committed: {
    low: 0.2,
    high: 0.6,
    why: '\u00a729 A1, Seed A2, Seed A3 (14.1): what one institution promised the fund this world opens with, AS A SHARE OF THE MONEY IT HAS TO PUT TO WORK when the promise is opened. It is an OPENING CONDITION and not a decision \u2014 a closed-end fund was raised before it existed, which is what a vintage is \u2014 and the SPREAD is what makes two investors two different sizes of obligation when the call comes, which is what decides which of them can meet one. A share and not an amount, because it was an amount (forty to a hundred and forty million, drawn blind) promised by an institution that opened holding nothing, so every insurer in every world defaulted on the first call in period 1 and went to its estate: a seeded promise nobody could keep is a seeded default (Seed E1).',
  },
};

/**
 * \u00a729 A1, A2, Seed A3 (item 13.5): the house, its fund, and the commitments it was raised on.
 *
 * The investors are named by the SEED, because they are parties another module creates and this one
 * may not name them (`no-cross-module-import`). What they promised is drawn, so two of them are two
 * different obligations \u2014 and when the call comes, which of them can meet it is a fact about
 * their own balance sheets and not about this table (A2.b).
 */
export function drawPrivateEquity(
  banks: readonly { readonly bank: string; readonly size: number }[],
  investors: readonly string[],
  seed: string,
): readonly FundDecl[] {
  const at = banks.find((b) => b.size >= SPONSOR_SIZE);
  if (at === undefined || investors.length === 0) return [];
  const rng = prng(seed, 'privateEquity');
  const commitments: Record<string, number> = {};
  for (const who of [...investors].sort()) {
    commitments[who] = between(rng, PRIVATE_EQUITY_SPREAD.committed);
  }
  return [
    {
      fund: 'fund.buyout',
      manager: 'manager.buyout',
      managerName: 'Cornerstone Capital',
      bank: at.bank,
      // A5: unlisted equity — the class a share is, and the one read that separates a private
      // company from a public one. Nothing in this world answers it yet, which is 13.5b's.
      blueprint: { classes: ['residual'], currencies: [], listed: false },
      ownCurrencyOnly: true,
      mayWrite: [],
      // B2.a: the debt in a buyout is the TARGET's, which is why a failed buyout kills the firm and
      // not the fund. The fund itself borrows nothing.
      leverage: false,
      performanceFee: between(rng, PRIVATE_EQUITY_SPREAD.carry),
      targetLeverage: 1,
      // A2: committed and called. Nobody gets in at the door and nobody gets out.
      liquidity: { how: 'closed' },
      offeredPublicly: false,
      // A2: it holds no buffer against redemptions because there are none to hold one against.
      buffer: 0,
      fee: between(rng, PRIVATE_EQUITY_SPREAD.fee),
      requiredYield: between(rng, CREDIT_SPREAD),
      commitments,
      why: '\u00a729 A1-A5: a closed-end fund over unlisted equity, raised on commitments from named institutions and drawn when it calls. What makes it private equity is four terms of its mandate and nothing else \u2014 there is no private-equity party kind in this world.',
    },
  ];
}

/**
 * F3, Labour A2, item 10e.4: THE HOUSES THAT RUN THE POOLS — a manager is a business, and what it
 * is drawn with is what makes two of them different businesses rather than one with two names.
 */
export interface ManagerDecl {
  readonly manager: string;
  readonly name: string;
  /**
   * F3, D2: WHAT IT GIVES UP TO BE CHOSEN. An entrant copying somebody's product has exactly one
   * lever — price — and this is how hard it pulls it: the share it comes in under the cheapest
   * fee anybody already charges for that product. It is a PREFERENCE (Law 2) and not an outcome:
   * how much margin a house will sacrifice for scale is a fact about the house.
   *
   * It is what makes fees FALL where several managers run the same blueprint, and what stops them
   * falling is not a floor — it is that the next entrant's fee would no longer cover what a pool
   * costs it in people, so it does not open one (Law 6: the refusal is the mechanism).
   */
  readonly undercut: number;
  /**
   * F3: HOW LONG IT GIVES A NEW PRODUCT. A pool opened this week has been offered to nobody yet —
   * the strike publishes it, a saver reads that and decides the week after — so a manager that
   * judged it immediately would close every fund it ever opened.
   *
   * Drawn, because two managers with the same patience are one manager with two names (Ratings
   * A4.b, and it is the reason this world's assessors are deliberately unalike).
   */
  readonly patience: number;
}

/**
 * Labour A2, A3: WHAT RUNNING ONE POOL TAKES, in hours of the `analysis` trade per period.
 *
 * TECHNOLOGY (Law 2): somebody forms the views, somebody deals, somebody answers for the pool to
 * its holders, and that is about as much work for a small pool as for a large one — which is the
 * whole economics of this industry, because the FEE is on the assets. Sixty hours is under two
 * people at the 35-hour week this world's people work (`LABOUR_NUMBERS.hoursPerMember`).
 */
export const MANAGER_NUMBERS = { hoursPerPool: 60 } as const;

export const MANAGER_SPREAD: Readonly<Record<'undercut' | 'patience', Spread>> = {
  undercut: {
    low: 0.05,
    high: 0.3,
    why: 'Fund Shares F3, D2: the share a manager comes in under the cheapest fee anybody charges for the product it is copying. Price is the only lever an entrant has, and how hard a house pulls it is its own preference \u2014 which is what makes two managers two bidders for the same saver rather than one number repeated.',
  },
  patience: {
    low: 4,
    high: 26,
    why: 'Fund Shares F3: periods a manager gives a pool it opened before it judges whether the fee covers what running it costs. A month at the short end and half a year at the long, and the difference between two houses is the difference between one that pulls a product that has not caught on and one that gives it a year \u2014 which decides which of them is still running it when it does.',
  },
};

/**
 * F3, Seed A3: the houses this world opens with, one per fund it opens — read off the pools, so a
 * manager exists because something names it and never the other way round (Law 4).
 */
export function drawManagers(rows: readonly FundDecl[], seed: string): readonly ManagerDecl[] {
  const rng = prng(seed, 'managers');
  const out: ManagerDecl[] = [];
  const seen = new Set<string>();
  for (const r of [...rows].sort((a, b) => (a.manager < b.manager ? -1 : 1))) {
    if (seen.has(r.manager)) continue;
    seen.add(r.manager);
    out.push({
      manager: r.manager,
      name: r.managerName,
      undercut: between(rng, MANAGER_SPREAD.undercut),
      patience: Math.round(between(rng, MANAGER_SPREAD.patience)),
    });
  }
  return out;
}

export const FUND_SPREAD: Readonly<Record<'buffer' | 'requiredYield' | 'fee', Spread>> = {
  buffer: {
    low: 0.05,
    high: 0.2,
    why: "Fund Shares C2.a: the share of its net assets a fund keeps in cash, so an ordinary redemption needs no sale. Its own caution, and it is the whole of what decides which fund becomes a forced seller first when the redemptions come (XI-2 door 2) — two funds with the same buffer are one fund and would go through the door together.",
  },
  requiredYield: {
    low: 0.002,
    high: 0.009,
    why: 'Fund Shares D2, D2.a: what its investors require of it over what a deposit pays them, per annum, which is what it will pay for paper. It exists to be a substitute for a deposit, so what it demands of a bill is what a saver demands of it — and the spread between two funds is what makes a bill auction have two bidders rather than one.',
  },
  fee: {
    low: 0.001,
    high: 0.004,
    why: 'Fund Shares B3, F3: what the manager charges, per annum on net assets. It is the manager\'s income and the fund\'s cost, and a dearer manager has to earn it back or its investors leave — which is a flow this world has and not a label.',
  },
};

/**
 * The money funds of a world, drawn from the banks that sponsor them. A manager is a SEPARATE party
 * from the fund (F3): the fee is its income and the fund's cost, and neither is the other's.
 */
export function drawFunds(
  banks: readonly { readonly bank: string; readonly size: number }[],
  seed: string,
): readonly FundDecl[] {
  const rng = prng(seed, 'funds');
  const out: FundDecl[] = [];
  /**
   * F3, item 10e.4: ONE HOUSE PER BANK, RUNNING EVERYTHING IT SPONSORS.
   *
   * Every pool used to be given a manager of its own — `manager.x`, `manager.credit.x`,
   * `manager.physical.x` — and the comment that did it argued *"two funds at one bank are not one
   * business: a shared name would be two funds' fees arriving in one account nobody could take
   * apart (Law 4)"*. THAT WAS WRONG, and it is worth saying why rather than quietly changing it.
   * Two funds' fees arriving in one account is what an asset manager IS; what takes them apart is
   * the `fund.fee` event, which names the pool and the manager on every payment, and an event is a
   * writer of a fact in the way a party id is not.
   *
   * What the old shape actually cost was everything this item is about: a manager with one pool has
   * no book of business, so it cannot spread the cost of its people, cannot lose one product and
   * keep another, and cannot be bigger or smaller than a rival. An industry of one-fund firms has
   * no scale in it, and scale is most of what asset management is.
   */
  const houseOf = (bank: string): string => `manager.${bank}`;
  // Law 4, Law 9: one writer of what the house is CALLED. Two rows that named one manager two
  // things would give the party whichever name the seed reached first, silently.
  const houseName = (bank: string): string =>
    `North Asset Management ${banks.findIndex((x) => x.bank === bank) + 1}`;
  for (const b of banks) {
    if (b.size < SPONSOR_SIZE) continue;
    out.push({
      fund: `fund.money.${b.bank}`,
      mayWrite: [],
      leverage: false,
      performanceFee: 0,
      targetLeverage: 1,
      manager: houseOf(b.bank),
      managerName: houseName(b.bank),
      bank: b.bank,
      /**
       * D1, Short-Term Debt C1, C2: SHORT GOVERNMENT PAPER AND HIGH-GRADE COMMERCIAL PAPER, which
       * is what a money fund holds and what makes it the thing a saver holds instead of a deposit.
       *
       * It was bills and bills only, and that was the whole demand side of §9 missing: a firm could
       * bring paper and the only buyers in the world were bank liquidity books. A money fund IS the
       * cash investor C1 names first, and its appetite is why commercial paper is a market at all.
       *
       * IT IS ALSO WHAT MAKES B3.b's RUN REACH ANYBODY. A fund that holds a firm's paper and meets
       * a redemption it cannot cover out of its buffer sells into that market at whatever it gives
       * (XI-2 door 2) — so an issuer that cannot roll and a saver who wants their money back are
       * connected, which is the transmission the clause is about and which bills alone cannot carry.
       *
       * Item 10e: AND IT IS A BAND NOW, NOT A LIST. It was `['sovereign.bill', 'commercial.paper']`
       * — two kind ids, which said "the two short things I know the name of" and would have gone
       * stale the next time anybody issued a third. What the clause actually says is SHORT,
       * HIGH-QUALITY PAPER, and that is what the band says: a dated promise, under a year, from a
       * name the assessors are content with. A bill and a piece of commercial paper both answer it
       * without this file having to know either exists.
       */
      blueprint: {
        classes: ['government', 'corporate'],
        currencies: [],
        duration: { to: 1 },
        worstGrade: 'a',
      },
      ownCurrencyOnly: true,
      // D2: in and out at NAV whenever a saver wants, which is what a deposit substitute IS — and
      // what makes this the vehicle XI-2 door 2 runs through.
      liquidity: { how: 'liquid' },
      // Item 10e.6: a deposit substitute that asked anything of a saver would not be one. This is
      // the other rung retail reaches, and it is why a money fund is where a household's cushion
      // lives (D2) rather than an investment it has to qualify for.
      offeredPublicly: true,
      buffer: between(rng, FUND_SPREAD.buffer),
      fee: between(rng, FUND_SPREAD.fee),
      requiredYield: between(rng, FUND_SPREAD.requiredYield),
      why: 'Fund Shares D1, D2: a fund of short government paper, which is what a saver holds instead of a deposit. It is the vehicle XI-2 door 2 runs through: a redemption it cannot meet out of its buffer is a sale into the bill market at whatever that market gives.',
    });
  }
  /**
   * A4, C1.a, Corporate Credit E5: THE CREDIT FUNDS — the buy side of every corporate market here.
   *
   * One per bank big enough to sponsor one, drawn like the money funds beside them and differing in
   * the two things that make it a different business: WHAT IT MAY HOLD (a company's paper rather
   * than a state's) and HOW LONG (years, because that is how long a company borrows for). Its
   * buffer, its fee and what its investors require are its own draws, so two credit funds are two
   * bidders and not one repeated — which is what a book needs to have a shape at all.
   */
  for (const b of banks) {
    if (b.size < WIDE_SPONSOR_SIZE) continue;
    out.push({
      fund: `fund.credit.${b.bank}`,
      mayWrite: [],
      leverage: false,
      performanceFee: 0,
      targetLeverage: 1,
      // F3: the SAME HOUSE that runs the money fund at this bank. A credit fund is a second product
      // on the same shelf — different mandate, same people, same fee income — which is what lets a
      // manager carry a small product on a large one and what makes losing one survivable.
      manager: houseOf(b.bank),
      managerName: houseName(b.bank),
      bank: b.bank,
      /**
       * A4: A COMPANY'S PAPER, in the three shapes this world issues it in — a bond (item 10), its
       * short paper (10b), and a layer of a pool of claims on companies (10c). They are one asset
       * class to a manager: all three are somebody's promise to pay, priced off what the manager
       * requires of that name, and a mandate that took the bond and refused the note would be
       * drawing a line no credit investor draws.
       *
       * What is NOT here is as deliberate: no share, no bill, no grain. This fund is not a balanced
       * fund and does not become one by holding whatever is cheap.
       */
      blueprint: { classes: ['corporate', 'structured'], currencies: [], duration: { from: 1, to: 30 } },
      ownCurrencyOnly: true,
      // A fund of company paper is open-ended: its investors may have their money back at NAV, and
      // meeting that out of a market for credit is a real sale at whatever that market gives.
      liquidity: { how: 'liquid' },
      // Item 10e.6: *"rich retail able to access funds"* (the owner). An entrant clears the
      // accredited line or it does not get in — which is what makes that line, and the parliament
      // that sets it, reach this sector at all.
      offeredPublicly: false,

      buffer: between(rng, FUND_SPREAD.buffer),
      fee: between(rng, FUND_SPREAD.fee),
      requiredYield: between(rng, CREDIT_SPREAD),
      why: 'Fund Shares A3, A4, C1.a: a long-only manager of somebody else\u2019s savings whose mandate is a company\u2019s credit. It is the demand side of \u00a77, \u00a79 and XI-11 \u2014 without it a corporate issue is brought to a book whose only bidders are bank dealing desks, which are the INTERMEDIARY and not the holder. C2.b makes it the second forced seller this world has: a redemption it cannot meet out of its buffer is a sale of a company\u2019s paper into whatever that market gives, which is how a saver wanting their money back reaches an issuer that cannot roll.',
    });
  }
  // Commodities Spot C3 (13c): ONE FUND THAT HOLDS THE THING ITSELF, sponsored by the largest bank
  // that sponsors anything. It is the party the storage market was missing: every producer opens
  // with room for exactly what it holds, so nobody is short and nobody has spare, and an investor
  // wanting to hold what it did not make is short of room by construction. Its reason is its own
  // outlook against the carry (B4), so what it will pay is a number nobody wrote down.
  const sponsor = banks.find((b) => b.size >= SPONSOR_SIZE);
  if (sponsor !== undefined) {
    out.push({
      fund: `fund.physical.${sponsor.bank}`,
      mayWrite: [],
      leverage: false,
      performanceFee: 0,
      targetLeverage: 1,
      // F3, item 10e.4: the same house again, and its third product.
      manager: houseOf(sponsor.bank),
      managerName: houseName(sponsor.bank),
      bank: sponsor.bank,
      // A4: a mandate of PHYSICAL things and nothing else. What makes it a commodity fund is that
      // every asset it may hold is one NOBODY ISSUED — which the classification reads off the
      // absence of a promise, so it is structural rather than a flag or a list of goods.
      blueprint: { classes: ['thing'], currencies: [] },
      ownCurrencyOnly: true,
      liquidity: { how: 'liquid' },
      // Item 10e.6: a fund, and not a deposit substitute — so it asks the same of an entrant that
      // every other fund does.
      offeredPublicly: false,
      buffer: between(rng, FUND_SPREAD.buffer),
      fee: between(rng, FUND_SPREAD.fee),
      requiredYield: between(rng, FUND_SPREAD.requiredYield),
      why: 'Commodities Spot C3, B4: an investor that buys to hold the thing itself, pays for the room it waits in, and is on the other side of every producer deciding whether to sell now. It is what makes a stock a market rather than an accident of who made what.',
    });
  }
  return out;
}
