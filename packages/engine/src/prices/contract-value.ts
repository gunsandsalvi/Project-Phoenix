/**
 * What a contract is worth, to each of its two sides.
 *
 * @spec Derivative D1 Derivative D1.b Derivative D8 Derivative D8.a Derivative Layer A3 Derivative Layer A4 Clearing D4 XI-6 Law 4 Law 19
 *
 * ONE NUMBER READ FROM TWO SIDES (A3). The kind's profile says what the contract is worth to `a`;
 * `b`'s is the negation. Nothing stores either: a mark is read when it is asked for, out of the
 * prints and events the rest of the world already produced (XI-6), exactly as a holding's is.
 *
 * WHAT THE EQUITY ACCOUNT HAS RECOGNISED is a different question and the same shape as a lot's
 * carrying value (prices/value.ts): the basis until a revaluation has moved it, last period's mark
 * after that, and this period's once revaluation has run (Clearing D4). `recognisedFor` is the one
 * switch, asked of the valuation that owns it, so a contract and a holding cannot disagree about
 * which moment it is.
 */
import type { Period } from '../calendar/calendar.js';
import type { PartyId } from '../core/ids.js';
import { sub } from '../core/num.js';
import type {
  Contract,
  ContractReads,
  DerivativeKindProfile,
} from '../registry/derivatives.js';

export interface ContractValueDeps {
  profile(kind: Contract['kind']): DerivativeKindProfile;
  /** The kernel's public reads as of a period — prints, marks, indices, curves, public events. */
  reads(at: Period): ContractReads;
  /** Clearing D4: which period's marks the equity accounts have recognised, for a reader at `now`. */
  recognisedFor(now: Period): Period;
}

/** D8: what it is worth to `a` at `at`. */
export function markOfContract(c: Contract, at: Period, d: ContractValueDeps): number {
  return d.profile(c.kind).mark(c, at, d.reads(at));
}

/** D1: an asset to one side and a liability to the other, at every instant. */
export function contractValueTo(
  c: Contract,
  party: PartyId,
  at: Period,
  d: ContractValueDeps,
): number {
  const mark = markOfContract(c, at, d);
  return party === c.a ? mark : party === c.b ? -mark : 0;
}

/**
 * What the equity accounts are carrying it at, to `a`. The basis while it is younger than the last
 * recognised marks; the mark of that period once it is not.
 */
export function carryingOfContract(c: Contract, now: Period, d: ContractValueDeps): number {
  const recognised = d.recognisedFor(now);
  // A contract younger than the last marks anybody recognised is carried at what it cost: there is
  // no earlier mark of it to read, and reading one would be asking what a contract was worth in a
  // period it did not exist in. A contract opened IN the recognised period is not younger than it —
  // its mark of that period is in the accounts — which is the same `>` a lot's `acquired < now` is.
  return c.opened > recognised ? c.basis : markOfContract(c, recognised, d);
}

/** D8.a: what the period's re-marking moves, a real gain to one side and a real loss to the other. */
export function revaluationOfContract(c: Contract, now: Period, d: ContractValueDeps): number {
  return sub(markOfContract(c, now, d), carryingOfContract(c, now, d), 'what the mark moved by');
}
