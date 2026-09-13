/**
 * A region's accounts with the rest of the world (13i): a read that sums to zero.
 *
 * @spec Cross-Border E1 Cross-Border E2 Cross-Border E3 Cross-Border E4 Money D3 Law 2 Law 5 Law 19
 */
import { describe, expect, it } from 'vitest';
import { external, financedBy, tradeBalanceOf } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('the accounts are a WALK, not a series anybody imported (Law 2, Law 19)', () => {
  it('declares no number at all: there is no trade balance and no capital flow to state', () => {
    // Appendix B forbids an exogenous trade or capital-flow series by name. There is nothing to
    // declare here because both halves are read off instructions that already happened.
    expect(external().params).toEqual([]);
    const w = rigWorld('ext-t');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('trade.balance');
      expect(id).not.toContain('capitalflow');
      expect(id).not.toContain('current.account');
    }
  });

  it('says how many legs it walked, so a reader can see it is a walk and not a total', () => {
    const w = rigWorld('ext-t');
    for (let i = 0; i < 6; i += 1) w.step();
    const said = w.journal.ofKind('external.accounts');
    expect(said.length).toBeGreaterThan(0);
    for (const e of said) expect(Number(e.data['legs'])).toBeGreaterThan(0);
  });
});

describe('they sum to zero, because every transaction had two sides (E3, Law 5)', () => {
  it('nets to nothing for every region, every period', () => {
    const w = rigWorld('ext-t');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const e of w.journal.ofKind('external.accounts')) {
      // Not an identity anybody enforces and not a residual anybody plugs: if the two halves did
      // not cancel, a leg went out with nothing coming back, which is the one-sided flow Law 5
      // forbids. That is the only thing this can find.
      expect(Number(e.data['net'])).toBe(0);
    }
  });

  it('never fires its own family over a run, and never repairs anything when it would', () => {
    const w = rigWorld('ext-t');
    for (let i = 0; i < 6; i += 1) {
      const report = w.step();
      const flows = report.audit.families.find((f) => f.family === 'flows');
      for (const v of flows?.violations ?? []) {
        expect(v.spec).not.toContain('Cross-Border E3');
      }
    }
  });

  it('puts what one region gave up on the other region’s side, with the opposite sign', () => {
    const w = rigWorld('ext-t');
    for (let i = 0; i < 6; i += 1) w.step();
    const byPeriod = new Map<number, number[]>();
    for (const e of w.journal.ofKind('external.accounts')) {
      const list = byPeriod.get(e.period) ?? [];
      list.push(Number(e.data['trade']));
      byPeriod.set(e.period, list);
    }
    for (const [, trades] of byPeriod) {
      // E3 again, from the world's end: what crossed left one place and arrived in another, so the
      // regions' trade balances cancel too. A world whose regions all ran a surplus would be
      // trading with somewhere that is not in it.
      const total = trades.reduce((a, b) => a + b, 0);
      expect(Math.abs(total)).toBeLessThanOrEqual(trades.length * Number.EPSILON * 
        trades.reduce((a, b) => a + Math.abs(b), 0));
    }
  });
});

describe('a deficit is financed by somebody who chose to (E2)', () => {
  it('is the other half of the trade balance, read from the region’s own end', () => {
    const said = { region: 'x' as never, trade: -100, finance: 100, legs: 4 };
    expect(tradeBalanceOf(said)).toBe(-100);
    // What somebody had to lend it to pay for what it bought — and a surplus is the other way.
    expect(financedBy(said)).toBe(100);
    expect(financedBy({ ...said, trade: 100 })).toBe(-100);
  });
});
