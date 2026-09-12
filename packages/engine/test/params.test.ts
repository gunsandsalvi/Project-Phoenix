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
    dimension: 'ratio',
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

describe('a declared unit is checked, not commented (Law 8, XI-14)', () => {
  it('refuses a read that names a different dimension from the declaration', () => {
    // `unit` was a free string nothing read back: the constructor checked it was non-empty and
    // `get(id)` handed out a bare number. So the one place in the engine where every
    // behaviour-shaping number declares its unit was the one place the unit could not be checked,
    // and a number declared in 'periods' multiplied by a day count was wrong at no site.
    const id = paramId('test.number');
    const r = new ParamRegister([decl({ dimension: 'periods', unit: 'periods', value: 52 })]);
    expect(r.periods(id)).toBe(52);
    for (const wrong of ['ratio', 'perAnnum', 'days', 'years', 'count', 'price'] as const) {
      // The message carries BOTH dimensions, because what a reader has to fix is the mismatch and
      // not either half of it.
      expect(() => r[wrong](id)).toThrow(/declared in periods .* read as /);
    }
  });

  it('keeps the four durations apart, which is the defect it exists to catch', () => {
    // Money G3.a: periods are this world's calendar and days, months and years are the civil one.
    // They are the same quantity in four units and mixing them is the whole point of the check.
    const id = paramId('test.number');
    const days = new ParamRegister([decl({ dimension: 'days', unit: 'days', value: 7 })]);
    expect(days.days(id)).toBe(7);
    expect(() => days.periods(id)).toThrow(/declared in days/);
    const years = new ParamRegister([decl({ dimension: 'years', unit: 'years', value: 3 })]);
    expect(() => years.months(id)).toThrow(/declared in years/);
  });

  it('still refuses a declared AMOUNT to every one of them: it is read with amount()', () => {
    const id = paramId('test.number');
    const r = new ParamRegister([
      decl({ dimension: 'amount', denominated: true, unit: 'of its own money', value: 10 }),
    ]);
    expect(() => r.count(id)).toThrow(/read it with amount/);
  });
});
