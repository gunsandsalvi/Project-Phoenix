/**
 * What each bank is like as a lender: its own numbers, stated one bank at a time.
 *
 * @spec Banks Lending C1 Banks Lending C1.b Banks Lending C1.c Banks Lending F3 Firm A3 Seed B4 Law 2 Law 15
 *
 * Data only (Law 15). Two banks that price a loan identically are not two lenders, and a borrower
 * with one price to take has not shopped (C2) — so what differs between them is stated here, per
 * bank, exactly as a firm's own productivity is. None of it is a shape: each number is one bank's
 * own preference, and stating a distribution's width instead is what would be.
 */
import { paramId, type ParamId } from '../../core/ids.js';

export interface BankDecl {
  readonly bank: string;
  /**
   * C1.b: how far back it looks when it judges a borrower. A bank with a long memory prices a
   * borrower's old failure into today's loan; one with a short memory has forgotten it.
   */
  readonly memoryPeriods: number;
  /** C1.c: what it needs to earn on the capital a loan consumes, per annum. Its own. */
  readonly returnOnCapital: number;
  /** B2.a: how much above the required ratio it insists on running. Its own caution. */
  readonly capitalBuffer: number;
  /** F3: the most it will have out to one name, as a share of its own capital. A limit that binds. */
  readonly limitPerBorrower: number;
  readonly why: string;
}

export const bankParam = (bank: string, what: string): ParamId => paramId(`bank.${what}.${bank}`);

export const BANKS: readonly BankDecl[] = [
  {
    bank: 'bank.a',
    memoryPeriods: 26,
    returnOnCapital: 0.1,
    capitalBuffer: 0.02,
    limitPerBorrower: 0.25,
    why: 'The larger and more cautious of the two: it remembers a borrower for half a year, wants a tenth on its capital, runs two points above what it must and will not have more than a quarter of its capital out to one name.',
  },
  {
    bank: 'bank.b',
    memoryPeriods: 8,
    returnOnCapital: 0.14,
    capitalBuffer: 0.005,
    limitPerBorrower: 0.4,
    why: 'The keener one: a short memory, a higher return demanded on its capital, almost no buffer above the requirement and a bigger appetite for a single name. It will win the business the other one turns away and it will wear what comes with it.',
  },
];
