/**
 * The funds this world has, and what each one may hold.
 *
 * @spec Fund Shares A1 Fund Shares A4 Fund Shares D1 Fund Shares F3 Seed B1 Law 2 Law 15
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
