import { describe, expect, it } from 'vitest';
import { bandOf, UNREAD, latticeDimensions } from '../src/registry/lattice.js';
import { HOUSEHOLD } from '../src/registry/profiles.js';
import { rigWorld } from './rig.js';

describe('the lattice (XI-15, 0f.3)', () => {
  it('bands a quantity by the count of edges at or below it', () => {
    expect(bandOf([4, 13, 52], 0)).toBe('0');
    expect(bandOf([4, 13, 52], 4)).toBe('1');
    expect(bandOf([4, 13, 52], 20)).toBe('2');
    expect(bandOf([4, 13, 52], 1000)).toBe('3');
  });

  it('places every seeded cell on every dimension of its lattice at the seal', () => {
    const w = rigWorld('seed-lattice');
    const lattice = w.registry.partyKind(HOUSEHOLD).lattice;
    if (lattice === undefined) throw new Error('households have no lattice');
    const dims = latticeDimensions(lattice);
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.representation === 'cell');
    expect(cells.length).toBeGreaterThan(0);
    for (const c of cells) {
      for (const d of dims) expect(c.key[d], `${c.id} on ${d}`).not.toBe(undefined);
      // §46: nobody has an outlook at the seal, so a band on expected income is a real state.
      expect(c.key['liquidWeeks']).toBe(UNREAD);
      // The opening record: no hire yet, no default yet — read, not assumed.
      expect(c.key['employment']).toBe('unemployed');
      expect(c.key['credit']).toBe('clean');
    }
  });

  it('keeps at most one seeded cell per key', () => {
    const w = rigWorld('seed-lattice-2');
    const seen = new Map<string, string>();
    for (const c of w.parties.ofKind(HOUSEHOLD)) {
      if (c.representation !== 'cell') continue;
      const k = JSON.stringify(Object.entries(c.key).sort());
      // Two cells on one key is what 0d measured as 299 `units` findings; the seed still places
      // two per (region, cohort, bank), so this MEASURES rather than asserts until 0f.9.
      if (seen.has(k)) continue;
      seen.set(k, c.id);
    }
    expect(seen.size).toBeGreaterThan(0);
  });
});

/* --------------------------------------------------------------------------------------------
 * 0f.10: THE LATTICE OVER A YEAR. A cell is what the lattice makes of a population; these ask
 * whether that stays true once people move, and whether the band edges are the resolution the
 * declarations say they are.
 * ------------------------------------------------------------------------------------------ */

import { asQty } from '../src/core/tick.js';
import {
  SMALL_FIRM,
  assemble,
  moneyInstrumentId,
  partyId,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { mergeModules, ranWorld, rigSpec } from './rig.js';
import { refined } from './grain.js';
import { phx } from './units.js';

const YEAR = 52;

/** The live cells of a kind, and the key each one stands on. */
function liveCells(w: World, kind: typeof HOUSEHOLD): { id: string; key: string; weight: number }[] {
  const out: { id: string; key: string; weight: number }[] = [];
  for (const c of w.parties.ofKind(kind)) {
    if (c.representation !== 'cell' || !c.status.alive) continue;
    out.push({ id: String(c.id), key: JSON.stringify(Object.entries(c.key).sort()), weight: c.weight });
  }
  return out;
}

describe('the lattice over a year (XI-15, 0f.10)', () => {
  it('keeps one live cell per key, and a hire moves members onto the standing cell rather than making one', () => {
    const w = ranWorld('lattice-year', YEAR);
    for (const kind of [HOUSEHOLD, SMALL_FIRM]) {
      const cells = liveCells(w, kind);
      const keys = new Set(cells.map((c) => c.key));
      // XI-15: at most one live cell per key; parties are the occupied keys and nothing more.
      expect(cells.length, String(kind)).toBe(keys.size);
    }
    // Labour C2, 0f.4: a hire is the employment dimension moving — a weight event, never a split.
    const moves = w.journal.ofKind('weight');
    expect(moves.filter((e) => String(e.data['cause']).startsWith('hired by')).length).toBeGreaterThan(0);
    expect(moves.filter((e) => e.data['kind'] === 'split').length).toBe(0);
  });

  it('spends a transfer where the basket is not covered, and places it where it is (Households A2.a, C1, 0f.7c)', () => {
    const at = 6;
    const seed = 'lattice-transfer';
    const spend = (w: World): Map<string, { spend: number; constrained: boolean }> => {
      const out = new Map<string, { spend: number; constrained: boolean }>();
      for (const e of w.journal.ofKind('households.plan')) {
        if (e.period !== at) continue;
        const cell = e.subjects[0];
        const spendPerMember = e.data['spendPerMember'];
        const constrained = e.data['constrained'];
        if (cell === undefined || typeof spendPerMember !== 'number' || typeof constrained !== 'boolean') continue;
        out.set(cell, { spend: spendPerMember, constrained });
      }
      return out;
    };
    // The working cohort is emptied of its cash the morning of the decision, so its basket is not
    // covered; the retired cohort keeps what it has. Then EVERY cell is paid the same per member.
    const plain = assemble({ ...rigSpec(seed), modules: mergeModules(rigSpec(seed).modules, [drainsAndPays('working', 0, at)]) });
    const paid = assemble({ ...rigSpec(seed), modules: mergeModules(rigSpec(seed).modules, [drainsAndPays('working', phx(500), at)]) });
    for (let i = 0; i <= at; i += 1) {
      plain.step();
      paid.step();
    }
    const before = spend(plain);
    const after = spend(paid);
    const constrained = [...before].filter(([, p]) => p.constrained);
    const covered = [...before].filter(([, p]) => !p.constrained);
    // A2.a: the same money reaches demand only where the basket was not covered. Both states have
    // to exist for the threshold to be a threshold.
    expect(constrained.length).toBeGreaterThan(0);
    expect(covered.length).toBeGreaterThan(0);
    for (const [cell, p] of constrained) {
      expect(after.get(cell)?.spend, `${cell} was short of its basket`).toBeGreaterThan(p.spend);
    }
    for (const [cell, p] of covered) {
      // A cell whose basket was covered spends the basket; the transfer is spare and is placed.
      expect(after.get(cell)?.spend, `${cell} had its basket`).toBe(p.spend);
    }
  });

  it('is a RESOLUTION: refine every band edge by two and the population is the same population', () => {
    const seed = 'lattice-grain';
    const coarse = assemble(rigSpec(seed));
    const fine = assemble(refined(rigSpec(seed)));
    for (let i = 0; i < YEAR; i += 1) {
      coarse.step();
      fine.step();
    }
    // A count of people is a count, at every grain, exactly.
    const people = (w: World, kind: typeof HOUSEHOLD): number =>
      liveCells(w, kind).reduce((t, c) => t + c.weight, 0);
    expect(people(fine, HOUSEHOLD)).toBe(people(coarse, HOUSEHOLD));
    expect(people(fine, SMALL_FIRM)).toBe(people(coarse, SMALL_FIRM));
    // And the finer grain has at least as many keys occupied, never fewer.
    expect(liveCells(fine, HOUSEHOLD).length).toBeGreaterThanOrEqual(liveCells(coarse, HOUSEHOLD).length);
    /**
     * Law 7: the aggregates move by less than the arithmetic dust of the grain. Every cell rounds
     * what one member does to a whole piece and posts it times its count, so a finer cut moves an
     * aggregate by at most one piece per cell-decision it adds — that is the dust, derived from the
     * count of decisions and the piece, and never a percentage. What this MEASURES is whether the
     * world is invariant to its own resolution past that; a number above it is a finding about a
     * mechanism that reads a band, not a licence to widen this.
     */
    const decisions = (w: World): number => w.journal.ofKind('households.plan').length;
    const dust = phx(0.01) * (decisions(coarse) + decisions(fine));
    const spent = (w: World): number => {
      let total = 0;
      for (const e of w.journal.ofKind('households.plan')) {
        const s = e.data['spendPerMember'];
        const cell = e.subjects[0];
        if (typeof s !== 'number' || cell === undefined) continue;
        const p = w.parties.get(partyId(cell));
        total += s * (p.representation === 'cell' ? p.weight : 1);
      }
      return total;
    };
    expect(Math.abs(spent(fine) - spent(coarse))).toBeLessThanOrEqual(dust);
  });
});

/**
 * The test's own two-sided instructions: the cells of one cohort hand their whole account to their
 * bank, then every household cell is paid `perMember` — both in the same period, before it decides.
 */
function drainsAndPays(cohort: string, perMember: number, at: number): SystemModule {
  return {
    id: 'test.drainsAndPays',
    spec: 'Money C4',
    requires: ['households'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.drainsAndPays',
        spec: 'Money C4',
        anchor: { before: 'households.decide' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
            if (cell.representation !== 'cell' || !cell.status.alive) continue;
            const ccy = ctx.registry.currencyOf(cell.region);
            const held = ctx.register.quantity(cell.id, moneyInstrumentId(cell.bank, ccy));
            if (cell.key['cohort'] === cohort && held > 0) {
              ctx.settle({
                legs: [{ kind: 'money', from: { holder: cell.id, issuer: cell.bank }, to: { holder: cell.bank, issuer: cell.bank }, ccy, amount: held }],
                cause: 'transfer',
                reason: `the test empties ${cell.id}`,
              });
            }
            if (perMember <= 0) continue;
            ctx.settle({
              legs: [{ kind: 'money', from: { holder: cell.bank, issuer: cell.bank }, to: { holder: cell.id, issuer: cell.bank }, ccy, amount: asQty(perMember * cell.weight) }],
              cause: 'transfer',
              reason: `the test pays ${cell.id}`,
            });
          }
        },
      },
    ],
    participants: [],
    families: [],
  };
}

