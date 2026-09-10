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
   * Seed A3: who holds its shares at launch and how many. A fund is launched by somebody putting a
   * basket in and taking the shares that came out, and in this world that is its sponsor and the
   * desks that will make its market — an authorised participant with no shares can only ever
   * create, and a gap the other way would have nobody able to close it (E3.a).
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

export const ETFS: readonly EtfDecl[] = [
  {
    fund: 'etf.north',
    name: 'North Listed Equity Fund',
    manager: 'manager.etf.north',
    managerName: 'North Index Managers',
    bank: 'bank.b',
    // Equity C2.c: an index fund does not price at all — it holds weight, whatever it costs. One
    // share of it is one share of each of the three firms this world listed, and that is the whole
    // of its mandate: it never bids for anything and it never sells anything.
    basket: { 'equity.firm.4': 1, 'equity.firm.5': 1, 'equity.firm.6': 1 },
    // Law 8: whole shares, and enough of them to be a real fund beside the float the desks make a
    // market in — a launch of sixty shares against a line of a hundred and twenty thousand is a
    // rounding error with a manager attached, and nothing it did would reach a price.
    launchedBy: { 'manager.etf.north': 20_000, 'desk.a': 25_000, 'desk.b': 15_000 },
    needs: ['equity', 'dealers'],
    fee: 0.001,
    why: 'Fund Shares E1-E4: the vehicle that has TWO values. Its shares trade, so a session prices them; its book is the three listed firms, so a read prices them too; and the gap between the two is what somebody has to want to close for it to close at all (E3.a).',
  },
];

export const FUNDS: readonly FundDecl[] = [
  {
    fund: 'fund.money.north',
    name: 'North Money Fund',
    manager: 'manager.north',
    managerName: 'North Asset Management',
    bank: 'bank.a',
    // D1: bills, and bills only. It is the shortest paper this world issues.
    eligible: ['sovereign.bill'],
    maxTenorPeriods: 52,
    buffer: 0.1,
    fee: 0.002,
    requiredYield: 0.005,
    why: 'Fund Shares D1, D2: a fund of short government paper, which is what a saver holds instead of a deposit. It is the vehicle XI-2 door 2 runs through: a redemption it cannot meet out of its buffer is a sale into the bill market at whatever that market gives.',
  },
];
