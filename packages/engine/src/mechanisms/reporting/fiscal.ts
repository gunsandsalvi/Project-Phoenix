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
export function endOfMonth(c: Civil): Civil {
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
  // Back to one that has closed, and then FORWARD to the latest one that has. Only walking back
  // found the anchor month's own close and stopped there, so a company reported its first quarter
  // and then the same quarter for ever — three reports in forty periods where there should have
  // been nine, and never the one that had just closed.
  let ends = endOfMonth(civil(on.y, anchor, 1));
  while (compareCivil(ends, on) > 0) ends = endOfMonth(addMonths(ends, -MONTHS_IN_QUARTER));
  for (;;) {
    const next = endOfMonth(addMonths(ends, MONTHS_IN_QUARTER));
    if (compareCivil(next, on) > 0) break;
    ends = next;
  }
  return quarterEndingOn(anchor, ends);
}

/**
 * The quarter whose CLOSE is this date. One builder, because "three months back to the first" is
 * the arithmetic both callers want and a second copy of it would be the one that drifts (Law 4).
 *
 * The close is taken to the month's own last day first. Adding three months to the 30th of April
 * lands on the 30th of July, and July closes on the 31st — so a walk that carried the day number
 * would step a day short every time a short month met a long one, and a quarter would silently
 * become the one before it.
 */
export function quarterEndingOn(anchor: number, close: Civil): Quarter {
  const ends = endOfMonth(close);
  const opens = addMonths(ends, -(MONTHS_IN_QUARTER - 1));
  return { begins: civil(opens.y, opens.m, 1), ends, label: labelOf(anchor, ends) };
}

/**
 * Which quarter of the company's OWN year this is — its fourth quarter ends in its anchor month by
 * construction — and which fiscal year. A reader who saw only `2027-Q1` for two companies with
 * different year ends would think they covered the same three months.
 */
function labelOf(anchor: number, ends: Civil): string {
  // How many quarters this close is BEFORE the year's own end: none for the anchor month itself,
  // which is the fourth quarter, and three for the one that opens the year.
  const before = ((anchor - ends.m + MONTHS_IN_YEAR) % MONTHS_IN_YEAR) / MONTHS_IN_QUARTER;
  const quarter = QUARTERS_IN_YEAR - before;
  // The fiscal year is named for the calendar year it ENDS in, which is this year when the close is
  // at or before the anchor month and the next one when it is past it.
  const year = ends.m <= anchor ? ends.y : ends.y + 1;
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
