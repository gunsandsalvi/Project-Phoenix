/**
 * The employment register: the relationship itself, recorded — as an AGREEMENT (item 9.1).
 *
 * @spec Labour A4 Labour A4.a Labour A4.b Labour A4.c Labour B3 Labour D1.c Labour D2 Labour F1 Labour F2 XI-8 XI-10 XI-15 Law 15 Law 19
 *
 * A firm, a worker, a wage, a start date (A4.a). The wage bill, the going rate, the unemployment
 * count and the separation flow are READS over these rows — not stored numbers — which is the whole
 * point of XI-10: without the row there is no contract for stickiness to be a consequence of, and
 * nothing a severance payment could sever.
 *
 * IT USED TO BE A PRIVATE BOOK IN `ctx.state`, and that was the defect item 9 is about. An
 * employment is a bilateral commitment — two named parties, dated terms, a state — and so is a
 * lease, an invoice, a stock loan, a covenant and a deal; seven modules each invented their own,
 * none of them visible to the kernel. Kept privately it ranked nowhere in an estate, no other
 * module could see who was employed (which is why `A-43` has the labour module building the
 * household's own schedule), and the going rate was a read over rows only labour could reach.
 *
 * The rows live in the kernel's agreement store now, of kind `labour.employment`, with everything
 * the kernel does not understand — the trade, the wage, the hours, the headcount — in this module's
 * own `EmploymentTerms`. What stays here is the INDEX, and that is the right division: how a module
 * finds a row is not something it knows (Observer E3, and this file already said so).
 *
 * Where the worker is a cell, the row carries the whole cell's weight as its headcount (A4.b), and a
 * hire or a separation that takes part of a cell splits it first (A4.c), so no cell is ever half
 * employed. That is what makes "employed plus unemployed plus inactive equals the population"
 * checkable rather than approximate (B5).
 */
import type { Period } from '../../calendar/calendar.js';
import {
  agreementKindId,
  type AgreementId,
  type PartyId,
  type RegionId,
} from '../../core/ids.js';
import { sum } from '../../core/num.js';
import { asRatio, type Cash, over, type PerPiece, scale, valueAt } from '../../core/measure.js';
import { NO_QTY, type Qty, scaleQty } from '../../core/tick.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { MechanismContext } from '../../world/context.js';

/** Labour A4, XI-10: one employment RELATIONSHIP, and it is a kind of agreement like the rest. */
export const EMPLOYMENT = agreementKindId('labour.employment');

/** The identity of a job is the identity of the commitment it is (XI-8: one row, one id). */
export type EmploymentId = AgreementId;

/** A4, D2: everything about the job the kernel has no business understanding (Law 15). */
export interface EmploymentTerms extends AgreementTerms {
  readonly kind: typeof EMPLOYMENT;
  readonly occupation: string;
  readonly region: RegionId;
  /** D2: the wage is the contract's, struck at the match; it moves only by renegotiation. */
  readonly wagePerHour: PerPiece;
  readonly hoursPerMember: Qty;
  readonly start: Period;
  /** C2: the period from which the person is productive — finding a job is not starting it. */
  readonly productiveFrom: Period;
  /** A4.b: the whole worker cell's weight, always an integer count of people. */
  readonly headcount: number;
}

/**
 * Law 15: the module that declared the kind narrows a row back to it, and the test is STRUCTURAL —
 * what makes these terms an employment is that they name a trade, an hour count and a headcount.
 */
export const isEmployment = (t: AgreementTerms): t is EmploymentTerms =>
  'occupation' in t && 'hoursPerMember' in t && 'headcount' in t;

/**
 * One employment, as this module reads it: the commitment's two named parties from the agreement,
 * its terms from here. It is a VIEW built at the read and never a second copy of the row (Law 19) —
 * the kernel's row is the source, and nothing here writes one except through `enter`, `restate`
 * and `leave` below.
 */
export interface EmploymentRow extends EmploymentTerms {
  readonly id: EmploymentId;
  /** F1: the named firm the job is at, and whose account the wage leaves. It is the DEBTOR. */
  readonly employer: PartyId;
  /** The named household cell whose members hold the job (A4.b: a cohort that can be told). */
  readonly worker: PartyId;
}

/** The one reader of an agreement as an employment; anything else asking is a defect in it. */
export function employmentOf(a: Agreement): EmploymentRow {
  if (!isEmployment(a.terms)) {
    throw new TypeError(`Labour A4: ${a.id} is not an employment`);
  }
  return { ...a.terms, id: a.id, employer: a.debtor, worker: a.creditor };
}

/** What a hire declares. The kernel's four are derived: the employer owes, the worker is owed. */
export interface Hiring extends Omit<EmploymentTerms, 'kind'> {
  readonly employer: PartyId;
  readonly worker: PartyId;
}

/**
 * What this module knows: the occupation each cell can work in — which is the job it last held. A
 * person with no history can enter any occupation, at the bottom (A3.b); one with a trade looks for
 * that trade.
 */
export interface EmploymentBook {
  skill: Record<string, string>;
}

export function emptyBook(): EmploymentBook {
  return { skill: {} };
}

/**
 * Law 18: THE ROWS UNDER A DIFFERENT ARRANGEMENT, and nothing below changes what a read returns.
 * Every question this module asks is about ONE worker or ONE trade, and each was answered by walking
 * every employment relationship in the world: the labour venue asked, for each household cell,
 * whether any of the rows was that cell's, so a world of more people employed at more firms cost
 * every one of them the whole register. It was the largest single cost of a period.
 *
 * These lists hold IDS, and every read resolves them against the kernel's store — so this is a
 * traversal and never a mirror (Law 19: a stale copy of a fact is the defect that rule is about).
 * They are written where the row is written, by `enter`, `restate` and `leave` and by nothing else,
 * so there is one writer of the fact. They live beside the book rather than in it because a
 * module's state slot is what the module KNOWS, and how it finds a row is not something it knows
 * (Observer E3).
 */
interface Index {
  readonly byWorker: Map<PartyId, EmploymentId>;
  readonly byTrade: Map<string, EmploymentId[]>;
  readonly byEmployer: Map<string, EmploymentId[]>;
}

const indexes = new WeakMap<EmploymentBook, Index>();

const SEP = String.fromCharCode(0);
const tradeKey = (occupation: string, region: RegionId): string => `${occupation}${SEP}${region}`;
const employerKey = (employer: PartyId, occupation: string, region: RegionId): string =>
  `${employer}${SEP}${occupation}${SEP}${region}`;

function indexOf(ctx: MechanismContext, book: EmploymentBook): Index {
  const held = indexes.get(book);
  if (held !== undefined) return held;
  const made: Index = { byWorker: new Map(), byTrade: new Map(), byEmployer: new Map() };
  indexes.set(book, made);
  // A module whose slot was made before the rows were: the index is rebuilt from the kernel's own
  // store, which is what makes it a traversal and not a second copy.
  for (const a of ctx.agreements.ofKind(EMPLOYMENT)) {
    if (a.state !== 'performing') continue;
    into(made, employmentOf(a));
  }
  return made;
}

function into(ix: Index, row: EmploymentRow): void {
  ix.byWorker.set(row.worker, row.id);
  list(ix.byTrade, tradeKey(row.occupation, row.region)).push(row.id);
  list(ix.byEmployer, employerKey(row.employer, row.occupation, row.region)).push(row.id);
}

function list(of: Map<string, EmploymentId[]>, key: string): EmploymentId[] {
  const held = of.get(key);
  if (held !== undefined) return held;
  const made: EmploymentId[] = [];
  of.set(key, made);
  return made;
}

function drop(of: Map<string, EmploymentId[]>, key: string, id: EmploymentId): void {
  const held = of.get(key);
  if (held === undefined) return;
  const at = held.indexOf(id);
  if (at >= 0) held.splice(at, 1);
}

const rowsFrom = (ctx: MechanismContext, ids: readonly EmploymentId[]): EmploymentRow[] =>
  ids.map((id) => employmentOf(ctx.agreements.get(id)));

const termsOf = (row: EmploymentRow): EmploymentTerms => ({
  kind: EMPLOYMENT,
  occupation: row.occupation,
  region: row.region,
  wagePerHour: row.wagePerHour,
  hoursPerMember: row.hoursPerMember,
  start: row.start,
  productiveFrom: row.productiveFrom,
  headcount: row.headcount,
});

/**
 * A4, XI-8: the row is written here and in two other places, `restate` and `leave`.
 *
 * An employment owes NOTHING the instant it is struck, and that is a real answer rather than an
 * absence: the wage falls due at the end of the period and is paid then, and a wage that does not
 * arrive is a `labour.wagesInArrears` row of its own. It is why the store admits a zero.
 */
export function enter(ctx: MechanismContext, book: EmploymentBook, d: Hiring): EmploymentRow {
  const terms: EmploymentTerms = {
    kind: EMPLOYMENT,
    occupation: d.occupation,
    region: d.region,
    wagePerHour: d.wagePerHour,
    hoursPerMember: d.hoursPerMember,
    start: d.start,
    productiveFrom: d.productiveFrom,
    headcount: d.headcount,
  };
  const row = employmentOf(
    ctx.owes({
      debtor: d.employer,
      creditor: d.worker,
      ccy: ctx.registry.currencyOf(d.region),
      owed: 0,
      terms,
      why: `${d.worker} works for ${d.employer} as a ${d.occupation}`,
    }),
  );
  into(indexOf(ctx, book), row);
  return row;
}

/**
 * D2, A4.c: the terms changed and the commitment did not — a renegotiated wage, or a worker cell
 * that split so fewer of its members are on this row. Same two parties, same id.
 */
export function restate(
  ctx: MechanismContext,
  book: EmploymentBook,
  row: EmploymentRow,
  to: Partial<Omit<EmploymentTerms, 'kind'>>,
): EmploymentRow {
  const next = employmentOf(ctx.restate(row.id, { ...termsOf(row), ...to }));
  // The trade or the region could have moved with the terms, so the row leaves its old lists and
  // joins its new ones — one writer, and no list can hold an id under a key that is not true.
  const ix = indexOf(ctx, book);
  drop(ix.byTrade, tradeKey(row.occupation, row.region), row.id);
  drop(ix.byEmployer, employerKey(row.employer, row.occupation, row.region), row.id);
  into(ix, next);
  return next;
}

/**
 * C3: the relationship ends, and every arrangement of it ends in the same call.
 *
 * XI-8: it is TERMINATED and not discharged — the commitment ended by its own terms, and the row
 * stays in the kernel's book saying so. A job that vanished from the record would leave a severance
 * nothing could be a severance FROM.
 */
export function leave(
  ctx: MechanismContext,
  book: EmploymentBook,
  row: EmploymentRow,
  why: string,
): void {
  ctx.endAgreement(row.id, why);
  const ix = indexOf(ctx, book);
  if (ix.byWorker.get(row.worker) === row.id) ix.byWorker.delete(row.worker);
  drop(ix.byTrade, tradeKey(row.occupation, row.region), row.id);
  drop(ix.byEmployer, employerKey(row.employer, row.occupation, row.region), row.id);
}

/**
 * Every row, as a list taken now. It is a copy of the LIST and not of the rows: a separation during
 * a walk over it removes the row from the index, and the walk still sees the relationship it was
 * about to end.
 */
export function allRows(ctx: MechanismContext, book: EmploymentBook): EmploymentRow[] {
  const out: EmploymentRow[] = [];
  for (const ids of indexOf(ctx, book).byTrade.values()) out.push(...rowsFrom(ctx, ids));
  return out;
}

/** The row a cell holds, if it holds one: a person is in exactly one state (B3). */
export function rowOfWorker(
  ctx: MechanismContext,
  book: EmploymentBook,
  worker: PartyId,
): EmploymentRow | undefined {
  const id = indexOf(ctx, book).byWorker.get(worker);
  return id === undefined ? undefined : employmentOf(ctx.agreements.get(id));
}

/** Hours an employer has under contract in one occupation and region (its current employment). */
export function hoursAt(
  ctx: MechanismContext,
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
): Qty {
  const ids = indexOf(ctx, book).byEmployer.get(employerKey(employer, occupation, region));
  if (ids === undefined) return NO_QTY;
  return sum(
    rowsFrom(ctx, ids).map((r) => scaleQty(r.hoursPerMember, r.headcount, 'hours under contract')),
  ).value;
}

/** The rows in one trade in one place: what a going rate averages and what a cut sheds. */
export function rowsInTrade(
  ctx: MechanismContext,
  book: EmploymentBook,
  occupation: string,
  region: RegionId,
): readonly EmploymentRow[] {
  return rowsFrom(ctx, indexOf(ctx, book).byTrade.get(tradeKey(occupation, region)) ?? []);
}

/** The rows one employer holds in one trade in one place, most recently hired first (C3). */
export function rowsAt(
  ctx: MechanismContext,
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
): readonly EmploymentRow[] {
  return rowsFrom(
    ctx,
    indexOf(ctx, book).byEmployer.get(employerKey(employer, occupation, region)) ?? [],
  );
}

/**
 * D1.c: the going rate is the employment-weighted average of what is actually paid — a read over
 * the rows, never a series anybody writes. An occupation nobody is employed in has no going rate.
 */
export function goingRate(
  ctx: MechanismContext,
  book: EmploymentBook,
  occupation: string,
  region: RegionId,
): PerPiece | undefined {
  const rows = rowsInTrade(ctx, book, occupation, region);
  const heads = sum(rows.map((r) => r.headcount));
  if (heads.value === 0) return undefined;
  const paid = sum(
    rows.map((r) => scale(r.wagePerHour, asRatio(r.headcount, 'the people on it'), 'wage weight')),
  );
  return over(paid.value, asRatio(heads.value, 'the people employed'), 'going rate');
}

/** F2: the headcount employed, which is a count of people and can never exceed the workforce. */
export function employed(ctx: MechanismContext, book: EmploymentBook): number {
  return sum(allRows(ctx, book).map((r) => r.headcount)).value;
}

/** What the employer owes this period on one row: the wage, per member of the worker cell. */
export function wagePerMember(row: EmploymentRow): Cash {
  return valueAt(row.wagePerHour, row.hoursPerMember, 'wage per member');
}
