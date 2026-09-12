/**
 * Labour mobility and participation (13d steps 1-3): people move trade when their own does not take
 * them, and looking for work is a decision with the wage in it.
 *
 * @spec Labour A3 Labour A3.a Labour A3.b Labour B1 Labour B3 Labour C4 XI-10 Law 2 Law 15
 */
import { describe, expect, it } from 'vitest';
import { LABOUR_PARAMS, paramId } from '../src/index.js';
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
