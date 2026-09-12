/**
 * Ratings: named opinions made from state, sold to the issuers they are about — and the books that
 * have nobody with a view in them.
 *
 * @spec Ratings A1 Ratings A2 Ratings A2.a Ratings A3 Ratings A4 Ratings A5 Ratings B1 Ratings B2 Ratings D1 Ratings D5 Ratings E1 Ratings E2 Ratings E4 XI-13 Law 3 Law 15
 */
import { describe, expect, it } from 'vitest';
import {
  ASSESSOR,
  ASSESSOR_COUNT,
  GRADES,
  TREASURY_US,
  bandOf,
  forInstrument,
  partyId,
  period,
  type AssessorDecl,
} from '../src/index.js';
import { rigWorld } from './rig.js';

const methodology = (firstBoundary: number, boundaryStep: number): AssessorDecl => ({
  assessor: 'assessor.test',
  name: 'A Test',
  bank: 'bank.a',
  patience: 2,
  firstBoundary,
  boundaryStep,
  fee: 0,
  why: 'a methodology stated by a test',
});

describe('the scale (Ratings A3, B1)', () => {
  it('is coarse, ordinal and widens, so a worse state never gets a better grade', () => {
    const d = methodology(0.05, 2);
    const grades = [-1, 0, 0.04, 0.06, 0.2, 0.5, 2, 50].map((strain) => bandOf(strain, d));
    for (let i = 1; i < grades.length; i += 1) {
      const before = GRADES.indexOf(grades[i - 1] ?? 'c');
      const now = GRADES.indexOf(grades[i] ?? 'c');
      expect(now).toBeGreaterThanOrEqual(before);
    }
    expect(grades[0]).toBe(GRADES[0]);
    expect(grades[grades.length - 1]).toBe(GRADES[GRADES.length - 1]);
  });

  it('gives two assessors with different methodologies different answers about one state (XI-13)', () => {
    const keen = methodology(0.02, 2);
    const relaxed = methodology(0.08, 2);
    expect(bandOf(0.05, keen)).not.toBe(bandOf(0.05, relaxed));
  });
});

describe('an instrument is not its issuer (Ratings B2)', () => {
  it('lifts a secured claim and drops one that ranks behind, from the queue and nothing else', () => {
    const at = (g: string): number => GRADES.indexOf(g as never);
    expect(at(forInstrument('bbb', 0, true))).toBeLessThan(at('bbb'));
    expect(at(forInstrument('bbb', -2, false))).toBeGreaterThan(at('bbb'));
    // The best grade cannot be improved past the end of the scale, and the worst cannot fall off it.
    expect(forInstrument(GRADES[0], 0, true)).toBe(GRADES[0]);
    expect(forInstrument('c', -9, false)).toBe('c');
  });
});

describe('the assessors of a world (Ratings A1, A5, D5)', () => {
  it('are three named parties, each with its own methodology in the register', () => {
    const w = rigWorld('rate-A');
    const them = w.parties.ofKind(ASSESSOR);
    expect(them.length).toBe(ASSESSOR_COUNT);
    const boundaries = new Set<number>();
    for (const a of them) {
      expect(a.representation).toBe('named');
      // A5: it banks somewhere, because it is paid and it pays.
      expect(w.parties.has(a.bank)).toBe(true);
      boundaries.add(w.params.ratio(`rating.firstBoundary.${a.id}` as never));
    }
    // D5, XI-13: three opinions and not one — and they are actually different opinions.
    expect(boundaries.size).toBeGreaterThan(1);
  });

  it('publishes an opinion about everybody who borrows, with what moved it (A4, D1, E1, E2)', () => {
    const w = rigWorld('rate-B');
    w.step();
    const actions = w.journal.ofKind('rating.action').filter((e) => e.period === w.period);
    expect(actions.length).toBeGreaterThan(0);
    const subjects = new Set<string>();
    for (const e of actions) {
      expect(e.public).toBe(true);
      expect(GRADES).toContain(e.data['grade']);
      // E2: what moved it is in the action — the measure, and what the grade was before.
      expect(typeof e.data['strain']).toBe('number');
      expect('was' in e.data).toBe(true);
      const who = e.data['subject'];
      if (typeof who === 'string') subjects.add(who);
    }
    // D1: the state is rated too, on the same measure as anybody else.
    for (const t of w.parties.ofKind(partyId('treasury') as never))
      expect(subjects.has(t.id)).toBe(true);
  });

  it('is sticky: a grade does not move the period its measure crosses a boundary (A3)', () => {
    const w = rigWorld('rate-C');
    for (let i = 0; i < 8; i += 1) w.step();
    const moves = w.journal
      .ofKind('rating.action')
      .filter((e) => e.data['was'] !== null && e.data['was'] !== undefined);
    for (const e of moves) {
      const assessor = e.data['assessor'];
      if (typeof assessor !== 'string') continue;
      // A3: it waited its own patience. A world of assessors that all moved at once would have
      // every mandated holder acting on one event (C1.a), and that is what the spread prevents.
      expect(w.params.periods(`rating.patience.${assessor}` as never)).toBeGreaterThanOrEqual(1);
      expect(e.period).toBeGreaterThan(0);
    }
  });

  it('never moves a grade because a price moved (A2.a)', () => {
    const w = rigWorld('rate-D');
    for (let i = 0; i < 6; i += 1) w.step();
    // A2.a is structural: the assessor decides from a view with the prices closed. What that view
    // answers is Missing, for everything a price could reach — so a grade cannot have read one.
    const blind = w.blindView(w.parties.ofKind(ASSESSOR)[0]?.id ?? partyId('assessor.a'));
    const line = w.instruments.all().find((i) => i.market.some);
    expect(line).toBeDefined();
    if (line === undefined) return;
    expect(blind.print(line.id).some).toBe(false);
    expect(blind.mark(line.id).some).toBe(false);
    expect(blind.index('equity.us').some).toBe(false);
    expect(() => blind.curve('gov.us' as never)).toThrow();
  });
});

describe('the second opinion (XI-13, Ratings A5.a)', () => {
  it('says so, every period, for a book whose orders all come from mandates', () => {
    const w = rigWorld('rate-E');
    for (let i = 0; i < 3; i += 1) w.step();
    const said = w.journal.ofKind('market.noView').filter((e) => e.period === w.period);
    for (const e of said) {
      const market = e.data['market'];
      expect(typeof market).toBe('string');
      // It is only said of a book that actually ran with orders in it: silence about an empty
      // book would be a claim nobody made.
      const orders = e.data['orders'];
      expect(typeof orders === 'number' && orders > 0).toBe(true);
      expect(e.public).toBe(true);
    }
    // And a credit book has a desk in it, so it is never one of them.
    const credit = w.markets.find((m) => String(m.id).startsWith('mkt.ust.'));
    expect(credit).toBeDefined();
    expect(said.some((e) => e.data['market'] === credit?.id)).toBe(false);
  });
});

describe('what the measure is (Ratings A2, B1; Reporting G2)', () => {
  /** The assessor's own horizon: as many periods back as there are grades on its scale (B3). */
  const WINDOW = GRADES.length;

  it('is what falls due against what the issuer TAKES IN, not against what it is worth', () => {
    // 12-17: the measure used to be `owedIn` against `equity`, and a state's book equity is deeply
    // negative BY CONSTRUCTION — it owes its whole debt and owns nothing — so every treasury in the
    // world graded worst from period one and told nobody anything. A state's capacity to pay is its
    // tax base; a firm's is what it sells. Both are "what reached it", which is the equity ledger
    // item 12a built, with the MARKS EXCLUDED: a revaluation is what the world now thinks a thing
    // is worth and nobody handed it over (Clearing D4).
    const w = rigWorld('ratings-measure', 4, 24);
    for (let i = 0; i < 12; i += 1) w.step();
    const treasury = w.blindView(TREASURY_US);
    expect(treasury.equity()).toBeLessThan(0);
    // What it takes in is a different number from what it is worth, and it is the one the measure
    // uses now. A world where the two agreed would not be testing anything.
    expect(treasury.earned(WINDOW)).not.toBe(treasury.equity());
    expect(treasury.earned(WINDOW)).toBeGreaterThan(0);
  });

  it('lets a missed payment AGE OUT, because a grade is about a party state now (A2, A3)', () => {
    // `failedPayments` took a COUNT of failures rather than a horizon, so an assessor asking for
    // "the failures in its own memory" got every failure that party had ever had — and an issuer
    // that missed one payment in its first week was graded the worst there is for the rest of the
    // run. A count of events is not a horizon.
    const w = rigWorld('ratings-measure', 4, 24);
    for (let i = 0; i < 12; i += 1) w.step();
    const view = w.blindView(TREASURY_US);
    const all = view.failedPayments(period(0)).length;
    const recent = view.failedPayments(period(w.period - 1)).length;
    expect(all).toBeGreaterThan(0);
    expect(recent).toBeLessThan(all);
  });
});
