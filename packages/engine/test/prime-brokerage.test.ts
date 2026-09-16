/**
 * What a prime broker requires, and the one subtraction that must be allowed to come out negative.
 *
 * @spec Prime Brokerage C1 Prime Brokerage C1.b Prime Brokerage C3 Prime Brokerage C3.b Prime Brokerage C4.a Hedge Funds B1 Law 6 Law 19
 *
 * C3.b IS WHY THIS FILE EXISTS. *"A client drawn past its line is over the line, and the shortfall
 * is what forces the sale. Flooring it makes the whole path unreachable — and lending the shortfall
 * straight back at a penalty, from the same broker, makes it unreachable twice."* A floor at zero
 * here is one character, it looks defensive, and it deletes §15 D and most of XI-2 with it. So the
 * negative case is asserted rather than left to a reader's discipline.
 *
 * And C1 is the other half: the requirement is the broker's OWN view of the risk, so what this
 * asserts about it is not a number but a SHAPE — it comes from the broker's own outlook, it is the
 * whole line where the broker has no view at all, and it can never exceed what the line is worth.
 */
import { USD } from '../src/seeds/foundation.js';
import { describe, expect, it } from 'vitest';
import { callOf, type Line, type Position, requirementOn } from '../src/mechanisms/banks/prime.js';
import { asCash, type Cash } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { none, some } from '../src/core/option.js';
import { instrumentId, type InstrumentId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import type { ParticipantView } from '../src/world/context.js';

const LINE_A = instrumentId('equity.a');
const LINE_B = instrumentId('equity.b');

/** A broker with a view of some lines and not others, and a book it holds for the client. */
function broker(
  views: Readonly<Record<string, number>>,
  held: Readonly<Record<string, number>>,
): ParticipantView {
  return {
    outlook: (v: { readonly on: string; readonly instrument?: InstrumentId }) => {
      const wide = v.instrument === undefined ? undefined : views[String(v.instrument)];
      return wide === undefined
        ? none()
        : some({ expected: 0, unit: 'price', per: 'period', confidence: wide, formed: period(0) });
    },
    quantity: (i: InstrumentId) => asQty(held[String(i)] ?? 0),
  } as unknown as ParticipantView;
}

const at = (instrument: InstrumentId, worth: number): Position => ({
  instrument,
  worth: asCash(worth, USD, 'what the client holds of it'),
});

const lineWith = (available: Cash): Line => ({
  portfolio: asCash(0, USD, 'not what this asserts'),
  required: asCash(0, USD, 'not what this asserts'),
  financed: asCash(0, USD, 'not what this asserts'),
  equity: asCash(0, USD, 'not what this asserts'),
  available,
});

describe('C3.b: the available line is never floored', () => {
  it('turns a client drawn past its line into a call of exactly the shortfall', () => {
    // The whole of D1 starts here: over the line by 40 is a call for 40, not a call for nothing.
    expect(callOf(lineWith(asCash(-40, USD, 'over its line by'))).pieces).toBe(40);
  });

  it('calls nothing from a client inside its line', () => {
    expect(callOf(lineWith(asCash(60, USD, 'room left on it'))).pieces).toBe(0);
    expect(callOf(lineWith(asCash(0, USD, 'exactly at it'))).pieces).toBe(0);
  });
});

describe('C1: the requirement is the broker’s own view of the risk', () => {
  it('is how wide its own surprises about each line have been, over what the client holds', () => {
    // A line worth 1,000 (100 units at 10), about which this broker's own surprises have been 2 a
    // unit: it wants 200 against it. Nothing states 20% — that number is this broker's history of
    // being wrong, and another broker that had watched the same line calmly would want less.
    const view = broker({ [LINE_A]: 2 }, { [LINE_A]: 100 });
    expect(requirementOn(view, [at(LINE_A, 1000)])).toBe(200);
  });

  it('finances NOTHING it has no view of, which is a refusal and not a zero', () => {
    const view = broker({}, { [LINE_B]: 50 });
    // The whole line. A lender that cannot say how wrong it has been about a thing has no basis
    // for lending against it, and the honest answer is that it lends none of it (App A).
    expect(requirementOn(view, [at(LINE_B, 500)])).toBe(500);
  });

  it('never wants more against a line than the line is worth', () => {
    // A view so wide that a move the size of its own surprises would take the line past nothing.
    // A line cannot fall by more than the whole of it, which is arithmetic about a position and
    // not a floor on a decision.
    const view = broker({ [LINE_A]: 40 }, { [LINE_A]: 100 });
    expect(requirementOn(view, [at(LINE_A, 1000)])).toBe(1000);
  });

  it('adds line by line, and nothing offsets across two of them', () => {
    // C1.b: what it is measured on is the NET position in each line, which is what a register
    // holding IS. An offset BETWEEN two lines is a claim that they move together — a correlation,
    // and a correlation nobody measured is a number invented to make a requirement smaller.
    const view = broker({ [LINE_A]: 2, [LINE_B]: 1 }, { [LINE_A]: 100, [LINE_B]: 100 });
    expect(requirementOn(view, [at(LINE_A, 1000), at(LINE_B, 1000)])).toBe(300);
  });
});
