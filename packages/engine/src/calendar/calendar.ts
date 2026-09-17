/**
 * One calendar (Money G3): an epoch, a period length, one mapping period to date, and one placement
 * of every periodicity on that grid, by date and never by a count of periods (G3.a).
 *
 * @spec Money G1 Money G2 Money G3 Money G3.a Money G3.b Money G3.c Money G4 Money G4.a
 *
 * Time within a period is cycles (G1, G2); nothing finer exists.
 */
import { Impossible, Mismatch } from '../core/errors.js';
import type { Brand } from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import type { Periodicity } from '../core/rate.js';
import { addDays, addMonths, type Civil, compareCivil, dayNumber, fromDayNumber } from './civil.js';

/** A period index on the one calendar. There is no default period (G4.a). */
export type Period = Brand<number, 'Period'>;
/** A settlement cycle within a period, 0-based. */
export type Cycle = Brand<number, 'Cycle'>;

export function period(n: number): Period {
  if (!Number.isInteger(n) || n < 0) {
    throw new Impossible('Money G4', `period ${n} is not a period index`);
  }
  return n as Period;
}

/**
 * The next date on a stated CYCLE: the first period at or after `at` that is a whole number of
 * `every` from the epoch.
 *
 * An exchange lists a ladder — March, June, September — and everything written between two of them
 * settles on the same date into the same book. A contract dated `at + life` instead would open a
 * new book every period and leave the last one with one trade in it, which is not a market: it is
 * a queue of private agreements wearing a market's name.
 */
export function nextCycle(at: Period, every: number): Period {
  if (!(every > 0)) throw new Impossible('Law 8', `a contract cycle of ${every} periods`);
  return period(Math.ceil((at + 1) / every) * every);
}

export function nextPeriod(p: Period): Period {
  return period(p + 1);
}

/**
 * Money G3.a (item 20): DOES AN ANNIVERSARY OF `epoch` FALL IN THIS PERIOD?
 *
 * Anything that recurs in this world recurs on a DATE — a term of parliament, a year of a rating,
 * a quarter of accounts — and the date is walked in MONTHS from the day the thing began. What a
 * period then is, is the period that date falls into, which is what this answers. `period % n` is
 * the thing it exists instead of: a second calendar, off by a day a month and by a week a year.
 *
 * The anniversaries are `epoch + k·months` for k ≥ 1, so the period of the epoch itself is not one
 * of them — a thing does not have its anniversary on the day it happened.
 */
export function crossesAnniversary(
  cal: Pick<Calendar, 'startOf'>,
  epoch: Period,
  months: number,
  at: Period,
): boolean {
  if (!(months > 0) || at <= epoch) return false;
  const from = cal.startOf(epoch);
  const today = cal.startOf(at);
  const before = cal.startOf(period(at - 1));
  let last = from;
  for (let next = addMonths(from, months); compareCivil(next, today) <= 0; ) {
    last = next;
    next = addMonths(last, months);
  }
  return compareCivil(last, from) !== 0 && compareCivil(last, before) > 0;
}

export interface CalendarSpec {
  /** The date period 0 starts. */
  readonly epoch: Civil;
  /** RESOLUTION: days per period (7, a week; docs/ARCHITECTURE.md 4.7). */
  readonly periodDays: number;
  /** RESOLUTION: settlement cycles per period (G1). */
  readonly cyclesPerPeriod: number;
}

export class Calendar {
  readonly epoch: Civil;
  readonly periodDays: number;
  readonly cyclesPerPeriod: number;
  private readonly epochDay: number;
  /**
   * Law 18: the first and last day of a period, worked out once each. A `Civil` is frozen and a
   * period's dates cannot move, so the same date handed out twice is the same date — and almost
   * everything that reads a date asks for one of these two. Building them afresh every ask was the
   * calendar arithmetic of a whole world done again for an answer that had not changed.
   */
  private readonly starts = new Map<Period, Civil>();
  private readonly ends = new Map<Period, Civil>();

  constructor(spec: CalendarSpec) {
    this.epoch = spec.epoch;
    this.periodDays = positiveCount(spec.periodDays, 'periodDays');
    this.cyclesPerPeriod = positiveCount(spec.cyclesPerPeriod, 'cyclesPerPeriod');
    if (this.cyclesPerPeriod < 2) {
      throw new Impossible('Money G1', 'a period contains more than one settlement cycle');
    }
    this.epochDay = dayNumber(spec.epoch);
  }

  /** The first day of a period. */
  startOf(p: Period): Civil {
    const held = this.starts.get(p);
    if (held !== undefined) return held;
    const made = fromDayNumber(this.epochDay + p * this.periodDays);
    this.starts.set(p, made);
    return made;
  }

  /** The last day of a period. */
  endOf(p: Period): Civil {
    const held = this.ends.get(p);
    if (held !== undefined) return held;
    const made = fromDayNumber(this.epochDay + (p + 1) * this.periodDays - 1);
    this.ends.set(p, made);
    return made;
  }

  /**
   * The period containing a date; a date before the epoch is not on the grid.
   *
   * G3.a: a dated obligation lands in the period that contains it, and this is the one function
   * that says so. `place` was the same function under a second name, with a doc describing a rule
   * ("the first period on or after its date") that it did not compute — two names for one fact, so
   * a reader could not tell whether they were two questions (Law 4, item 13b.1).
   */
  periodOf(c: Civil): Period {
    const d = dayNumber(c) - this.epochDay;
    if (d < 0) throw new Impossible('Money G3', 'date before the epoch', { date: c });
    return period(Math.floor(d / this.periodDays));
  }

  cycle(n: number): Cycle {
    if (!Number.isInteger(n) || n < 0 || n >= this.cyclesPerPeriod) {
      throw new Impossible('Money G2', `cycle ${n} is not in [0, ${this.cyclesPerPeriod})`);
    }
    return n as Cycle;
  }

  get lastCycle(): Cycle {
    return (this.cyclesPerPeriod - 1) as Cycle;
  }

  /**
   * Advance a date by one periodicity. Nothing finer than a period can be placed (G3.b), so a
   * per-period periodicity advances by the period length.
   */
  advance(c: Civil, per: Periodicity): Civil {
    switch (per.kind) {
      case 'annual':
        return addMonths(c, 12);
      case 'months':
        return addMonths(c, per.n);
      case 'perPeriod':
        return addDays(c, this.periodDays);
    }
  }

  /**
   * The schedule of dates after `start` up to and including `end` at a periodicity, generated by
   * advancing the start date k times (G3.a) so month-end days do not drift. The final date is `end`
   * itself when the grid does not land on it.
   */
  schedule(start: Civil, end: Civil, per: Periodicity): Civil[] {
    if (dayNumber(end) <= dayNumber(start)) {
      throw new Mismatch('Money G3.a', 'schedule end must be after start', { start, end });
    }
    const out: Civil[] = [];
    for (let k = 1; ; k += 1) {
      const next = this.advanceTimes(start, per, k);
      if (dayNumber(next) >= dayNumber(end)) {
        out.push(end);
        return out;
      }
      out.push(next);
    }
  }

  private advanceTimes(start: Civil, per: Periodicity, k: number): Civil {
    switch (per.kind) {
      case 'annual':
        return addMonths(start, 12 * k);
      case 'months':
        return addMonths(start, per.n * k);
      case 'perPeriod':
        return addDays(start, this.periodDays * k);
    }
  }
}
