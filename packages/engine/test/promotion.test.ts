/**
 * The kernel's doors for a party born after the seed (Firm Birth A1, XI-14, XI-15, 12.4a.1).
 *
 * @spec Firm Birth A1 Firm Birth A3 Small-Business Pools A6.c Small-Business Pools E5 XI-14 XI-15 Law 4
 */
import { describe, expect, it } from 'vitest';
import { FIRM, SMALL_FIRM, assemble, equityLineOf, firmParam, labourScaleId, paramId, partyId, type MechanismContext, type SystemModule } from '../src/index.js';
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

describe('a small firm that outgrew its tier becomes a named firm to every module (A6.c, Firm Birth A1–A3, 12.4a.2)', () => {
  it('is borne one per member with its own declared numbers, gets its share line, and decides like a seeded one', () => {
    let cell: string | undefined;
    let members = 0;
    let owner: string | undefined;
    const probe: SystemModule = {
      id: 'test.outgrew',
      spec: 'Small-Business Pools A6.c',
      requires: ['small-business', 'firms', 'equity'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.outgrew',
          spec: 'Small-Business Pools A6.c',
          anchor: { before: 'firms.bear' },
          reads: [],
          writes: [{ kind: 'event', name: 'smallBusiness.promotion' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            // The trigger is a read of the world (the smallest named firm's worth); here the world
            // is told that this cell has reached it, so the rest of the mechanism can be measured.
            const c = [...ctx.parties.ofKind(SMALL_FIRM)]
              .filter((p) => p.representation === 'cell' && p.status.alive && p.weight >= 2)
              .sort((a, b) => (a.representation === 'cell' && b.representation === 'cell' ? a.weight - b.weight : 0))[0];
            if (c?.representation !== 'cell') return;
            const row = ctx.agreements.owedBy(c.id).find((a) => a.state === 'performing');
            if (row === undefined) return;
            cell = String(c.id);
            members = c.weight;
            owner = String(row.creditor);
            ctx.record('smallBusiness.promotion', [c.id, row.creditor], { cell, members, line: c.key['line'] ?? '', bank: c.key['bank'] ?? '', region: String(c.region), owner }, true);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('outgrew', { makes: ['coalRaw'] });
    const spec = rigSpec('outgrew', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    const unitsReds: string[] = [];
    for (let i = 0; i < 6; i += 1) {
      const r = w.step();
      for (const f of r.audit.families) for (const v of f.violations) if (v.family === 'units' && v.owner === String(SMALL_FIRM)) unitsReds.push(v.message);
    }
    expect(cell).toBeDefined();
    if (cell === undefined || owner === undefined) return;
    const born = w.journal.ofKind('firm.born').filter((e) => e.data['cell'] === cell);
    // A1: one named firm per member, each with a new name.
    expect(born).toHaveLength(members);
    for (const e of born) {
      const firm = String(e.data['firm']);
      const party = w.parties.get(firm as never);
      expect(party.kind).toBe(FIRM);
      expect(party.representation).toBe('named');
      // XI-14: its three numbers, declared the period it was born.
      expect(w.params.decl(labourScaleId(firm)).kind).toBe('technology');
      expect(w.params.decl(firmParam(firm, 'hurdle')).kind).toBe('preference');
      expect(w.params.decl(firmParam(firm, 'horizon')).kind).toBe('preference');
      // A2, A3: its residual has a line, held by whoever owned it as a small firm.
      const line = equityLineOf(firm);
      expect(w.instruments.has(line)).toBe(true);
      const issued = w.journal.ofKind('equity.born').find((x) => x.data['firm'] === firm);
      expect(issued?.data['outcome']).toBe('settled');
      expect(w.register.quantity(owner as never, line)).toBeGreaterThan(0);
      // B1: it is a firm to the firms module from its first decision.
      expect(w.journal.ofKind('firms.plan').some((x) => x.subjects[0] === firm)).toBe(true);
    }
    // The last member took the cell with it: it ceased into the party it became.
    expect(w.parties.get(cell as never).status.alive).toBe(false);
    // XI-15, E5: every departure is a promotion event with before and after; the family is silent.
    const events = w.journal.ofKind('weight').filter((e) => e.data['kind'] === 'promotion' && e.data['from'] === cell);
    expect(events).toHaveLength(members);
    expect(unitsReds).toEqual([]);
  });
});
