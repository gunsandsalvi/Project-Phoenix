/**
 * The fiscal calendar: a quarter is a pair of DATES, and the periods in it are a read.
 *
 * @spec Reporting A3 Reporting A4 Reporting G6 Money G3.a Money G3.b Law 19
 *
 * G6 is the whole design constraint: no reporting calendar finer than a period, and none placed by a
 * COUNT of periods rather than by a date. So nothing here counts periods. A company's year ends in a
 * month; its quarters end on the last day of that month and of the three months before it, three at
 * a time, round the year; and which periods those dates fall in is asked of the one calendar
 * (Money G3.a). A fiscal quarter is a whole number of periods only by accident, which is exactly
 * what A3 says it is.
 */
import {
  MONTHS_IN_QUARTER,
  MONTHS_IN_YEAR,
  QUARTERS_IN_YEAR,
  addDays,
  addMonths,
  civil,
  compareCivil,
  daysInMonth,
  type Civil,
} from '../../calendar/civil.js';
import type { Calendar, Period } from '../../calendar/calendar.js';
import { forbid } from '../../core/assert.js';

/** One fiscal quarter, as the two dates that bound it and the name a reader would call it. */
export interface Quarter {
  /** The first day of the quarter. */
  readonly begins: Civil;
  /** The last day of the quarter — the fiscal close, which is what the lag runs from (A4). */
  readonly ends: Civil;
  /** `2027-Q3` in the company's OWN year, so two companies with different anchors are not confused. */
  readonly label: string;
}

/** The last day of the month `c` is in, which is where a fiscal period ends. */
function endOfMonth(c: Civil): Civil {
  return civil(c.y, c.m, daysInMonth(c.y, c.m));
}

/**
 * A3: the fiscal quarter that CLOSED most recently on or before `on`, for a company whose year ends
 * in month `anchor`.
 *
 * Walked from the anchor rather than solved, because the arithmetic of "which quarter is a date in"
 * is the arithmetic that goes wrong at a year boundary, and a walk over four candidates cannot.
 */
export function quarterClosedBy(anchor: number, on: Civil): Quarter {
  forbid(
    Number.isInteger(anchor) && anchor >= 1 && anchor <= MONTHS_IN_YEAR,
    'Reporting A3',
    `a fiscal year ends in a month of the year, not in month ${anchor}`,
    { anchor },
  );
  // The anchor's own month-end in the year `on` is in, and then back three months at a time until
  // one of them is on or before `on`. Four steps at most: a year has four quarters.
  let ends = endOfMonth(civil(on.y, anchor, 1));
  while (compareCivil(ends, on) > 0) ends = endOfMonth(addMonths(ends, -MONTHS_IN_QUARTER));
  const opens = addMonths(ends, -(MONTHS_IN_QUARTER - 1));
  const begins = civil(opens.y, opens.m, 1);
  return { begins, ends, label: labelOf(anchor, ends) };
}

/**
 * Which quarter of the company's OWN year this is — its fourth quarter ends in its anchor month by
 * construction — and which fiscal year. A reader who saw only `2027-Q1` for two companies with
 * different year ends would think they covered the same three months.
 */
function labelOf(anchor: number, ends: Civil): string {
  const back = (ends.m - anchor + MONTHS_IN_YEAR) % MONTHS_IN_YEAR;
  const quarter = QUARTERS_IN_YEAR - back / MONTHS_IN_QUARTER;
  // The fiscal year is named for the calendar year its LAST quarter ends in.
  const year = back === 0 ? ends.y : ends.y + (anchor < ends.m ? 1 : 0);
  return `${year}-FQ${quarter}`;
}

/**
 * A4: the day the report may come out — the close plus the lag somebody legislated. A date, because
 * the lag is a number of days and a period is a number of days (G3.b).
 */
export function publishableOn(q: Quarter, lagDays: number): Civil {
  return addDays(q.ends, lagDays);
}

/**
 * A3, Money G3.a: the periods this quarter covers, asked of the one calendar. Inclusive of both
 * ends, and it is a whole number of periods only by accident — which is why the span is asked for
 * rather than counted.
 */
export function spanOf(q: Quarter, calendar: Calendar): { from: Period; to: Period } {
  return { from: calendar.periodOf(q.begins), to: calendar.periodOf(q.ends) };
}
