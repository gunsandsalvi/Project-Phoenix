/**
 * The contract store: a derivative is a bilateral obligation and not a holding (Derivative X1).
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative D8 Derivative D10 Derivative D11 Derivative D12 Derivative X1 Derivative Layer A1 Derivative Layer A2 Derivative Layer A3 Derivative Layer B2 Derivative Layer B3 Derivative Layer B3.a Derivative Layer B4 Derivative Layer C1 Derivative Layer C1.a Derivative Layer C2 Derivative Layer G1 Derivative Layer G3 Derivative Layer G4 Law 4
 *
 * WHY IT IS A SECOND STORE. Nobody issued a contract, so it has no issued amount and no holders;
 * putting it in the register would put it into the ownership identity, where a row that nobody
 * issued and two parties are on would be a defect in every period it existed (X1). What it enters
 * instead is the ZERO-SUM identity: the marks across its two sides come to nothing, exactly (D1.b).
 *
 * WHY IT IS INDEXED BOTH WAYS. C1.a nets per counterparty PAIR, and G3 forbids netting across
 * counterparties — so the read a party is entitled to is "what I have with you", and there is no
 * door anywhere that adds up what it has with everybody. A store that answered only "my contracts"
 * would make the forbidden read the easy one.
 *
 * WHAT IT NEVER DOES IS COLLAPSE (B3.a). An offsetting trade with a different counterparty is a
 * third row: the market risk is flat and the credit risk has doubled, and a store that netted them
 * would hide the thing that actually breaks. Two contracts on one underlying at different strikes
 * are two rows for the same reason (D12).
 */
import { forbid } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import { contractId, type ContractId, type DerivativeKindId, type PartyId } from '../core/ids.js';
import { finite } from '../core/num.js';
import { none, some, type Option } from '../core/option.js';
import type { Period } from '../calendar/calendar.js';
import type { Contract } from '../registry/derivatives.js';

export interface ContractDecl {
  readonly kind: DerivativeKindId;
  readonly a: PartyId;
  readonly b: PartyId;
  readonly terms: Contract['terms'];
  readonly ccy: Contract['ccy'];
  readonly notional: number;
  readonly struckAt: number;
  /** Register D4: what it is worth to `a` at inception — the basis the equity account recognised. */
  readonly basis: number;
  readonly house: PartyId | null;
}

/** C1.a: what two named parties have with each other, and nothing wider (G3). */
export interface PairKey {
  readonly a: PartyId;
  readonly b: PartyId;
}

const pairOf = (x: PartyId, y: PartyId): string => (x < y ? `${x}|${y}` : `${y}|${x}`);

export class Contracts {
  private readonly rows = new Map<ContractId, Contract>();
  private readonly byParty = new Map<PartyId, Set<ContractId>>();
  private readonly byPair = new Map<string, Set<ContractId>>();
  private next = 1;

  /**
   * D1, G1: open a row with two named sides. Settlement is the one caller (Money D4) — a contract
   * appearing on two balance sheets is a change of state and goes over the wire like every other.
   */
  open(decl: ContractDecl, at: Period): Contract {
    forbid(decl.a !== decl.b, 'Derivative D1', 'a contract has two counterparties, and they differ');
    forbid(
      finite(decl.notional, 'notional') > 0,
      'Derivative D2',
      'a contract has a notional and it is positive',
    );
    finite(decl.struckAt, 'the level it was struck at');
    const id = contractId(`contract.${decl.kind}.${this.next}`);
    this.next += 1;
    const row: Contract = {
      id,
      kind: decl.kind,
      a: decl.a,
      b: decl.b,
      terms: decl.terms,
      ccy: decl.ccy,
      notional: decl.notional,
      struckAt: decl.struckAt,
      basis: finite(decl.basis, 'what the contract was worth at inception'),
      opened: at,
      state: 'open',
      terminated: none(),
      house: decl.house,
    };
    this.rows.set(id, row);
    this.index(row);
    return row;
  }

  /** D11: it ceases to exist on both books at once. The row stays, so the chain stays traceable (F4). */
  close(id: ContractId, at: Period): Contract {
    const row = this.get(id);
    forbid(row.state === 'open', 'Derivative D11', `${id} is already terminated`);
    const done: Contract = { ...row, state: 'terminated', terminated: some(at) };
    this.rows.set(id, done);
    return done;
  }

  /**
   * B4: novation is a real change of who faces whom — the position moves to a new counterparty and
   * the old one is out of it. The consent is the decision of the party leaving, taken by its own
   * module before this is called; what happens here is the change of name on the row.
   */
  novate(id: ContractId, from: PartyId, to: PartyId): Contract {
    const row = this.get(id);
    forbid(row.state === 'open', 'Derivative Layer B4', `${id} is terminated and cannot be novated`);
    forbid(
      row.a === from || row.b === from,
      'Derivative Layer B4',
      `${from} is not a side of ${id}`,
    );
    forbid(to !== from, 'Derivative Layer B4', 'a novation moves the position to somebody else');
    const other = row.a === from ? row.b : row.a;
    forbid(to !== other, 'Derivative D1', `${to} is already the other side of ${id}`);
    this.unindex(row);
    const moved: Contract = row.a === from ? { ...row, a: to } : { ...row, b: to };
    this.rows.set(id, moved);
    this.index(moved);
    return moved;
  }

  has(id: ContractId): boolean {
    return this.rows.has(id);
  }

  get(id: ContractId): Contract {
    const row = this.rows.get(id);
    if (row === undefined) throw new Missing('Derivative X1', `no contract ${id}`, { id });
    return row;
  }

  all(): readonly Contract[] {
    return [...this.rows.values()];
  }

  /** Every open row in the world, which is what the zero-sum family walks (A4). */
  open_(): readonly Contract[] {
    return this.all().filter((c) => c.state === 'open');
  }

  /** A party's own rows, open and terminated (Observer A4: its own side is what it may see). */
  of(party: PartyId): readonly Contract[] {
    const ids = this.byParty.get(party);
    return ids === undefined ? [] : [...ids].map((id) => this.get(id));
  }

  openOf(party: PartyId): readonly Contract[] {
    return this.of(party).filter((c) => c.state === 'open');
  }

  /** C1, C1.a: the rows two named parties have with each other. The only netting door there is. */
  between(a: PartyId, b: PartyId): readonly Contract[] {
    const ids = this.byPair.get(pairOf(a, b));
    return ids === undefined ? [] : [...ids].map((id) => this.get(id)).filter((c) => c.state === 'open');
  }

  /** Which side of a row a party is on, or none when it is on neither. */
  sideOf(c: Contract, party: PartyId): Option<'a' | 'b'> {
    if (c.a === party) return some('a');
    if (c.b === party) return some('b');
    return none();
  }

  private index(row: Contract): void {
    for (const p of [row.a, row.b]) {
      const set = this.byParty.get(p) ?? new Set<ContractId>();
      set.add(row.id);
      this.byParty.set(p, set);
    }
    const key = pairOf(row.a, row.b);
    const pair = this.byPair.get(key) ?? new Set<ContractId>();
    pair.add(row.id);
    this.byPair.set(key, pair);
  }

  private unindex(row: Contract): void {
    for (const p of [row.a, row.b]) this.byParty.get(p)?.delete(row.id);
    this.byPair.get(pairOf(row.a, row.b))?.delete(row.id);
  }
}

/** The reads a context hands out: everything above except the two writers (Law 4). */
export type ContractReadsFacade = Pick<
  Contracts,
  'has' | 'get' | 'all' | 'open_' | 'of' | 'openOf' | 'between' | 'sideOf'
>;
