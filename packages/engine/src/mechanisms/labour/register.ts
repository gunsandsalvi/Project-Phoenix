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
import { moduleKey, type ModuleKey, type PartyId, type RegionId } from '../../core/ids.js';
import { sum } from '../../core/num.js';
import { asRatio, type Cash, over, type PerPiece, scale, valueAt } from '../../core/measure.js';
import { NO_QTY, type Qty, scaleQty } from '../../core/tick.js';

/**
 * Labour A4, XI-10: the identity of one employment RELATIONSHIP. It is the labour module's own —
 * nothing outside labour names a job — so it is branded here rather than in `core/ids.ts`, which
 * is what `ModuleKey` is for (ARCHITECTURE 4.9b).
 */
export type EmploymentId = ModuleKey<'Employment'>;
export const employmentId = (s: string): EmploymentId => moduleKey(s, 'Employment');

export interface EmploymentRow {
  readonly id: EmploymentId;
  /** F1: the named firm the job is at, and whose account the wage leaves. */
  readonly employer: PartyId;
  /** The named household cell whose members hold the job (A4.b: a cohort that can be told). */
  readonly worker: PartyId;
  readonly occupation: string;
  readonly region: RegionId;
  /** D2: the wage is the contract's, struck at the match; it moves only by renegotiation. */
  readonly wagePerHour: PerPiece;
  readonly hoursPerMember: Qty;
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
  readonly rows: Map<EmploymentId, EmploymentRow>;
  skill: Record<string, string>;
  next: number;
}

export function emptyBook(): EmploymentBook {
  return { rows: new Map(), skill: {}, next: 1 };
}

/**
 * Law 18: THE SAME ROWS UNDER A DIFFERENT ARRANGEMENT, and nothing below changes what a read
 * returns. Every question this module asks is about ONE worker or ONE trade, and each was answered
 * by walking every employment relationship in the world: the labour venue asked, for each household
 * cell, whether any of the rows was that cell's, so a world of more people employed at more firms
 * cost every one of them the whole register. It was the largest single cost of a period.
 *
 * These lists hold the same row objects the book holds — a reference, never a copy of a number — and
 * they are written where the row is written, by `enter` and `leave` below and by nothing else, so
 * there is one writer of the fact and nothing here can go stale (Law 4). They live beside the book
 * rather than in it because a module's state slot is what the module KNOWS, and how it finds a row
 * is not something it knows (Observer E3).
 */
interface Index {
  readonly byWorker: Map<PartyId, EmploymentRow>;
  readonly byTrade: Map<string, EmploymentRow[]>;
  readonly byEmployer: Map<string, EmploymentRow[]>;
}

const indexes = new WeakMap<EmploymentBook, Index>();

const tradeKey = (occupation: string, region: RegionId): string => `${occupation}\u0000${region}`;
const employerKey = (employer: PartyId, occupation: string, region: RegionId): string =>
  `${employer}\u0000${occupation}\u0000${region}`;

function indexOf(book: EmploymentBook): Index {
  const held = indexes.get(book);
  if (held !== undefined) return held;
  const made: Index = { byWorker: new Map(), byTrade: new Map(), byEmployer: new Map() };
  indexes.set(book, made);
  for (const row of book.rows.values()) into(made, row);
  return made;
}

function into(ix: Index, row: EmploymentRow): void {
  ix.byWorker.set(row.worker, row);
  list(ix.byTrade, tradeKey(row.occupation, row.region)).push(row);
  list(ix.byEmployer, employerKey(row.employer, row.occupation, row.region)).push(row);
}

function list(of: Map<string, EmploymentRow[]>, key: string): EmploymentRow[] {
  const held = of.get(key);
  if (held !== undefined) return held;
  const made: EmploymentRow[] = [];
  of.set(key, made);
  return made;
}

function drop(of: Map<string, EmploymentRow[]>, key: string, row: EmploymentRow): void {
  const held = of.get(key);
  if (held === undefined) return;
  const at = held.indexOf(row);
  if (at >= 0) held.splice(at, 1);
}

/** A4: the row is written here and in one other place, and that place is `leave`. */
export function enter(book: EmploymentBook, row: EmploymentRow): void {
  book.rows.set(row.id, row);
  into(indexOf(book), row);
}

/** C3: the relationship ends, and every arrangement of it ends in the same call. */
export function leave(book: EmploymentBook, row: EmploymentRow): void {
  book.rows.delete(row.id);
  const ix = indexOf(book);
  if (ix.byWorker.get(row.worker) === row) ix.byWorker.delete(row.worker);
  drop(ix.byTrade, tradeKey(row.occupation, row.region), row);
  drop(ix.byEmployer, employerKey(row.employer, row.occupation, row.region), row);
}

/**
 * Every row, as a list taken now. It is a copy of the LIST and not of the rows: a separation during
 * a walk over it removes the row from the book, and the walk still sees the relationship it was
 * about to end.
 */
export function allRows(book: EmploymentBook): EmploymentRow[] {
  return [...book.rows.values()];
}

/** The row a cell holds, if it holds one: a person is in exactly one state (B3). */
export function rowOfWorker(book: EmploymentBook, worker: PartyId): EmploymentRow | undefined {
  return indexOf(book).byWorker.get(worker);
}

/** Hours an employer has under contract in one occupation and region (its current employment). */
export function hoursAt(
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
): Qty {
  const rows = indexOf(book).byEmployer.get(employerKey(employer, occupation, region));
  if (rows === undefined) return NO_QTY;
  return sum(rows.map((r) => scaleQty(r.hoursPerMember, r.headcount, 'hours under contract'))).value;
}

/** The rows in one trade in one place: what a going rate averages and what a cut sheds. */
export function rowsInTrade(
  book: EmploymentBook,
  occupation: string,
  region: RegionId,
): readonly EmploymentRow[] {
  return indexOf(book).byTrade.get(tradeKey(occupation, region)) ?? [];
}

/** The rows one employer holds in one trade in one place, most recently hired first (C3). */
export function rowsAt(
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
): readonly EmploymentRow[] {
  return indexOf(book).byEmployer.get(employerKey(employer, occupation, region)) ?? [];
}

/**
 * D1.c: the going rate is the employment-weighted average of what is actually paid — a read over
 * the rows, never a series anybody writes. An occupation nobody is employed in has no going rate.
 */
export function goingRate(
  book: EmploymentBook,
  occupation: string,
  region: RegionId,
): PerPiece | undefined {
  const rows = rowsInTrade(book, occupation, region);
  const heads = sum(rows.map((r) => r.headcount));
  if (heads.value === 0) return undefined;
  const paid = sum(
    rows.map((r) => scale(r.wagePerHour, asRatio(r.headcount, 'the people on it'), 'wage weight')),
  );
  return over(paid.value, asRatio(heads.value, 'the people employed'), 'going rate');
}

/** F2: the headcount employed, which is a count of people and can never exceed the workforce. */
export function employed(book: EmploymentBook): number {
  const heads: number[] = [];
  for (const row of book.rows.values()) heads.push(row.headcount);
  return sum(heads).value;
}

/** What the employer owes this period on one row: the wage, per member of the worker cell. */
export function wagePerMember(row: EmploymentRow): Cash {
  return valueAt(row.wagePerHour, row.hoursPerMember, 'wage per member');
}
