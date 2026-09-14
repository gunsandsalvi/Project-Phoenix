/**
 * An institution's whole investment decision: which mandates it can ACCEPT, and which it feeds.
 *
 * @spec Insurers B2 Insurers B2.a Insurers B2.b Fund Shares A4 Fund Shares D2 Law 2 Law 3 Law 6 Law 19
 *
 * ITEM 14.0. *"Insurance companies and pension funds don't invest themselves. Their assets are
 * always third party managed"* (the owner), so there is no portfolio to test here and no allocation
 * rule — there is a set of doors it may go through and a rule for which one this period's money
 * goes to.
 *
 * ITEM 10f.5, THE OWNER'S CORRECTION: *"insurance and pension don't only go for duration. They
 * invest in tons of different strategies."* What 14.0 built matched the NEAREST stated duration and
 * refused every pool that stated none — which is every strategy, equity and private-equity pool in
 * this world, for ever. What is asserted here is the two refusals that replaced it, because both of
 * them are the kind of thing a helpful default would quietly delete: a pool longer than what it
 * promised is a risk nobody asked for, and a pool that has told it it earns less than its promises
 * require has told it something a pool that said nothing has not.
 */
import { describe, expect, it } from 'vitest';
import { acceptable } from '../src/mechanisms/insurers/allocate.js';
import { asPerPiece } from '../src/core/measure.js';
import { venueId } from '../src/core/ids.js';

const door = (fund: string, years: number | undefined, offered?: number) => ({
  venue: venueId(`funds.${fund}`),
  fund,
  perShare: asPerPiece(100, 'what a share is worth'),
  years,
  offered,
  asks: undefined,
});

describe('B2.b: what an institution will not take is duration it did not promise', () => {
  it('refuses a mandate longer than its longest promise, and takes any shorter one', () => {
    // A book of promises running eight years may hold a one-year or a seven-year mandate: both of
    // them come back before it owes anything. A thirty-year one is a rate risk nobody asked it to
    // take, and it is refused however attractive it is.
    expect(acceptable(door('short', 1), 8, undefined)).toBe(true);
    expect(acceptable(door('medium', 7), 8, undefined)).toBe(true);
    expect(acceptable(door('long', 30), 8, undefined)).toBe(false);
  });

  it('does NOT refuse a pool that states no duration (10f.5)', () => {
    // Equity has no duration to mismatch. This is the line the owner's correction turns on: 14.0
    // refused exactly this pool, and refusing it is what left an institution with one asset class.
    expect(acceptable(door('equity', undefined), 8, undefined)).toBe(true);
    expect(acceptable(door('strategy', undefined), 1, undefined)).toBe(true);
  });

  it('applies no duration test at all to an institution that has promised nothing', () => {
    // It has capital and no liabilities, so there is nothing for an asset to be mismatched against.
    expect(acceptable(door('long', 30), undefined, undefined)).toBe(true);
  });
});

describe('B2: and it will not take less than its promises require', () => {
  it('refuses a pool that has told it it earns less than what it needs', () => {
    expect(acceptable(door('thin', 5, 0.01), 8, 0.04)).toBe(false);
    expect(acceptable(door('fat', 5, 0.06), 8, 0.04)).toBe(true);
    // Exactly enough is enough: there is no margin anybody declared (Law 6).
    expect(acceptable(door('exact', 5, 0.04), 8, 0.04)).toBe(true);
  });

  it('does not refuse a pool that has claimed nothing (App A)', () => {
    // An absence of evidence is returned as one. A pool with no curve to read publishes no offer,
    // and it has not claimed anything to fail against — which is why an institution reaches past
    // bonds at all rather than sitting in cash when nothing yields enough.
    expect(acceptable(door('quiet', undefined, undefined), 8, 0.04)).toBe(true);
    expect(acceptable(door('quiet', 5, undefined), 8, 0.04)).toBe(true);
  });

  it('applies no return test where the world published no curve to require anything from', () => {
    expect(acceptable(door('thin', 5, 0.01), 8, undefined)).toBe(true);
  });
});
