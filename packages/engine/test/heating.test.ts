/**
 * A cold week burns more: the household basket reads the region's warmth where the fuel line needs
 * it, and nothing else in the basket does (Goods B4, Commodities Spot E2, 12d.3).
 *
 * @spec Goods B4 Commodities Spot E2 Households C3 Observer A3 Law 19
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD } from '../src/index.js';
import { basketOf } from '../src/mechanisms/households/consume.js';
import { CONSUMPTION_TAX, HOUSEHOLD_PARAMS } from '../src/mechanisms/households/index.js';
import { CONSUMPTION } from '../src/mechanisms/households/data.js';
import { WARMTH, conditionsSeen } from '../src/registry/environment.js';
import { rigWorld } from './rig.js';

describe('the fuel line of the basket stands against the week’s warmth (12d.3)', () => {
  it('a member burns a normal week’s litres over how warm the week was, and a loaf is a loaf whatever the weather', () => {
    const w = rigWorld('heating');
    let fuelChecked = 0;
    let cold = 0;
    for (let i = 0; i < 6; i += 1) {
      w.step();
      for (const p of w.parties.ofKind(HOUSEHOLD)) {
        if (p.representation !== 'cell' || !p.status.alive || p.key['estate'] !== 'living') continue;
        const view = w.participantView(p.id);
        const here = conditionsSeen(view, p.region);
        if (here === undefined) continue;
        const warmth = here.get(WARMTH);
        expect(warmth).toBeDefined();
        if (warmth === undefined) continue;
        // The basket is costed at the tax alone; patience is the spend's and a cell's own working store.
        const basket = basketOf(view, CONSUMPTION, { patience: 1, steps: view.params.count(HOUSEHOLD_PARAMS.steps), consumptionTax: view.params.ratio(CONSUMPTION_TAX) });
        for (const line of basket.lines) {
          const normal = line.row.neededPerMember;
          if (line.row.subUnit === 'retailFuel') {
            // Goods B4: the litres are a normal week's over the warmth — a colder week burns more.
            expect(line.needed).toBeCloseTo(normal / warmth, 12);
            expect(line.wanted).toBeCloseTo(line.row.wantedPerMember / warmth, 12);
            fuelChecked += 1;
            if (Math.abs(warmth - 1) > 1e-9) cold += 1;
          } else {
            expect(line.row.standsAgainst).toEqual([]);
            expect(line.needed).toBe(normal);
          }
        }
      }
    }
    expect(fuelChecked).toBeGreaterThan(0);
    // The read bites: the environment moved off normal in some week, and the basket moved with it.
    expect(cold).toBeGreaterThan(0);
  });
});
