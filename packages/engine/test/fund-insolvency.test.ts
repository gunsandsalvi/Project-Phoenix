/**
 * A share cannot be worth less than nothing, and that is what lets a levered pool die.
 *
 * @spec Fund Shares A3 Fund Shares B1 Hedge Funds E3 Prime Brokerage D2 XI-3 Law 6 Law 19
 *
 * ITEM 13.4, AND IT IS THE ONE THING §28 E3 TURNS ON. A fund's equity is zero by construction: its
 * share liability follows its book. While that claim could go NEGATIVE the construction held however
 * far underwater the pool went — assets, less the loan, less (assets less the loan), is zero — so
 * the solvency trigger could never fire for a fund, and *"a vehicle that absorbs losses indefinitely
 * is the buyer of last resort in a different costume"* described every fund in this world.
 *
 * A share ranks behind everything else the fund owes and takes WHAT IS LEFT (`ranking`), and what is
 * left of a book that does not cover its senior claims is nothing. That is not a floor: a negative
 * share value is a claim that its HOLDERS OWE THE FUND MONEY, and a share is a limited liability.
 *
 * With the claim honest the account stops netting: assets less senior claims is a real negative, the
 * pool is insolvent, its estate opens, and its broker eats the shortfall (§15 D2). Nothing was
 * written to make that happen — it stopped being prevented, which is what 13.4 asked for.
 */
import { describe, expect, it } from 'vitest';
import { navOf } from '../src/mechanisms/funds/nav.js';
import { period } from '../src/calendar/calendar.js';
import { asCash, type Cash } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { none, some } from '../src/core/option.js';
import { USD, instrumentId, partyId, type InstrumentId } from '../src/index.js';
import type { DerivedReads } from '../src/registry/kinds.js';
import type { Instrument } from '../src/register/instruments.js';

const AT = period(9);
const POOL = partyId('pool.levered');
const BROKER = partyId('bank.broker');
const SHARE = instrumentId('share.pool.levered');
const BOOK = instrumentId('equity.something');
const LOAN = instrumentId('loan.bank.broker.pool.levered.1');

const share = {
  id: SHARE,
  issuer: some(POOL),
  ccy: USD,
  issued: asQty(1000),
  status: { live: true },
} as unknown as Instrument;

const loan = {
  id: LOAN,
  issuer: some(POOL),
  ccy: USD,
  issued: asQty(0),
  status: { live: true },
} as unknown as Instrument;

/** A pool holding one line worth `book`, financed by one loan the broker holds for `owed`. */
function reads(book: Cash, owed: Cash): DerivedReads {
  return {
    holdingsOf: () => [{ instrument: BOOK, lots: [] }] as never,
    holdersOf: (i: InstrumentId) => (i === LOAN ? [BROKER] : []),
    quantity: () => asQty(0),
    worthOf: (holder: unknown, instrument: InstrumentId) =>
      instrument === BOOK
        ? some({ value: book, from: AT, ccy: USD })
        : instrument === LOAN && holder === BROKER
          ? some({ value: owed, from: AT, ccy: USD })
          : none(),
    inOwnMoney: (_party: unknown, value: Cash) => value,
    instruments: () => [share, loan],
    issued: () => share.issued,
    // A loan a pool issued IS a liability of its issuer, which is what puts it ahead of the shares.
    kindOf: (i: InstrumentId) => ({ liabilityOfIssuer: i === LOAN }) as never,
    curve: () => ({}) as never,
    on: () => ({}) as never,
    contractsOf: () => [],
  } as unknown as DerivedReads;
}

describe('a levered pool that is still above water', () => {
  it('gives its holders what is left after its lender', () => {
    const read = navOf(share, AT, reads(asCash(500_000, 'its book'), asCash(400_000, 'its loan')));
    expect(read.assets).toBe(500_000);
    expect(read.owed).toBe(400_000);
    // B1: the shares take the residual, and a small move in the book is a large move in the claim
    // — which is what leverage IS and is why §28 D1 starts where it does.
    expect(read.perShare).toBe(100);
  });
});

describe('a levered pool whose book no longer covers its loan', () => {
  const read = navOf(share, AT, reads(asCash(350_000, 'its book'), asCash(400_000, 'its loan')));

  it('leaves its shares NOTHING, and never a negative', () => {
    // A3, N13.a: a residual claim on a book that is short is worth nothing. The alternative — a
    // share worth −50 — is a claim that its holders owe the fund 50, which nobody agreed to.
    expect(read.perShare).toBe(0);
  });

  it('reports both sides as they are, so the shortfall is visible and not netted away', () => {
    // The assets and what is owed are what they are: the fifty thousand between them is the
    // creditors' loss, and it is the number §15 D2 says hits the BROKER's capital. A read that
    // had netted it into a negative share price would have hidden whose loss it is.
    expect(read.assets).toBe(350_000);
    expect(read.owed).toBe(400_000);
  });

  it('is what makes the equity account stop netting to zero', () => {
    // This is the consequence the whole item turns on, stated as arithmetic rather than asserted
    // through a world: the pool holds `assets`, owes `owed` to its lender and `perShare × shares`
    // to its holders. While the claim could go negative those three cancelled exactly and the
    // solvency test saw zero. They no longer cancel, and what is left over is the insolvency.
    const claim = read.perShare * read.shares;
    expect(read.assets - read.owed - claim).toBe(-50_000);
  });
});
