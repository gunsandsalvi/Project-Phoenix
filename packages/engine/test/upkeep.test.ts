/**
 * Keeping a thing in service, and what it costs not to (17e.2).
 *
 * @spec Capital Programme A6 Housing A5 Firm A3 Law 2 Law 6
 *
 * Wear was an event nobody could answer: a holder's only reply to a machine getting older was to
 * hold fewer machines. What is tested here is the outlay that answers it — what keeping a stock
 * takes this period, the share of it the holder went without, and what that costs it — and that
 * every one of those is a read of numbers the thing already states.
 */
import { describe, expect, it } from 'vitest';
import { asRatio } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { failedForWant, upkeepFor, wentWithout } from '../src/registry/physical.js';
import { CAPITAL_KINDS } from '../src/registry/physical.js';
import { GOODS } from '../src/mechanisms/goods/data.js';

describe('what keeping a thing takes (Capital Programme A6, Housing A5)', () => {
  it('is whole pieces of what the thing is made of', () => {
    // Law 8: half a part does not repair half a machine. What a holder must buy is the piece above.
    expect(upkeepFor(asQty(1000), asRatio(0.001, 'what a machine takes a period'))).toBe(1);
    expect(upkeepFor(asQty(1001), asRatio(0.001, 'what a machine takes a period'))).toBe(2);
    expect(upkeepFor(asQty(0), asRatio(0.001, 'what a machine takes a period'))).toBe(0);
  });

  it('says what share of it the holder went without, and never less than none', () => {
    const needed = asQty(10);
    expect(wentWithout(needed, asQty(0))).toBe(1);
    expect(wentWithout(needed, asQty(4))).toBeCloseTo(0.6, 12);
    expect(wentWithout(needed, asQty(10))).toBe(0);
    // A holder that bought more than keeping it took went without NOTHING. That is a zero the
    // arithmetic reaches, not a floor put under it (Law 6), and so is needing nothing at all.
    expect(wentWithout(needed, asQty(25))).toBe(0);
    expect(wentWithout(asQty(0), asQty(0))).toBe(0);
  });

  it('ages the stock beyond the calendar by exactly what it went without (A6, Law 2)', () => {
    // A hundred machines with ten periods left hold a thousand unit-periods of service. A period
    // with none of the upkeep bought is one of those periods gone on top of the calendar's — ten
    // machines — so a shop that maintains nothing runs its plant out in half the time.
    const units = asQty(100);
    expect(failedForWant(units, 10, asRatio(1, 'it bought none of it'))).toBeCloseTo(10, 12);
    // Half the upkeep bought is half the extra ageing, continuously: nothing happens AT any level.
    expect(failedForWant(units, 10, asRatio(0.5, 'it bought half of it'))).toBeCloseTo(5, 12);
    expect(failedForWant(units, 10, asRatio(0, 'it bought all of it'))).toBe(0);
    // And it bites harder the older the plant is, because there is less service left to lose: the
    // same neglect on a vintage with one period left takes the whole of it.
    expect(failedForWant(units, 1, asRatio(1, 'it bought none of it'))).toBeCloseTo(100, 12);
    // A vintage with nothing left is not plant in poor condition; it is plant that is gone.
    expect(failedForWant(units, 0, asRatio(1, 'it bought none of it'))).toBe(0);
  });
});

describe('which things can be kept, and which cannot (Housing A5, Law 2)', () => {
  it('states an upkeep on a dwelling and on every kind of plant, and on nothing else', () => {
    // A dwelling is a GOOD (13d) and what wears it is a rate; a machine is PLANT and what wears it
    // is two dates. Both can be KEPT, and what they share is the share gone without, not a phase.
    const kept = GOODS.filter((g) => g.upkeep !== undefined).map((g) => g.subUnit);
    expect(kept).toEqual(['dwelling']);
    // Nothing keeps a tonne of grain: it goes the whole of its own spoilage, as it always did.
    expect(GOODS.find((g) => g.subUnit === 'grain')?.upkeep).toBeUndefined();
    // Every kind of plant this module declares takes something to stay in service, and every one of
    // those numbers is positive: a kind that took nothing would be plant nobody could neglect.
    for (const k of CAPITAL_KINDS) expect(k.upkeepPerUnitPerPeriod).toBeGreaterThan(0);
  });
});
