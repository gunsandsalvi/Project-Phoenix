/**
 * Productivity is an outcome: hours per unit are a read of what the line has made (Firm A3,
 * Goods A2.c, 12c.1).
 *
 * @spec Firm A3 Goods A2.c Law 2 Law 19
 */
import { describe, expect, it } from 'vitest';
import { FIRM } from '../src/index.js';
import { learnedHoursPerUnit } from '../src/registry/physical.js';
import { technologyOf } from '../src/mechanisms/firms/decide.js';
import { asRatio } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { rigDraw, rigWorld } from './rig.js';

describe('hours per unit fall with what the line has made (12c.1)', () => {
  it('is the recipe\'s hours for a line that has made nothing, and less for every doubling after', () => {
    const base = asRatio(9, 'the recipe');
    const rate = asRatio(0.32, 'an 80% curve');
    expect(learnedHoursPerUnit(base, asQty(0, 'nothing'), rate)).toBe(9);
    const one = learnedHoursPerUnit(base, asQty(1, 'one'), rate);
    const three = learnedHoursPerUnit(base, asQty(3, 'three'), rate);
    const seven = learnedHoursPerUnit(base, asQty(7, 'seven'), rate);
    expect(one).toBeLessThan(9);
    // Wright: every doubling of cumulative output takes the same fraction off.
    expect(three / one).toBeCloseTo(seven / three, 9);
    expect(three / one).toBeCloseTo(Math.pow(2, -0.32), 9);
    // A recipe with no learning takes the same hours for ever — the rate is the whole of it.
    expect(learnedHoursPerUnit(base, asQty(1000, 'many'), asRatio(0, 'no learning'))).toBe(9);
  });

  it('is read off the ledger: a firm that has made something takes fewer hours than its recipe says, and nothing stores it', () => {
    const draw = rigDraw('learning');
    const w = rigWorld('learning');
    for (let i = 0; i < 8; i += 1) w.step();
    let seen = 0;
    for (const f of draw.firms) {
      if (!w.parties.has(f.firm as never)) continue;
      const party = w.parties.get(f.firm as never);
      if (!party.status.alive || party.kind !== FIRM) continue;
      const view = w.participantView(party.id);
      const tech = technologyOf(view, f);
      const made = view.made(`good.${tech.terms.subUnit}.${String(party.region)}` as never);
      const base = view.params.ratio(tech.terms.recipe.labourHoursPerUnit) * view.params.ratio(`firms.labourScale.${f.firm}` as never);
      if (made > 0) {
        seen += 1;
        expect(tech.hoursPerUnit).toBeLessThan(base);
        expect(tech.hoursPerUnit).toBeCloseTo(learnedHoursPerUnit(asRatio(base, 'base'), made, view.params.ratio(tech.terms.recipe.learningRate)), 9);
      } else {
        expect(tech.hoursPerUnit).toBeCloseTo(base, 9);
      }
    }
    expect(seen).toBeGreaterThan(0);
  });
});
