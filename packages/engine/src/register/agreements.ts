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
import { agreementId, type AgreementId, type CurrencyCode, type PartyId } from '../core/ids.js';
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

export interface AgreementDecl {
  /** Who owes. Both sides are named: an agreement with one side is not one (Law 5). */
  readonly debtor: PartyId;
  readonly creditor: PartyId;
  readonly ccy: CurrencyCode;
  /** What is owed now, in the smallest piece of `ccy`. */
  readonly owed: number;
  /** What kind of commitment this is, in the words its own mechanism uses. */
  readonly what: string;
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
  private next = 1;

  /**
   * Money E1, D3: what did not arrive is still owed, by somebody, to somebody.
   *
   * It refuses an agreement with one party on both sides and one that owes nothing: neither is a
   * commitment, and admitting either would let a mechanism record a fact about nobody.
   */
  open(decl: AgreementDecl, at: Period): Agreement {
    forbid(decl.debtor !== decl.creditor, 'Law 5', `${decl.debtor} cannot owe itself: ${decl.what}`);
    forbid(finite(decl.owed, 'owed') > 0, 'Money E1', `an agreement owing nothing: ${decl.what}`);
    forbid(decl.what.length > 0, 'Law 16', 'an agreement that does not say what it is');
    const id = agreementId(`agreement.${this.next}`);
    this.next += 1;
    const row: Agreement = { ...decl, id, since: at, state: 'performing' };
    this.rows.set(id, Object.freeze(row));
    index(this.byDebtor, decl.debtor, id);
    index(this.byCreditor, decl.creditor, id);
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

  all(): readonly Agreement[] {
    return [...this.rows.values()];
  }
}

function index(ix: Map<PartyId, Set<AgreementId>>, party: PartyId, id: AgreementId): void {
  const set = ix.get(party);
  if (set === undefined) ix.set(party, new Set([id]));
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
export type AgreementReads = Pick<Agreements, 'get' | 'owedBy' | 'owedTo' | 'all'>;

export function agreementReads(store: Agreements): AgreementReads {
  return Object.freeze({
    get: (id: AgreementId) => store.get(id),
    owedBy: (party: PartyId) => store.owedBy(party),
    owedTo: (party: PartyId) => store.owedTo(party),
    all: () => store.all(),
  });
}
