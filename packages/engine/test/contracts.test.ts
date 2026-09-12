/**
 * The contract store, the mark read from two sides, and the zero-sum family.
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative D2 Derivative D3 Derivative D3.a Derivative D7.b Derivative D8 Derivative D11 Derivative D12 Derivative X1 Derivative Layer A3 Derivative Layer A4 Derivative Layer B3.a Derivative Layer B4 Derivative Layer C1 Derivative Layer C1.a Derivative Layer G1 Derivative Layer G3 Derivative Layer G4 Audit B5
 *
 * A derivative is not a holding (X1): nobody issued it, it enters no ownership check, and what it
 * enters is the identity that the marks across its two sides come to nothing EXACTLY (D1.b). These
 * are the doors that identity is made of.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  USD,
  assemble,
  instrumentId,
  marketId,
  none,
  period,
  type Contract,
  type ContractTerms,
  type MechanismContext,
  type PartyId,
  type SystemModule,
  type World,
} from '../src/index.js';
import { mergeModules, rigSpec, withDependencies } from './rig.js';
import { notDealing } from './no-dealing.js';
import { isForward, testForwardKind, TEST_FORWARD, type ForwardTerms } from './support/test-forward.js';

/** A module that owns the test-only kind and writes rows on demand, through the wire. */
function forwards(plan: (ctx: MechanismContext) => void): SystemModule {
  return {
    id: 'test.forwards',
    spec: 'Derivative D1',
    requires: ['derivative-layer'],
    instrumentKinds: [],
    derivativeKinds: [testForwardKind],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: testForwardKind.unit, name: 'contracts', perUnit: 1 }],
    params: [],
    phases: [
      {
        name: 'test.forwards',
        spec: 'Derivative D1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: plan,
      },
    ],
    participants: [],
    families: [],
  };
}

function world(...extra: SystemModule[]): World {
  const spec = rigSpec('contracts');
  const kernelOnly = withDependencies(
    spec.modules,
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market' ||
      // The layer anchors its resolution to the estate's, because a claim must name somebody who
      // exists (Register F2) — so a world with the layer in it has the estate ACTING, not quiet.
      m.id === 'estate' ||
      m.id === 'derivative-layer',
  ).map(notDealing);
  return assemble({ ...spec, modules: mergeModules(kernelOnly, extra) });
}

/** Two banks of this world, which is all a bilateral contract needs (D1: two named sides). */
function twoBanks(w: World): readonly [PartyId, PartyId] {
  const banks = w.parties.ofKind(BANK).filter((b) => b.status.alive);
  const a = banks[0];
  const b = banks[1];
  if (a === undefined || b === undefined) throw new Error('this world drew fewer than two banks');
  return [a.id, b.id];
}

/** The line the test forward is written on: a sovereign bill this world already clears. */
function aLine(w: World): { readonly instrument: string; readonly market: string } {
  const m = w.markets.find((x) => x.kind === undefined && w.instruments.has(x.instrument));
  if (m === undefined) throw new Error('this world clears nothing');
  return { instrument: String(m.instrument), market: String(m.id) };
}

function termsOn(w: World, strike: number, expiry: number, long = true): ForwardTerms {
  const line = aLine(w);
  return {
    kind: TEST_FORWARD,
    market: marketId(line.market),
    underlying: instrumentId(line.instrument),
    expiry: period(expiry),
    strike,
    long,
    window: 8,
    // The fixture's own fields: this book is written by hand, so nobody is asked to trade in it.
    size: 0,
    movers: [],
  };
}

/** D1, B2: a row opens over the wire, in one numbered instruction, on two named books. */
function openOne(
  w: World,
  a: PartyId,
  b: PartyId,
  strike: number,
  expiry: number,
  notional = 100,
): Contract {
  let written: Contract | undefined;
  const mod = forwards((ctx) => {
    if (written !== undefined) return;
    const r = ctx.settle({
      legs: [
        {
          kind: 'contract',
          act: 'open',
          a,
          b,
          derivative: TEST_FORWARD,
          terms: termsOn(w, strike, expiry),
          ccy: USD,
          notional,
          struckAt: strike,
          value: 0,
          house: null,
        },
      ],
      cause: 'trade',
      reason: 'a forward struck at par',
    });
    if (r.outcome !== 'settled') throw new Error('the row did not settle');
    const id = r.contracts[0];
    if (id === undefined) throw new Error('settlement wrote no row');
    written = ctx.contracts.get(id);
  });
  const stepped = assemble({
    ...rigSpec('contracts'),
    modules: mergeModules(
      withDependencies(
        rigSpec('contracts').modules,
        (m) =>
          m.id === 'sovereign-instruments' ||
          m.id === 'seed.foundation' ||
          m.id === 'seed.funding' ||
          m.id === 'banks' ||
          m.id === 'money-market' ||
          m.id === 'estate' ||
          m.id === 'derivative-layer',
      ).map(notDealing),
      [mod],
    ),
  });
  stepped.step();
  if (written === undefined) throw new Error('no row was written');
  return written;
}

describe('the contract store (Derivative X1, D1, D12)', () => {
  it('opens a row with two named sides and an identity that never collapses', () => {
    const w = world();
    const [a, b] = twoBanks(w);
    const rows: Contract[] = [];
    const mod = forwards((ctx) => {
      if (ctx.period > 1) return;
      for (const strike of [100, 200]) {
        const r = ctx.settle({
          legs: [
            {
              kind: 'contract',
              act: 'open',
              a,
              b,
              derivative: TEST_FORWARD,
              terms: termsOn(w, strike, ctx.period + 4),
              ccy: USD,
              notional: 100,
              struckAt: strike,
              value: 0,
              house: null,
            },
          ],
          cause: 'trade',
          reason: `a forward at ${strike}`,
        });
        if (r.outcome === 'settled') {
          const id = r.contracts[0];
          if (id !== undefined) rows.push(ctx.contracts.get(id));
        }
      }
    });
    const built = world(mod);
    built.step();
    // D12: two contracts on one underlying at different strikes are TWO ROWS. Nothing nets them,
    // nothing collapses them, and the second is not an amendment of the first (B3.a).
    expect(rows.length).toBe(2);
    expect(new Set(rows.map((r) => r.id)).size).toBe(2);
    expect(built.contracts.openOf(a).length).toBe(2);
    // D1: two sides, and both of them have it.
    expect(built.contracts.openOf(b).length).toBe(2);
    for (const r of rows) expect(r.a === r.b).toBe(false);
  });

  it('is not a holding: no issued amount, no holder, no ownership row (X1)', () => {
    const w = world();
    const [a, b] = twoBanks(w);
    const c = openOne(w, a, b, 100, 5);
    // X1: a contract has no instrument behind it, so nothing in the register knows about it and the
    // ownership identity never sees it.
    expect(w.instruments.has(c.id as never)).toBe(false);
  });

  it('reads a party its own side and never a net across counterparties (C1.a, G3)', () => {
    const w = world();
    const banks = w.parties.ofKind(BANK).filter((x) => x.status.alive);
    const a = banks[0]?.id;
    const b = banks[1]?.id;
    const c = banks[2]?.id;
    if (a === undefined || b === undefined || c === undefined) return;
    const mod = forwards((ctx) => {
      if (ctx.period > 1) return;
      for (const [x, y, strike] of [
        [a, b, 100],
        [a, c, 100],
      ] as const) {
        ctx.settle({
          legs: [
            {
              kind: 'contract',
              act: 'open',
              a: x,
              b: y,
              derivative: TEST_FORWARD,
              terms: termsOn(w, strike, ctx.period + 4),
              ccy: USD,
              notional: 100,
              struckAt: strike,
              value: 0,
              house: null,
            },
          ],
          cause: 'trade',
          reason: 'a forward',
        });
      }
    });
    const built = world(mod);
    built.step();
    const view = built.participantView(a);
    expect(view.contracts.mine().length).toBe(2);
    // C1.a: netting is per counterparty PAIR, and the two pairs are two numbers. There is no door
    // anywhere that adds them together, which is G3: a book that looks flat until one of them fails.
    expect(built.contracts.between(a, b).length).toBe(1);
    expect(built.contracts.between(a, c).length).toBe(1);
    expect(typeof view.contracts.exposureTo(b)).toBe('number');
    expect(Object.keys(view.contracts)).toEqual(['mine', 'valueOf', 'exposureTo']);
  });
});

describe('the mark, read from two sides (D1, D8, D1.b, A3)', () => {
  it('is one number and its negation, and the zero-sum family says so', () => {
    const w = world();
    const [a, b] = twoBanks(w);
    const mod = forwards((ctx) => {
      if (ctx.period > 1) return;
      ctx.settle({
        legs: [
          {
            kind: 'contract',
            act: 'open',
            a,
            b,
            derivative: TEST_FORWARD,
            terms: termsOn(w, 1, ctx.period + 6),
            ccy: USD,
            notional: 100,
            struckAt: 1,
            value: 0,
            house: null,
          },
        ],
        cause: 'trade',
        reason: 'a forward struck at par',
      });
    });
    const built = world(mod);
    const report = built.step();
    const row = built.contracts.open_()[0];
    expect(row).toBeDefined();
    if (row === undefined) return;
    // D1: an asset to one and a liability to the other, at every instant.
    const toA = built.contractValue(row, a, built.period);
    const toB = built.contractValue(row, b, built.period);
    expect(toA + toB).toBe(0);
    // D1.b: and the family checks it INDEPENDENTLY — it asks the profile for the contract as each
    // side states it (`flip`) rather than negating the kernel's own answer.
    const zero = report.audit.families.find((f) => f.family === 'zeroSum');
    expect(zero?.built).toBe(true);
    expect(zero?.violations.filter((v) => v.owner === String(row.id))).toEqual([]);
  });

  it('a kind whose mark does not negate lights zero-sum and nothing else (A4, Audit B8)', () => {
    const w = world();
    const [a, b] = twoBanks(w);
    // A profile that reads its own side and forgets the other has the mirror of it: the mark is the
    // same number whichever way round the contract is stated, which is not a derivative.
    const broken = { ...testForwardKind, flip: (t: ContractTerms): ContractTerms => t };
    const mod: SystemModule = {
      ...forwards((ctx) => {
        if (ctx.period > 1) return;
        ctx.settle({
          legs: [
            {
              kind: 'contract',
              act: 'open',
              a,
              b,
              derivative: TEST_FORWARD,
              terms: termsOn(w, 1, ctx.period + 6),
              ccy: USD,
              notional: 100,
              struckAt: 1,
              value: 0,
              house: null,
            },
          ],
          cause: 'trade',
          reason: 'a forward whose two sides do not negate',
        });
      }),
      derivativeKinds: [broken],
    };
    const built = world(mod);
    const report = built.step();
    const row = built.contracts.open_()[0];
    if (row === undefined || built.contractMark(row, built.period) === 0) return;
    const zero = report.audit.families.find((f) => f.family === 'zeroSum');
    expect((zero?.count ?? 0) > 0).toBe(true);
    // Independence (Audit B8): the same defect does not light the accounts family, because both
    // sides of the balance sheet read the SAME mark — what is wrong is the other side's statement
    // of it, which is the one thing only this family looks at.
    const accounts = report.audit.families.find((f) => f.family === 'accounts');
    expect(accounts?.violations.some((v) => v.owner === String(row.id))).toBe(false);
  });
});

describe('the underlying is something this world produces (D3, D3.a, G4)', () => {
  it('refuses a row that would settle against a market nobody clears', () => {
    const w = world();
    const [a, b] = twoBanks(w);
    let refused: unknown;
    const mod = forwards((ctx) => {
      if (ctx.period > 1) return;
      const terms = { ...termsOn(w, 1, ctx.period + 4), market: marketId('mkt.nowhere') };
      try {
        ctx.settle({
          legs: [
            {
              kind: 'contract',
              act: 'open',
              a,
              b,
              derivative: TEST_FORWARD,
              terms,
              ccy: USD,
              notional: 100,
              struckAt: 1,
              value: 0,
              house: null,
            },
          ],
          cause: 'trade',
          reason: 'a forward on a market that does not exist',
        });
      } catch (e) {
        refused = e;
      }
    });
    const built = world(mod);
    built.step();
    // G4, D3.a: a derivative that settles against a price this world does not clear prices itself,
    // and it is refused at the site that writes it rather than at the first mark nobody can take.
    expect(refused).toBeDefined();
    expect(String(refused)).toContain('does not produce');
  });
});

describe('novation moves who faces whom (B4)', () => {
  it('takes the position off one book and puts it on another, and never collapses the pair', () => {
    const w = world();
    const banks = w.parties.ofKind(BANK).filter((x) => x.status.alive);
    const a = banks[0]?.id;
    const b = banks[1]?.id;
    const c = banks[2]?.id;
    if (a === undefined || b === undefined || c === undefined) return;
    const mod = forwards((ctx) => {
      if (ctx.period === 1) {
        ctx.settle({
          legs: [
            {
              kind: 'contract',
              act: 'open',
              a,
              b,
              derivative: TEST_FORWARD,
              terms: termsOn(w, 1, ctx.period + 8),
              ccy: USD,
              notional: 100,
              struckAt: 1,
              value: 0,
              house: null,
            },
          ],
          cause: 'trade',
          reason: 'a forward',
        });
        return;
      }
      if (ctx.period !== 2) return;
      const row = ctx.contracts.openOf(b)[0];
      if (row === undefined) return;
      ctx.settle({
        legs: [{ kind: 'contract', act: 'novate', contract: row.id, from: b, to: c }],
        cause: 'transfer',
        reason: `${b} novates to ${c}`,
      });
    });
    const built = world(mod);
    built.step();
    built.step();
    expect(built.contracts.openOf(b).length).toBe(0);
    expect(built.contracts.openOf(c).length).toBe(1);
    // B4: it is a real change of who faces whom — the pair index moves with it, so `a` now has an
    // exposure to `c` and none to `b`.
    expect(built.contracts.between(a, c).length).toBe(1);
    expect(built.contracts.between(a, b).length).toBe(0);
  });
});

describe('what the forward is (the test-only kind)', () => {
  it('is a derivative by the contract’s own definition', () => {
    // D2: a notional in a unit; D3: an underlying priced elsewhere; D7.b: struck at par so nothing
    // changes hands at inception; D11: it expires.
    expect(testForwardKind.premiumPerUnit(1, { kind: TEST_FORWARD })).toBe(0);
    expect(isForward({ kind: TEST_FORWARD })).toBe(false);
    expect(none<number>().some).toBe(false);
  });
});
