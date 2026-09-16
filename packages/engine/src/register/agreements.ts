/**
 * Agreements: what one party owes another that is not a tradeable instrument.
 *
 * @spec XI-8 Money E1 Money D3 Law 5 Law 15 Appendix B
 *
 * An employment, a lease, an invoice, a repo, a stock loan, an insurance policy, a mandate and an
 * overdraft facility are ONE NOUN: two named parties, dated terms, and a state. Seven modules each
 * invented their own book of them in `ctx.state`, none visible to the kernel, so there was no answer
 * to "what does this party owe that is not an instrument" — and the answer to that question is what
 * an estate divides.
 *
 * WHAT ITS ABSENCE COST, and this store exists for these four before anything else:
 *
 *  - an unpaid wage and an unpaid severance leave no obligation anywhere, and the record says
 *    NOTHING IS OWED (A-41);
 *  - a probate transfer that fails becomes an unhandled throw, because the unpaid part has nowhere
 *    to live (A-20);
 *  - a central bank's loss is forgotten at the next remittance, and the comment says it is carried
 *    (A-62);
 *  - a levy that fails is recorded on `treasury.receipts` as `unpaid` and nothing carries it: the
 *    cell does not owe it next period and the treasury does not chase it (D-1).
 *
 * Each of those is the same shape. Money E1 says a payer that cannot pay has not paid, and D3 says
 * the flow has two sides — so what did not arrive is still owed BY somebody TO somebody, and this
 * is where that fact lives instead of evaporating between two balance sheets.
 *
 * IT IS NOT AN INSTRUMENT AND MUST NOT BECOME ONE. Nobody trades it, it has no issued quantity and
 * no holder: it is a relation between two named parties (Law 5). The register holds what is OWNED;
 * this holds what is OWED where the owing is not a security.
 */
import { none, some, type Option } from '../core/option.js';
import { type Cash, type PerPiece } from '../core/measure.js';
import { type Outlook, type OutlookVariable } from '../world/context.js';
import { type Civil } from '../calendar/civil.js';
import { type CurveRead } from '../prices/curve.js';
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import {
  agreementId,
  type AgreementId,
  type AgreementKindId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type CurveFamilyId,
  type RegionId,
} from '../core/ids.js';
import type { Qty } from '../core/tick.js';
import type { ParamRegister } from '../registry/params.js';
import { Missing } from '../core/errors.js';
import { finite } from '../core/num.js';

/**
 * Law 15: the lifecycle of a commitment, and it is the same four for every kind of one.
 *
 * `performing` is the ordinary course. `breached` is a payment that did not arrive and stands
 * unpaid — the state Money E1 describes and which nothing could hold. `discharged` is paid in full.
 * `terminated` is ended by its own terms or by an estate closing, with something still owed and
 * nobody left to pay it: a write-off, and it says so rather than quietly becoming a discharge.
 */
export type AgreementState = 'performing' | 'breached' | 'discharged' | 'terminated';

/**
 * Law 15, item 9.1: WHAT THIS COMMITMENT IS AND WHAT ITS OWN TERMS ARE — the same construction
 * `Terms` is for an instrument and `Contract['terms']` for a contract, for the same reason.
 *
 * Every agreement shares four things: two named parties, a money, what is owed now and a state.
 * Everything else belongs to the KIND — a wage and a notice period, a rent and a term, a line and a
 * covenant test, a pool and what a mandate may hold — and only the module that declared the kind
 * can read it. The kernel narrows nothing and branches on nothing: it holds the row, indexes it,
 * ranks it in an estate, and hands the terms back to whoever knows what they mean.
 *
 * A module extends this with its own interface and its own `kind`, and narrows a row back to it
 * with its own type predicate — `isWagesOwed(t): t is WagesOwed` — exactly as it does for an
 * instrument's `Terms` (`isShare`, `isPolicy`, `isRow`). The narrowing belongs to the module that
 * declared the kind, because only it knows what would make a row really be one.
 */
export interface AgreementTerms {
  readonly kind: AgreementKindId;
}

/** What a module declares about a kind of agreement, so an undeclared kind cannot be opened. */
export interface AgreementKindDecl {
  readonly id: AgreementKindId;
  /** What a commitment of this kind IS, for a reader and for the estate's report (Law 16). */
  readonly what: string;
  /**
   * Register F2, XI-8, Law 15: WHO A COMMITMENT OF THIS KIND STILL BINDS once the party that made
   * it has ceased — the one question a succession has to answer, asked of the kind rather than
   * branched on by the kernel.
   *
   * `whoeverSucceeds` is a DEBT: what is owed is owed, and it passes to whoever succeeded the name.
   * That is what an estate divides, and a wage, a levy, a declared dividend and a rating fee are
   * each one of these.
   *
   * `aGoingConcern` is a RELATIONSHIP: a commitment to keep DOING something — running a pool,
   * employing a worker, standing behind a line, holding a client's book. A bank whose book an
   * acquirer bought keeps every one of them, because that is what buying a book IS. An estate keeps
   * none: it is where the chain of successors ends and its office is to realise what is there, not
   * to run it (XI-8), so a relationship that reaches one ends there with what it still owed
   * recorded as the write-off it is.
   */
  readonly binds: 'whoeverSucceeds' | 'aGoingConcern';
  /**
   * Insurers B1, B2, B2.a (14.5): WHAT A LIVE ROW OF THIS KIND IS WORTH NOW, in its own money — a
   * schedule of what is owed in future periods, discounted at a rate read from a market. A kind
   * that answers is marked by the kernel at every revaluation: the creditor's asset and the
   * debtor's liability move by the same amount in the same pass, so falling rates raise a promise
   * and lower the promiser's equity (B2.a) with nothing stored beside the row but its last mark.
   * A kind that does not answer is carried at what is owed and is never re-marked (a wage, a levy).
   */
  readonly valued?: (row: Agreement, at: Period, reads: RowValuationReads) => Cash;
  /**
   * Banks Lending A3.a, A3.b (17.3): WHAT IS PROMISED ON THIS ROW AND NOT YET DRAWN — the headroom
   * a committed facility leaves. A creditor's capital has to stand behind it before the borrower
   * draws, because the creditor cannot refuse when it does; a kind that is not a commitment answers
   * nothing and nothing stands behind it. The kind answers because only the kind knows what its own
   * limit is and what drawing on it looks like (Law 15).
   */
  readonly headroom?: (row: Agreement, at: Period, reads: RowValuationReads) => Cash;
}

/** Insurers B2 (14.5): what the kernel lends a kind that values its rows — a curve, a day, a party's outlook. */
export interface RowValuationReads {
  curve(family: CurveFamilyId, at: Period): CurveRead;
  on(at: Period): Civil;
  outlook(party: PartyId, variable: OutlookVariable): Option<Outlook>;
  /** XI-15 (14.6): how many people a row's party stands for — a cell's weight, one for a named party. */
  weightOf(party: PartyId): number;
  /** Labour D1.c (14.6): what an hour last cleared at in a trade and place, for a promise indexed to it. */
  goingRate(occupation: string, region: RegionId): PerPiece | undefined;
  /**
   * Banks Lending A3.a (17.3): what is OUTSTANDING of a line a kind knows the name of — how much of
   * a committed facility has been drawn, which lives where every other claim does (the register)
   * and is read there rather than mirrored onto the commitment row (Law 19).
   */
  drawnOn(instrument: InstrumentId): Qty;
  /** Money A3 (16.0): the money a place's wages clear in, for a promise indexed to them. */
  readonly registry: { currencyOf(region: RegionId): CurrencyCode };
  /** The scheme's declared rules and the households' table, for a schedule that reads them. */
  readonly params: Pick<ParamRegister, 'ratio' | 'amount' | 'all'>;
}

export interface AgreementDecl {
  /** Who owes. Both sides are named: an agreement with one side is not one (Law 5). */
  readonly debtor: PartyId;
  readonly creditor: PartyId;
  readonly ccy: CurrencyCode;
  /**
   * What is owed NOW, in the smallest piece of `ccy`. Zero is a real answer and not an absence: a
   * performing employment owes nothing this instant and is still an employment. It became a real
   * answer at item 9.1, when this store stopped being a book of arrears and became the book of
   * every bilateral commitment — before that, everything in it was a payment that had failed, and
   * `owed > 0` was true of all of them by construction.
   */
  readonly owed: number;
  /** Law 15: what sort of commitment, and everything about it the kernel does not understand. */
  readonly terms: AgreementTerms;
  /** Why it exists, for a reader (Law 16). */
  readonly why: string;
}

export interface Agreement extends AgreementDecl {
  readonly id: AgreementId;
  readonly since: Period;
  readonly state: AgreementState;
}

export class Agreements {
  private readonly rows = new Map<AgreementId, Agreement>();
  private readonly byDebtor = new Map<PartyId, Set<AgreementId>>();
  private readonly byCreditor = new Map<PartyId, Set<AgreementId>>();
  private readonly byKind = new Map<AgreementKindId, Set<AgreementId>>();
  private next = 1;
  /** Law 15: the kinds this world declared. An undeclared kind is refused where it is opened. */
  private readonly kinds: ReadonlyMap<AgreementKindId, AgreementKindDecl>;
  /** 14.5: what each valued row was last marked at — the kernel's, written by revaluation alone. */
  private readonly marks = new Map<AgreementId, number>();

  constructor(kinds: readonly AgreementKindDecl[] = []) {
    const m = new Map<AgreementKindId, AgreementKindDecl>();
    for (const k of kinds) {
      // Law 4: two modules claiming one kind of commitment is two owners of one fact, and the
      // second would silently decide what the first one's terms mean.
      forbid(!m.has(k.id), 'Law 4', `agreement kind ${k.id} declared twice`);
      m.set(k.id, k);
    }
    this.kinds = m;
  }

  /** What a kind of commitment IS, in the words the module that declared it used. */
  kind(id: AgreementKindId): AgreementKindDecl {
    const k = this.kinds.get(id);
    if (k === undefined) {
      throw new Missing('Law 15', `agreement kind ${id} has no declaration registered`, { id });
    }
    return k;
  }

  /**
   * Money E1, D3, Law 15: a commitment between two named parties, of a declared kind.
   *
   * It refuses an agreement with one party on both sides, one owing a negative amount, and one of a
   * kind no module declared. The first is not a commitment; the second is the other party's row
   * written backwards; the third is a mechanism naming a category nobody owns (Law 15) — which is
   * what the free-text `what` let it do, in six different spellings.
   */
  open(decl: AgreementDecl, at: Period): Agreement {
    const what = this.kind(decl.terms.kind).what;
    forbid(decl.debtor !== decl.creditor, 'Law 5', `${decl.debtor} cannot owe itself: ${what}`);
    // Law 6: what is owed is a count of pieces and a count is not negative. This is arithmetic
    // impossibility and not a floor — a party owing minus five is owed five, which is the OTHER
    // row, with the parties the other way round.
    forbid(finite(decl.owed, 'owed') >= 0, 'Money E1', `an agreement owing ${decl.owed}: ${what}`);
    const id = agreementId(`agreement.${this.next}`);
    this.next += 1;
    const row: Agreement = { ...decl, id, since: at, state: 'performing' };
    this.rows.set(id, Object.freeze(row));
    index(this.byDebtor, decl.debtor, id);
    index(this.byCreditor, decl.creditor, id);
    index(this.byKind, decl.terms.kind, id);
    return row;
  }

  get(id: AgreementId): Agreement {
    const row = this.rows.get(id);
    if (row === undefined) throw new Missing('XI-8', `no agreement ${id}`);
    return row;
  }

  /**
   * Part of what was owed has been paid. What is left is what is left; nothing is discharged by a
   * payment that did not cover it (Money E1).
   */
  paid(id: AgreementId, amount: number): Agreement {
    const row = this.get(id);
    forbid(row.state !== 'discharged', 'XI-8', `${id} is discharged and cannot be paid again`);
    forbid(row.state !== 'terminated', 'XI-8', `${id} is terminated and cannot be paid`);
    const left = finite(row.owed - finite(amount, 'paid'), 'what is left owed');
    forbid(left >= 0, 'Money E1', `${amount} paid against ${row.owed} owed on ${id}`);
    const next: Agreement = {
      ...row,
      owed: left,
      state: left === 0 ? 'discharged' : row.state,
    };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /**
   * Law 15, item 9.1: THE TERMS CHANGED, and that is an event and not a correction.
   *
   * A wage is renegotiated (Labour D2), a worker cell splits so the headcount on the row falls, a
   * borrow is rolled at a new fee. The row is the same commitment between the same two parties —
   * the identity does not move — and what it says has changed. It refuses a change of KIND: an
   * employment cannot become a lease, and a row that did would be two facts under one id.
   */
  restate(id: AgreementId, terms: AgreementTerms): Agreement {
    const row = this.get(id);
    forbid(
      row.terms.kind === terms.kind,
      'Law 15',
      `${id} is a ${row.terms.kind} and would be restated as a ${terms.kind}`,
    );
    forbid(row.state !== 'discharged', 'XI-8', `${id} is discharged and its terms cannot change`);
    forbid(row.state !== 'terminated', 'XI-8', `${id} is terminated and its terms cannot change`);
    const next: Agreement = { ...row, terms };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** The payment did not arrive and stands unpaid — the state Money E1 names and D3 requires. */
  breached(id: AgreementId): Agreement {
    const row = this.get(id);
    forbid(row.state === 'performing', 'Money E1', `${id} is ${row.state} and cannot breach`);
    const next: Agreement = { ...row, state: 'breached' };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /**
   * Ended with something still owed and nobody left to pay it. XI-8: a write-off is an OUTCOME with
   * a size and a date, and calling it a discharge would say somebody was paid who was not.
   */
  terminate(id: AgreementId): Agreement {
    const row = this.get(id);
    forbid(
      row.state !== 'discharged',
      'XI-8',
      `${id} is discharged; there is nothing to terminate`,
    );
    const next: Agreement = { ...row, state: 'terminated' };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /**
   * Register F2, XI-8: THE NAME ON ONE SIDE CEASED AND ITS SUCCESSOR IS ON THE ROW NOW.
   *
   * A commitment does not end because the party that made it ended. A line a bank committed, a wage
   * a worker is owed, a mandate a manager runs: each passes to whoever succeeded the name, which is
   * what a succession IS, and until it did, every module holding such a row was addressing an
   * instruction to somebody who is not there (Money E4). The row keeps its id, its terms and its
   * date — only one name changes — so what the successor owes is the same commitment, not a new one.
   *
   * It refuses a party that is not on the row, and a move that would leave one party on both sides:
   * a debt to yourself is not a commitment, and the caller ends that row instead.
   */
  succeed(id: AgreementId, was: PartyId, now: PartyId): Agreement {
    const row = this.get(id);
    const onDebtor = row.debtor === was;
    forbid(onDebtor || row.creditor === was, 'Register F2', `${was} is not a party to ${id}`);
    const next: Agreement = onDebtor ? { ...row, debtor: now } : { ...row, creditor: now };
    forbid(next.debtor !== next.creditor, 'Law 5', `${id} would leave ${now} owing itself`);
    this.rows.set(id, Object.freeze(next));
    reindex(onDebtor ? this.byDebtor : this.byCreditor, was, now, id);
    return next;
  }

  /**
   * Banks Lending A3.a (17.3): WHAT IS PROMISED ON THIS ROW AND NOT YET DRAWN, asked of the kind —
   * only the kind knows what its own limit is and what drawing on it looks like (Law 15). Nothing
   * for a kind that is not a commitment, which is most of them.
   */
  headroomOf(row: Agreement, at: Period, reads: RowValuationReads): Option<Cash> {
    const kind = this.kind(row.terms.kind);
    return kind.headroom === undefined ? none<Cash>() : some(kind.headroom(row, at, reads));
  }

  /** What this party owes that is not an instrument — the question an estate has to ask (XI-8). */
  /** 14.5: the mark revaluation left on a row, or nothing for a row never marked. */
  markOf(id: AgreementId): number | undefined {
    return this.marks.get(id);
  }

  mark(id: AgreementId, value: number): void {
    if (value === 0) this.marks.delete(id);
    else this.marks.set(id, finite(value, `the mark on ${id}`));
  }

  owedBy(party: PartyId): readonly Agreement[] {
    return rowsOf(this.rows, this.byDebtor.get(party));
  }

  /** What is owed TO it, which is the other half and the reason a write-off has a victim. */
  owedTo(party: PartyId): readonly Agreement[] {
    return rowsOf(this.rows, this.byCreditor.get(party));
  }

  /**
   * Law 18 (12.5): what one party owes of one kind — the rows a payer walks when it pays a
   * declaration, read off the debtor index and never a scan of every row of the kind.
   */
  byDebtorAndKind(debtor: PartyId, kind: AgreementKindId): readonly Agreement[] {
    return this.owedBy(debtor).filter((a) => a.terms.kind === kind);
  }

  /** Every row of one kind — how the module that declared a kind reads its own book back. */
  ofKind(kind: AgreementKindId): readonly Agreement[] {
    return rowsOf(this.rows, this.byKind.get(kind));
  }

  all(): readonly Agreement[] {
    return [...this.rows.values()];
  }
}

function index<K>(ix: Map<K, Set<AgreementId>>, at: K, id: AgreementId): void {
  const set = ix.get(at);
  if (set === undefined) ix.set(at, new Set([id]));
  else set.add(id);
}

function reindex(
  ix: Map<PartyId, Set<AgreementId>>,
  was: PartyId,
  now: PartyId,
  id: AgreementId,
): void {
  ix.get(was)?.delete(id);
  index(ix, now, id);
}

function rowsOf(
  rows: ReadonlyMap<AgreementId, Agreement>,
  ids: ReadonlySet<AgreementId> | undefined,
): readonly Agreement[] {
  if (ids === undefined) return [];
  const out: Agreement[] = [];
  for (const id of ids) {
    const row = rows.get(id);
    if (row !== undefined) out.push(row);
  }
  return out;
}

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export type AgreementReads = Pick<
  Agreements,
  'get' | 'owedBy' | 'owedTo' | 'ofKind' | 'byDebtorAndKind' | 'kind' | 'all' | 'markOf'
> & {
  /**
   * A3.a (17.3): what a commitment kind says is promised on one of its rows and not yet drawn. The
   * kernel supplies the world the kind is asked with, so a reader asks the QUESTION and never has
   * to assemble the reads — which is what stops two readers asking it of two different worlds.
   */
  headroomOf(row: Agreement, at: Period): Option<Cash>;
};

export function agreementReads(
  store: Agreements,
  /** 17.3: the world a commitment kind is asked its own question with (`headroomOf`). */
  rows: () => RowValuationReads,
): AgreementReads {
  return Object.freeze({
    get: (id: AgreementId) => store.get(id),
    owedBy: (party: PartyId) => store.owedBy(party),
    owedTo: (party: PartyId) => store.owedTo(party),
    ofKind: (kind: AgreementKindId) => store.ofKind(kind),
    byDebtorAndKind: (debtor: PartyId, kind: AgreementKindId) =>
      store.byDebtorAndKind(debtor, kind),
    kind: (id: AgreementKindId) => store.kind(id),
    all: () => store.all(),
    // 14.6: what the kernel last marked a row at — a read of the mark, never a second valuation.
    markOf: (id: AgreementId) => store.markOf(id),
    // A3.a (17.3): what a commitment kind says is promised and not yet drawn on one of its rows.
    headroomOf: (row: Agreement, at: Period) => store.headroomOf(row, at, rows()),
  });
}
