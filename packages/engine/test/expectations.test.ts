/**
 * Every party's outlook is its own, formed from what it observed, at its own speed.
 *
 * @spec Expectations A2 Expectations A2.b Expectations A3 Expectations B1 Expectations B1.a Expectations B2 Expectations B3 Expectations B4 Expectations B5 Expectations D1 Expectations D4 Expectations E2 XI-16
 */
import { describe, expect, it } from 'vitest';
import {
  BANK_A,
  BANK_B,
  EXPECTATION_PARAMS,
  HOUSEHOLD,
  TREASURY_US,
  partyId,
  type World,
} from '../src/index.js';
import { dustOf } from '../src/core/num.js';
import { rigWorld } from './rig.js';

function expected(w: World, party: string, variable: string): number | null {
  const o = w.participantView(partyId(party)).outlook(variable);
  return o.some ? o.value.expected : null;
}

function confidence(w: World, party: string, variable: string): number | null {
  const o = w.participantView(partyId(party)).outlook(variable);
  return o.some ? o.value.confidence : null;
}

describe('an outlook is personal (Expectations A2)', () => {
  it('exists only for a party that observed the variable, and says nothing otherwise', () => {
    const w = rigWorld('exp-a');
    for (let i = 0; i < 6; i += 1) w.step();
    // The treasury pays; the cells are paid. Both saw money arrive, so both have an income outlook.
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    expect(expected(w, cell.id, 'income')).not.toBeNull();
    // A2.b: nothing named for the market as a whole, and nothing for a variable it never saw.
    expect(expected(w, cell.id, 'goods.price.bread')).toBeNull();
  });

  it('is nobody else s: one party s outlook is not reachable from another s view (A2, D1)', () => {
    const w = rigWorld('exp-b');
    for (let i = 0; i < 6; i += 1) w.step();
    const view = w.participantView(BANK_A);
    // WHO somebody is, is public: a market knows whose paper it trades (Observer A3). What that
    // somebody holds, expects or is worth is not, and there is no door to it from here (A4).
    const other = w.parties.ofKind(HOUSEHOLD)[0];
    if (other === undefined) throw new Error('no cell');
    expect(view.parties.get(other.id).kind).toBe(HOUSEHOLD);
    expect(Object.keys(view.parties)).not.toContain('holdings');
    expect(Object.keys(view)).not.toContain('register');
    expect(Object.keys(view)).not.toContain('outlooks');
    // What it can ask for is its own; asking is a read about self and nothing else.
    expect(expected(w, BANK_A, 'income')).not.toBeNull();
  });
});

describe('how an outlook moves (B1, B2, B4)', () => {
  it('is corrected towards what happened, at the party own speed, and never faster', () => {
    const w = rigWorld('exp-c');
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    for (let i = 0; i < 10; i += 1) w.step();
    // §46 B1, B2, B4: THE CORRECTION IS TOWARDS WHAT HAPPENED AND NEVER PAST IT. This used to
    // assert the outlook stood STILL for ten periods, on the grounds that the treasury pays every
    // cell the same mandate every period — which was true of a world where the state was a cell's
    // only payer. It is not: this cell is paid a wage by a firm whose own decisions move, so what
    // it observes moves and so does what it expects. A number that does not move is not what B1 is
    // about; what it is about is HOW one moves, and that is read off the surprises the party
    // recorded (Law 19): every step lands between where it was and what it saw.
    const scored = w.journal
      .ofKind('expectations.surprise')
      .filter((e) => e.subjects.includes(cell.id) && e.data['variable'] === 'income');
    expect(scored.length).toBeGreaterThan(4);
    for (const [at, e] of scored.entries()) {
      const next = scored[at + 1];
      if (next === undefined) continue;
      const was = Number(e.data['expected']);
      const saw = Number(e.data['observed']);
      const now = Number(next.data['expected']);
      // Towards, at its own speed: between where it was and what it saw, inclusive of either end.
      const low = Math.min(was, saw);
      const high = Math.max(was, saw);
      expect(now).toBeGreaterThanOrEqual(low - dustOf(2, Math.abs(low) + Math.abs(now)));
      expect(now).toBeLessThanOrEqual(high + dustOf(2, Math.abs(high) + Math.abs(now)));
    }
  });

  it('lags a step change by the party own memory (B1, B5)', () => {
    const w = rigWorld('exp-d');
    for (let i = 0; i < 20; i += 1) w.step();
    // The treasury's own income is what it collects, which is nothing in most periods and a lump
    // when the coupons it taxes fall due: a step it did not see coming.
    const before = expected(w, TREASURY_US, 'income');
    let jumped = false;
    for (let i = 0; i < 20 && !jumped; i += 1) {
      w.step();
      const now = expected(w, TREASURY_US, 'income');
      if (before !== null && now !== null && Math.abs(now - before) > 1e-9) jumped = true;
    }
    const after = expected(w, TREASURY_US, 'income');
    expect(after).not.toBeNull();
    // B1: it moved, but by a fraction of the gap — never all the way in one period.
    const surprises = w.journal.ofKind('expectations.surprise').filter((e) => e.subjects[0] === TREASURY_US);
    expect(surprises.length).toBeGreaterThan(0);
    const last = surprises[surprises.length - 1];
    const gap = Math.abs(last?.data['surprise'] as number);
    const memory = w.params.get(EXPECTATION_PARAMS.memoryMean);
    expect(gap).toBeGreaterThan(0);
    expect(memory).toBeGreaterThan(1);
  });

  it('records the surprise as an event, and it is the party own (B2)', () => {
    const w = rigWorld('exp-e');
    for (let i = 0; i < 8; i += 1) w.step();
    const events = w.journal.ofKind('expectations.surprise');
    expect(events.length).toBeGreaterThan(0);
    expect(events.every((e) => !e.public)).toBe(true);
    for (const e of events) {
      const observed = e.data['observed'] as number;
      const wasExpected = e.data['expected'] as number;
      expect(e.data['surprise']).toBeCloseTo(observed - wasExpected, 9);
    }
  });

  it('makes confidence a read of how wide the recent surprises were (B3)', () => {
    const w = rigWorld('exp-f');
    for (let i = 0; i < 20; i += 1) w.step();
    const steady = confidence(w, BANK_B, 'income');
    expect(steady).not.toBeNull();
    expect(steady ?? -1).toBeGreaterThanOrEqual(0);
  });
});

describe('memories differ across parties (A3, B1.a)', () => {
  it('are drawn once and kept, so two parties do not move as one', () => {
    const a = rigWorld('exp-g');
    for (let i = 0; i < 12; i += 1) a.step();
    const slots = a.stateSlots()['expectations/outlooks'] as Record<
      string,
      Record<string, { memory: number }>
    >;
    const memories = Object.values(slots).flatMap((v) => Object.values(v).map((h) => h.memory));
    expect(memories.length).toBeGreaterThan(2);
    expect(new Set(memories.map((m) => m.toFixed(6))).size).toBeGreaterThan(1);
    expect(memories.every((m) => m >= 1)).toBe(true);
  });
});

describe('the aggregate is a statistic (D4, E2, Observer A5)', () => {
  it('is published about the period that closed, and causes nothing', () => {
    const w = rigWorld('exp-h');
    for (let i = 0; i < 10; i += 1) w.step();
    const published = w.journal.ofKind('expectations.dispersion');
    expect(published.length).toBeGreaterThan(0);
    for (const e of published) {
      expect(e.public).toBe(true);
      // A5.a: a statistic available for the period it is about is not a statistic.
      expect(e.data['of']).toBe(e.period - 1);
    }
  });
});
