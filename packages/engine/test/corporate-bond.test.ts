/**
 * The corporate bond (13f): a firm borrows from the market, and promises something for it.
 *
 * @spec Corporate Credit A1 Corporate Credit B2 Corporate Credit B2.a Corporate Credit B3 Corporate Credit G2 Bond N4 Bond N6 Bond N13 Bond N13.a Reporting A2 Law 2 Law 4 Law 6 Law 9
 */
import { USD } from '../src/seeds/foundation.js';
import { asCash, asRatio } from '../src/core/measure.js';
import { describe, expect, it } from 'vitest';
import {
  annualCostOf,
  headroomOn,
  instrumentId,
  isCorporateBond,
  period,
  CORPORATE_BOND,
  corporateBondModule,
  type Event,
  type World,
} from '../src/index.js';
import { ranWorld, rigWorld } from './rig.js';

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

  it('declares one number, and it is a market convention (Law 2, Law 6)', () => {
    // B2: what a given firm promised is an outcome of what it had to promise to be lent to. A
    // world with a `corporate.covenant.leverage` in it would have every issuer promising the same
    // thing, which is one issuer with many names. So: no covenant number, no spread, no leverage
    // line, and the one number there IS is how long a firm's paper runs for — a TECHNOLOGY of the
    // market, declared in the unit the calendar takes (Law 8).
    const declared = corporateBondModule().params;
    expect(declared.map((d) => String(d.id))).toEqual(['corporateBond.tenor']);
    expect(declared[0]?.kind).toBe('technology');
    expect(declared[0]?.dimension).toBe('months');
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
    const c = { leverage: asRatio(0.6, 'what it promised'), coverage: asRatio(2, 'what it promised') };
    // Headroom is what is left of the promise. Negative is a breach, and it is arithmetic on two
    // published numbers rather than a threshold anybody tuned.
    expect(headroomOn({ assets: asCash(100, USD, 'what it holds'), liabilities: asCash(50, USD, 'what it owes') }, c)).toBeCloseTo(0.1, 12);
    expect(headroomOn({ assets: asCash(100, USD, 'what it holds'), liabilities: asCash(70, USD, 'what it owes') }, c)).toBeLessThan(0);
    // A firm with no assets has no ratio that means anything and HAS breached — which is what the
    // worst case is, rather than a number pushed back inside a range (Law 6).
    expect(headroomOn({ assets: asCash(0, USD, 'what it holds'), liabilities: asCash(1, USD, 'what it owes') }, c)).toBeLessThan(0);
  });

  it('never repairs or accelerates by itself: a breach is an event and that is all', () => {
    const m = corporateBondModule();
    expect(m.phases.map((f) => f.name)).toEqual(['bond.issue', 'covenant.test']);
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

describe('a firm issues because a market was cheaper than its bank (A1, B1, E5.d)', () => {
  /** Every issue this world brought, oldest first. A read of what was announced (Observer A3). */
  const offers = (w: World): readonly Event[] => w.journal.ofKind('bond.offered');

  it('brings paper at all, which is the whole of what item 10 was for', () => {
    const w = ranWorld('bond-issue', 10);
    // B1: it needed the banks to have quoted and the holders to have published what they require,
    // and both are published in `lending.write` — so nothing can come before the first of those.
    expect(offers(w).length).toBeGreaterThan(0);
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND) continue;
      // Law 9: named as a market names it — the issuer and the maturity, never an internal id.
      expect(String(i.id)).toMatch(/^bond:firm\.\d+:/);
      expect(i.market.some).toBe(true);
    }
  });

  it('says which of B1’s two reasons brought it, with both prices on the record', () => {
    const w = ranWorld('bond-issue', 10);
    for (const e of offers(w)) {
      const quoted = e.data['quoted'];
      const required = e.data['requiredByHolders'];
      const lendable = e.data['lendable'];
      expect(typeof required).toBe('number');
      // Either the market was cheaper than the bank, or the bank would not lend it enough, or
      // nobody quoted it at all. A third reason would be a reason nothing published.
      if (typeof quoted === 'number' && typeof lendable === 'number') {
        const short = e.data['short'];
        expect(
          (required as number) < quoted || (typeof short === 'number' && lendable < short),
        ).toBe(true);
      } else {
        expect(quoted).toBeNull();
      }
    }
  });

  it('never prices its own issue: the reservation is its alternative, and the price is the book’s', () => {
    const w = ranWorld('bond-issue', 10);
    for (const e of offers(w)) {
      const line = String(e.data['line']);
      const reservation = e.data['reservation'];
      expect(typeof reservation).toBe('number');
      expect(reservation as number).toBeGreaterThan(0);
      // C4, Law 3: what it brought is a size and a walk-away. Whatever it got for the paper is a
      // PRINT, and a print is the auction's, so nothing here may equal the reservation by
      // construction — it may only be at or above it, because below it the paper is withdrawn.
      for (const p of w.prices.history(instrumentId(line))) {
        expect(p.price).toBeGreaterThanOrEqual(reservation as number);
      }
    }
  });

  it('taps the line it has rather than minting a second one at the same date (C8, Law 4)', () => {
    const w = ranWorld('bond-issue', 12);
    const seen = new Set<string>();
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND) continue;
      expect(seen.has(String(i.id))).toBe(false);
      seen.add(String(i.id));
    }
    // And a second offer on a line that already exists is a TAP: same instrument, more face.
    const byLine = new Map<string, number>();
    for (const e of offers(w)) {
      const line = String(e.data['line']);
      byLine.set(line, (byLine.get(line) ?? 0) + 1);
    }
    for (const [line] of byLine) expect(seen.has(line)).toBe(true);
  });

  it('promises its own published accounts as this borrowing leaves them (B2, Reporting A2)', () => {
    const w = ranWorld('bond-issue', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND || !isCorporateBond(i.terms)) continue;
      // B2: both lines are real promises, and neither is a parameter anybody declared.
      expect(i.terms.covenants.leverage).toBeGreaterThan(0);
      expect(i.terms.covenants.coverage).toBeGreaterThan(0);
      // N13.a: senior unsecured, which is what a firm's first market borrowing is.
      expect(i.terms.seniority).toBe(1);
      // B2.a: a promise the issuer could actually make — its own published accounts, so the firm
      // that has not deteriorated since is inside it and only the one that has is not.
      const said = w.published.lastStatement(i.terms.issuer);
      if (said === undefined || said.balance.assets.pieces <= 0) continue;
      expect(i.terms.covenants.leverage).toBeGreaterThanOrEqual(
        said.balance.liabilities.pieces / said.balance.assets.pieces,
      );
    }
  });

  it('funds one shortfall through one channel, never both (Law 4, Law 5)', () => {
    const w = ranWorld('bond-issue', 12);
    for (const e of w.journal.ofKind('bond.offered')) {
      const issuer = e.subjects[0];
      if (issuer === undefined) continue;
      // Corporate Credit A1, C7: a bank writing a loan against the same published number would
      // fund the same hole twice, and the second one is a residual with no holder.
      const wrote = w.journal
        .ofKindIn('credit.written', period(e.period + 1))
        .filter((c) => c.subjects.includes(issuer));
      expect(wrote).toEqual([]);
    }
  });

  it('has something to test, which is why the covenant table exists (10.3)', () => {
    const w = ranWorld('bond-issue', 12);
    const live = w.instruments.all().filter((i) => i.kind === CORPORATE_BOND && i.status.live);
    expect(live.length).toBeGreaterThan(0);
    // 10.4, G2: and every one of them cross-defaults, which a sovereign line does not (Sovereign G3).
    expect(w.registry.instrumentKind(CORPORATE_BOND).accelerates).toBe(true);
  });
});
