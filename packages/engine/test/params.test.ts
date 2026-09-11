/**
 * The parameter register's own guards: what a declared number must say about itself.
 *
 * @spec Law 2 XI-14 Appendix A
 */
import { describe, expect, it } from 'vitest';
import {
  ParamRegister,
  paramId,
  type ParamDecl,
} from '../src/index.js';
import { rigWorld } from './rig.js';

function decl(over: Partial<ParamDecl>): ParamDecl {
  return {
    id: paramId('test.number'),
    value: 1,
    unit: 'ratio',
    kind: 'preference',
    owner: 'model',
    why: 'a reason, because Law 16 asks for one',
    ...over,
  };
}

describe('a shape with a scheduled death is a placeholder (Law 2)', () => {
  it('refuses a shape whose reason names a worklist item', () => {
    // The death was in the prose and both field guards passed: `standsInFor` was absent, so the
    // placeholder guard had nothing to check, and the kind was not placeholder, so the other guard
    // had nothing either. XI-14 calls the count of placeholders the honest measure of how much
    // mechanism is missing, and a number that names the item which deletes it belongs in that count.
    expect(
      () =>
        new ParamRegister([
          decl({
            kind: 'shape',
            why: 'a claim about the answer until managers compete (worklist 13h).',
          }),
        ]),
    ).toThrow(/placeholder/);
  });

  it('lets a shape stand when nothing is scheduled to produce it', () => {
    // The opening levels are shapes forever: a market that has never traded has no price, and no
    // item will ever delete the number a world with stock in it has to open at (Seed C4).
    const r = new ParamRegister([
      decl({
        kind: 'shape',
        why: 'the level this world opens at; nothing produces it and nothing will.',
      }),
    ]);
    expect(r.report().counts.shape).toBe(1);
    expect(r.report().placeholders).toEqual([]);
  });

  it('does not fire on a policy or a preference that cites an item for something else', () => {
    // Eleven numbers name a future item in their reason and are not placeholders: a rate parliament
    // owns from 14 is still that rate at 14, and what changes is who sets it. A guard that read
    // "worklist" and stopped would have called every one of them a defect.
    const r = new ParamRegister([
      decl({
        id: paramId('test.retirementAge'),
        kind: 'policy',
        owner: 'parliament',
        why: 'Labour B3: parliament owns it from worklist 14; until then it stands where the cohorts were drawn.',
      }),
      decl({
        id: paramId('test.requiredYield'),
        kind: 'preference',
        why: 'what its investors require over a deposit, and a deposit returns nothing until worklist 11.',
      }),
    ]);
    expect(r.report().counts.policy).toBe(1);
    expect(r.report().counts.preference).toBe(1);
  });

  it('still refuses a placeholder that names no mechanism, and a death on any other kind', () => {
    expect(() => new ParamRegister([decl({ kind: 'placeholder' })])).toThrow(/stands in for/);
    expect(
      () =>
        new ParamRegister([
          decl({ kind: 'technology', standsInFor: { mechanism: 'Goods A2', worklistItem: '15' } }),
        ]),
    ).toThrow(/scheduled death/);
  });
});

describe('what the foundation world declares (XI-14)', () => {
  it('names every placeholder and the item that kills it (Law 2)', () => {
    const report = rigWorld('params').params.report();
    // Law 2: A PLACEHOLDER NAMES THE MECHANISM IT STANDS IN FOR AND THE ITEM THAT DELETES IT, and
    // that is what is asserted — not a count of them and not a list. This named the two management
    // fees of a world with one region in it (`fund.fee.fund.money.north`, `fund.fee.etf.north`),
    // and this world has four countries, a money fund per bank and two more placeholders since:
    // a list that every item has to edit is a stale doc with ids in it (Law 16).
    expect(report.placeholders.length).toBeGreaterThan(0);
    for (const p of report.placeholders) {
      expect(p.mechanism.length, `${p.id} stands in for nothing`).toBeGreaterThan(0);
      expect(p.worklistItem, `${p.id} names no item that kills it`).not.toBe('');
    }
    // And the count is a READ of the register, never a number anybody wrote beside it (Law 19).
    expect(report.counts.placeholder).toBe(report.placeholders.length);
    expect(report.counts.shape).toBe(report.shapes.length);
  });
});
