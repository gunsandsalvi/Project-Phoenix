/**
 * An institution's whole investment decision: which mandates it can ACCEPT, and which it feeds.
 *
 * @spec Insurers B2 Insurers B2.a Insurers B2.b Fund Shares A4 Fund Shares C2 Fund Shares D2 Private Equity A2 Private Equity A2.a Private Equity A2.b XI-2 Law 2 Law 3 Law 6 Law 19
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
import { ranWorld } from './rig.js';
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

/**
 * 13.5c: §29 A2.a — *"an investor must hold liquidity against calls it did not choose the timing of,
 * and in a stress the calls and its own troubles arrive together."*
 */
describe('A2.a: what an investor does about a call it did not choose the timing of', () => {
  it('keeps back what a call took, and never the undrawn commitment', () => {
    /**
     * The buffer is its own EXPERIENCE, exactly as the claim buffer is (A4.c): what its last call
     * took. Holding the whole undrawn commitment in cash would be money already paid, and the whole
     * of A2 is that capital is committed and NOT paid — so nothing here reads a commitment's size.
     */
    const w = ranWorld('calls', 12);
    for (const e of w.journal.ofKind('insurer.allocated')) {
      // What it put to work is a number, and it is never the whole of its account: the two buffers
      // come off it first. Both are reads of what happened to it, and neither is a ratio.
      expect(typeof e.data['putToWork']).toBe('number');
      expect(Number(e.data['putToWork'])).toBeGreaterThan(0);
    }
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('callbuffer');
      expect(id).not.toContain('liquidityratio');
    }
  });

  it('asks for its money back when a call went unpaid, and names no price (XI-2)', () => {
    const w = ranWorld('calls', 26);
    for (const e of w.journal.ofKind('insurer.raised')) {
      // It is raising against a call it missed, and it asks for whole shares it actually holds.
      expect(Number(e.data['missed'])).toBeGreaterThan(0);
      expect(Number(e.data['asked'])).toBeGreaterThan(0);
      expect(Number.isInteger(Number(e.data['asked']))).toBe(true);
      // A2.b: and the call it missed was REFUSED in full rather than trimmed to what it held.
      const missed = w.journal
        .forSubject('fund.called', String(e.data['insurer']))
        .filter((c) => c.data['paid'] === false);
      expect(missed.length).toBeGreaterThan(0);
    }
  });

  it('cannot redeem out of the pool that called it, which is the trap A2.a names', () => {
    /**
     * A closed-end fund has no redemption — the money was committed for its life — so an investor
     * called by one cannot meet the call by asking that one for its money back. It sells something
     * ELSE, or it defaults, and the refusal is recorded rather than silently dropped.
     */
    const w = ranWorld('calls', 26);
    for (const e of w.journal.ofKind('fund.notRedeemable')) {
      // The refusal names the pool, the holder and the TERMS it came in on — an investor asking for
      // money it agreed it could not have is a real fact about its own position (App A).
      expect(String(e.data['fund'])).not.toBe('');
      expect(['closed', 'listed']).toContain(String(e.data['terms']));
    }
  });
});
