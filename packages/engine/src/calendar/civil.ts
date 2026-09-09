/**
 * Civil dates as day numbers. No Date object: the engine has no clock and no time zone.
 * Algorithms after Howard Hinnant's days_from_civil / civil_from_days (proleptic Gregorian).
 */
import { Impossible } from '../core/errors.js';

export interface Civil {
  readonly y: number;
  readonly m: number;
  readonly d: number;
}

/** Days since 1970-01-01 (may be negative). */
export type DayNumber = number;

export function isLeap(y: number): boolean {
  return (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
}

export function daysInMonth(y: number, m: number): number {
  const table = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  const base = table[m - 1];
  if (base === undefined) throw new Impossible('Money G3', `month ${m} does not exist`);
  return m === 2 && isLeap(y) ? 29 : base;
}

export function civil(y: number, m: number, d: number): Civil {
  if (!Number.isInteger(y) || !Number.isInteger(m) || !Number.isInteger(d)) {
    throw new Impossible('Money G3', `date parts must be integers: ${y}-${m}-${d}`);
  }
  if (m < 1 || m > 12 || d < 1 || d > daysInMonth(y, m)) {
    throw new Impossible('Money G3', `${y}-${m}-${d} is not a date`);
  }
  return Object.freeze({ y, m, d });
}

export function dayNumber(c: Civil): DayNumber {
  const y = c.m <= 2 ? c.y - 1 : c.y;
  const era = Math.floor(y / 400);
  const yoe = y - era * 400;
  const mp = (c.m + 9) % 12;
  const doy = Math.floor((153 * mp + 2) / 5) + c.d - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return era * 146097 + doe - 719468;
}

export function fromDayNumber(z: DayNumber): Civil {
  if (!Number.isInteger(z)) throw new Impossible('Money G3', `day number ${z} is not an integer`);
  const zz = z + 719468;
  const era = Math.floor(zz / 146097);
  const doe = zz - era * 146097;
  const yoe = Math.floor(
    (doe - Math.floor(doe / 1460) + Math.floor(doe / 36524) - Math.floor(doe / 146096)) / 365,
  );
  const y = yoe + era * 400;
  const doy = doe - (365 * yoe + Math.floor(yoe / 4) - Math.floor(yoe / 100));
  const mp = Math.floor((5 * doy + 2) / 153);
  const d = doy - Math.floor((153 * mp + 2) / 5) + 1;
  const m = mp < 10 ? mp + 3 : mp - 9;
  return civil(m <= 2 ? y + 1 : y, m, d);
}

/** Advance by whole months, clamping the day to the target month's length (end-of-month convention). */
export function addMonths(c: Civil, months: number): Civil {
  if (!Number.isInteger(months))
    throw new Impossible('Money G3.a', `months ${months} not an integer`);
  const total = c.y * 12 + (c.m - 1) + months;
  const y = Math.floor(total / 12);
  const m = total - y * 12 + 1;
  const dim = daysInMonth(y, m);
  // This is a calendar convention (the 31st of a month advanced to a 30-day month lands on the 30th),
  // not a bound covering a decision: the target day does not exist.
  return civil(y, m, c.d > dim ? dim : c.d);
}

export function addDays(c: Civil, days: number): Civil {
  return fromDayNumber(dayNumber(c) + days);
}

export function compareCivil(a: Civil, b: Civil): number {
  return dayNumber(a) - dayNumber(b);
}

export function formatCivil(c: Civil): string {
  const mm = c.m < 10 ? `0${c.m}` : `${c.m}`;
  const dd = c.d < 10 ? `0${c.d}` : `${c.d}`;
  return `${c.y}-${mm}-${dd}`;
}

export function parseCivil(s: string): Civil {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
  if (m === null) throw new Impossible('Money G3', `"${s}" is not an ISO date`);
  return civil(Number(m[1]), Number(m[2]), Number(m[3]));
}
