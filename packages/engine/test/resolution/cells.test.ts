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
  FIRMS,
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
import { unexpected } from '../expected.js';

const PERIODS = 26;
/** Goods A2: the hours the recipe names for the finished good, and Firm A3's leanest firm at it. */
const GOODS_HOURS_PER_TONNE = 14;
const LEANEST_FIRM = Math.min(...FIRMS.filter((f) => f.subUnit === 'bread').map((f) => f.labourScale));

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
    // Every family at zero at every grain, but for the one red this world is expected to show
    // (test/expected.ts): a bank overdrawn at the central bank with no money market to lend to it.
    expect(unexpected(w.step().audit)).toEqual([]);
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
  // Labour D1.c: the going rate is a public read of what is actually paid, occupation by
  // occupation. The most any venue pays is the most a whole person could be earning, which is what
  // bounds the money that follows one — a single venue's last print is one thin book and is not.
  const rates = w.journal.ofKind('labour.goingRate');
  const last = rates[rates.length - 1];
  const paid = (last?.data['wagePerHour'] ?? {}) as Record<string, number>;
  const wage = Object.values(paid).reduce((a, x) => (x > a ? x : a), 0);
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

  it('differs only by what whole people do, and by no more than those people are worth', () => {
    // Labour A4.b: a relationship carries a headcount, so the marginal match in a venue can land on
    // a different person at a different grain — at most one of them per venue. Everything that
    // moves, moves because of those people: they earn, they spend, and they make things.
    const venues = one.venues;
    expect(venues).toBeGreaterThan(0);
    // The error bar (XI-15, standing observations). It is derived twice over from the mechanism and
    // is a quantity in each case, never a percentage of anything (Law 7): the hours a whole person
    // supplies over the run, at the most any venue is paying, is what the money may differ by; and
    // those same hours over the fewest hours a tonne takes anybody is what the STOCK may differ by.
    const hours = venues * LABOUR_NUMBERS.hoursPerMember * PERIODS;
    const worth = hours * one.wage;
    const made = hours / (GOODS_HOURS_PER_TONNE * LEANEST_FIRM);
    for (const other of [two, four]) {
      expect(Math.abs(other.employed - one.employed)).toBeLessThanOrEqual(venues);
      expect(Math.abs(other.cash - one.cash)).toBeLessThanOrEqual(worth);
      expect(Math.abs(other.money - one.money)).toBeLessThanOrEqual(worth);
      // The stock of goods used to come out identical at every grain, because almost nothing was
      // being made. Now that the lines run, WHO was hired decides what got made, so the real side
      // moves with the grain too — inside what those people's hours could have produced.
      expect(Math.abs(other.bread.value - one.bread.value)).toBeLessThanOrEqual(made);
    }
    // Law 7 again: a bar bigger than the thing it bounds would not be a bar at all.
    expect(worth).toBeLessThan(one.cash);
    expect(made).toBeLessThan(one.people);
  });
});
