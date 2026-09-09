/**
 * Day-count conventions read the calendar's dates (Money G3.c). A convention computed from a count
 * of periods would be a second calendar.
 *
 * @spec Money G3.c Bond N6
 */
import { assertNever } from '../core/assert.js';
import { type Civil, dayNumber, isLeap } from './civil.js';

export type DayCount = 'ACT/365F' | 'ACT/360' | '30/360' | 'ACT/ACT';

/** Year fraction between two dates under a convention. */
export function yearFraction(dc: DayCount, from: Civil, to: Civil): number {
  const days = dayNumber(to) - dayNumber(from);
  switch (dc) {
    case 'ACT/365F':
      return days / 365;
    case 'ACT/360':
      return days / 360;
    case '30/360': {
      const d1 = from.d === 31 ? 30 : from.d;
      const d2 = to.d === 31 && d1 === 30 ? 30 : to.d;
      return (360 * (to.y - from.y) + 30 * (to.m - from.m) + (d2 - d1)) / 360;
    }
    case 'ACT/ACT': {
      // ISDA: split at year boundaries, each year's days over that year's length.
      let acc = 0;
      let y = from.y;
      let start = dayNumber(from);
      const end = dayNumber(to);
      while (y < to.y) {
        const nextYearStart = dayNumber({ y: y + 1, m: 1, d: 1 });
        acc += (nextYearStart - start) / (isLeap(y) ? 366 : 365);
        start = nextYearStart;
        y += 1;
      }
      acc += (end - start) / (isLeap(y) ? 366 : 365);
      return acc;
    }
    default:
      return assertNever(dc, 'DayCount');
  }
}
