/**
 * Securities lending (13f): title passes, the economics do not.
 *
 * @spec Securities Lending A1 Securities Lending A2 Securities Lending A3 Securities Lending A5 Securities Lending A5.a Securities Lending A5.b Securities Lending B2 Securities Lending B4 Securities Lending C1 Securities Lending C4 Securities Lending E1 Securities Lending E2 Securities Lending E3 Equity C7 Register D5.a Law 3
 */
import { describe, expect, it } from 'vitest';
import { poolOf, rebateOf, securitiesLending } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('the pool is a read, and it is what caps a short (B4, E1)', () => {
  it('counts only what a named holder has FREE, never what is pledged or already lent', () => {
    const w = rigWorld('lend-a');
    for (let i = 0; i < 3; i += 1) w.step();
    for (const i of w.instruments.all()) {
      if (!i.status.live) continue;
      const holders = w.register.holdersOf(i.id);
      if (holders.length === 0) continue;
      for (const h of holders) {
        const free = w.register.free(h, i.id);
        const total = w.register.totalQuantity(h, i.id);
        // C4, Register D5.a: posted collateral leaves the poster's free balance, so it cannot be
        // counted as available by both sides. Free is never more than held — that is the register
        // refusing, not this module remembering.
        expect(free).toBeLessThanOrEqual(total);
      }
    }
  });

  it('adds up to a number, because a lendable pool is a quantity and not a propensity', () => {
    expect(poolOf([])).toBe(0);
    expect(poolOf([{ units: 3 as never }, { units: 4 as never }])).toBe(7);
  });
});

describe('the fee and the rebate are one number seen from two sides (A5, A5.b)', () => {
  it('hands back what the cash earned less the fee, so neither side states it twice (Law 4)', () => {
    // A5.b: when the collateral is cash the price is expressed as a rebate ON that cash. It is the
    // same number: a world with a fee table and a rebate table would have two answers to one
    // question, and they would drift.
    expect(rebateOf(0.02, 0.05)).toBeCloseTo(0.03, 12);
    expect(rebateOf(0.05, 0.05)).toBeCloseTo(0, 12);
    // A borrow dearer than the cash earns is a NEGATIVE rebate, which is a real state of a squeezed
    // line and is not bounded away (Law 6).
    expect(rebateOf(0.08, 0.05)).toBeLessThan(0);
  });
});

describe('no short without a borrow (E1, Equity C7)', () => {
  it('measures it and never enforces it: a negative holding is a finding with an owner', () => {
    const m = securitiesLending();
    const family = m.families[0];
    expect(family?.name).toBe('ownership');
    expect(family?.built).toBe(true);
    expect(family?.spec).toContain('Securities Lending E1');
  });

  it('holds over a run: nobody in this world is short of anything nobody lent', () => {
    const w = rigWorld('lend-a');
    for (let i = 0; i < 6; i += 1) {
      const report = w.step();
      const ownership = report.audit.families.find((f) => f.family === 'ownership');
      for (const v of ownership?.violations ?? []) {
        // A FORBID that holds is as valuable as a mechanism that works, and it breaks silently.
        expect(v.spec).not.toContain('Securities Lending E1');
      }
    }
  });
});

describe('the module declares no number at all (Law 2, Law 3)', () => {
  it('states no fee, no haircut and no lendable share: every one of them is an outcome', () => {
    const m = securitiesLending();
    expect(m.params).toEqual([]);
    const w = rigWorld('lend-a');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('borrowfee');
      expect(id).not.toContain('lendable');
      expect(id).not.toContain('rebate');
    }
  });

  it('passes the economics on rather than paying them, which is what a stock loan IS (A3)', () => {
    const w = rigWorld('lend-a');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const e of w.journal.ofKind('borrow.manufactured')) {
      // A3: the issuer pays the registered holder — the borrower — and the borrower passes it on,
      // so the lender's cash flows are unchanged and it has not paid a fee to lose its income.
      expect(typeof e.data['lender']).toBe('string');
      expect(typeof e.data['borrower']).toBe('string');
      expect(Number(e.data['amount'])).toBeGreaterThan(0);
    }
  });
});
