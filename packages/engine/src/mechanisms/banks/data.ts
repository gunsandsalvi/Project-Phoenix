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
import { Missing } from '../../core/errors.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { between, betweenWhole, drawSize, type Spread, type Tail } from '../../rng/spread.js';

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
   * Dealer Desks D1, F1, XI-4: the most of its OWN CAPITAL it will have standing behind EACH OF ITS
   * LINES OF BUSINESS, by the line's own name — a dealing book being what it is holding away from
   * where its own treasury wants it. Every capacity is finite and enumerable, and a book full of one
   * thing stops bidding for everything, which is how one line's trouble reaches another. A dealer
   * without a limit is a synthetic counterparty wearing a dealer's name (Clearing B3.a), and this is
   * the number that makes it one.
   *
   * AND EVERY LINE HAS ONE, which is what makes the treasury's allocation a decision. A line whose
   * ask is "whatever is left" is not asking for anything: the first line served takes the whole
   * headroom and the order that was meant to choose between them chooses nothing — measured as a
   * dealing line allotted zero in every bank in every period of the world (`13b-7`).
   */
  readonly appetite: Readonly<Record<string, number>>;
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

/**
 * The lines of business a bank's own room is allotted between. The liquidity book is not one: it is
 * what the treasury must hold, not a line competing for room to grow.
 *
 * Law 15: they are DATA — a name, a spread to draw each one's appetite from (`BANK_SPREAD.appetite`)
 * and a parameter per bank per line. Adding a line of business is a name here and a spread there,
 * and no mechanism anywhere learns what the lines are called.
 */
export const LENDING = 'lending';
export const DEALING = 'dealing';

/**
 * A LINE OF BUSINESS's own numbers, under the bank, because the line IS the bank (A1, F2).
 *
 * Law 15: the line names its own parameter, so a mechanism asks a row for its number instead of
 * knowing which rows there are. Adding a line of business is then a row and a draw, and no loop
 * anywhere learns its name.
 */
export const lineParam = (bank: string, line: string, what: string): ParamId =>
  paramId(`bank.${line}.${what}.${bank}`);

/** D2: the capital a trading position consumes, as a rule somebody wrote rather than a fact. */
export const TRADING_BOOK_RISK_WEIGHT: ParamId = paramId('regulation.riskWeight.tradingBook');
export const TRADING_BOOK_CAPITAL_RATIO: ParamId = paramId('regulation.capitalRatio');

/** The bank a named party is, if this world gave it one (Law 15: the data says). */
export function bankOf(rows: readonly BankDecl[], bank: string): BankDecl | undefined {
  return rows.find((r) => r.bank === bank);
}

/**
 * Seed B1, B4, XI-16 B1.a: WHAT A BANK IS LIKE IS DRAWN, ONCE, AT THE SEED — the same way a
 * household's memory is, and for the same reason: a sector whose members are all alike moves as one
 * and never produces a market.
 *
 * IT USED TO BE A TABLE OF THREE, and the table was the defect. Eleven numbers a bank, written out
 * one bank at a time, is not a distribution — it is thirty-three numbers claiming to be one, and a
 * world of two hundred banks would have been two thousand two hundred. Law 2 asks for the FEWEST
 * primitives: what a bank varies over is stated once, as a range with a reason, and how many banks
 * there are is a count. Change the count and the world has that many, each with its own
 * disposition, and no number anywhere is restated.
 *
 * Every draw is still a PREFERENCE, and every drawn value is declared in the parameter register
 * under that bank's own name (`banks/index.ts`), so the register prints what each one is like and
 * a reader can see why it did what it did. What is stated here is the WIDTH, which is what a
 * dispersion is; the values are the world's.
 */
export interface BankDispersion {
  readonly size: Tail;
  readonly memoryPeriods: Spread;
  readonly returnOnCapital: Spread;
  readonly capitalBuffer: Spread;
  readonly liquidityCushion: Spread;
  readonly limitPerBorrower: Spread;
  readonly depositMargin: Spread;
  /** One spread per line of business, by name: what each line's appetite is drawn from. */
  readonly appetite: Readonly<Record<string, Spread>>;
  readonly concentration: Spread;
}

export const BANK_SPREAD: BankDispersion = {
  size: {
    concentration: 1.1,
    why: 'Seed B4: how big it is beside the others. A banking system is not a few banks of similar size with a spread on them — it is a handful that settle most of the payments and a long tail that settle almost none, and no uniform draw produces that at any width. Just above the heaviest tail there is, so the largest bank in a world of thirty holds a couple of orders of magnitude more than the smallest. It is a SHAPE: what a bank is worth is an outcome of who banks with it, what it lent and what it lost, and this stands in for the history that produced the sector until a world has run one (Part XII, worklist 16).',
  },
  memoryPeriods: {
    low: 8,
    high: 52,
    why: 'Banks Lending C1.b, Banks Funding C2.a: how far back it looks — at a borrower it has judged and at its own worst weeks. A quarter to a whole year: one bank prices an old failure into today loan and another has forgotten it, which is what makes two lenders quote a name differently.',
  },
  returnOnCapital: {
    low: 0.07,
    high: 0.14,
    why: 'Banks Lending C1.c: what it needs to earn on the capital a loan consumes, per annum. Its own, and the whole of why a borrower shopping two banks gets two prices (C2) — a borrower with one price to take has not shopped.',
  },
  capitalBuffer: {
    low: 0.005,
    high: 0.04,
    why: 'Banks Capital B2, Banks Lending B2.a: how far above the requirement it insists on running. Its own caution, and it is why two banks stop lending at different moments rather than all at once.',
  },
  liquidityCushion: {
    low: 0.05,
    high: 0.4,
    why: 'Banks Funding C2, Money Market A2.a: what it holds liquid above what the rule asks, as a share of the money that could leave. A different caution from the capital one: a bank can be bold about capital and timid about liquidity, and the two get it into different sorts of trouble.',
  },
  limitPerBorrower: {
    low: 0.15,
    high: 0.4,
    why: 'Banks Lending F3: the most it will have out to one name, as a share of its own capital. A limit that binds, and binds at a different size for each of them — which is what makes a large borrower shop past the first bank that fills up.',
  },
  depositMargin: {
    low: 0.004,
    high: 0.008,
    why: 'Banks Funding B1.a, B3: what it keeps for itself out of what money is worth to it. Two banks that keep the same margin are one bank; the gap between them is what a depositor is deciding about, and it has to be wider than what a wholesale account will sit still for and narrower than what an operational one will move for.',
  },
  appetite: {
    [DEALING]: {
      low: 0.15,
      high: 0.8,
      why: 'Dealer Desks D1, F1: the most of its own capital it will have standing behind its dealing book. Every capacity is finite and enumerable, and a book full of one thing stops bidding for everything — which is how one line trouble reaches another. A dealer without a limit is a synthetic counterparty wearing a dealer name.',
    },
    [LENDING]: {
      low: 0.4,
      high: 0.95,
      why: 'Banks Lending A1, XI-4: the most of its own capital it will have standing behind its loan book. Higher than the dealing appetite because lending is what a bank IS and dealing is a line it chose to run, and wide because a conservative bank and an aggressive one differ here more than anywhere else. It has to be a number for the same reason the dealing one does: a line whose ask is whatever is left is not competing for anything, and the treasury that serves it first hands it the lot.',
    },
  },
  concentration: {
    low: 0.25,
    high: 0.5,
    why: 'Dealer Desks D1: the most of that book it will have in ONE line. A share rather than a count of pieces, because a count means something different in a line quoted in shares and one quoted in par, and would have to be restated every time a price moved.',
  },
};

/**
 * Seed B1: HOW MANY BANKS THIS WORLD HAS. It is a count and nothing else follows from changing it
 * but the world having that many — which is what makes it measurable the way the cell grain is
 * (XI-15, `test/resolution/banks.test.ts`). THIRTY, which is what a country's banking system has:
 * a handful that settle most of the payments and a tail of small ones. With two, every depositor
 * that answers a rate is the whole of one side of the deposit market, every interbank session is
 * one name facing one name, and a bank in trouble has exactly one place to go — so the count was
 * silently load-bearing on every mechanism that needs somebody else to be there, and the world was
 * a test of the mechanisms rather than a run of them.
 */
export const BANK_COUNT = 30;

/**
 * The banks of a world, drawn from the spread above. Deterministic in the world's seed value and in
 * nothing else (Seed A5, Audit D3): the same seed gives the same banks, and a world asked for two
 * hundred of them gets two hundred without a line of data being written.
 */
export function drawBanks(count: number, seed: string): readonly BankDecl[] {
  const rng = prng(seed, 'banks');
  const out: BankDecl[] = [];
  for (let n = 0; n < count; n += 1) {
    // Law 15: one draw per declared line, in the order the spreads declare them. A world with a
    // third line of business draws three, here, and nothing else is touched.
    const appetite: Record<string, number> = {};
    for (const [line, spread] of Object.entries(BANK_SPREAD.appetite)) {
      appetite[line] = between(rng, spread);
    }
    out.push({
      bank: bankName(n),
      size: drawSize(rng, BANK_SPREAD.size),
      // Law 8: a memory is a count of periods, so it is drawn as one.
      memoryPeriods: betweenWhole(rng, BANK_SPREAD.memoryPeriods),
      returnOnCapital: between(rng, BANK_SPREAD.returnOnCapital),
      capitalBuffer: between(rng, BANK_SPREAD.capitalBuffer),
      liquidityCushion: between(rng, BANK_SPREAD.liquidityCushion),
      limitPerBorrower: between(rng, BANK_SPREAD.limitPerBorrower),
      depositMargin: between(rng, BANK_SPREAD.depositMargin),
      bufferMemory: betweenWhole(rng, BANK_SPREAD.memoryPeriods),
      // Dealer Desks A1, C5: WHICH LINES IT QUOTES IS DERIVED FROM WHAT IT WILL RISK, not drawn
      // separately. A bank that will not put much capital behind a book does not run one: it makes
      // a market in the paper its own treasury holds for liquidity and in nothing else, because a
      // share is a claim on a business it has no view of and a bank that will not take a view does
      // not quote one. So `makes` is a consequence of a preference it already has (Law 2).
      makes:
        appetiteOf(appetite, DEALING) > midpoint(spreadOf(BANK_SPREAD.appetite, DEALING))
          ? ['equity.share', 'fund.share', 'sovereign.bill', 'sovereign.bond']
          : ['sovereign.bill', 'sovereign.bond'],
      appetite,
      concentration: between(rng, BANK_SPREAD.concentration),
      why: `Seed B4: drawn at the seed from the stated spread, like every other bank in this world. Nothing about it is stated one bank at a time, and what it is like is printed in the parameter register under its own name.`,
    });
  }
  return out;
}

const midpoint = (s: Spread): number => s.low + (s.high - s.low) / 2;

/**
 * What this bank will put behind one of its lines. A line with no appetite is a defect and not a
 * zero (Appendix A: missing is missing) — the draw walks the declared lines, so a name that is not
 * in it is a name nothing declared.
 */
export function appetiteOf(appetite: Readonly<Record<string, number>>, line: string): number {
  const share = appetite[line];
  if (share === undefined) {
    throw new Missing('Dealer Desks D1', `no appetite was drawn for the ${line} line`, { line });
  }
  return share;
}

/** The same read over the spreads, for the draw itself. */
function spreadOf(spreads: Readonly<Record<string, Spread>>, line: string): Spread {
  const s = spreads[line];
  if (s === undefined) {
    throw new Missing('Dealer Desks D1', `no appetite spread is declared for the ${line} line`, {
      line,
    });
  }
  return s;
}

/**
 * Law 9: what a bank is called. `bank.a` … `bank.z`, then `bank.aa` — a name a reader can say, for
 * as many of them as the world asks for.
 */
export function bankName(n: number): string {
  let at = n;
  let out = '';
  do {
    out = String.fromCharCode(97 + (at % 26)) + out;
    at = Math.floor(at / 26) - 1;
  } while (at >= 0);
  return `bank.${out}`;
}

