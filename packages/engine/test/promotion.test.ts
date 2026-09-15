/**
 * The kernel's doors for a party born after the seed (Firm Birth A1, XI-14, XI-15, 12.4a.1).
 *
 * @spec Firm Birth A1 Firm Birth A3 Small-Business Pools A6.c Small-Business Pools E5 XI-14 XI-15 Law 4
 */
import { describe, expect, it } from 'vitest';
import { FIRM, SMALL_FIRM, assemble, paramId, partyId, type MechanismContext, type SystemModule } from '../src/index.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

describe('a named party is born, its numbers are declared, and members are promoted into it', () => {
  it('moves the members’ share of every lot, records the event with before and after, and the population accounts for it', () => {
    let born: string | undefined;
    let from: string | undefined;
    let weightBefore = 0;
    const probe: SystemModule = {
      id: 'test.born',
      spec: 'Firm Birth A1',
      requires: ['small-business', 'firms'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.born',
          spec: 'Firm Birth A1',
          anchor: { after: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            const cell = ctx.parties.ofKind(SMALL_FIRM).find((p) => p.representation === 'cell' && p.status.alive && p.weight > 1);
            if (cell?.representation !== 'cell') return;
            const id = partyId(`${String(cell.id)}.born`);
            // A1: a new named party with a new name; A3: it holds nothing and owes nothing yet.
            ctx.enter({ id, kind: FIRM, representation: 'named', region: cell.region, name: 'a firm that was a small one', bank: cell.bank, status: { alive: true, standing: 'good' } });
            // XI-14: its own number, declared the period it is born, through the one register.
            ctx.declare({ id: paramId(`test.hurdle.${String(id)}`), value: 0.04, unit: 'per annum', dimension: 'perAnnum', kind: 'preference', owner: 'model', why: 'a born firm’s own hurdle, drawn under its own name' });
            weightBefore = cell.weight;
            ctx.cells.promote(cell.id, 1, id, 'outgrew the tier');
            born = String(id);
            from = String(cell.id);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('promote', { makes: ['coalRaw'] });
    const spec = rigSpec('promote', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    const unitsReds: string[] = [];
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      for (const f of r.audit.families) for (const v of f.violations) if (v.family === 'units' && v.owner === String(SMALL_FIRM)) unitsReds.push(v.message);
    }
    expect(born).toBeDefined();
    expect(from).toBeDefined();
    if (born === undefined || from === undefined) return;
    // The number is declared and readable like any other, and its declaration is on the record.
    expect(w.params.decl(paramId(`test.hurdle.${born}`)).kind).toBe('preference');
    expect(w.journal.ofKind('param.declared').some((e) => e.subjects[0] === `test.hurdle.${born}`)).toBe(true);
    // The member left with its share of every lot: the firm holds what one member of the cell held.
    const firmHoldings = w.register.holdingsOf(born as never);
    expect(firmHoldings.length).toBeGreaterThan(0);
    // (The cell may have gained members since — founders enter it — so the event is the read.)
    const event = w.journal.ofKind('weight').find((e) => e.data['kind'] === 'promotion' && e.data['to'] === born);
    expect(event).toBeDefined();
    expect(event?.data['before']).toBe(weightBefore);
    expect(event?.data['after']).toBe(weightBefore - 1);
    expect(typeof event?.data['moved']).toBe('object');
    // XI-15, E5: the population fell by exactly the member that left, and the family is silent.
    expect(unitsReds).toEqual([]);
  });
});
