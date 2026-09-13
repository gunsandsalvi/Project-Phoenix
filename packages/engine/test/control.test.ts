/**
 * The market for control (13g): what somebody will pay for a whole firm, and what its owners take.
 *
 * @spec M&A A1 M&A A3 M&A B1 M&A B2.a M&A C2 M&A E2 Equity A1 Equity B1 Law 2 Law 3 Law 11
 */
import { describe, expect, it } from 'vitest';
import { control, premiumOver } from '../src/index.js';
import { ranWorld, rigWorld } from './rig.js';

describe('a premium is a distance between two numbers (B2.a, Law 3)', () => {
  it('is what the book produced over what a share was trading at, and never a percentage', () => {
    expect(premiumOver(120, 100)).toBe(20);
    // A bid that clears BELOW the last print is a real outcome — a firm whose holders wanted out
    // more than the buyer wanted in — and nothing bounds it away (Law 6).
    expect(premiumOver(90, 100)).toBeLessThan(0);
  });

  it('declares no number at all: there is no control premium and no synergy term (Law 2)', () => {
    const m = control();
    expect(m.params).toEqual([]);
    // And nothing anywhere in this world declares what a takeover is worth. (A liquidity premium
    // and an insurance premium are different words for a different thing and are somebody else's.)
    const w = rigWorld('ctrl-p');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('control.');
      expect(id).not.toContain('synergy');
      expect(id).not.toContain('takeover');
      expect(id).not.toContain('acquisition');
    }
  });
});

describe('what a bidder looks for is the RESIDUAL claim (Equity A1, Law 15)', () => {
  it('reads it off the kind profile rather than branching on a kind id', () => {
    // Buying every liability of a firm buys you nothing; buying the residual buys you the firm. So
    // the test is "a claim ON somebody that its issuer does not owe", which the register already
    // says — and a module that named the share kind would be a module importing another module.
    const w = rigWorld('ctrl-p');
    const residual = w.instruments
      .all()
      .filter((i) => i.issuer.some && !w.registry.instrumentKind(i.kind).liabilityOfIssuer);
    for (const i of residual) expect(w.registry.instrumentKind(i.kind).owes).toBe('face');
    expect(residual.length).toBeGreaterThan(0);
  });
});

describe('what this world actually does with it (Law 11)', () => {
  it('bids for nothing, because every listed firm here publishes a LOSS — and says so', () => {
    /**
     * A FINDING, written as a test so it cannot be lost. An acquirer values a target the way it
     * values a machine: what it would get, against what it requires. What it would get is what the
     * target PUBLISHED, and in this world every listed firm publishes negative earnings — so there
     * is no stream to capitalise and no bid to make, which is the correct answer to the question
     * rather than a mechanism that does not work.
     *
     * The day a listed firm here earns something, this assertion fails and the tender machinery
     * starts running. Nobody should have to notice that by accident.
     */
    const w = ranWorld('ctrl-c', 30, 4, 40);
    const reports = w.journal.ofKind('reporting.report');
    expect(reports.length).toBeGreaterThan(0);
    for (const e of reports) expect(Number(e.data['earned'])).toBeLessThan(0);
    expect(w.journal.ofKind('control.acquired')).toHaveLength(0);
  });

  it('holds its own family over a run: nobody acquired is still trading (E2)', () => {
    const w = rigWorld('ctrl-p');
    for (let i = 0; i < 6; i += 1) {
      const report = w.step();
      const names = report.audit.families.find((f) => f.family === 'names');
      for (const v of names?.violations ?? []) expect(v.spec).not.toContain('M&A');
    }
  });
});
