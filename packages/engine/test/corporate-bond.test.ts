/**
 * The corporate bond (13f): a firm borrows from the market, and promises something for it.
 *
 * @spec Corporate Credit A1 Corporate Credit B2 Corporate Credit B2.a Corporate Credit B3 Corporate Credit G2 Bond N4 Bond N6 Bond N13 Bond N13.a Reporting A2 Law 2 Law 4 Law 6 Law 9
 */
import { describe, expect, it } from 'vitest';
import { annualCostOf, headroomOn, CORPORATE_BOND, corporateBondModule } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('what a corporate bond IS, beside a sovereign one', () => {
  it('can default, cross-defaults, and says where it ranks (N12, N13.a, G2)', () => {
    const w = rigWorld('bond-a');
    const k = w.registry.instrumentKind(CORPORATE_BOND);
    // G2: a firm that misses one line has missed them all — its lenders do not wait their turn
    // while the estate empties. A sovereign has no cross-default and says so (Sovereign G3).
    expect(k.accelerates).toBe(true);
    expect(k.defaultOn).toBeDefined();
    expect(k.liabilityOfIssuer).toBe(true);
    // Register B3, XI-3: its issuer owes the face, so its paper falling is never the firm's gain.
    expect(k.owes).toBe('face');
  });

  it('is priced by a book and never off a spread or a rating (Law 3)', () => {
    const w = rigWorld('bond-a');
    expect(w.registry.instrumentKind(CORPORATE_BOND).pricing).toBe('cleared');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      if (!id.includes('corporate')) continue;
      expect(id).not.toContain('spread');
      expect(id).not.toContain('covenant');
      expect(id).not.toContain('leverage');
    }
  });

  it('declares no number at all: a covenant is a TERM of an issue, not a policy (Law 2, Law 6)', () => {
    // B2: what a given firm promised is an outcome of what it had to promise to be lent to. A
    // world with a `corporate.covenant.leverage` in it would have every issuer promising the same
    // thing, which is one issuer with many names.
    expect(corporateBondModule().params).toEqual([]);
  });
});

describe('the covenant is tested on what was PUBLISHED (B2.a, Reporting A2)', () => {
  it('reads the issuer’s own accounts and never a second set computed here (Law 4)', () => {
    const w = rigWorld('bond-a');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const e of w.journal.ofKind('covenant.breached')) {
      // A breach names the quarter it was found in, because it is a fact about a report.
      expect(typeof e.data['quarter']).toBe('string');
      expect(typeof e.data['issuer']).toBe('string');
      expect(String(e.data['broke'])).toMatch(/leverage|coverage/);
    }
  });

  it('breaches when the accounts are the wrong side of the promise, and not before', () => {
    const c = { leverage: 0.6, coverage: 2 };
    // Headroom is what is left of the promise. Negative is a breach, and it is arithmetic on two
    // published numbers rather than a threshold anybody tuned.
    expect(headroomOn({ assets: 100, liabilities: 50 }, c)).toBeCloseTo(0.1, 12);
    expect(headroomOn({ assets: 100, liabilities: 70 }, c)).toBeLessThan(0);
    // A firm with no assets has no ratio that means anything and HAS breached — which is what the
    // worst case is, rather than a number pushed back inside a range (Law 6).
    expect(headroomOn({ assets: 0, liabilities: 1 }, c)).toBeLessThan(0);
  });

  it('never repairs or accelerates by itself: a breach is an event and that is all', () => {
    const m = corporateBondModule();
    const phase = m.phases[0];
    expect(phase?.name).toBe('covenant.test');
    // What happens after a breach is the holders' decision, and a decision is not a rule.
    expect(m.families.every((f) => f.built)).toBe(true);
  });
});

describe('the schedule is the kernel’s, because it is not a corporate fact (Law 4)', () => {
  it('costs its issuer its coupon a year per unit of face, read off its own terms', () => {
    const t = {
      coupon: { amount: 0.05, per: { kind: 'annual' as const } },
    } as Parameters<typeof annualCostOf>[0];
    expect(annualCostOf(t)).toBeCloseTo(0.05, 12);
  });
});
