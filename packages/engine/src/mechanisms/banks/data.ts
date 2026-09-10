/**
 * What each bank is like: its own numbers, stated one bank at a time, ONE ROW PER BANK.
 *
 * @spec Banks Lending C1 Banks Lending C1.b Banks Lending C1.c Banks Lending F3 Banks Funding B1.a Banks Funding B3 Banks Funding C2 Banks Funding C2.a Dealer Desks A1 Dealer Desks D1 Dealer Desks E3 Money Market A2.a Firm A3 Seed B4 Law 2 Law 4 Law 15
 *
 * Data only (Law 15), and ALL of it, because a bank is one party: what it is like as a lender, as a
 * funder, as a treasury and as a dealer are four sides of one disposition, and holding them in four
 * tables was what let one bank hold four opinions of itself (Law 4). Two banks that price a loan
 * identically are not two lenders, and a borrower with one price to take has not shopped (C2) — so
 * what differs between them is stated here, per bank, exactly as a firm's own productivity is. None
 * of it is a shape: each number is one bank's own preference, and stating a distribution's width
 * instead is what would be.
 *
 * THIS TABLE IS THE BANKS OF THIS WORLD, and there is no second list of them anywhere. HOW MANY
 * there are is what it has rows, because a count held beside it would be two representations of one
 * thing (Law 4) and every reader that walked the table would be reading the wrong one. The module
 * and the seed are both built from whatever rows they are given, so a world with two banks or with
 * four is this world with a different table and no number in it restated — which is what lets the
 * count be MEASURED the way the cell grain is (XI-15, `test/resolution/banks.test.ts`), because it
 * is load-bearing in a way no mechanism states: with two, every depositor that answers a rate is
 * the whole of one side of the deposit market, every interbank session is one name facing one name,
 * and a bank in trouble has exactly one place to go.
 *
 * NOTHING HERE IS AN AMOUNT OF MONEY. Every limit is a share of something the bank itself publishes
 * — its capital, its own book, what could leave it — because a limit stated in money is a limit that
 * has to be restated every time the world changes size, and a number restated to keep a result is a
 * result wearing a preference's name (Law 2).
 */
import { paramId, type ParamId } from '../../core/ids.js';

export interface BankDecl {
  readonly bank: string;
  /**
   * Seed B4: HOW BIG IT IS, relative to the other banks in the world it opens in. It is a weight
   * and not a share, so the same table describes a world of two banks and a world of four without
   * any number in it being restated: what each one holds is its weight over the weights of the
   * banks that exist.
   *
   * A sector of equals never produces a market (B4), and the seed used to state one line at a time
   * how much of it each bank held — eighteen numbers whose spread came to a few per cent, so three
   * banks of nearly the same size held nearly the same book. One number each, dispersed, says the
   * thing the eighteen were trying to say.
   */
  readonly size: number;
  /**
   * C1.b: how far back it looks when it judges a borrower. A bank with a long memory prices a
   * borrower's old failure into today's loan; one with a short memory has forgotten it.
   */
  readonly memoryPeriods: number;
  /** C1.c: what it needs to earn on the capital a loan consumes, per annum. Its own. */
  readonly returnOnCapital: number;
  /** B2.a: how much above the required ratio it insists on running. Its own caution. */
  readonly capitalBuffer: number;
  /**
   * Banks Funding C2, Money Market A2.a: what it holds LIQUID above what the rule asks of it, as a
   * share of the money that could leave. Its own caution again, and a different one: a bank can be
   * bold about capital and timid about liquidity, and the two get it into different sorts of trouble.
   */
  readonly liquidityCushion: number;
  /** F3: the most it will have out to one name, as a share of its own capital. A limit that binds. */
  readonly limitPerBorrower: number;
  /**
   * Dealer Desks C5, E3: the instrument kinds its DEALING line makes a market in. One posted quote
   * belongs in every book it makes, and every bank that makes a line is in that line's session — so
   * two banks that make the same lines face each other through them, which is the interdealer
   * market (E3) without a second venue for it.
   */
  readonly makes: readonly string[];
  /**
   * Dealer Desks D1, F1: the most of its OWN CAPITAL it will have standing behind its dealing book —
   * that book being what it is holding away from where its own treasury wants it, either way. Every
   * capacity is finite and enumerable, and a book full of one thing stops bidding for everything,
   * which is how one line's trouble reaches another. A dealer without a limit is a synthetic
   * counterparty wearing a dealer's name (Clearing B3.a), and this is the number that makes it one.
   */
  readonly capitalAtRisk: number;
  /**
   * Dealer Desks D1: the most of that book it will have in ONE line, as a share of the whole. A
   * share rather than a count of pieces, because a count means something different in a line quoted
   * in shares and a line quoted in par, and would have to be restated every time a price moved.
   */
  readonly concentration: number;
  /**
   * Banks Funding B1.a, B3: what it keeps for itself out of what money is worth to it — its margin
   * on funding, per annum. Its own: two banks that keep the same margin are one bank, and the one
   * that keeps less wins the deposit and earns less on it, which is what a net interest margin IS.
   */
  readonly depositMargin: number;
  /**
   * Banks Funding C2.a, Money Market A2.a: how far back it looks at its own account when it decides
   * what it holds against a bad week. A bank with a long memory holds against the worst week it has
   * seen; a short one has forgotten it. It is the same memory it prices its funding off.
   */
  readonly bufferMemory: number;
  readonly why: string;
}

export const bankParam = (bank: string, what: string): ParamId => paramId(`bank.${what}.${bank}`);

/** The dealing line's own numbers live under the bank, because the line IS the bank (A1, F2). */
export const dealingParam = (bank: string, what: string): ParamId =>
  paramId(`bank.dealing.${what}.${bank}`);

/** D2: the capital a trading position consumes, as a rule somebody wrote rather than a fact. */
export const TRADING_BOOK_RISK_WEIGHT: ParamId = paramId('regulation.riskWeight.tradingBook');
export const TRADING_BOOK_CAPITAL_RATIO: ParamId = paramId('regulation.capitalRatio');

/** The bank a named party is, if this world gave it one (Law 15: the data says). */
export function bankOf(rows: readonly BankDecl[], bank: string): BankDecl | undefined {
  return rows.find((r) => r.bank === bank);
}

export const BANKS: readonly BankDecl[] = [
  {
    bank: 'bank.a',
    size: 4,
    memoryPeriods: 26,
    returnOnCapital: 0.1,
    capitalBuffer: 0.02,
    liquidityCushion: 0.2,
    limitPerBorrower: 0.25,
    depositMargin: 0.006,
    bufferMemory: 26,
    // NOT sovereign paper only, and not equities only: a bank makes a market in every priced line
    // this world has, including the paper its own treasury holds for liquidity — which is why the
    // treasury never has to post (Dealer Desks C2.a: it hands the desk a target and the quote does
    // the rest).
    makes: ['equity.share', 'fund.share', 'sovereign.bill', 'sovereign.bond'],
    capitalAtRisk: 0.5,
    concentration: 0.3,
    why: 'The larger and more cautious of the two: it remembers a borrower for half a year, wants a tenth on its capital, runs two points above what it must, holds a fifth of what could leave beyond what the rule asks — four times the cushion the other one keeps, exactly as it runs four times the capital buffer and will not have more than a quarter of its capital out to one name, and it will not have more than half of it standing behind a trading book. It is the larger dealer in absolute terms because it is the larger bank, and the more timid one as a share of what it owns. It keeps the wider margin on the money it takes in rather than bidding for deposits it does not need: the gap between the two margins is what a depositor decides about, and it is a fifth of a point, which is more than the wholesale money will sit still for and less than the operational money will move for.',
  },
  {
    bank: 'bank.b',
    size: 3,
    memoryPeriods: 8,
    returnOnCapital: 0.14,
    capitalBuffer: 0.005,
    liquidityCushion: 0.05,
    limitPerBorrower: 0.4,
    depositMargin: 0.004,
    bufferMemory: 8,
    makes: ['equity.share', 'fund.share', 'sovereign.bill', 'sovereign.bond'],
    capitalAtRisk: 0.8,
    concentration: 0.5,
    why: 'The keener one: a short memory, a higher return demanded on its capital, a twentieth of a cushion above the liquidity rule and half a point above the capital one and a bigger appetite for a single name and for a trading book: four fifths of its capital will stand behind one, and it spreads that book over fewer lines. It is the smaller book and the harder-skewing one, and it is the book whose stopping is what a thin market looks like. It will win the business the other one turns away and it will wear what comes with it. A short memory of its own outflows means a thinner buffer, and a thin margin on funding means it pays up for deposits and lives on the volume — the same disposition on both sides of its balance sheet.',
  },
  {
    bank: 'bank.c',
    size: 2,
    memoryPeriods: 52,
    returnOnCapital: 0.07,
    capitalBuffer: 0.04,
    liquidityCushion: 0.4,
    limitPerBorrower: 0.15,
    depositMargin: 0.008,
    bufferMemory: 52,
    // It makes a market in the paper it holds for liquidity and in the fund whose shares its own
    // depositors buy, and in NOTHING ELSE. A share is a claim on a business it has no view of, and
    // a bank that will not take a view does not quote one — which is a real disposition and the
    // reason `makes` is data about a bank rather than a list every bank shares (Law 15).
    makes: ['fund.share', 'sovereign.bill', 'sovereign.bond'],
    capitalAtRisk: 0.2,
    concentration: 0.25,
    why: 'The careful one, and the smallest: it remembers a borrower and its own bad weeks for a whole year, asks the least on its capital and runs the widest cushion over both requirements, will not have more than a seventh of its capital out to one name, keeps the widest margin on the money it takes in and puts a fifth of its capital behind a trading book it spreads over more lines than either of the others. It wins nothing on price and it is still there when the other two have filled up — which is what a third bank is for: with two, every depositor that moves is the whole of one side of the market and every session is one name facing one name.',
  },
];
