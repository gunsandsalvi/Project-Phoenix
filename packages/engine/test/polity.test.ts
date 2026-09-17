/**
 * The polity: the constitution's own numbers, and the rule that turns votes into seats.
 *
 * @spec Polity A1 Polity A4 Polity C1 Polity C2 Polity C4 Polity D1 Polity D5 XI-17 Law 15
 */
import { describe, expect, it } from 'vitest';
import { ALLOTMENT_RULES, POLITY_PARAMS, allotmentBy } from '../src/mechanisms/polity/index.js';
import { TREASURY_PARAMS } from '../src/mechanisms/treasury/index.js';
import { rigWorld } from './rig.js';
import { PLATFORMS } from '../src/mechanisms/polity/platforms.js';
import { platformPositions, distanceBetween, spreadAcross } from '../src/registry/platforms.js';

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

describe('every party states a position on every number parliament owns (Polity A2, D5, 19.3)', () => {
  const w = rigWorld('platforms');
  const mine = w.params.all().filter((d) => d.kind === 'policy' && d.owner === 'parliament');

  it('covers all of them, in every platform, and the world would not have opened otherwise', () => {
    expect(PLATFORMS.length).toBeGreaterThan(1);
    expect(mine.length).toBeGreaterThan(0);
    const said = platformPositions(PLATFORMS, w.params.all());
    expect(said.size).toBe(PLATFORMS.length);
    for (const positions of said.values()) {
      // A2: EVERY one. A party that says nothing about a number has not given a cell enough to
      // vote on, and whatever it did about it afterwards would arrive from nowhere.
      expect(positions.size).toBe(mine.length);
    }
  });

  it('refuses a missing position, one on something parliament does not own, and a duplicate', () => {
    const [first] = PLATFORMS;
    expect(first).toBeDefined();
    if (first === undefined) return;
    const short = { ...first, positions: first.positions.slice(1) };
    expect(() => platformPositions([short], w.params.all())).toThrow();
    // D5, F2: a party cannot promise the central bank's rate — it is not parliament's to set.
    const overreach = {
      ...first,
      positions: [...first.positions, { on: 'centralBank.policyRate.USD', value: 0, why: 'it cannot.' }],
    };
    expect(() => platformPositions([overreach], w.params.all())).toThrow();
    // And two positions on one number is two answers to one question (Law 4).
    const twice = {
      ...first,
      positions: [...first.positions, { on: 'treasury.tax.income', value: 0.9, why: 'and also this.' }],
    };
    expect(() => platformPositions([twice], w.params.all())).toThrow();
    // Two platforms with one name is two parties nobody can tell apart.
    expect(() => platformPositions([first, first], w.params.all())).toThrow();
  });

  it('measures how far apart two platforms are against how far apart they all are (B2)', () => {
    const said = platformPositions(PLATFORMS, w.params.all());
    const spread = spreadAcross(said);
    expect(spread.size).toBeGreaterThan(0);
    const [a, b, c] = [...said.values()];
    expect(a).toBeDefined();
    expect(b).toBeDefined();
    expect(c).toBeDefined();
    if (a === undefined || b === undefined || c === undefined) return;
    // A platform is no distance at all from itself, and the two that differ most are further apart
    // than either is from the one between them — which is what "between" means here.
    expect(distanceBetween(a, a, spread)).toBe(0);
    expect(distanceBetween(a, b, spread)).toBeGreaterThan(distanceBetween(a, c, spread));
    expect(distanceBetween(a, b, spread)).toBeGreaterThan(distanceBetween(c, b, spread));
  });
});
