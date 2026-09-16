/**
 * A call is not a budget. That is the whole of §29 A2.b.
 *
 * @spec Private Equity A1 Private Equity A2 Private Equity A2.b Private Equity A3 Private Equity A5 Money E1 Law 2
 *
 * ITEM 13.5. Every other payment in this world is bounded by what the payer has, and correctly so: a
 * household spends what it holds, a fund redeems what its cash reaches, a bank lends what its room
 * allows. **A capital call is the one obligation here that is not**, and A2.b says so as a FORBID:
 * *"a call bounded by the investor's spare cash is not an obligation."*
 *
 * The property itself — that nothing trims the demand to fit the balance — is an ABSENCE in code,
 * and a test cannot assert an absence. It is guarded instead where it is visible, by
 * `tools/check-forbids.ts`, which refuses `atMost`, `atLeast` and `Math.min` anywhere in the call
 * path — and that guard was proved to bite before it was trusted. What is asserted here is the
 * arithmetic around it and the SHAPE of the vehicle, which a reader can otherwise only get by
 * believing a comment.
 */
import { USD } from '../src/seeds/foundation.js';
import { describe, expect, it } from 'vitest';
import { drawPrivateEquity, drawFunds } from '../src/mechanisms/funds/data.js';
import { undrawnOn, undrawnTo, type Commitment } from '../src/mechanisms/funds/commitment.js';
import { asCash } from '../src/core/measure.js';
import { partyId, agreementId } from '../src/core/ids.js';
import { COMMITMENT } from '../src/mechanisms/funds/commitment.js';

const BANKS = [
  { bank: 'bank.a', size: 9 },
  { bank: 'bank.b', size: 7 },
];
const INVESTORS = ['insurance.us', 'insurance.eu'];

const promised = (committed: number, drawn: number): Commitment => ({
  kind: COMMITMENT,
  committed: asCash(committed, USD, 'what it promised'),
  drawn: asCash(drawn, USD, 'what it has paid in'),
  id: agreementId('commitment.1'),
  investor: partyId('insurance.us'),
  pool: partyId('fund.buyout'),
});

describe('what a commitment is', () => {
  it('is what was promised less what has been paid, and never what the investor can afford', () => {
    // A2: capital is COMMITTED, not paid. What is left to call is a fact about the promise and
    // about the calls that have settled — the investor's balance is not in it, here or anywhere.
    expect(undrawnOn(promised(100_000_000, 0))).toBe(100_000_000);
    expect(undrawnOn(promised(100_000_000, 40_000_000)).pieces).toBe(60_000_000);
    expect(undrawnOn(promised(100_000_000, 100_000_000)).pieces).toBe(0);
  });

  it('adds across the investors who raised the fund', () => {
    expect(undrawnTo([promised(100_000_000, 40_000_000), promised(50_000_000, 0)], USD).pieces).toBe(110_000_000);
    expect(undrawnTo([], USD).pieces).toBe(0);
  });
});

describe('what makes a pool private equity', () => {
  const [pe] = drawPrivateEquity(BANKS, INVESTORS, 'a-seed');

  it('is four terms of its mandate, and there is no private-equity party kind', () => {
    expect(pe).toBeDefined();
    if (pe === undefined) return;
    // A2: committed and called. Nobody gets in at the door and nobody gets out — the one vehicle
    // here that can never be a forced seller in either direction.
    expect(pe.liquidity.how).toBe('closed');
    // A5: UNLISTED equity, which is the read that separates a private company from a public one.
    expect(pe.blueprint.classes).toEqual(['residual']);
    expect(pe.blueprint.listed).toBe(false);
    // A3: carry — the asymmetric second fee, the same one a strategy house charges.
    expect(pe.performanceFee).toBeGreaterThan(0);
    // B2.a: the debt in a buyout is the TARGET's, which is why a failed buyout kills the firm and
    // not the fund. The fund itself borrows nothing.
    expect(pe.leverage).toBe(false);
  });

  it('holds no buffer, because there is nothing to hold one against', () => {
    // C2.a's buffer exists for redemptions, and a closed-end fund has none. A fund keeping cash
    // back against a redemption that cannot happen is a fund with money doing nothing.
    expect(pe?.buffer).toBe(0);
  });

  it('was raised on commitments from named investors, and two of them are two sizes', () => {
    const rows = Object.entries(pe?.commitments ?? {});
    expect(rows.map(([who]) => who).sort()).toEqual([...INVESTORS].sort());
    // Seed A3: the SPREAD is what makes two investors two different obligations when the call
    // comes — which is what decides which of them can meet one.
    expect(new Set(rows.map(([, v]) => v)).size).toBe(rows.length);
    for (const [, v] of rows) expect(v).toBeGreaterThan(0);
  });

  it('is the only pool this world opens with that was raised rather than sold', () => {
    // Every other vehicle takes money at a door. This one was raised before it existed, which is
    // what a vintage is — and it is why `commitments` is absent everywhere else.
    for (const p of drawFunds(BANKS, 'a-seed')) expect(p.commitments).toBeUndefined();
  });
});
