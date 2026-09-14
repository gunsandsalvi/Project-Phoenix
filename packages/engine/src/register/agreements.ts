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
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import {
  agreementId,
  type AgreementId,
  type AgreementKindId,
  type CurrencyCode,
  type PartyId,
} from '../core/ids.js';
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
    forbid(row.state !== 'discharged', 'XI-8', `${id} is discharged; there is nothing to terminate`);
    const next: Agreement = { ...row, state: 'terminated' };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** What this party owes that is not an instrument — the question an estate has to ask (XI-8). */
  owedBy(party: PartyId): readonly Agreement[] {
    return rowsOf(this.rows, this.byDebtor.get(party));
  }

  /** What is owed TO it, which is the other half and the reason a write-off has a victim. */
  owedTo(party: PartyId): readonly Agreement[] {
    return rowsOf(this.rows, this.byCreditor.get(party));
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
  'get' | 'owedBy' | 'owedTo' | 'ofKind' | 'kind' | 'all'
>;

export function agreementReads(store: Agreements): AgreementReads {
  return Object.freeze({
    get: (id: AgreementId) => store.get(id),
    owedBy: (party: PartyId) => store.owedBy(party),
    owedTo: (party: PartyId) => store.owedTo(party),
    ofKind: (kind: AgreementKindId) => store.ofKind(kind),
    kind: (id: AgreementKindId) => store.kind(id),
    all: () => store.all(),
  });
}
