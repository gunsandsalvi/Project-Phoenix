/**
 * What an issuer owes when the price moves (13f).
 *
 * @spec Register B3 XI-3 XI-9 Law 4 Law 19 Fund Shares A3
 *
 * A BORROWER OWES WHAT IT AGREED TO PAY. When the market marks its paper down because it is walking
 * towards default, the holder has lost something real and the issuer has gained nothing: it still
 * has to find the whole amount on the day. Debt is not carried at market value on an issuer's
 * balance sheet, and a world that carried it there would have a firm growing MORE solvent the less
 * anybody trusted it — which puts the solvency trigger out of reach exactly when it should fire.
 */
import { describe, expect, it } from 'vitest';
import { assemble } from '../src/index.js';
import { rigSpec, rigWorld } from './rig.js';

describe('every kind says which situation its issuer is in (Register B3)', () => {
  it('declares `owes` on every instrument kind, with no default anywhere', () => {
    const w = rigWorld('own-a');
    for (const k of w.registry.instrumentKinds.values()) {
      expect(['face', 'value']).toContain(k.owes);
    }
  });

  it('lets exactly the residual claims owe their value, and everything else its face', () => {
    const w = rigWorld('own-a');
    const kinds = [...w.registry.instrumentKinds.values()];
    const byValue = kinds.filter((k) => k.owes === 'value');
    // A fund share is a claim ON a book: what the issuer owes IS what the pool is worth, and its
    // move is what keeps a fund's own equity at zero, where a fund's equity belongs (A3).
    for (const k of byValue) expect(k.liabilityOfIssuer).toBe(true);
    // And it is the exception rather than the rule: a world where several kinds owed their value
    // would be a world where several issuers gained from their own paper falling.
    expect(byValue.length).toBeLessThan(kinds.filter((k) => k.liabilityOfIssuer).length);
  });

  it('refuses at assembly a kind that owes its value and is nobody’s liability', () => {
    const spec = rigSpec('own-b');
    const broken = spec.modules.map((m) => ({
      ...m,
      instrumentKinds: m.instrumentKinds.map((k) =>
        k.liabilityOfIssuer ? k : { ...k, owes: 'value' as const },
      ),
    }));
    // A rule that can be a check should be one: saying an issuer owes what a thing is worth, when
    // the thing is nobody's promise, is saying something about a party that is not there.
    expect(() => assemble({ ...spec, modules: broken })).toThrow(/Register B3/);
  });
});

describe('an issuer books nothing when its own paper re-marks (XI-3, XI-9)', () => {
  it('never records a revaluation of own liabilities for a face-owed line', () => {
    const w = rigWorld('own-a');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const e of w.journal.ofKind('revaluation')) {
      if (e.data['liabilities'] !== true) continue;
      // The only issuer whose own liabilities move is one whose claim IS its book. A treasury, a
      // bank or a firm appearing here would be booking a profit on its own deterioration.
      const who = e.subjects[0] ?? "";
      expect(who.startsWith('fund.')).toBe(true);
    }
  });

  it('keeps assets minus liabilities equal to the equity account, which is the whole test', () => {
    // Audit B5: if a liability is carried at face on one side of the identity and at the market on
    // the other, the difference is exactly the fiction — so this is what catches it coming back.
    const w = rigWorld('own-a');
    for (let i = 0; i < 6; i += 1) {
      const report = w.step();
      const accounts = report.audit.families.find((f) => f.family === 'accounts');
      expect(accounts?.violations ?? []).toEqual([]);
    }
  });
});
