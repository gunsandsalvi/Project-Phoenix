import { describe, expect, it } from 'vitest';
import fc from 'fast-check';
import {
  ANNUAL,
  Calendar,
  asQty,
  downTick,
  Impossible,
  NonFinite,
  SEMI_ANNUAL,
  addMonths,
  civil,
  count,
  dayNumber,
  finite,
  formatCivil,
  fromDayNumber,
  months,
  period,
  prng,
  rate,
  scaleQty,
  splitOnTick,
  sum,
  toTick,
  upTick,
  withinDust,
  yearFraction,
} from '../src/index.js';

describe('num (Law 7)', () => {
  it('refuses NaN and Infinity', () => {
    expect(() => finite(Number.NaN, 'x')).toThrow(NonFinite);
    expect(() => finite(Number.POSITIVE_INFINITY, 'x')).toThrow(NonFinite);
    expect(finite(-0, 'x')).toBe(0);
  });

  it('sum returns the value and a dust bound that covers the rounding', () => {
    fc.assert(
      fc.property(
        fc.array(fc.double({ noNaN: true, noDefaultInfinity: true, min: -1e9, max: 1e9 }), {
          maxLength: 200,
        }),
        (xs) => {
          const s = sum(xs);
          const reversed = sum([...xs].reverse());
          return withinDust(s.value, reversed.value, s.dust + reversed.dust);
        },
      ),
    );
  });

  it('dust is arithmetic, never a percentage', () => {
    const s = sum([1e12, 1, -1e12]);
    expect(s.dust).toBeLessThan(1e-2);
    expect(withinDust(s.value, 1, s.dust)).toBe(true);
  });

  it('a count is a non-negative integer (Law 6)', () => {
    expect(count(3, 'n')).toBe(3);
    expect(() => count(1.5, 'n')).toThrow(Impossible);
    expect(() => count(-1, 'n')).toThrow(Impossible);
  });
});

// Money A2.b — "two currencies are never added" — was tested here against a value object in
// `core/money.ts` that NOTHING in the engine ever used: a second representation of money and of a
// quantity, standing beside the real ones and enforcing nothing about them (Law 4). It is deleted,
// and what replaces the read is the wire: `ledger/settlement.ts` refuses any leg in a currency the
// party it touches does not book in, and `test/world.test.ts` asserts that. A guard on the one path
// every movement takes is the rule; a guard on a type nobody holds is a comfort.

describe('civil dates', () => {
  it('round-trips through day numbers', () => {
    fc.assert(
      fc.property(
        fc.integer({ min: -200000, max: 200000 }),
        (d) => dayNumber(fromDayNumber(d)) === d,
      ),
    );
    expect(dayNumber(civil(1970, 1, 1))).toBe(0);
    expect(formatCivil(fromDayNumber(dayNumber(civil(2026, 2, 28))))).toBe('2026-02-28');
  });

  it('advances months by the calendar, landing month-end days on the target month end', () => {
    expect(formatCivil(addMonths(civil(2026, 1, 31), 1))).toBe('2026-02-28');
    expect(formatCivil(addMonths(civil(2026, 3, 15), 6))).toBe('2026-09-15');
    expect(() => civil(2026, 2, 30)).toThrow(Impossible);
  });

  it('day counts read dates (Money G3.c)', () => {
    expect(yearFraction('ACT/365F', civil(2026, 1, 1), civil(2027, 1, 1))).toBeCloseTo(1, 12);
    expect(yearFraction('ACT/ACT', civil(2026, 1, 1), civil(2027, 1, 1))).toBeCloseTo(1, 12);
    expect(yearFraction('30/360', civil(2026, 3, 15), civil(2026, 9, 15))).toBeCloseTo(0.5, 12);
  });
});

describe('calendar (Money G3)', () => {
  const cal = new Calendar({ epoch: civil(2026, 1, 5), periodDays: 7, cyclesPerPeriod: 5 });

  it('places dates in the period containing them', () => {
    expect(cal.periodOf(civil(2026, 1, 5))).toBe(0);
    expect(cal.periodOf(civil(2026, 1, 11))).toBe(0);
    expect(cal.periodOf(civil(2026, 1, 12))).toBe(1);
    expect(() => cal.periodOf(civil(2025, 12, 31))).toThrow(Impossible);
  });

  it('generates a schedule by advancing dates, ending on the end date', () => {
    const s = cal.schedule(civil(2026, 3, 15), civil(2028, 3, 15), SEMI_ANNUAL).map(formatCivil);
    expect(s).toEqual(['2026-09-15', '2027-03-15', '2027-09-15', '2028-03-15']);
  });

  it('has no period finer than a period and no default period', () => {
    expect(() => period(-1)).toThrow(Impossible);
    expect(() => period(1.5)).toThrow(Impossible);
    expect(
      () => new Calendar({ epoch: civil(2026, 1, 5), periodDays: 7, cyclesPerPeriod: 1 }),
    ).toThrow(Impossible);
  });

  it('a rate carries its periodicity (Law 8)', () => {
    expect(rate(0.02, ANNUAL).per.kind).toBe('annual');
  });
});

describe('prng (Seed A5)', () => {
  it('is reproducible from the seed and independent per label', () => {
    const a = prng('seed-1');
    const b = prng('seed-1');
    expect([a.next(), a.next(), a.int(10)]).toEqual([b.next(), b.next(), b.int(10)]);
    const x = prng('seed-1').derive('households');
    const y = prng('seed-1').derive('firms');
    expect(x.next()).not.toBe(y.next());
    const again = prng('seed-1').derive('households');
    expect(again.next()).toBe(prng('seed-1').derive('households').next());
  });
});

describe('the doors that let something through without looking (item 13b.1)', () => {
  it('refuses a quantity that is not a whole number of pieces, at every rounding door', () => {
    // `asQty` and `onTick` checked it; `toTick`, `downTick` and `upTick` cast straight to `Qty`,
    // so a magnitude past the safe-integer range came back branded as a count of pieces and threw
    // later at a site that had not made the error (Law 8).
    for (const door of [toTick, downTick, upTick]) {
      expect(() => door(2 ** 60)).toThrow(/whole number/);
      expect(() => door(Number.MAX_SAFE_INTEGER + 10)).toThrow(/whole number/);
    }
    expect(toTick(2.5)).toBe(3);
    expect(downTick(2.9)).toBe(2);
    expect(upTick(2.1)).toBe(3);
  });

  it('scales a count by a whole number of them and refuses a fraction', () => {
    // Clearing C3, Law 8: a per-member amount times a headcount is a count. Multiplying by a
    // fraction is a rounding, and a rounding is a decision somebody makes by name.
    expect(scaleQty(asQty(7), 3)).toBe(21);
    expect(() => scaleQty(asQty(7), 0.5)).toThrow(/whole number/);
  });

  it('splits a total into parts that sum to exactly the total, and never loses a piece', () => {
    // The loop that hands out the remainders used to `break` on an index it had built itself,
    // which would have left the parts summing to LESS than the total with nothing saying so —
    // a residual with no holder, in the function written to prevent exactly that.
    for (const [total, weights] of [
      [100, [1, 1, 1]],
      [7, [5, 3, 1]],
      [-11, [2, 2, 2, 2]],
      [1, [1, 1, 1, 1, 1]],
    ] as const) {
      const parts = splitOnTick(total, [...weights]);
      expect(parts.reduce<number>((a, b) => a + b, 0)).toBe(total);
      for (const p of parts) expect(Number.isSafeInteger(p)).toBe(true);
    }
  });

  it('refuses a periodicity of no months, which a schedule would loop on for ever', () => {
    // Money G3.a. A hang is the worst failure this engine can have: it reports nothing at all.
    expect(() => months(0)).toThrow(/spacing/);
    expect(() => months(-1)).toThrow(/spacing/);
    expect(() => months(1.5)).toThrow(/spacing/);
    expect(months(3).kind).toBe('months');
  });
});
