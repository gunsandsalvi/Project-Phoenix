/**
 * What is public about what a party is exposed to reaches its outlook as an observation (§46 A2.a,
 * C2.a, 12d.1). The exposure set is a read of its holdings and its rows; nothing private crosses.
 *
 * @spec Expectations A2 Expectations A2.a Expectations C2.a Expectations D1 Law 19
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD, type World } from '../src/index.js';
import { about } from '../src/world/context.js';
import { rigWorld } from './rig.js';

type Book = Record<string, Record<string, { expected: number; unit: string }>>;

/** The party's own book of outlooks, read off the store the module declares (a test may). */
function outlooksOf(w: World, party: string): Record<string, { expected: number; unit: string }> {
  const book = w.stateSlots()['expectations/outlooks'] as Book;
  return book[party] ?? {};
}

describe('what is public about what a party is exposed to reaches it (12d.1)', () => {
  const w = rigWorld('observe');
  // Twenty-two periods: the first quarter's statement is published at period 20 in this model.
  for (let i = 0; i < 22; i += 1) w.step();

  it('holds an outlook on the price of every line it holds that printed this period, traded or not', () => {
    let checked = 0;
    for (const p of w.parties.all()) {
      if (!p.status.alive) continue;
      const own = outlooksOf(w, String(p.id));
      for (const h of w.register.holdingsOf(p.id)) {
        if (!w.prices.read(h.instrument, w.period).some) continue;
        // A lot that arrived after the close — an estate's distribution, a probate move — is seen
        // next period; what it held when the period was scored is what it observed.
        if (h.lots.every((l) => l.acquired === w.period)) continue;
        expect(own[about({ on: 'price', instrument: h.instrument })]).toBeDefined();
        checked += 1;
      }
    }
    expect(checked).toBeGreaterThan(0);
  });

  it('a party on an employment row holds an outlook on the going rate where it works or hires; one outside the trade does not', () => {
    expect(w.journal.ofKind('labour.goingRate').length).toBeGreaterThan(0);
    const rows = w.employment.all();
    expect(rows.length).toBeGreaterThan(0);
    const onARow = new Set<string>();
    for (const r of rows) {
      onARow.add(String(r.worker));
      onARow.add(String(r.employer));
    }
    let seen = 0;
    for (const who of onARow) {
      if (!w.parties.has(who as never) || !w.parties.get(who as never).status.alive) continue;
      const wages = Object.keys(outlooksOf(w, who)).filter((k) => k.startsWith('wage.'));
      if (wages.length > 0) seen += 1;
    }
    expect(seen).toBeGreaterThan(0);
    // D1: a household with no row in any trade has been told nothing about any wage.
    for (const p of w.parties.ofKind(HOUSEHOLD)) {
      if (!p.status.alive || onARow.has(String(p.id))) continue;
      if (w.employment.everEmployed(p.id) || w.employment.ofWorker(p.id) !== undefined) continue;
      const wages = Object.keys(outlooksOf(w, String(p.id))).filter((k) => k.startsWith('wage.'));
      // A cell that once worked keeps what it saw; one that never did saw nothing.
      const everWorked = w.journal.ofKind('labour.hire').some((e) => e.subjects.includes(String(p.id)));
      if (!everWorked) expect(wages).toEqual([]);
    }
  });

  it('a depositor holds an outlook on the board of the bank it banks at, per annum', () => {
    const boards = w.journal.ofKind('bank.depositRate');
    expect(boards.length).toBeGreaterThan(0);
    const posted = new Set(boards.map((e) => String(e.subjects[0])));
    let seen = 0;
    for (const p of w.parties.ofKind(HOUSEHOLD)) {
      if (!p.status.alive || !posted.has(String(p.bank))) continue;
      const o = outlooksOf(w, String(p.id))[about({ on: 'deposit', bank: p.bank })];
      if (o === undefined) continue;
      expect(o.unit).toBe('per annum');
      seen += 1;
    }
    expect(seen).toBeGreaterThan(0);
  });

  it('a holder of a company’s paper learns what it published, the period after, and a stranger does not', () => {
    const reports = w.journal.ofKind('reporting.report').filter((e) => e.period < w.period);
    expect(reports.length).toBeGreaterThan(0);
    let holders = 0;
    for (const e of reports) {
      const company = String(e.subjects[0]);
      const lines = w.instruments.all().filter((i) => i.issuer.some && String(i.issuer.value) === company && i.status.live);
      const holding = new Set<string>();
      for (const line of lines) for (const h of w.register.holdersOf(line.id)) if (String(h) !== company) holding.add(String(h));
      for (const h of holding) {
        if (!w.parties.has(h as never) || !w.parties.get(h as never).status.alive) continue;
        // XI-15: a cell split off its parent after the statement came out has a fresh book — its
        // parent saw the statement, and what a new cell inherits of that is 21.20's finding.
        if (w.journal.ofKind('weight').some((x) => x.data['to'] === h && x.period > e.period)) continue;
        const o = outlooksOf(w, h)[about({ on: 'reported', party: company as never })];
        expect(o).toBeDefined();
        holders += 1;
      }
      // D1: a party holding nothing the company issued has no view of what it reported.
      for (const p of w.parties.ofKind(HOUSEHOLD)) {
        if (!p.status.alive || holding.has(String(p.id))) continue;
        expect(outlooksOf(w, String(p.id))[about({ on: 'reported', party: company as never })]).toBeUndefined();
      }
    }
    expect(holders).toBeGreaterThan(0);
  });
});
