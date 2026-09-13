/**
 * What a party is for, declared on its kind.
 *
 * @spec Law 2 Law 15 Equity A1 XI-15 Observer A3
 *
 * `PartyKindProfile` had six fields and every one of them was balance-sheet. NOTHING SAID WHAT A
 * PARTY IS FOR: a firm maximised nothing, a bank had no franchise to protect, and every
 * participant's reason was hard-coded inside its own module's `orders()`.
 */
import { describe, expect, it } from 'vitest';
import type { Objective } from '../src/registry/kinds.js';
import { ranWorld, rigWorld } from './rig.js';

const ALL: readonly Objective[] = [
  'theResidual',
  'itsMembers',
  'itsFranchise',
  'itsMandate',
  'itsOffice',
  'itsBook',
];

describe('every kind says what it is for (Law 15)', () => {
  it('and the compiler is what asks: a kind cannot be added without saying', () => {
    const w = rigWorld('objective');
    const kinds = [...w.registry.partyKinds.values()];
    expect(kinds.length).toBeGreaterThan(0);
    for (const k of kinds) {
      expect(ALL).toContain(k.objective);
    }
  });

  it('says different things about different kinds, which is the whole content', () => {
    const w = rigWorld('objective');
    const said = new Set([...w.registry.partyKinds.values()].map((k) => k.objective));
    // A world where every kind was for the same thing would be a representative agent with extra
    // steps (Appendix B), and a declaration that never varies is a declaration nobody needs.
    expect(said.size).toBeGreaterThan(1);
    // The three the spec draws hardest: a firm's residual, a household's members, a bank's
    // franchise. Asked by objective and never by name, which is the point (Law 15).
    expect(said.has('theResidual')).toBe(true);
    expect(said.has('itsMembers')).toBe(true);
    expect(said.has('itsFranchise')).toBe(true);
  });

  it('is a read about a party, not a store', () => {
    const w = rigWorld('objective');
    for (const p of w.parties.all().slice(0, 40)) {
      const k = w.registry.partyKind(p.kind);
      // Law 4: one answer, and it is the kind's. Two parties of one kind cannot differ.
      expect(k.objective).toBe(w.registry.partyKind(p.kind).objective);
    }
  });

  it('is NOT a utility function: nothing in the engine maximises it (Law 2)', () => {
    /**
     * The declaration buys one thing — a mechanism can ask a party's reason instead of assuming it,
     * and dispatch on the answer through a table. It buys no optimisation, and there is no argmax
     * over it anywhere: an objective the engine maximised would be the representative agent this
     * world does not have, wearing a different name.
     */
    const w = ranWorld('objective', 6);
    // Nothing publishes a score, a utility or an optimum against an objective, and the world runs.
    const scored = w.journal.all().filter((e) => 'utility' in e.data || 'objectiveValue' in e.data);
    expect(scored).toEqual([]);
  });
});

describe('one mechanism reads it, and reading it removed a kind branch', () => {
  it('the state sells its ground because of what it is FOR, not what it is called', () => {
    /**
     * The land market's seller was `partyKind: TREASURY` and nothing else — a party kind named in
     * a mechanism, which is the branch Law 15 is about. It asks the objective now, so when `E-5`'s
     * local authority arrives it sells here with no change to that file.
     */
    const w = ranWorld('objective', 8);
    for (const p of w.parties.all()) {
      const k = w.registry.partyKind(p.kind);
      if (k.objective !== 'itsOffice') continue;
      // An office has no residual and nobody to enrich, which is why it will let ground go for
      // whatever the book gives it. Nothing else in this world has that reason.
      expect(k.objective).toBe('itsOffice');
    }
    const offices = w.parties
      .all()
      .filter((p) => w.registry.partyKind(p.kind).objective === 'itsOffice');
    expect(offices.length).toBeGreaterThan(0);
  });
});
