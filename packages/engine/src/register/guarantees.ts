/**
 * A third party standing behind a second: the one relation in this world that has THREE sides.
 *
 * @spec Banks Funding A1.a Banks Capital D4 Banks Capital D5 Money Market B3.c XI-8 Law 5 Law 15 Appendix B
 *
 * The only thing of this shape in the codebase was `Contracts.novate`, for derivatives. A parent
 * guaranteeing a subsidiary, a deposit insurer standing behind a bank, a clearing house interposing,
 * a sovereign backstop, a letter of credit — none was expressible. Item 8's `Agreement` does not
 * absorb it and could not: an agreement is two named parties, and what makes a guarantee a
 * guarantee is that the party who PAYS is not the party who OWES.
 *
 * WHAT ITS ABSENCE COST. Deposit insurance works — the insurer collects premiums and pays when a
 * bank fails — and it works by being written into the resolution path in one place, as an ordering
 * of payments rather than as a thing anybody holds. So:
 *
 *  - nothing can be ASKED who stands behind a party, which is the first question a lender has;
 *  - a guaranteed claim ranks the same as an unguaranteed one in an estate, because nothing says
 *    it is guaranteed (XI-8);
 *  - LOLR's four classical conditions have nothing to attach to;
 *  - and a second guarantee — a parent behind a subsidiary — would have to be written into a
 *    second place, which is where two ways of being wrong come from (Law 4).
 *
 * A LIMIT HERE IS NOT A BOUND (Law 6). "Insured up to a limit per member" is a TERM of the
 * guarantee — a POLICY somebody set, with an owner — and not a clamp on a computed number. What it
 * bounds is what the guarantor promised, which is a fact about the promise. `null` is a guarantee
 * with no limit, which is what a parent gives a subsidiary and what a sovereign gives its own
 * banking system, and it is a real answer rather than a large number.
 */
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { finite } from '../core/num.js';
import { Missing } from '../core/errors.js';
import {
  guaranteeId,
  type CurrencyCode,
  type GuaranteeId,
  type PartyId,
} from '../core/ids.js';

/**
 * Law 15: WHY one party stands behind another. A dispatch key, never a severity — nothing compares
 * two of them, and behaviour that varies by one belongs in a profile.
 */
export type GuaranteeBasis =
  | 'insurance'
  | 'parent'
  | 'sovereign'
  | 'clearingHouse'
  | 'letterOfCredit';

/**
 * `standing` is the ordinary course. `called` is a guarantee somebody has demanded on and which has
 * paid what it could. `exhausted` is one that has paid its limit and has nothing left to give — not
 * the same as released, because the promise was kept and ran out. `released` is one that has ended
 * without being called: the obligation it stood behind was met, or its term ran.
 */
export type GuaranteeState = 'standing' | 'called' | 'exhausted' | 'released';

export interface GuaranteeDecl {
  /** Who pays if the obligor does not. */
  readonly guarantor: PartyId;
  /** Whose obligation this stands behind. */
  readonly obligor: PartyId;
  /**
   * Who may call it. A guarantee of a bank's DEPOSITS is given to whoever holds one, and naming
   * every holder would be naming a set that changes every period — so the beneficiary is the
   * obligor's counterparties in whatever the guarantee is OF, said in `what` and read there.
   */
  readonly beneficiary: PartyId | 'whoeverHolds';
  /** What is guaranteed, in the words its own mechanism uses. */
  readonly what: string;
  readonly ccy: CurrencyCode;
  /** A TERM of the promise, not a clamp: `null` is a guarantee with no limit (Law 6). */
  readonly limit: number | null;
  readonly basis: GuaranteeBasis;
  readonly why: string;
}

export interface Guarantee extends GuaranteeDecl {
  readonly id: GuaranteeId;
  readonly since: Period;
  readonly state: GuaranteeState;
  /** What has actually been paid under it, which is what makes a limit mean anything. */
  readonly paid: number;
}

export class Guarantees {
  private readonly rows = new Map<GuaranteeId, Guarantee>();
  private readonly byObligor = new Map<PartyId, Set<GuaranteeId>>();
  private readonly byGuarantor = new Map<PartyId, Set<GuaranteeId>>();
  private next = 1;

  /**
   * A1.a: one party stands behind another. It refuses the two things that are not guarantees: a
   * party standing behind itself, which is not a third side (Law 5), and a limit of nothing, which
   * is a promise to pay nothing and should be said by not making one.
   */
  give(decl: GuaranteeDecl, at: Period): Guarantee {
    forbid(
      decl.guarantor !== decl.obligor,
      'Law 5',
      `${decl.guarantor} cannot stand behind itself: ${decl.what}`,
    );
    forbid(
      decl.limit === null || finite(decl.limit, 'the limit') > 0,
      'Law 2',
      `a guarantee of nothing: ${decl.what}`,
    );
    forbid(decl.what.length > 0, 'Law 16', 'a guarantee that does not say what it is of');
    const id = guaranteeId(`guarantee.${this.next}`);
    this.next += 1;
    const row: Guarantee = { ...decl, id, since: at, state: 'standing', paid: 0 };
    this.rows.set(id, Object.freeze(row));
    index(this.byObligor, decl.obligor, id);
    index(this.byGuarantor, decl.guarantor, id);
    return row;
  }

  get(id: GuaranteeId): Guarantee {
    const row = this.rows.get(id);
    if (row === undefined) throw new Missing('Banks Funding A1.a', `no guarantee ${id}`);
    return row;
  }

  /**
   * D4: IT IS CALLED, and what it pays is recorded against it. A guarantee that has paid its limit
   * is exhausted — the promise was kept and ran out, which is a different state from released.
   *
   * It refuses a call for more than the limit leaves, because a guarantor that paid past its own
   * promise would be paying somebody else's — and what the guarantee could not meet is what the
   * next thing behind it is FOR (D5's purse), not something to be quietly absorbed here.
   */
  called(id: GuaranteeId, amount: number): Guarantee {
    const row = this.get(id);
    forbid(row.state !== 'released', 'Banks Capital D4', `${id} was released and cannot be called`);
    forbid(row.state !== 'exhausted', 'Banks Capital D4', `${id} is exhausted and has nothing left`);
    const paid = finite(row.paid + finite(amount, 'called'), 'what it has paid');
    forbid(
      row.limit === null || paid <= row.limit,
      'Banks Capital D4',
      `${amount} called on ${id} takes it past its limit of ${String(row.limit)}`,
    );
    const next: Guarantee = {
      ...row,
      paid,
      state: row.limit !== null && paid >= row.limit ? 'exhausted' : 'called',
    };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** It ends without being called: what it stood behind was met, or its term ran (A1.a). */
  release(id: GuaranteeId): Guarantee {
    const row = this.get(id);
    const next: Guarantee = { ...row, state: 'released' };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** What is left under it, or nothing because there is no limit — a real answer, not a number. */
  headroom(id: GuaranteeId): number | null {
    const row = this.get(id);
    return row.limit === null ? null : row.limit - row.paid;
  }

  /** WHO STANDS BEHIND THIS PARTY — the first question a lender has, and there was no answer. */
  behind(obligor: PartyId): readonly Guarantee[] {
    return rowsOf(this.rows, this.byObligor.get(obligor));
  }

  /** What this party has promised for somebody else, which is an exposure it does not carry. */
  given(guarantor: PartyId): readonly Guarantee[] {
    return rowsOf(this.rows, this.byGuarantor.get(guarantor));
  }

  all(): readonly Guarantee[] {
    return [...this.rows.values()];
  }
}

function index(ix: Map<PartyId, Set<GuaranteeId>>, party: PartyId, id: GuaranteeId): void {
  const set = ix.get(party);
  if (set === undefined) ix.set(party, new Set([id]));
  else set.add(id);
}

function rowsOf(
  rows: ReadonlyMap<GuaranteeId, Guarantee>,
  ids: ReadonlySet<GuaranteeId> | undefined,
): readonly Guarantee[] {
  if (ids === undefined) return [];
  const out: Guarantee[] = [];
  for (const id of ids) {
    const row = rows.get(id);
    if (row !== undefined) out.push(row);
  }
  return out;
}

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export type GuaranteeReads = Pick<
  Guarantees,
  'get' | 'behind' | 'given' | 'headroom' | 'all'
>;

export function guaranteeReads(store: Guarantees): GuaranteeReads {
  return Object.freeze({
    get: (id: GuaranteeId) => store.get(id),
    behind: (obligor: PartyId) => store.behind(obligor),
    given: (guarantor: PartyId) => store.given(guarantor),
    headroom: (id: GuaranteeId) => store.headroom(id),
    all: () => store.all(),
  });
}
