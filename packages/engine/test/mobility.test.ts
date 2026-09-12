/**
 * Labour mobility and participation (13d steps 1-3): people move trade when their own does not take
 * them, and looking for work is a decision with the wage in it.
 *
 * @spec Labour A3 Labour A3.a Labour A3.b Labour B1 Labour B3 Labour C4 XI-10 Law 2 Law 15
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD, LABOUR_PARAMS, paramId } from '../src/index.js';
import { LABOUR_NUMBERS, OCCUPATIONS } from '../src/mechanisms/labour/data.js';
import { rigWorld } from './rig.js';

describe('what changing trade costs (A3.b, XI-10)', () => {
  it('is TIME and never a fee, because a fee would be a flow with no payee (Law 5)', () => {
    const w = rigWorld('mobility-a');
    const d = w.params.decl(LABOUR_PARAMS.retraining);
    expect(d.kind).toBe('technology');
    expect(d.dimension).toBe('periods');
    expect(d.value).toBeGreaterThan(0);
    // What a new trade costs an employer is weeks of wages for work it does not yet get, and it is
    // paid in the ordinary wage bill rather than in a transfer nobody receives.
    for (const p of w.params.all()) {
      const id = String(p.id).toLowerCase();
      if (!id.startsWith('labour.')) continue;
      expect(id).not.toContain('retrainingcost');
      expect(id).not.toContain('mobilityrate');
      expect(id).not.toContain('transition');
    }
  });

  it('makes a mover wait longer than somebody who already has the trade', () => {
    // C2 plus A3.b: the ordinary lag, and the retraining on top of it. Direction, never a level.
    expect(LABOUR_NUMBERS.retrainingPeriods).toBeGreaterThan(0);
    expect(
      LABOUR_NUMBERS.hiringLagPeriods + LABOUR_NUMBERS.retrainingPeriods,
    ).toBeGreaterThan(LABOUR_NUMBERS.hiringLagPeriods);
  });

  it('states no flow between one occupation and another anywhere (Law 2)', () => {
    const w = rigWorld('mobility-a');
    for (const a of OCCUPATIONS) {
      for (const b of OCCUPATIONS) {
        if (a.id === b.id) continue;
        // Who moves is whoever the first round left over. A world with a rate from field work to
        // baking would be a world where the answer was written down.
        expect(() => w.params.decl(paramId(`labour.move.${a.id}.${b.id}`))).toThrow();
      }
    }
  });

  it('leaves every trade its own venue, so a nurse is not a bricklayer’s vacancy (A3.a)', () => {
    const w = rigWorld('mobility-a');
    const venues = w.venues.filter((v) => String(v.id).startsWith('labour.'));
    const trades = new Set(venues.map((v) => v.key['occupation']));
    expect(trades.size).toBeGreaterThan(1);
    for (const v of venues) {
      expect(v.key['skill']).toBeDefined();
      expect(v.key['sector']).toBeDefined();
    }
  });
});

describe('a bank employs people, and the borrower pays them (13d, Banks Lending C1.d)', () => {
  it('has no operating cost parameter anywhere: it was a wage bill paid to nobody (XI-14, Law 5)', () => {
    const w = rigWorld('mobility-a');
    // It was half a per cent a year on every principal, added into every quote and landing nowhere.
    expect(() => w.params.decl(paramId('loan.operatingCost'))).toThrow();
    for (const d of w.params.all()) {
      expect(String(d.id)).not.toContain('operatingCost');
    }
  });

  it('declares instead what ONE LOAN takes, in hours somebody is paid for', () => {
    const w = rigWorld('mobility-a');
    const d = w.params.decl(paramId('bank.hoursPerLoanPeriod'));
    expect(d.kind).toBe('technology');
    expect(d.dimension).toBe('count');
    expect(d.value).toBeGreaterThan(0);
    // Per LOAN, because a loan costs about the same to make whatever its size — which is why the
    // cost per unit of principal now falls as the loan gets bigger, out of the arithmetic.
    expect(d.unit).toContain('per loan');
  });

  it('hires in a trade of its own, in the same venue everybody else does (Labour A3)', () => {
    const w = rigWorld('mobility-a');
    const trades = OCCUPATIONS.filter((o) => o.sector === 'finance').map((o) => o.id);
    expect(trades.length).toBeGreaterThan(0);
    for (const t of trades) {
      const venues = w.venues.filter((v) => v.key['occupation'] === t);
      // A lending officer is not a baker, and the venue says so.
      expect(venues.length).toBeGreaterThan(0);
    }
  });
});

describe('a cell moves between keys and nothing crosses (13d.1, XI-15)', () => {
  it('ages people by re-keying them, which is a split with a different key', () => {
    const w = rigWorld('rekey-a');
    const before = w.parties.ofKind(HOUSEHOLD).reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 0), 0);
    for (let i = 0; i < 3; i += 1) w.step();
    const after = w.parties.ofKind(HOUSEHOLD).reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 0), 0);
    // XI-15: NOBODY APPEARS AND NOBODY DISAPPEARS. A re-key is exact by construction, because it is
    // a split: the part that crossed carries the same per-member state and the totals add up.
    expect(after).toBe(before);
    const moves = w.journal
      .ofKind('weight')
      .filter((e) => e.data['kind'] === 'promotion' && typeof e.data['to'] === 'string');
    expect(moves.length).toBeGreaterThan(0);
    for (const m of moves) {
      // The event names both cells and says which dimension moved, so a reader can see what
      // happened without being told; and no instruction was needed, because nothing was paid.
      expect(m.subjects.length).toBe(2);
      expect(m.data['key']).toBeDefined();
    }
  });

  it('states no retirement mechanism anywhere: retiring is an age (F3)', () => {
    const w = rigWorld('rekey-a');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('retirementrate');
      expect(id).not.toContain('retire.share');
    }
    // What IS declared is the age the band starts at, which is a fact about people.
    expect(w.registry.cohorts.length).toBeGreaterThan(1);
    for (const c of w.registry.cohorts) expect(c.fromAge).toBeGreaterThanOrEqual(0);
  });
});
