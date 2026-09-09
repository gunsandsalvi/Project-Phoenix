/**
 * Resolution: how many cells stand for a population is not supposed to be an answer.
 *
 * @spec XI-15 Households A2.e Households A2.f Labour A4.b Labour A4.c Law 7 Law 11 Part XII
 *
 * A cell is one possible household carried with a multiplicity, so the same world at one cell per
 * key, at two and at four is the same world seen at three grains. Two things must hold at every
 * grain and they are what this asserts: the POPULATION is exactly the same number of people, and
 * the world is CONSISTENT — every audit family at zero, every period, at every grain.
 *
 * What does NOT come out identical is what a WHOLE PERSON does. An employment relationship carries
 * a headcount and a hire of part of a cell splits it (Labour A4.b, A4.c), so which people a venue
 * matches, and at what contract wage, turns on how the seekers were grouped; and each cell draws
 * its own memory at entry (XI-16), so a finer grain is a finer draw of the outlooks that give the
 * market two sides (§46 A3). The money follows those people. That is not error to be tuned away —
 * per XI-15's standing observation the size of that move IS the honest error bar on every number
 * the represented sectors produce, and the bound here is what a person is worth over the run, never
 * a percentage of anything (Law 7).
 *
 * What that error bar MEANS — whether it is material — is a measurement, and measurement is Part
 * XII (worklist 16). This test states it; it does not judge it.
 */
import { describe, expect, it } from 'vitest';
import {
  HOUSEHOLD,
  LABOUR_NUMBERS,
  PHX,
  REGION,
  assemble,
  foundationSpec,
  goodId,
  type Sum,
  type World,
} from '../../src/index.js';

const PERIODS = 26;

interface Aggregates {
  readonly people: number;
  readonly cells: number;
  readonly venues: number;
  readonly employed: number;
  readonly cash: number;
  readonly bread: Sum;
  readonly money: number;
  readonly wage: number;
}

/** The foundation world at a stated grain, stepped a half-year, audited every period. */
function at(cellsPerKey: number): Aggregates {
  const spec = foundationSpec('resolution');
  const modules = spec.modules.map((m) =>
    m.id === 'seed.foundation'
      ? {
          ...m,
          params: m.params.map((p) =>
            p.id === 'seed.households.cellsPerKey' ? { ...p, value: cellsPerKey } : p,
          ),
        }
      : m,
  );
  const w = assemble({ ...spec, modules });
  for (let i = 0; i < PERIODS; i += 1) {
    const r = w.step();
    expect(r.audit.total).toBe(0);
  }
  return read(w);
}

function read(w: World): Aggregates {
  const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
  const weightOf = (p: (typeof cells)[number]): number =>
    p.representation === 'cell' ? p.weight : 1;
  const rows = (
    w.stateSlots()['labour/employment'] as { rows: Record<string, { headcount: number }> }
  ).rows;
  const prints = w.journal.ofKind('labour.print');
  const last = prints[prints.length - 1];
  const wage = typeof last?.data['wagePerHour'] === 'number' ? last.data['wagePerHour'] : 0;
  return {
    people: cells.reduce((a, p) => a + weightOf(p), 0),
    cells: cells.length,
    venues: w.venues.filter((v) => v.clearedBy === 'labour').length,
    employed: Object.values(rows).reduce((a, r) => a + r.headcount, 0),
    cash: cells.reduce((a, p) => a + w.cash(p.id, PHX) * weightOf(p), 0),
    bread: w.register.heldTotal(goodId('bread', REGION)),
    money: w.moneyStock()['PHX'] ?? 0,
    wage,
  };
}

describe('the same world at three grains (XI-15)', () => {
  const one = at(1);
  const two = at(2);
  const four = at(4);

  it('carries the same population, however finely it is cut', () => {
    // A weight is a count of people and a split is exact, so the grain cannot change the total.
    expect(two.people).toBe(one.people);
    expect(four.people).toBe(one.people);
    expect(two.cells).toBeGreaterThan(one.cells);
    expect(four.cells).toBeGreaterThan(two.cells);
  });

  it('holds the same stock of goods, to the dust of reading it', () => {
    // The real side is what the lines produced out of what they held and the hours they had; it
    // does not turn on how the buyers were grouped. Two independent sums, so two dusts (Law 7).
    for (const other of [two, four]) {
      const dust = one.bread.dust + other.bread.dust;
      expect(Math.abs(other.bread.value - one.bread.value)).toBeLessThanOrEqual(dust);
    }
  });

  it('differs only by what whole people do, and by what those people are worth', () => {
    // Labour A4.b: a relationship carries a headcount, so the marginal match in a venue can land on
    // a different person at a different grain — at most one of them per venue.
    const venues = one.venues;
    expect(venues).toBeGreaterThan(0);
    // The error bar (XI-15, standing observations): what those people would earn over the run.
    const worth = venues * LABOUR_NUMBERS.hoursPerMember * one.wage * PERIODS;
    for (const other of [two, four]) {
      expect(Math.abs(other.employed - one.employed)).toBeLessThanOrEqual(venues);
      expect(Math.abs(other.cash - one.cash)).toBeLessThanOrEqual(worth);
      expect(Math.abs(other.money - one.money)).toBeLessThanOrEqual(worth);
    }
    // Law 7: the bar is a quantity of money, derived from a wage and a run, not a band on a total.
    expect(worth).toBeLessThan(one.cash);
  });
});
