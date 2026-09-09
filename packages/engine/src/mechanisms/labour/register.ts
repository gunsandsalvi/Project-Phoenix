/**
 * The employment register: the relationship itself, recorded.
 *
 * @spec Labour A4 Labour A4.a Labour A4.b Labour A4.c Labour B3 Labour D1.c Labour F1 Labour F2 XI-10 XI-15 Law 19
 *
 * A firm, a worker, a wage, a start date (A4.a). The wage bill, the going rate, the unemployment
 * count and the separation flow are READS over these rows — not stored numbers — which is the whole
 * point of XI-10: without the row there is no contract for stickiness to be a consequence of, and
 * nothing a severance payment could sever.
 *
 * Where the worker is a cell, the row carries the whole cell's weight as its headcount (A4.b), and a
 * hire or a separation that takes part of a cell splits it first (A4.c), so no cell is ever half
 * employed. That is what makes "employed plus unemployed plus inactive equals the population"
 * checkable rather than approximate (B5).
 */
import type { Period } from '../../calendar/calendar.js';
import type { PartyId, RegionId } from '../../core/ids.js';
import { div, mul, sum } from '../../core/num.js';

export interface EmploymentRow {
  readonly id: string;
  /** F1: the named firm the job is at, and whose account the wage leaves. */
  readonly employer: PartyId;
  /** The named household cell whose members hold the job (A4.b: a cohort that can be told). */
  readonly worker: PartyId;
  readonly occupation: string;
  readonly region: RegionId;
  /** D2: the wage is the contract's, struck at the match; it moves only by renegotiation. */
  readonly wagePerHour: number;
  readonly hoursPerMember: number;
  readonly start: Period;
  /** C2: the period from which the person is productive — finding a job is not starting it. */
  readonly productiveFrom: Period;
  /** A4.b: the whole worker cell's weight, always an integer count of people. */
  headcount: number;
}

/**
 * What this module knows: the rows, and the occupation each cell can work in — which is the job it
 * last held. A person with no history can enter any occupation, at the bottom (A3.b); one with a
 * trade looks for that trade.
 */
export interface EmploymentBook {
  rows: Record<string, EmploymentRow>;
  skill: Record<string, string>;
  next: number;
}

export function emptyBook(): EmploymentBook {
  return { rows: {}, skill: {}, next: 1 };
}

export function allRows(book: EmploymentBook): EmploymentRow[] {
  return Object.values(book.rows);
}

/** The row a cell holds, if it holds one: a person is in exactly one state (B3). */
export function rowOfWorker(book: EmploymentBook, worker: PartyId): EmploymentRow | undefined {
  return allRows(book).find((r) => r.worker === worker);
}

/** Hours an employer has under contract in one occupation and region (its current employment). */
export function hoursAt(
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
): number {
  return sum(
    allRows(book)
      .filter((r) => r.employer === employer && r.occupation === occupation && r.region === region)
      .map((r) => mul(r.headcount, r.hoursPerMember, 'hours under contract')),
  ).value;
}

/**
 * D1.c: the going rate is the employment-weighted average of what is actually paid — a read over
 * the rows, never a series anybody writes. An occupation nobody is employed in has no going rate.
 */
export function goingRate(
  book: EmploymentBook,
  occupation: string,
  region: RegionId,
): number | undefined {
  const rows = allRows(book).filter((r) => r.occupation === occupation && r.region === region);
  const heads = sum(rows.map((r) => r.headcount));
  if (heads.value === 0) return undefined;
  const paid = sum(rows.map((r) => mul(r.wagePerHour, r.headcount, 'wage weight')));
  return div(paid.value, heads.value, 'going rate');
}

/** F2: the headcount employed, which is a count of people and can never exceed the workforce. */
export function employed(book: EmploymentBook): number {
  return sum(allRows(book).map((r) => r.headcount)).value;
}

/** What the employer owes this period on one row: the wage, per member of the worker cell. */
export function wagePerMember(row: EmploymentRow): number {
  return mul(row.wagePerHour, row.hoursPerMember, 'wage per member');
}
