/**
 * How much capital a bank must have, and which of the two rules is the one biting.
 *
 * @spec Banks Capital A1 Banks Capital A1.a Banks Capital B1 Banks Capital B1.a Banks Capital B1.b Banks Capital B1.c Banks Capital B2 Banks Capital B3 Banks Capital B3.a Banks Lending B2.a Banks Lending B2.b Sovereign E5 XI-3 Law 19
 *
 * B1.c is the clause these are about: WHICH CONSTRAINT BINDS IS AN OUTCOME. A model with one
 * capital rule in it has nothing to be an outcome — every bank is stopped by the same thing at the
 * same time — and the leverage backstop exists precisely because the weighted rule can be passed by
 * holding what the rule calls safe. So both are here, and what stops a given bank is a read of what
 * that bank actually holds.
 */
import { describe, expect, it } from 'vitest';
import {
  LENDING_PARAMS,
  assemble,
  foundationSpec,
  foundationWorld,
  partyId,
  type Event,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

/** The same world with one declared number set differently, wherever it was declared. */
function withParam(seed: string, over: Readonly<Record<string, number>>): World {
  const spec = foundationSpec(seed);
  const modules: SystemModule[] = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((p) => {
      const value = over[String(p.id)];
      return value === undefined ? p : { ...p, value };
    }),
  }));
  return assemble({ ...spec, modules });
}

function position(w: World, bank: string): Event | undefined {
  const said = w.journal.ofKind('bank.capital').filter((e) => e.subjects.includes(bank));
  return said[said.length - 1];
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

describe('the requirement (Banks Capital B1, B1.a)', () => {
  const w = run(foundationWorld('cap-a'), 8);

  it('publishes a position that is a read of the register, and says so (B3.a, A1)', () => {
    for (const bank of [BANK_A, BANK_B]) {
      const said = position(w, bank);
      expect(said?.public).toBe(true);
      // A1: capital is the RESIDUAL — the equity account, not a pot somebody filled.
      expect(num(said, 'capital')).toBe(w.register.equity(bank));
      // B1: and what it is measured against is a walk over what the bank holds, at marks.
      expect(num(said, 'assets')).toBeGreaterThan(0);
      expect(num(said, 'leverageRatio')).toBeCloseTo(num(said, 'capital') / num(said, 'assets'), 9);
      // B1: and a bank whose book weighs NOTHING has no weighted ratio, rather than an infinite
      // one. That is where these two banks are — see the next test — and saying it plainly is what
      // an Option is for (Appendix A).
      const weighted = num(said, 'weighted');
      if (weighted > 0) {
        expect(num(said, 'weightedRatio')).toBeCloseTo(num(said, 'capital') / weighted, 9);
      } else {
        expect(said?.data['weightedRatio']).toBeNull();
      }
    }
  });

  it('weighs a claim on somebody who cannot fail at nothing, and everything else at one (B1.a, E5, XI-3)', () => {
    // B1.a: the weight is a property of the ASSET. A bank's reserves and the sovereign's paper are
    // claims on parties whose kinds name no way to die, in the money those parties issue — so they
    // weigh zero, and the rest of the book weighs what an ordinary exposure weighs. That is why a
    // bank holds sovereign paper as its buffer instead of lending the money out (E5).
    for (const bank of [BANK_A, BANK_B]) {
      const said = position(w, bank);
      expect(num(said, 'weighted')).toBeLessThan(num(said, 'assets'));
      expect(num(said, 'weighted')).toBeGreaterThanOrEqual(0);
    }
    // And it is not nothing: a bank's dealing line carries the float of every share line it makes a
    // market in, on the bank's own balance sheet (Dealer Desks A1, F2), so the weighted requirement
    // has something to ask about. Before 11.2 that inventory sat in a separate party and the rule
    // asked this bank for zero — which is what "no desk exempt from its own bank's capital" means
    // when it is broken by construction rather than by a mistake.
    expect(num(position(w, BANK_A), 'weighted')).toBeGreaterThan(0);
    expect(num(position(w, BANK_A), 'weighted')).toBeLessThan(num(position(w, BANK_A), 'assets'));
    // And the weight is a rule, not a fact: make the sovereign's paper weigh what a loan weighs and
    // the same book weighs its whole size.
    const heavy = run(withParam('cap-a', { 'regulation.riskWeight.sovereign': 1 }), 8);
    const said = position(heavy, BANK_A);
    expect(num(said, 'weighted')).toBeCloseTo(num(said, 'assets'), 6);
  });
});

describe('which one binds (Banks Capital B1.b, B1.c)', () => {
  it('is an outcome of what the bank holds, and moves when the rules move (B1.c)', () => {
    // With an ordinary backstop the weighted rule is what a bank runs into: its book is mostly
    // zero-weighted paper, so the backstop is far away.
    const ordinary = run(foundationWorld('cap-b'), 8);
    // Ask a bank to fund every asset it holds out of its own capital — a backstop no bank with a
    // depositor can meet — and the SAME bank, holding exactly the same assets, is stopped by that
    // rule instead. Nothing about the bank changed; the answer to "what stops it" did (B1.c).
    const backstopped = run(withParam('cap-b', { [String(LENDING_PARAMS.leverageRatio)]: 1 }), 8);
    for (const bank of [BANK_A, BANK_B]) {
      expect(num(position(ordinary, bank), 'headroom')).toBeGreaterThan(
        num(position(backstopped, bank), 'headroom'),
      );
      expect(String(position(backstopped, bank)?.data['binds'])).toBe('leverage');
    }
    // B1.b: and the backstop uses no weights, so it bites on a book the weighted rule calls empty.
    const said = position(backstopped, BANK_A);
    expect(num(said, 'leverageRatio')).toBeLessThan(1);
    expect(said?.data['breach']).toBe(true);
  });

  it('reaches the credit decision, so a bank with no room declines (B3, B2.b)', () => {
    // B3, B3.a: A BANK NEAR THE LINE BEHAVES DIFFERENTLY, or the requirement is decorative. The
    // published position is what its own credit decision reads (Law 19: read, never recomputed), so
    // a bank with no headroom declines — and the decline is an answer with the binding rule on it
    // (Banks Lending C3.a), not a silent absence.
    const tight = run(withParam('cap-c', { [String(LENDING_PARAMS.leverageRatio)]: 1 }), 10);
    const loose = run(foundationWorld('cap-c'), 10);
    for (const bank of [BANK_A, BANK_B]) {
      expect(num(position(tight, bank), 'headroom')).toBeLessThan(0);
      expect(num(position(loose, bank), 'headroom')).toBeGreaterThan(0);
    }
    // B3: and the breach is public, with what it is short of and which rule it is short against —
    // the plan demanded and the supervision intensified, as events somebody outside can read.
    const plans = tight.journal.ofKind('bank.capitalPlan');
    expect(plans.length).toBeGreaterThan(0);
    expect(num(plans[plans.length - 1], 'short')).toBeGreaterThan(0);
    expect(loose.journal.ofKind('bank.capitalPlan').length).toBe(0);
    // And what a borrower is told is the rule that stopped it: a decline names what bound.
    const declined = tight.journal
      .ofKind('credit.declined')
      .filter((e) => e.data['binds'] === 'capital');
    for (const e of declined) expect(num(e, 'capitalRoom')).toBeLessThanOrEqual(0);
  });
});

describe('the buffer above it (Banks Capital B2)', () => {
  it('is the bank own choice, and a cautious bank has less room at the same rules (B2)', () => {
    // B2: the buffer is not a stated ratio — it is what THIS bank insists on running above the
    // line, and the two banks in this world differ in it because they are different banks.
    const w = run(foundationWorld('cap-d'), 8);
    expect(num(position(w, BANK_A), 'buffer')).toBeGreaterThan(num(position(w, BANK_B), 'buffer'));
    // Give bank A the same caution as bank B and its room grows, with everything else identical.
    const bolder = run(withParam('cap-d', { 'bank.capitalBuffer.bank.a': 0.005 }), 8);
    expect(num(position(bolder, BANK_A), 'headroom')).toBeGreaterThan(
      num(position(w, BANK_A), 'headroom'),
    );
  });
});
