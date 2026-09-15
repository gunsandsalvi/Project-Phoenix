/**
 * One register answers who works where; a bank with one bad period keeps its desk (Labour A4,
 * A4.a, B3, C1, F1, F2, 12b.6).
 *
 * @spec Labour A4 Labour A4.a Labour B3 Labour C1 Labour F1 Labour F2 Law 4 Law 19
 */
import { describe, expect, it } from 'vitest';
import { BANK, EMPLOYMENT, employmentOf, type ParticipantView } from '../src/index.js';
import { staffOrders, staffVenue } from '../src/mechanisms/banks/staff.js';
import { rigWorld } from './rig.js';

describe('one register answers who works where (12b.1, 12b.2a)', () => {
  const w = rigWorld('employment');
  for (let i = 0; i < 6; i += 1) w.step();

  it('is the same answer from every door: the store, the register, the employer, the worker', () => {
    const live = w.agreements.ofKind(EMPLOYMENT).filter((a) => a.state === 'performing');
    expect(live.length).toBeGreaterThan(0);
    const all = w.employment.all();
    expect(all.length).toBe(live.length);
    let people = 0;
    for (const a of live) {
      const row = employmentOf(a);
      people += row.headcount;
      // Law 4: one row, reached four ways.
      expect(w.employment.get(row.id).id).toBe(row.id);
      expect(w.employment.ofWorker(row.worker)?.id).toBe(row.id);
      expect(w.employment.by(row.employer).some((r) => r.id === row.id)).toBe(true);
      expect(w.employment.at(row.employer, row.occupation, row.region).some((r) => r.id === row.id)).toBe(true);
      // B3, A4.c (12b.2a): a cell holds one job, and the row's people are the cell's people.
      const cell = w.parties.get(row.worker);
      expect(cell.representation === 'cell' ? cell.weight : -1).toBe(row.headcount);
      expect(w.employment.ofWorker(row.worker)?.employer).toBe(row.employer);
    }
    expect(w.employment.employed()).toBe(people);
    // F1, E1: an employer's payroll is its rows and nothing else.
    for (const employer of new Set(all.map((r) => r.employer))) {
      const rows = w.employment.by(employer);
      const p = w.employment.payrollOf(employer, w.period);
      expect(p.headcount).toBe(rows.reduce((t, r) => t + r.headcount, 0));
      expect(p.hours).toBe(rows.reduce((t, r) => t + r.hoursPerMember * r.headcount, 0));
      expect(w.participantView(employer).employs().map((r) => r.id).sort()).toEqual(rows.map((r) => r.id).sort());
    }
  });

  it('hires land where the register says, and nowhere else', () => {
    for (const e of w.journal.ofKind('labour.hire')) {
      const worker = w.parties.resolve(String(e.data['worker']) as never);
      const row = w.employment.ofWorker(worker.id);
      const separated = w.journal.ofKind('labour.separation').some((s) => String(s.data['row']) === String(e.data['row']) && s.period >= e.period);
      // A4.a: the person hired is on the employer's row now, or was separated since — one or the other.
      expect(row !== undefined ? String(row.employer) : 'separated').toBe(row !== undefined ? String(e.data['employer']) : 'separated');
      if (row === undefined) expect(separated || !worker.status.alive).toBe(true);
    }
  });
});

describe('a bank with one bad period keeps its desk (Labour C1, C3, 12b.3)', () => {
  it('bids for its desk from its outlook, so a week it lost money on is not a week it fires anybody', () => {
    const w = rigWorld('desk');
    for (let i = 0; i < 6; i += 1) w.step();
    const bank = w.parties.ofKind(BANK).find((b) => b.status.alive && w.employment.by(b.id).length > 0);
    expect(bank).toBeDefined();
    if (bank === undefined) return;
    const real = w.participantView(bank.id);
    const venue = staffVenue(real);
    expect(venue).toBeDefined();
    if (venue === undefined) return;
    // The same bank, having lost money this week: everything it knows is the same except last
    // week's result. What it posts for its desk must not be a cut.
    const badWeek: ParticipantView = { ...real, earned: () => -1_000_000 as never };
    const orders = staffOrders(badWeek, venue);
    expect(orders.some((o) => o.side === 'sell')).toBe(false);
    expect(orders).toEqual(staffOrders(real, venue));
  });
});
