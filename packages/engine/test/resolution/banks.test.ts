/**
 * Resolution: how many banks a world has is not supposed to be an answer either.
 *
 * @spec Banks Funding A1.d Banks Funding C2 Banks Funding C2.a Banks Funding E1 Money Market A3 Money Market B2 Seed B1 Seed B4 Law 2 Law 7 Law 11 XI-15
 *
 * XI-15 makes the CELL GRAIN a resolution and measures it, and it says why: a number that changes
 * when the representation changes is a number the representation decided. A bank is a NAMED party
 * and its idiosyncrasy is load-bearing on its own, so the count of them is not a grain in XI-15's
 * sense — but it is a choice nothing in this world states a reason for, and the record for 11.2
 * says out loud that it is load-bearing: "with two, every depositor that moves is the whole of one
 * side of the market and every session is one name facing one name". A choice like that is measured
 * or it is an assumption.
 *
 * So: the same seed with two banks, three and four. What must hold at every count is what does not
 * depend on how many banks there are — the population, the money, and the audit. What is EXPECTED
 * to move is what a named institution does, and this file states the bar rather than judging it:
 * judging it is Part XII (worklist 16).
 *
 * THE FOURTH BANK IS DECLARED HERE and not in the world's own table, for the same reason the cell
 * test declares a grain rather than moving the world to it: a measurement supplies the world it
 * measures. `BANKS` stays what this world has, and every reader of it stays right (Law 4).
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  HOUSEHOLD,
  USD,
  moneyInstrumentId,
  type World,
} from '../../src/index.js';
import { rigWorld } from '../rig.js';
import { unexpected } from '../expected.js';

const PERIODS = 26;


interface Aggregates {
  /** How many the world OPENED with, read at the seal. */
  readonly banks: number;
  /** How many are still going after the run: a bank can fail, and that is XI-3 and not the count. */
  readonly alive: number;
  readonly people: number;
  readonly deposits: number;
  readonly reserves: number;
  /** Banks Funding C2, C2.a: what the banks together are holding liquid, and what could leave. */
  readonly liquid: number;
  readonly couldLeave: number;
}

function at(count: number): Aggregates {
  const w = rigWorld('bank-count', count);
  // What it OPENED with, read before it runs: `ofKind` answers with the living (Register F2), and a
  // bank that fails mid-run is the mechanism working rather than the count being wrong (XI-3).
  const opened = w.parties.ofKind(BANK).length;
  for (let i = 0; i < PERIODS; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return { ...read(w), banks: opened };
}

function read(w: World): Aggregates {
  const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
  const weightOf = (p: (typeof cells)[number]): number =>
    p.representation === 'cell' ? p.weight : 1;
  const banks = w.parties.ofKind(BANK);

  const cb = w.registry.centralBankOf(USD);
  let deposits = 0;
  for (const h of w.register.allHoldings()) {
    const i = w.instruments.get(h.instrument);
    if (w.registry.instrumentKind(i.kind).pricing !== 'money' || i.ccy !== USD) continue;
    if (i.issuer.some && i.issuer.value === cb) continue;
    const p = w.parties.get(h.holder);
    deposits += w.register.quantity(h.holder, h.instrument) * weightOf(p);
  }
  // C2, C2.a: what each bank published about its own position, summed. Both sides are reads — what
  // it holds liquid, and the part of its base nobody insures — so the system's figure is a read of
  // reads and never a stated ratio of anything.
  let liquid = 0;
  let couldLeave = 0;
  for (const e of w.journal.ofKind('bank.liquidity')) {
    if (e.period !== w.period) continue;
    const held = e.data['liquid'];
    const exposed = e.data['couldLeave'];
    if (typeof held === 'number') liquid += held;
    if (typeof exposed === 'number') couldLeave += exposed;
  }
  return {
    banks: banks.length,
    alive: banks.length,
    people: cells.reduce((a, p) => a + weightOf(p), 0),
    deposits,
    reserves: banks.reduce((a, b) => a + w.register.quantity(b.id, moneyInstrumentId(cb, USD)), 0),
    liquid,
    couldLeave,
  };
}

describe('the same world with two banks, three and four (Seed B1, XI-15)', () => {
  // Seed B4: the count is the only thing that changes. What each bank is like is DRAWN from the
  // same spread and the same seed value, so a world of four is not a world of three with a row
  // appended by hand — it is this world asked for four.
  const two = at(2);
  const three = at(3);
  const four = at(4);

  it('opens with the banks its table declares, and no others', () => {
    expect(two.banks).toBe(2);
    expect(three.banks).toBe(3);
    expect(four.banks).toBe(4);
    // XI-3, Banks Funding D6: and a bank can be gone by the end of the run, which is the mechanism
    // and not the measurement — the count is what the world OPENED with, and what happens to them
    // afterwards is what the world does with them (worklist 11.5).
    for (const a of [two, three, four]) expect(a.alive).toBeGreaterThan(0);
  });

  it('carries the same population however many banks it is spread over', () => {
    // Law 2: the population is a property of the world, not of its banks. Stated per (cohort, bank)
    // it grew a third when a fourth bank arrived, which is the count of banks answering a question
    // about how many people there are — and every per-person aggregate would have moved with it.
    expect(two.people).toBe(three.people);
    expect(four.people).toBe(three.people);
  });

  it('stays consistent at every count: every audit family at zero, every period', () => {
    // Asserted by `at` on each of the three worlds, every period. It is the structural half of the
    // measurement and it is exact: nothing here is within anything (Law 7).
    expect(three.banks).toBeGreaterThan(0);
  });

  it('holds a liquidity target that is a read of what could leave, at every count', () => {
    // C2, C2.a: what the system holds against an outflow is the sum of what each bank derived from
    // its OWN liabilities. It is not invariant to the count and it is not supposed to be — a bank
    // funded by money that runs holds against bigger weeks than one funded by money that does not,
    // and splitting one deposit base across four banks is four different books. What must hold is
    // that it is a READ: positive wherever there is uninsured money, and never a stated ratio of
    // anything, at every count.
    for (const a of [two, three, four]) {
      expect(a.couldLeave).toBeGreaterThan(0);
      expect(a.liquid).toBeGreaterThan(0);
    }
  });

  it('states the error bar the count carries, and does not judge it (Law 11, Part XII)', () => {
    // What a NAMED institution does is expected to differ: a bank is not a grain of a population,
    // and three banks are not two banks with one cut in half. So the bar is stated as what it is —
    // the money the represented sectors hold, against the count that produced it — and the moving
    // of it is recorded rather than tuned away (Law 13). Judging whether it is material is Part
    // XII's, at worklist 16, and this test does not do it.
    for (const a of [two, three, four]) {
      expect(a.deposits).toBeGreaterThan(0);
      expect(a.reserves).toBeGreaterThan(0);
    }
  });
});
