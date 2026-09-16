/**
 * Monetary policy: a rate somebody sets, and the door they set it through (18a.1).
 *
 * @spec Central Bank B1 Central Bank B1.a Central Bank B2 XI-14 Expectations A2 Law 3 Law 6
 */
import { describe, expect, it } from 'vitest';
import {
  assemble,
  paramId,
  type MechanismContext,
  type SystemModule,
} from '../src/index.js';
import { POLICY_PARAMS } from '../src/mechanisms/money-market/policy.js';
import { policyRateOf } from '../src/mechanisms/money-market/data.js';
import { mergeModules, rigSpec, rigFor } from './rig.js';

function worldWith(probe: SystemModule) {
  const { banks, firms } = rigFor('policy', { makes: ['coalRaw'] });
  const spec = rigSpec('policy', banks, firms);
  return assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
}

describe('a policy number is set by whoever owns it (XI-14, 18a.1)', () => {
  it('moves the rate, records it publicly, and refuses everything that is not its owner’s policy', () => {
    const refused: string[] = [];
    const probe: SystemModule = {
      id: 'test.policy',
      spec: 'Central Bank B1',
      requires: ['money-market'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.policy',
          spec: 'Central Bank B1',
          anchor: { after: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            ctx.setByMandate(policyRateOf('USD'), 0.03, 'centralBank', 'a test moved it.');
            // A TECHNOLOGY is a fact about the world and does not move because somebody wants it to.
            try {
              ctx.setByMandate(paramId('index.base'), 1, 'centralBank', 'it should not.');
            } catch (e) {
              refused.push(`kind:${e instanceof Error ? 'threw' : 'no'}`);
            }
            // And nobody sets another mandate's number.
            try {
              ctx.setByMandate(POLICY_PARAMS.target('USD'), 0.05, 'parliament', 'not yet, §47.');
            } catch (e) {
              refused.push(`owner:${e instanceof Error ? 'threw' : 'no'}`);
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const w = worldWith(probe);
    for (let i = 0; i < 4; i += 1) w.step();
    // It moved, and it stayed moved: the register holds one value and it is the new one (Law 4).
    expect(w.params.perAnnum(policyRateOf('USD'))).toBeCloseTo(0.03, 12);
    // Observer A1: a policy decision is public, with what it was and what it is.
    const said = w.journal.ofKind('param.set').filter((e) => e.data['id'] === String(policyRateOf('USD')));
    expect(said.length).toBe(1);
    expect(said[0]?.public).toBe(true);
    expect(said[0]?.data['by']).toBe('centralBank');
    expect(Number(said[0]?.data['now'])).toBeCloseTo(0.03, 12);
    expect(Number(said[0]?.data['was'])).not.toBe(0.03);
    // Both refusals fired, at the site, with a citation (they are contract violations, not findings).
    expect(refused).toEqual(['kind:threw', 'owner:threw']);
  });

  it('gives every central bank a target, a step and a calendar of its own, and none of them is a gain', () => {
    const w = worldWith({
      id: 'test.none',
      spec: 'Central Bank B1',
      requires: [],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [],
      participants: [],
      families: [],
    });
    // B1.a: the mandate is a POLICY with an owner, and it is the rate of change of a basket.
    expect(w.params.decl(POLICY_PARAMS.target('USD')).kind).toBe('policy');
    expect(w.params.decl(POLICY_PARAMS.target('USD')).owner).toBe('centralBank');
    // B1: a STEP, which is a grain and not a coefficient — nothing multiplies a gap by anything.
    expect(w.params.perAnnum(POLICY_PARAMS.step('USD'))).toBeGreaterThan(0);
    expect(w.params.decl(POLICY_PARAMS.step('USD')).why).toContain('NOT a gain');
    // Money G3.a: it meets on a DATE, in months, not every nth period.
    expect(w.params.months(POLICY_PARAMS.every('USD'))).toBeGreaterThan(0);
    expect(w.params.decl(POLICY_PARAMS.every('USD')).dimension).toBe('months');
  });
});
