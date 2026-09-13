/**
 * Who controls whom: the relation the register did not have.
 *
 * @spec M&A A4 M&A A5 M&A D4 M&A E1 M&A E2 XI-8 Law 4 Law 5 Law 15 Appendix B
 *
 * Grep `subsidiary|parentOf|controls|consolidat|group` across `register/`, `parties/` and
 * `registry/` and the answer was ZERO HITS. Owning 51% of a company was a large holding and nothing
 * more, so a takeover bought the shares and nothing happened: `combine` was exported, documented at
 * length and CALLED BY NOBODY, because there was nothing for a completed tender to write to.
 *
 * OWNERSHIP AND CONTROL ARE DIFFERENT FACTS and 51% of the votes is not 51% of the economics. The
 * register holds the first; this holds the second. Without it there is no consolidation (a group's
 * accounts are the parent's), no ring-fencing, no transferring a subsidiary out of a resolution, and
 * **private equity — which is definitionally about control — is inexpressible.**
 *
 * IT IS NOT AN AGREEMENT AND NOT A HOLDING. Nobody owes anything under it and nobody holds it: it is
 * a standing fact about two named parties that arises from what one of them holds, and it ends when
 * that stops being true. What it is FOR is the reads at the foot of this file: the group a party is
 * in, and who is at the top of it.
 */
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import type { PartyId } from '../core/ids.js';

/**
 * Law 15: WHY one party controls another, which is not always the shares.
 *
 * `shares` is a majority of the votes and is the ordinary case. `contract` is control by agreement
 * without the votes. `appointment` is the right to name the board. `resolution` is an authority
 * taking a failed institution over, which is control with no purchase at all (§25 C). They are a
 * dispatch key and never a severity: nothing compares two of them.
 */
export type ControlBasis = 'shares' | 'contract' | 'appointment' | 'resolution';

export interface ControlDecl {
  readonly controller: PartyId;
  readonly subject: PartyId;
  readonly basis: ControlBasis;
  /** Law 16: what made it true, in the words of whatever established it. */
  readonly why: string;
}

export interface Controls extends ControlDecl {
  readonly since: Period;
}

export class ControlRegister {
  /** Law 4: ONE controller per subject. Two would be two answers to one question. */
  private readonly bySubject = new Map<PartyId, Controls>();
  private readonly bySubjectOf = new Map<PartyId, Set<PartyId>>();

  /**
   * A4: one party takes control of another. It refuses the three things that are not control:
   * controlling itself, a second controller for one subject, and a CYCLE — a group whose parent is
   * its own subsidiary has no party at the top, so `ultimate` would not terminate and a consolidated
   * sheet would count the same balance sheet twice (Law 4, and arithmetic impossibility).
   */
  take(decl: ControlDecl, at: Period): Controls {
    forbid(decl.controller !== decl.subject, 'Law 5', `${decl.controller} cannot control itself`);
    const held = this.bySubject.get(decl.subject);
    forbid(
      held === undefined || held.controller === decl.controller,
      'Law 4',
      `${decl.subject} is already controlled by ${String(held?.controller)}`,
    );
    forbid(
      !this.reaches(decl.subject, decl.controller),
      'M&A A4',
      `${decl.controller} is already under ${decl.subject}; a group cannot contain its own parent`,
    );
    const row: Controls = { ...decl, since: at };
    this.bySubject.set(decl.subject, Object.freeze(row));
    index(this.bySubjectOf, decl.controller, decl.subject);
    return row;
  }

  /** It stops being true — the stake was sold, the contract ended, the resolution closed. */
  release(subject: PartyId): void {
    const held = this.bySubject.get(subject);
    if (held === undefined) return;
    this.bySubject.delete(subject);
    this.bySubjectOf.get(held.controller)?.delete(subject);
  }

  controllerOf(subject: PartyId): Controls | undefined {
    return this.bySubject.get(subject);
  }

  /** The parties this one controls DIRECTLY — its own subsidiaries and not their subsidiaries. */
  subsidiariesOf(controller: PartyId): readonly PartyId[] {
    return [...(this.bySubjectOf.get(controller) ?? [])];
  }

  /**
   * A4: THE GROUP — this party and everything under it, however deep, once each. It is the set a
   * consolidated balance sheet is taken over, and the reason the cycle above is refused.
   */
  groupOf(root: PartyId): readonly PartyId[] {
    // A Set is iterated in insertion order and `add` during iteration is seen, so growing it IS the
    // walk: each party is visited once and `take` refuses a cycle, so it terminates.
    const seen = new Set<PartyId>([root]);
    for (const at of seen) {
      for (const child of this.subsidiariesOf(at)) seen.add(child);
    }
    return [...seen];
  }

  /** Who is ultimately behind this party: the top of its chain, or itself if nobody is above it. */
  ultimateOf(subject: PartyId): PartyId {
    let at = subject;
    for (;;) {
      const up = this.bySubject.get(at);
      if (up === undefined) return at;
      at = up.controller;
    }
  }

  all(): readonly Controls[] {
    return [...this.bySubject.values()];
  }

  /** Whether `from` is at or above `to` in the tree — the cycle test, and the only walk downward. */
  private reaches(from: PartyId, to: PartyId): boolean {
    return this.groupOf(from).includes(to);
  }
}

function index(ix: Map<PartyId, Set<PartyId>>, key: PartyId, value: PartyId): void {
  const set = ix.get(key);
  if (set === undefined) ix.set(key, new Set([value]));
  else set.add(value);
}

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export type ControlReads = Pick<
  ControlRegister,
  'controllerOf' | 'subsidiariesOf' | 'groupOf' | 'ultimateOf' | 'all'
>;

export function controlReads(store: ControlRegister): ControlReads {
  return Object.freeze({
    controllerOf: (subject: PartyId) => store.controllerOf(subject),
    subsidiariesOf: (controller: PartyId) => store.subsidiariesOf(controller),
    groupOf: (root: PartyId) => store.groupOf(root),
    ultimateOf: (subject: PartyId) => store.ultimateOf(subject),
    all: () => store.all(),
  });
}
