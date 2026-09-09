import { describe, expect, it } from 'vitest';
import fc from 'fast-check';
import {
  ANNUAL,
  Calendar,
  Impossible,
  Mismatch,
  NonFinite,
  SEMI_ANNUAL,
  addMoney,
  addMonths,
  civil,
  count,
  currencyCode,
  dayNumber,
  finite,
  formatCivil,
  fromDayNumber,
  money,
  period,
  prng,
  rate,
  sum,
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

describe('money (Money A2.b)', () => {
  it('never adds two currencies', () => {
    const a = money(1, currencyCode('AAA'));
    const b = money(1, currencyCode('BBB'));
    expect(() => addMoney(a, b)).toThrow(Mismatch);
    expect(addMoney(a, money(2, currencyCode('AAA'))).amount).toBe(3);
  });
});

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
