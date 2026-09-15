/**
 * The employment register: ONE place that says who works where, for how many hours, at what wage,
 * since when, and on what notice (12b.1).
 *
 * @spec Labour A4 Labour A4.a Labour A4.b Labour A4.c Labour B3 Labour D1.c Labour D2 Labour E1 Labour F1 Labour F2 XI-8 XI-10 XI-15 Law 4 Law 15 Law 19
 *
 * An employment is a bilateral commitment — a named employer owes a named worker cell a wage for
 * hours, from a date — and it is a KIND of agreement, held in the kernel's agreement store beside
 * the lease, the invoice and the rest (item 9.1). What was missing was the READ: the rows were the
 * kernel's, and the index over them, the wage bill, the going rate and the headcount were a labour
 * module's private book (`labour/register.ts`) that nobody else could reach — so every other reader
 * of "what does this party pay its people" read a TALLY the labour module published once a period
 * (`labour.wages`: hours, due, paid, productive, headcount per employer), a stored aggregate of the
 * rows re-derived every period and left standing when the rows changed (Law 19: read the source;
 * Appendix B: no stored aggregate). A bank's staffing, a firm's payroll, the treasury's public
 * service and a small firm's own wage bill all read that copy.
 *
 * This file is the home of the noun. The rows stay where they are — the kernel's agreement store
 * indexes them by debtor, creditor and kind, and succeeds them when a party ceases (Register F2) —
 * and what is here is the KIND, its TERMS, and every read a mechanism has over them, built at the
 * read and never a second copy. Wages are instructions that READ it (`labour/matching.ts
 * payWages`); what was PAID is the ledger's, not a number beside the row.
 */
import type { Period } from '../calendar/calendar.js';
import { agreementKindId, type AgreementId, type PartyId, type RegionId } from '../core/ids.js';
import { asRatio, type Cash, over, type PerPiece, scale, valueAt, plus, asCash } from '../core/measure.js';
import { sum } from '../core/num.js';
import { addQty, NO_QTY, type Qty, scaleQty } from '../core/tick.js';
import type { Agreement, AgreementKindDecl, AgreementTerms, Agreements } from './agreements.js';

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
  /** A4.a: the period the job started. */
  readonly since: Period;
  /** C2: the period from which the person is productive — finding a job is not starting it. */
  readonly productiveFrom: Period;
  /**
   * C3 (12b.1, run at 12b.2): the NOTICE the job carries — the periods of wages a separation owes
   * before it ends. A term of the contract, struck at the match like the wage.
   */
  readonly notice: number;
  /** A4.b: the whole worker cell's weight, always an integer count of people. */
  readonly headcount: number;
}

/**
 * Law 15: a row is narrowed back to an employment STRUCTURALLY — what makes these terms an
 * employment is that they name a trade, an hour count and a headcount.
 */
export const isEmployment = (t: AgreementTerms): t is EmploymentTerms =>
  'occupation' in t && 'hoursPerMember' in t && 'headcount' in t;

/**
 * One employment, as every reader sees it: the commitment's two named parties from the agreement,
 * its terms from the kind. It is a VIEW built at the read and never a second copy of the row.
 */
export interface EmploymentRow extends EmploymentTerms {
  readonly id: EmploymentId;
  /** F1: the named employer the job is at, and whose account the wage leaves. It is the DEBTOR. */
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

/**
 * XI-8, Register F2: the kind, declared by the kernel because the kernel is where it is read. A
 * job is somebody's to do: an acquirer that bought the book employs the staff; an estate has no
 * work to give and the employment ends with the employer.
 */
export const employmentKind: AgreementKindDecl = {
  id: EMPLOYMENT,
  what: 'a named worker working for a named employer, at a wage, in a trade',
  binds: 'aGoingConcern',
};

/** What the employer owes this period on one row: the wage, per member of the worker cell. */
export function wagePerMember(row: EmploymentRow): Cash {
  return valueAt(row.wagePerHour, row.hoursPerMember, 'wage per member');
}

/** E1, F1: an employer's payroll as its rows say it — never a number kept beside them. */
export interface Payroll {
  /** The hours it has under contract, over every row. */
  readonly hours: Qty;
  /** What the rows say it owes for them this period. */
  readonly due: Cash;
  /** C2: the hours that can make something — somebody found last period is paid and not yet working. */
  readonly productive: Qty;
  readonly headcount: number;
}

/**
 * The reads, and there is one set of them. Every question a mechanism has about employment is
 * about ONE worker, ONE employer or ONE trade, and each is answered off the store's own indexes
 * (Law 18) — the rows are the kernel's, so the index is the kernel's too.
 */
export interface EmploymentReads {
  get(id: EmploymentId): EmploymentRow;
  /** B3: the one live row a cell holds, if it holds one — a person is in exactly one state. */
  ofWorker(cell: PartyId): EmploymentRow | undefined;
  /** F1: the live rows of one employer, wherever and whatever the trade. */
  by(employer: PartyId): readonly EmploymentRow[];
  /** The live rows one employer holds in one trade in one place, most recently hired first (C3). */
  at(employer: PartyId, occupation: string, region: RegionId): readonly EmploymentRow[];
  /** The live rows in one trade in one place: what a going rate averages and what a cut sheds. */
  inTrade(occupation: string, region: RegionId): readonly EmploymentRow[];
  /** Every live row, as a list taken now. */
  all(): readonly EmploymentRow[];
  /** Hours an employer has under contract in one occupation and region. */
  hoursAt(employer: PartyId, occupation: string, region: RegionId): Qty;
  /**
   * D1.c: the going rate is the employment-weighted average of what is actually paid — a read over
   * the rows, never a series anybody writes. An occupation nobody is employed in has no going rate.
   */
  goingRate(occupation: string, region: RegionId): PerPiece | undefined;
  /** F2: the headcount employed, which is a count of people and can never exceed the workforce. */
  employed(): number;
  /** E1: an employer's payroll off its rows, at a period (which rows are productive by then). */
  payrollOf(employer: PartyId, at: Period): Payroll;
  /** Whether this party has ever employed anybody — a row of its, live or ended. */
  everEmployed(employer: PartyId): boolean;
}

const live = (a: Agreement): boolean => a.state === 'performing' || a.state === 'breached';

export function employmentReads(
  store: Pick<Agreements, 'get' | 'owedBy' | 'owedTo' | 'ofKind' | 'byDebtorAndKind'>,
): EmploymentReads {
  const rowsOf = (rows: readonly Agreement[]): EmploymentRow[] =>
    rows.filter((a) => live(a) && isEmployment(a.terms)).map(employmentOf);
  const by = (employer: PartyId): EmploymentRow[] => rowsOf(store.byDebtorAndKind(employer, EMPLOYMENT));
  const at = (employer: PartyId, occupation: string, region: RegionId): EmploymentRow[] =>
    by(employer)
      .filter((r) => r.occupation === occupation && r.region === region)
      .sort((a, b) => b.since - a.since);
  const inTrade = (occupation: string, region: RegionId): EmploymentRow[] =>
    rowsOf(store.ofKind(EMPLOYMENT)).filter((r) => r.occupation === occupation && r.region === region);
  const all = (): EmploymentRow[] => rowsOf(store.ofKind(EMPLOYMENT));
  return Object.freeze({
    get: (id: EmploymentId) => employmentOf(store.get(id)),
    ofWorker: (cell: PartyId) => rowsOf(store.owedTo(cell))[0],
    by,
    at,
    inTrade,
    all,
    hoursAt: (employer: PartyId, occupation: string, region: RegionId): Qty =>
      sum(at(employer, occupation, region).map((r) => scaleQty(r.hoursPerMember, r.headcount, 'hours under contract'))).value,
    goingRate: (occupation: string, region: RegionId): PerPiece | undefined => {
      const rows = inTrade(occupation, region);
      const heads = sum(rows.map((r) => r.headcount));
      if (heads.value === 0) return undefined;
      const paid = sum(
        rows.map((r) => scale(r.wagePerHour, asRatio(r.headcount, 'the people on it'), 'wage weight')),
      );
      return over(paid.value, asRatio(heads.value, 'the people employed'), 'going rate');
    },
    employed: (): number => sum(all().map((r) => r.headcount)).value,
    payrollOf: (employer: PartyId, at: Period): Payroll => {
      let hours = NO_QTY;
      let productive = NO_QTY;
      let due = asCash(0, 'nothing due yet');
      let headcount = 0;
      for (const r of by(employer)) {
        const rowHours = scaleQty(r.hoursPerMember, r.headcount, 'hours under contract');
        hours = addQty(hours, rowHours, 'hours');
        if (r.productiveFrom <= at) productive = addQty(productive, rowHours, 'productive hours');
        due = plus(due, scale(wagePerMember(r), asRatio(r.headcount, 'the people on it'), 'wage bill'), 'wages due');
        headcount += r.headcount;
      }
      return { hours, due, productive, headcount };
    },
    everEmployed: (employer: PartyId): boolean =>
      store.byDebtorAndKind(employer, EMPLOYMENT).some((a) => isEmployment(a.terms)),
  });
}
