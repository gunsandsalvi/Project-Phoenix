/**
 * The funds this world has, and what each one may hold.
 *
 * @spec Fund Shares A1 Fund Shares A4 Fund Shares D1 Fund Shares E1 Fund Shares E2 Fund Shares E3 Fund Shares F3 Equity C2.c Seed A3 Seed B1 Law 2 Law 15
 *
 * Data only (Law 15). A MANDATE is what a fund may hold and how long it may hold it for, and it is
 * a real constraint on what the fund buys rather than a label on it (A4): the module never posts an
 * order for something outside it, which is why a flow into a fund becomes a purchase of exactly
 * what the mandate allows and nothing else. That is what makes a fund a transmission channel.
 */
import { paramId, type ParamId } from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import type { Blueprint } from '../../registry/blueprint.js';
import type { Liquidity } from './index.js';
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
export function nameOf(d: Pick<FundDecl, 'blueprint' | 'liquidity' | 'tracks' | 'bank'>): string {
  if (d.tracks !== undefined) return `${d.bank} ${d.tracks} tracker`;
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
  return `${d.bank} ${of} Fund`;
}

export const fundParam = (fund: string, what: string): ParamId => paramId(`fund.${what}.${fund}`);

export const FUND_PARAMS = {
  openingShare: paramId('fund.openingSharePrice'),
} as const;

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
): readonly FundDecl[] {
  if (lines.length === 0 || tracks.length === 0) return [];
  const rng = prng(seed, 'etf');
  const basket: Record<string, number> = {};
  for (const line of lines) basket[line] = 1;
  const manager = 'manager.etf.us';
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
    fund: n === 0 ? 'etf.us' : `etf.${index}`,
    manager,
    managerName: 'American Index Managers',
    bank: home ?? manager,
    /**
     * A4: what it MAY hold, which is not the same as what the index says it MUST. Listed equity —
     * the class a share is, and the one read that separates a public company from a private one.
     * The index picks the lines inside that; the mandate is what bounds it.
     */
    blueprint: { classes: ['residual'], currencies: [], listed: true },
    ownCurrencyOnly: true,
    // E1, G1.a: its shares TRADE and its investors come and go IN KIND against the basket, which is
    // why it is not a forced seller and why some other vehicle has to carry that.
    liquidity: { how: 'listed' },
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
      seeded: n === 0,
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
  for (const b of banks) {
    if (b.size < SPONSOR_SIZE) continue;
    const at = out.length + 1;
    out.push({
      fund: `fund.money.${b.bank}`,
      manager: `manager.${b.bank}`,
      managerName: `North Asset Management ${at}`,
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
      manager: `manager.credit.${b.bank}`,
      managerName: `North Credit Management ${out.length + 1}`,
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
      // F3: its own manager, and a separate party. Two funds at one bank are not one
      // business: the fee is this manager's income and the fund's cost, and a shared name would
      // be two funds' fees arriving in one account nobody could take apart (Law 4).
      manager: `manager.physical.${sponsor.bank}`,
      managerName: 'North Real Assets',
      bank: sponsor.bank,
      // A4: a mandate of PHYSICAL things and nothing else. What makes it a commodity fund is that
      // every asset it may hold is one NOBODY ISSUED — which the classification reads off the
      // absence of a promise, so it is structural rather than a flag or a list of goods.
      blueprint: { classes: ['thing'], currencies: [] },
      ownCurrencyOnly: true,
      liquidity: { how: 'liquid' },
      buffer: between(rng, FUND_SPREAD.buffer),
      fee: between(rng, FUND_SPREAD.fee),
      requiredYield: between(rng, FUND_SPREAD.requiredYield),
      why: 'Commodities Spot C3, B4: an investor that buys to hold the thing itself, pays for the room it waits in, and is on the other side of every producer deciding whether to sell now. It is what makes a stock a market rather than an accident of who made what.',
    });
  }
  return out;
}
