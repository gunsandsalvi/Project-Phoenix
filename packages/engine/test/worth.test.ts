import { describe, expect, it } from 'vitest';
import { Calendar, period } from '../src/calendar/calendar.js';
import { instrumentId, partyId } from '../src/core/ids.js';
import { none, some } from '../src/core/option.js';
import type { WorthReads } from '../src/registry/kinds.js';
import { shareKind } from '../src/mechanisms/equity/share.js';
import type { Instrument } from '../src/register/instruments.js';
import { rigWorld } from './rig.js';

/** A test never names a party: everything below asks the world what it drew. */
function liveOf(w: ReturnType<typeof rigWorld>, kind: string) {
  return w.instruments.all().filter((i) => String(i.kind) === kind && i.status.live);
}

describe('worth: the half of an instrument profile that was missing (§46, Equity B1)', () => {
  it('discounts a promise where the kind has one, and wants less of it as it requires more', () => {
    const w = rigWorld('worth');
    w.step();
    const bill = liveOf(w, 'sovereign.bill')[0];
    const party = w.parties.all().find((p) => p.status.alive);
    expect(bill).toBeDefined();
    expect(party).toBeDefined();
    if (bill === undefined || party === undefined) return;
    const view = w.participantView(party.id);
    const cheap = view.worth(bill.id, 0.01);
    const dear = view.worth(bill.id, 0.08);
    expect(cheap.some).toBe(true);
    expect(dear.some).toBe(true);
    // Nothing states this: it falls out of discounting the same dated payments at two rates, which
    // is the disagreement §46 A3 says a book is made of.
    if (cheap.some && dear.some) expect(cheap.value).toBeGreaterThan(dear.value);
  });

  it('refuses a required return that is not one, rather than answering anyway', () => {
    const w = rigWorld('worth');
    w.step();
    const bill = liveOf(w, 'sovereign.bill')[0];
    const party = w.parties.all().find((p) => p.status.alive);
    if (bill === undefined || party === undefined) return;
    const view = w.participantView(party.id);
    // Law 6, App A: nothing is worth anything at a required return of nothing. The arithmetic is a
    // division by zero and the honest answer is that there is no answer, never an infinity.
    expect(view.worth(bill.id, 0).some).toBe(false);
    expect(() => view.worth(bill.id, Number.NaN)).toThrow();
  });
});

/**
 * The share's own answer, asked of the profile directly.
 *
 * A listed line is DRAWN and the scale model does not draw one, so this exercises the function the
 * kernel dispatches to rather than waiting for a world to produce a company with a share and four
 * published quarters. What is being tested is the kind's arithmetic, which is where it lives.
 */
describe('a share, which had no answer at all before (Equity B1, B3)', () => {
  const calendar = new Calendar({
    epoch: { y: 2025, m: 1, d: 6 },
    periodDays: 7,
    cyclesPerPeriod: 4,
  });
  const line = {
    id: instrumentId('equity.firm.1'),
    kind: shareKind.id,
    issuer: some(partyId('firm.1')),
  } as unknown as Instrument;

  const reads = (earned: number | undefined, shares: number): WorthReads => ({
    calendar,
    period: period(10),
    lastReport: () => (earned === undefined ? none() : some({ earned, periods: 13 })),
    issued: () => shares,
  });

  it('capitalises what the company PUBLISHED, at what this holder requires', () => {
    const worth = shareKind.worthTo?.(line, 0.08, reads(1_300_000, 1000));
    expect(worth?.some).toBe(true);
    // A quarter's earnings annualised, capitalised at 8%, over the shares there are. Nothing here
    // is a multiple and nothing is a forecast: it is one published number and one required return.
    if (worth?.some === true) expect(worth.value).toBeGreaterThan(0);
  });

  it('two holders requiring different returns want it at different levels (§46 A3)', () => {
    const patient = shareKind.worthTo?.(line, 0.04, reads(1_300_000, 1000));
    const impatient = shareKind.worthTo?.(line, 0.12, reads(1_300_000, 1000));
    expect(patient?.some).toBe(true);
    expect(impatient?.some).toBe(true);
    if (patient?.some === true && impatient?.some === true) {
      expect(patient.value).toBeGreaterThan(impatient.value);
    }
  });

  it('a company that has published nothing is an ABSENCE, never a zero (App A)', () => {
    expect(shareKind.worthTo?.(line, 0.08, reads(undefined, 1000)).some).toBe(false);
    // And one that published a loss: there is no stream to capitalise, and saying "worth nothing"
    // would be a claim this door has no business making — a loss-making company is worth what
    // somebody will pay for it, which is a market's answer and not this one's.
    expect(shareKind.worthTo?.(line, 0.08, reads(-5000, 1000)).some).toBe(false);
    // A line with no shares in issue has no per-share anything.
    expect(shareKind.worthTo?.(line, 0.08, reads(1_300_000, 0)).some).toBe(false);
  });
});
