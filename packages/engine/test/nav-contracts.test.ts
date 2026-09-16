/**
 * A fund's book does not stop at the register's edge.
 *
 * @spec Fund Shares A3 Fund Shares B1 Fund Shares B2 Derivative X1 Derivative D1 Currency C4.a Law 4
 *
 * ITEM 13.6, AND IT IS A PASS OPENED BEFORE THE THING THAT NEEDS IT. A fund's equity is ZERO by
 * construction: its share liability follows whatever its book comes to (`owes: 'value'`), so assets
 * and liabilities move together and nothing is left over. That construction holds only while the
 * NAV can SEE the whole book — and a contract is not in the register (X1): nobody issued it, nobody
 * holds units of it, and it is on both sides' books at once.
 *
 * So a pool with a derivative position had a mark its own share value had never been told about,
 * while its equity ACCOUNT moved with every revaluation of it. The two answers diverge by exactly
 * the contract book, and that divergence IS a fund with equity — measured at 83,247,864 on one
 * vehicle the first time funds were let into the contract books.
 *
 * Nothing in this world reaches it today, because every mandate drawn says `mayWrite: []`. Item
 * 13.2 draws one that does not, which is why this is tested on the read itself rather than through
 * a world: the property has to hold before there is a party that can break it.
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

const AT = period(4);
const FUND = partyId('pool.under.test');
const SHARE = instrumentId('share.pool.under.test');
const BOND = instrumentId('bond.somebody.5.2031');

/** The share line, as the register holds one: issued by the fund, counted in shares. */
const share = {
  id: SHARE,
  issuer: some(FUND),
  ccy: USD,
  issued: asQty(1000),
  status: { live: true },
} as unknown as Instrument;

/**
 * The kernel's reads, with nothing in them but what this property turns on: one priced holding, and
 * whatever contract book the case under test gives it. A stub rather than a world, because what is
 * being asserted is an ARITHMETIC identity of the read and not a behaviour of the world.
 */
function reads(
  holdingWorth: Cash,
  contracts: readonly { readonly worth: Cash; readonly ccy: typeof USD }[],
): DerivedReads {
  return {
    holdingsOf: () => [{ instrument: BOND, lots: [] }] as never,
    holdersOf: () => [],
    quantity: () => asQty(0),
    worthOf: (_holder: unknown, instrument: InstrumentId) =>
      instrument === BOND ? some({ value: holdingWorth, from: AT, ccy: USD }) : none(),
    inOwnMoney: (_party: unknown, value: Cash) => value,
    instruments: () => [],
    issued: () => share.issued,
    kindOf: () => ({ liabilityOfIssuer: false }) as never,
    curve: () => ({}) as never,
    on: () => ({}) as never,
    contractsOf: () => contracts,
  } as unknown as DerivedReads;
}

describe('a NAV reads the contract book as well as the register', () => {
  it('counts a contract that is worth something as an asset', () => {
    const flat = navOf(share, AT, reads(asCash(100_000, USD, 'the bond'), []));
    const long = navOf(
      share,
      AT,
      reads(asCash(100_000, USD, 'the bond'), [{ worth: asCash(20_000, USD, 'in the money'), ccy: USD }]),
    );
    expect(long.assets.pieces).toBe(120_000);
    expect(long.owed.pieces).toBe(0);
    // B1: and the claim on the book moves with the book, which is the whole of why this matters —
    // the holders own the gain, and a NAV that did not see it would have left it in the fund's
    // equity, where a fund's equity may never be (A3).
    expect(long.perShare).toBeGreaterThan(flat.perShare);
  });

  it('counts one that is worth less than nothing as a LIABILITY, and never nets it away', () => {
    const both = navOf(
      share,
      AT,
      reads(asCash(100_000, USD, 'the bond'), [
        { worth: asCash(20_000, USD, 'one it is long'), ccy: USD },
        { worth: asCash(-15_000, USD, 'one it is short'), ccy: USD },
      ]),
    );
    // D1: an asset to one side and a liability to the other at every instant, and Appendix B: no
    // netting across counterparties. A pool long one contract and short another HAS an asset and a
    // liability — adding them first would hide half of both, which is exactly what B5's three
    // reads exist to prevent.
    expect(both.assets).toBe(120_000);
    expect(both.owed).toBe(15_000);
    // And the net is what its holders own: the book, plus the one, less the other.
    expect(both.perShare * both.shares).toBeCloseTo(105_000, 6);
  });

  it('leaves the NAV where it was when the contract book is empty', () => {
    const flat = navOf(share, AT, reads(asCash(100_000, USD, 'the bond'), []));
    expect(flat.assets.pieces).toBe(100_000);
    expect(flat.owed.pieces).toBe(0);
    expect(flat.perShare).toBe(100);
  });
});
