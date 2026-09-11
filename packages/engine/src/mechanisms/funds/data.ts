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
import { prng } from '../../rng/prng.js';
import { between, type Spread } from '../../rng/spread.js';

export interface FundDecl {
  /** A1: the named party. It has an account and a register of holdings like anything else. */
  readonly fund: string;
  readonly name: string;
  /** F3: the manager is a SEPARATE party. The fee is its income and the fund's cost. */
  readonly manager: string;
  readonly managerName: string;
  readonly bank: string;
  /** A4: the instrument kinds it may hold. Anything else it may not buy, at any price. */
  readonly eligible: readonly string[];
  /**
   * D1: short paper. The longest a holding may still have to run, in periods — a mandate of short,
   * high-quality paper is what makes a money fund a deposit substitute rather than a bond fund.
   */
  readonly maxTenorPeriods: number;
  /** C2.a: the share of its net assets it keeps in cash, so an ordinary redemption needs no sale. */
  readonly buffer: number;
  /** B3, F3: what the manager charges, per annum on net assets. */
  readonly fee: number;
  /**
   * D2, D2.a: what its investors require of it over what a deposit pays them, per annum. It is what
   * the fund will pay for paper: it exists to be a substitute for a deposit, so what it demands of
   * a bill is what a saver demands of it.
   */
  readonly requiredYield: number;
  readonly why: string;
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
export interface EtfDecl {
  readonly fund: string;
  readonly name: string;
  readonly manager: string;
  readonly managerName: string;
  readonly bank: string;
  /**
   * E3: the creation basket — units of each line one share is a claim on. It is what a creation
   * unit is made of before there is a fund to take a slice of; once there is one, a creation unit
   * is a pro-rata slice of what the fund actually holds, which is what keeps its composition its
   * own rather than a thing the last creator chose.
   */
  readonly basket: Readonly<Record<string, number>>;
  /**
   * Indices C2, C2.a, Equity C2.c: THE INDEX IT TRACKS, by id. A tracker's mandate is not a list of
   * lines somebody typed — it is a rule, and the rule is the index's. So what it should hold is
   * whatever the index says is in it at whatever the index weighs it, and a REBALANCE is the index
   * answering differently: a line listed, a line gone, a weight moved. The fund then has to trade,
   * in the same session, at whatever the book gives it (C2.a) — it is not choosing, which is the
   * whole of what makes a tracker a transmission channel rather than an investor.
   *
   * The id is a plain string because a fund may not import the module that declares the index
   * (`no-cross-module-import`); what index this world's tracker tracks is data about this world.
   */
  readonly tracks: string;
  /**
   * Seed A3: who holds its shares at launch and WHAT SHARE OF IT each of them took. A fund is
   * launched by somebody putting a basket in and taking the shares that came out, and in this world
   * that is its sponsor and the banks whose dealing lines will make its market — an authorised
   * participant with no shares can only ever create, and a gap the other way would have nobody able
   * to close it (E3.a).
   *
   * Shares of the launch, not counts of shares: how many shares that comes to is read off the lines
   * the basket names, which only the seed can see (`ETF_LAUNCH_SHARE`).
   */
  readonly launchedBy: Readonly<Record<string, number>>;
  /**
   * Law 15, Part XIII: the modules whose parties and lines this launch NAMES. It is data about
   * this world's fund rather than about funds, so the module reads its dependencies off it instead
   * of carrying them: a world whose exchange-traded fund holds nothing anybody else registered
   * needs none of them.
   */
  readonly needs: readonly string[];
  /** B3, F3: what the manager charges, per annum on net assets. */
  readonly fee: number;
  readonly why: string;
}

/**
 * Seed B1.a, Fund Shares E3: THE LAUNCH IS DRAWN FROM THE WORLD IT IS LAUNCHED INTO.
 *
 * It used to be a row: a basket naming three share lines and a launch naming three parties and
 * three counts. A world of three listed firms could be written out; one of three hundred cannot,
 * and a fund that held only the three somebody typed would not be an index fund at all — it would
 * be a portfolio with an index fund's name on it (Equity C2.c). What is stated here is what an
 * index fund IS — it holds every line there is, at the weight the line has — and how big a launch
 * has to be to reach a price.
 */

/**
 * Law 2, Law 19, E3.a: HOW BIG THE LAUNCH IS, as a share of the lines it tracks.
 *
 * A launch has to be big enough that a creation or a redemption is a real trade in every line it
 * touches: a fund launched at sixty shares against a float of a hundred thousand is a rounding
 * error with a manager attached, and nothing it did would reach a price.
 *
 * IT USED TO BE A COUNT — twenty thousand shares — and a count is a claim about a world whose scale
 * the seed STATED. 11.5 derives that scale from the hours this world's people offer against the
 * hours its chain needs, and the lines this fund tracks now open at a hundred and eighty million
 * shares apiece: twenty thousand is one ten-thousandth of the smallest of them, so a household cell
 * bidding for the whole fund was bidding a fraction of a cent per member, every session crossed and
 * settled nothing, and the price the file is about was the opening one for ever. The number was the
 * rounding error its own comment warned about.
 *
 * So it is read off the lines instead: the fund holds this share of the SMALLEST line in its basket,
 * which is the one that binds — a share of each is what tracking an index means, and the smallest is
 * as far as a common multiple of all of them reaches.
 */
export const ETF_LAUNCH_SHARE = 0.05;

/**
 * Seed A3: how the launch is divided between the sponsor and the banks whose dealing lines make its
 * market. An authorised participant with no shares can only ever create, and a gap the other way
 * would have nobody able to close it (E3.a), so both sides of the book open holding some.
 */
export const ETF_SPONSOR_SHARE = 0.4;

/** How many of this world's banks are authorised participants in it (Dealer Desks A3, E3). */
export const ETF_PARTICIPANTS = 4;

/**
 * Fund Shares E1-E4: the exchange-traded fund of a world, built from the lines that world listed
 * and the banks it has. Deterministic in the world's own seed value (Seed A5, Audit D3).
 *
 * Equity C2.c: AN INDEX FUND DOES NOT PRICE AT ALL — it holds weight, whatever it costs. One share
 * of it is one share of each listed line, which is the whole of its mandate: it never bids for
 * anything and it never sells anything.
 */
export function drawEtfs(
  lines: readonly string[],
  banks: readonly string[],
  seed: string,
  tracks: string,
): readonly EtfDecl[] {
  if (lines.length === 0) return [];
  const rng = prng(seed, 'etf');
  const basket: Record<string, number> = {};
  for (const line of lines) basket[line] = 1;
  const manager = 'manager.etf.us';
  const launchedBy: Record<string, number> = {};
  launchedBy[manager] = ETF_SPONSOR_SHARE;
  // The participants, drawn without repeating a name: a bank cannot be two of them.
  const pool = [...banks];
  const chosen: string[] = [];
  while (chosen.length < ETF_PARTICIPANTS && pool.length > 0) {
    const at = rng.int(pool.length);
    const taken = pool[at];
    pool.splice(at, 1);
    if (taken !== undefined) chosen.push(taken);
  }
  // E3: what is left for the participants once the sponsor has its own slice, written just above.
  // These are SHARES OF THE LAUNCH and not counts: how many shares that comes to is the seed's,
  // because it is the one that can see how big the lines turned out (`seedEtf`).
  const rest = 1 - ETF_SPONSOR_SHARE;
  for (const bank of [...chosen].sort()) launchedBy[bank] = rest / chosen.length;
  const home = chosen[0];
  return [
    {
      fund: 'etf.us',
      name: 'US Listed Equity Fund',
      manager,
      managerName: 'American Index Managers',
      bank: home ?? manager,
      basket,
      tracks,
      launchedBy,
      needs: ['equity', 'banks'],
      fee: 0.001,
      why: 'Fund Shares E1-E4: the vehicle that has TWO values. Its shares trade, so a session prices them; its book is every listed line in this world, so a read prices them too; and the gap between the two is what somebody has to want to close for it to close at all (E3.a).',
    },
  ];
}

/**
 * Seed B1.a: the money funds of a world. One per bank that is large enough to sponsor one, so a
 * world of thirty banks has a money-fund sector rather than one fund, and a redemption that cannot
 * be met out of a buffer is a sale into a bill market with other sellers already in it (XI-2).
 */
export const MONEY_FUND_SPONSOR_SIZE = 4;

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
    if (b.size < MONEY_FUND_SPONSOR_SIZE) continue;
    const at = out.length + 1;
    out.push({
      fund: `fund.money.${b.bank}`,
      name: `${b.bank} Money Fund`,
      manager: `manager.${b.bank}`,
      managerName: `North Asset Management ${at}`,
      bank: b.bank,
      // D1: bills, and bills only. It is the shortest paper this world issues.
      eligible: ['sovereign.bill'],
      maxTenorPeriods: 52,
      buffer: between(rng, FUND_SPREAD.buffer),
      fee: between(rng, FUND_SPREAD.fee),
      requiredYield: between(rng, FUND_SPREAD.requiredYield),
      why: 'Fund Shares D1, D2: a fund of short government paper, which is what a saver holds instead of a deposit. It is the vehicle XI-2 door 2 runs through: a redemption it cannot meet out of its buffer is a sale into the bill market at whatever that market gives.',
    });
  }
  return out;
}
