/**
 * The polity: the constitution's own numbers, and the rule that turns votes into seats.
 *
 * @spec Polity A1 Polity A4 Polity C1 Polity C2 Polity C4 Polity D1 Polity D5 XI-17 Law 15
 */
import { describe, expect, it } from 'vitest';
import { ALLOTMENT_RULES, POLITY_PARAMS, allotmentBy } from '../src/mechanisms/polity/index.js';
import { TREASURY_PARAMS } from '../src/mechanisms/treasury/index.js';
import { rigWorld } from './rig.js';

describe('the constitution states its own numbers (Polity A1, A4, C1, C2, C4, D5)', () => {
  const w = rigWorld('polity');

  it('names a seat count, a term in months, a rule, a distance and a lag — all the constitution’s', () => {
    for (const id of Object.values(POLITY_PARAMS)) {
      const d = w.params.decl(id);
      // D5: a primitive has an OWNER, and these are the constitution's — not parliament's, because
      // a body does not get to rewrite the rule that elected it.
      expect(d.kind).toBe('policy');
      expect(d.owner).toBe('constitution');
    }
    expect(w.params.count(POLITY_PARAMS.seats)).toBeGreaterThan(1);
    // A4: the term is in MONTHS, so the election falls on a day and not on a remainder.
    expect(w.params.decl(POLITY_PARAMS.termMonths).dimension).toBe('months');
    expect(w.params.periods(POLITY_PARAMS.mandateLag)).toBeGreaterThan(0);
  });

  it('turns votes into whole seats by the stated rule, and the house is always full (C1)', () => {
    const votes = new Map([
      ['a', 1000],
      ['b', 600],
      ['c', 401],
    ]);
    const seats = 100;
    for (const rule of ALLOTMENT_RULES) {
      const out = rule.allot(votes, seats);
      let total = 0;
      for (const n of out.values()) {
        expect(Number.isInteger(n)).toBe(true);
        expect(n).toBeGreaterThanOrEqual(0);
        total += n;
      }
      // A seat is a person: whole ones, and all of them sit.
      expect(total).toBe(seats);
    }
    // Proportional gives the shares; first past the post gives the lot to the largest. The two are
    // different parliaments from the SAME votes, which is what makes the rule a primitive (C1).
    const p = allotmentBy('proportional').allot(votes, seats);
    const f = allotmentBy('firstPastThePost').allot(votes, seats);
    expect(p.get('a')).toBeLessThan(seats);
    expect(p.get('b')).toBeGreaterThan(0);
    expect(f.get('a')).toBe(seats);
    expect(f.get('b')).toBe(0);
    // Determinism: the same votes give the same house, every time, with ties broken by name.
    expect([...allotmentBy('proportional').allot(votes, seats)]).toEqual([...p]);
  });

  it('has four tax bases with rates, and every one of them is parliament’s (D1, §30 C1)', () => {
    const bases = [
      TREASURY_PARAMS.taxProfits,
      TREASURY_PARAMS.taxInterest,
      TREASURY_PARAMS.taxGains,
      TREASURY_PARAMS.taxConsumption,
    ];
    for (const id of bases) {
      const d = w.params.decl(id);
      expect(d.kind).toBe('policy');
      expect(d.owner).toBe('parliament');
      expect(w.params.ratio(id)).toBeGreaterThan(0);
    }
    // 19.2: a gain is not a wage. The two rates are separate numbers and may differ, which is one
    // of the things an election is actually about.
    expect(String(TREASURY_PARAMS.taxGains)).not.toBe(String(TREASURY_PARAMS.taxIncome));
  });
});
