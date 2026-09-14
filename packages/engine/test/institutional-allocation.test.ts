/**
 * An institution's whole investment decision: whose mandate is shaped like what I promised.
 *
 * @spec Insurers B2 Insurers B2.a Insurers B2.b Fund Shares A4 Law 3 Law 19
 *
 * ITEM 14.0. *"Insurance companies and pension funds don't invest themselves. Their assets are
 * always third party managed"* (the owner), so there is no portfolio to test here and no allocation
 * rule — there is ONE choice, and B2.b says what it is made of: the duration of what it promised
 * against the duration of what a manager runs.
 *
 * What is asserted is the shape of that choice and its two REFUSALS, because both of them are the
 * kind of thing a helpful default would quietly delete: a pool that states no duration is not a
 * fallback for a liability schedule, and a tie is not broken by the order a list was built in.
 */
import { describe, expect, it } from 'vitest';
import { matchFor } from '../src/mechanisms/insurers/allocate.js';
import { asPerPiece } from '../src/core/measure.js';
import { venueId } from '../src/core/ids.js';

const door = (fund: string, years: number | undefined) => ({
  venue: venueId(`funds.${fund}`),
  fund,
  perShare: asPerPiece(100, 'what a share is worth'),
  years,
  asks: undefined,
});

describe('B2.b: duration matching is the whole of the decision', () => {
  it('picks the mandate nearest what it promised, from either side', () => {
    const open = [door('short', 1), door('medium', 7), door('long', 30)];
    // A book of promises running eight years wants the seven-year mandate, not the thirty-year one
    // — an institution that bought duration it did not promise has taken a risk nobody asked it to.
    expect(matchFor(open, 8)?.fund).toBe('medium');
    // And nearest from below is still nearest: it is a distance, not a floor or a ceiling.
    expect(matchFor(open, 2)?.fund).toBe('short');
    expect(matchFor(open, 25)?.fund).toBe('long');
  });

  it('refuses a pool that states no duration at all', () => {
    // An equity fund is not a place to put money you have promised somebody on a date, and a
    // mandate that says nothing about duration cannot be matched to a schedule. The refusal is the
    // answer (App A) — the alternative is an institution funding a life book out of shares because
    // nothing else was open.
    expect(matchFor([door('equity', undefined)], 8)).toBeUndefined();
    // And it stays refused even when it is the only thing there beside a bad match.
    expect(matchFor([door('equity', undefined), door('long', 30)], 8)?.fund).toBe('long');
  });

  it('breaks a tie on the fund’s own name and never on the order of the list', () => {
    const open = [door('zeta', 10), door('alpha', 10)];
    expect(matchFor(open, 10)?.fund).toBe('alpha');
    expect(matchFor([...open].reverse(), 10)?.fund).toBe('alpha');
  });

  it('has nothing to allocate to when nothing is open', () => {
    expect(matchFor([], 8)).toBeUndefined();
  });
});
