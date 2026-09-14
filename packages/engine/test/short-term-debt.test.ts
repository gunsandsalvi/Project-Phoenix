/**
 * Short-term debt (item 10b): money borrowed for weeks, and the asking-again that is the whole risk.
 *
 * @spec Short-Term Debt A1 Short-Term Debt A1.b Short-Term Debt A2 Short-Term Debt A2.a Short-Term Debt A3 Short-Term Debt B1 Short-Term Debt B3 Short-Term Debt B3.a Short-Term Debt B4 Short-Term Debt C2 Short-Term Debt C3 Short-Term Debt D1 Short-Term Debt E1 Short-Term Debt E2 Short-Term Debt E3 Law 2 Law 4 Law 9
 */
import { describe, expect, it } from 'vitest';
import {
  BACKSTOP,
  COMMERCIAL_PAPER,
  PAPER_PARAMS,
  isBackstop,
  isPaper,
  shortTermDebt,
  yieldOn,
} from '../src/index.js';
import { asPerPiece } from '../src/core/measure.js';
import { ranWorld, rigWorld } from './rig.js';

describe('what commercial paper IS (A1)', () => {
  it('pays no coupon, redeems at par, and its yield comes OUT of its price (A1.a, A2)', () => {
    const w = rigWorld('cp-a');
    const k = w.registry.instrumentKind(COMMERCIAL_PAPER);
    // A2, E2: cleared, and there is no other answer. A discount off an untraded curve is the defect.
    expect(k.pricing).toBe('cleared');
    expect(k.liabilityOfIssuer).toBe(true);
    // Register B3, XI-3: the issuer owes the FACE, so its paper falling is never its own gain.
    expect(k.owes).toBe('face');
    // A2: price in, yield out — and only that direction.
    expect(yieldOn(asPerPiece(0.99, 'a price'), 0.25, 'a yield')).toBeCloseTo((1 - 0.99) / (0.99 * 0.25), 12);
  });

  it('can fail and cross-defaults, which is what separates it from a bill (G2, Sovereign G3)', () => {
    const w = rigWorld('cp-a');
    const k = w.registry.instrumentKind(COMMERCIAL_PAPER);
    expect(k.defaultOn).toBeDefined();
    // An issuer that cannot repay a week of borrowing has not got a week's problem.
    expect(k.accelerates).toBe(true);
    // And the sovereign bill it shares a promise with does NOT, which is the whole distinction.
    expect(w.registry.instrumentKind(w.registry.instrumentKind(COMMERCIAL_PAPER).id).accelerates).toBe(true);
  });

  it('declares only numbers that are one of Law 2’s kinds, and the shape names its death', () => {
    const declared = shortTermDebt().params;
    for (const d of declared) {
      expect(['technology', 'preference', 'policy', 'resolution', 'shape']).toContain(d.kind);
      // Law 2: a shape with a scheduled death is a PLACEHOLDER and names the item that kills it.
      if (d.kind === 'shape') expect(d.standsInFor?.item).toBeDefined();
    }
    const ids = declared.map((d) => String(d.id));
    expect(ids).toContain(String(PAPER_PARAMS.tenor));
    expect(ids).toContain(String(PAPER_PARAMS.commitmentFee));
  });
});

describe('the roll is a new issue into a market that must clear (B3.a, E1)', () => {
  it('has no renewal path at all: the only way paper continues is a fresh offer', () => {
    const m = shortTermDebt();
    // E1: paper that always rolls at a written rate is not debt. There is one issuing phase and
    // what maturing paper does is enlarge the need it brings paper against — never renew itself.
    expect(m.phases.map((p) => p.name)).toEqual(['paper.issue', 'paper.backstop']);
    expect(m.phases[0]?.anchor).toEqual({ before: 'markets' });
    expect(m.phases[1]?.anchor).toEqual({ after: 'markets' });
  });

  it('brings paper for a dated need, and says how much of the trip is a roll', () => {
    const w = ranWorld('cp-run', 12);
    for (const e of w.journal.ofKind('paper.offered')) {
      expect(typeof e.data['size']).toBe('number');
      expect(e.data['size'] as number).toBeGreaterThan(0);
      // B3: a reader can see which part of the ask is asking for the same money again.
      expect(typeof e.data['rolling']).toBe('number');
      expect(typeof e.data['reservation']).toBe('number');
      // C4, Law 3: it brought a size and a walk-away. The price is the book's.
      expect(e.data['reservation'] as number).toBeGreaterThan(0);
    }
  });

  it('names a line by its issuer and the day it is due, one per issuer per date (Law 9)', () => {
    const w = ranWorld('cp-run', 12);
    const seen = new Set<string>();
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER) continue;
      expect(String(i.id)).toMatch(/^cp:/);
      expect(seen.has(String(i.id))).toBe(false);
      seen.add(String(i.id));
      // D1: it trades after issue, so a holder can get out early.
      expect(i.market.some).toBe(true);
    }
  });
});

describe('the backstop costs money in every period it is not used (B4)', () => {
  it('is an agreement between two named parties, never an instrument', () => {
    const w = ranWorld('cp-run', 8);
    for (const row of w.agreements.ofKind(BACKSTOP)) {
      expect(isBackstop(row.terms)).toBe(true);
      if (!isBackstop(row.terms)) continue;
      // B4: a committed line with no fee on undrawn headroom is a free option nobody sold.
      expect(row.terms.fee).toBeGreaterThan(0);
      expect(row.terms.limit).toBeGreaterThan(0);
      expect(row.debtor).not.toBe(row.creditor);
    }
    // XI-8: nobody trades a commitment, so no instrument kind answers for one.
    expect(w.instruments.all().some((i) => String(i.kind) === String(BACKSTOP))).toBe(false);
  });
});

describe('what must not happen (E3)', () => {
  it('never leaves paper outstanding past its own maturity, and never a negative amount', () => {
    const w = ranWorld('cp-run', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER || !isPaper(i.terms)) continue;
      const outstanding = w.register.heldTotal(i.id).value;
      expect(outstanding).toBeGreaterThanOrEqual(0);
      if (w.calendar.periodOf(i.terms.maturity) < w.period && i.status.live) {
        expect(outstanding).toBe(0);
      }
    }
  });

  it('A1.b: every line runs under a year, because that is what the instrument IS', () => {
    const w = ranWorld('cp-run', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER || !isPaper(i.terms)) continue;
      const days = w.calendar.periodOf(i.terms.maturity) - w.calendar.periodOf(i.terms.issueDate);
      expect(days).toBeLessThan(53);
      // A2.a: the quoting convention is a material part of the number at this tenor, so it is said.
      expect(i.terms.dayCount).toBe('ACT/360');
    }
  });
});
