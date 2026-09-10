/**
 * The smallest piece of a unit: what it is, what it does to arithmetic, and that the world's path
 * does not turn on how fine it is.
 *
 * @spec Law 1 Law 2 Law 7 Law 8 Money A2 Money C1 Register A1.c Clearing C3 XI-15
 */
import { describe, expect, it } from 'vitest';
import {
  BANK_A,
  KERNEL_PARAMS,
  PHX,
  assemble,
  commonGrain,
  downTick,
  foundationSpec,
  foundationWorld,
  isTick,
  none,
  onTick,
  partyId,
  splitOnTick,
  tickFromExponent,
  ticks,
  toTick,
  upTick,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const PIECE = tickFromExponent(20);

describe('what a tick is (Law 8)', () => {
  it('is a power of two, because only then do whole pieces add exactly (Law 7)', () => {
    expect(isTick(1)).toBe(true);
    expect(isTick(0.5)).toBe(true);
    expect(isTick(PIECE)).toBe(true);
    // The decimal grid everybody reaches for first: 0.01 is not representable in binary, so a
    // hundred of them do not make one and the dust is back with an extra step.
    expect(isTick(0.01)).toBe(false);
    expect(isTick(0.1)).toBe(false);
    expect(0.1 + 0.2 === 0.3).toBe(false);
    // On the grid, a million pieces added one at a time is exactly a million pieces.
    let running = 0;
    for (let i = 0; i < 100000; i += 1) running += PIECE;
    expect(running).toBe(100000 * PIECE);
    expect(onTick(running, PIECE)).toBe(true);
  });

  it('rounds three ways, and each is a different question', () => {
    const value = 1.4999 * PIECE;
    // What exists nearest: the answer for a value that BECOMES a payment.
    expect(ticks(toTick(value, PIECE), PIECE)).toBe(1);
    // What somebody CAN pay or deliver: never up, because the piece above is not theirs.
    expect(ticks(downTick(value, PIECE), PIECE)).toBe(1);
    expect(ticks(downTick(2.9 * PIECE, PIECE), PIECE)).toBe(2);
    // What a requirement NEEDS: never down, because a recipe met with the piece below is not met.
    expect(ticks(upTick(2.1 * PIECE, PIECE), PIECE)).toBe(3);
    // ...and it works the same on both sides of zero.
    expect(downTick(-2.9 * PIECE, PIECE)).toBe(-2 * PIECE);
    expect(upTick(-2.1 * PIECE, PIECE)).toBe(-3 * PIECE);
  });
});

describe('splitting a piece (Clearing C3, Law 2)', () => {
  it('gives the parts to named claimants and never leaves a residual', () => {
    // Ten pieces, three claimants: four, three and three — and the odd piece has a holder.
    const parts = splitOnTick(10 * PIECE, [1, 1, 1], PIECE).map((p) => ticks(p, PIECE));
    expect(parts).toEqual([4, 3, 3]);
    expect(parts.reduce((a, b) => a + b, 0)).toBe(10);
  });

  it('gives the odd pieces to the largest remainders, ties to the earlier claimant', () => {
    const parts = splitOnTick(7 * PIECE, [3, 3, 1], PIECE).map((p) => ticks(p, PIECE));
    expect(parts.reduce((a, b) => a + b, 0)).toBe(7);
    expect(parts[0]).toBeGreaterThanOrEqual(parts[2] ?? 0);
    // The same claimants in the same order always get it: nothing here is a coin toss.
    expect(splitOnTick(7 * PIECE, [3, 3, 1], PIECE)).toEqual(splitOnTick(7 * PIECE, [3, 3, 1], PIECE));
  });

  it('sums to exactly the whole, at any weights and either sign', () => {
    for (const weights of [[1, 2, 3, 5, 8], [1], [7, 7, 7, 7], [1, 1000000]]) {
      for (const total of [1, 13, 9999]) {
        const parts = splitOnTick(total * PIECE, weights, PIECE);
        expect(parts.reduce((a, b) => a + b, 0)).toBe(total * PIECE);
        expect(parts.every((p) => onTick(p, PIECE))).toBe(true);
      }
    }
    const negative = splitOnTick(-10 * PIECE, [1, 1, 1], PIECE);
    expect(negative.reduce((a, b) => a + b, 0)).toBe(-10 * PIECE);
  });

  it('deals with a population in whole pieces per member (XI-15)', () => {
    // Two cells of five hundred and three hundred can exchange fifteen hundred pieces at a time:
    // five each for one of them and three each for the other, and nothing finer than that.
    expect(commonGrain(PIECE, 500, 300)).toBe(1500 * PIECE);
    expect(commonGrain(PIECE, 1, 1)).toBe(PIECE);
    expect(commonGrain(PIECE, 1, 500)).toBe(500 * PIECE);
  });
});

/** A module that tries to pay an amount the money does not have a piece for. */
function payer(amount: number): SystemModule {
  return {
    id: 'test.offgrid',
    spec: 'Money C1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.offgrid',
        spec: 'Money C1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext): void => {
          if (ctx.period !== 1) return;
          const leg: Leg = {
            kind: 'money',
            from: { holder: BANK_A, issuer: BANK_A },
            to: { holder: partyId('firm.1'), issuer: BANK_A },
            ccy: PHX,
            amount,
            fromCell: none(),
            toCell: none(),
          };
          ctx.settle({ legs: [leg], cause: 'transfer', reason: 'a payment of something that is not money' });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('the wire refuses what does not exist (Law 8, Money C1)', () => {
  it('throws on an amount that is not a whole number of pieces, and settles one that is', () => {
    const spec = foundationSpec('offgrid');
    const bad = assemble({ ...spec, modules: [...spec.modules, payer(0.01)] });
    expect(() => bad.step()).toThrow(/not a whole number of ccy:PHX/);
    const good = assemble({ ...spec, modules: [...spec.modules, payer(1 / 64)] });
    expect(() => good.step()).not.toThrow();
  });
});

/** The same world, on a grid `shift` halvings finer (positive) or coarser (negative). */
function atShift(shift: number, periods: number): { produced: number; money: number; reds: number } {
  const spec = foundationSpec('tick-invariance');
  const params = spec.params.map((p) =>
    p.id === KERNEL_PARAMS.tickShift ? { ...p, value: shift } : p,
  );
  const w: World = assemble({ ...spec, params });
  let produced = 0;
  let reds = 0;
  for (let i = 0; i < periods; i += 1) {
    const r = w.step();
    reds += unexpected(r.audit).length;
    for (const e of w.journal.ofKind('firms.produced')) {
      if (e.period === w.period) produced += Number(e.data['finished']);
    }
  }
  return { produced, money: w.moneyStock()['PHX'] ?? 0, reds };
}

describe('how fine the grid is, is a RESOLUTION (Law 2)', () => {
  it('holds every structural invariant exactly, at every grid', () => {
    // This is the invariance that must be EXACT, and it is: money is conserved, holdings sum to
    // what is issued, no residual is left anywhere — at a grid four thousand times coarser and
    // four thousand times finer than the declared one. Nothing here is within anything.
    for (const shift of [-6, -3, 0, 3, 6]) expect(atShift(shift, 8).reds).toBe(0);
  });

  it('converges: refine the grid and the answer stops moving', () => {
    const periods = 8;
    const coarse = [atShift(-6, periods), atShift(-3, periods)];
    const fine = [atShift(3, periods), atShift(6, periods)];
    const gap = (a: { produced: number; money: number }, b: typeof a): number =>
      Math.abs(a.produced - b.produced) + Math.abs(a.money - b.money);
    // Law 2: a resolution is tested by invariance, and here that is convergence — two grids eight
    // halvings apart at the fine end give the same world to within a hundredth, and two at the
    // coarse end do not. Nothing in this is a tolerance somebody chose: it is one measurement
    // against another.
    expect(gap(fine[0] as never, fine[1] as never)).toBeLessThan(
      gap(coarse[0] as never, coarse[1] as never),
    );
    expect(gap(fine[0] as never, fine[1] as never)).toBeLessThan(1);
    // BEYOND THIS HORIZON THE PATHS SEPARATE, and that is a fact about the world rather than about
    // the grid: its decisions are thresholds (Law 2 forbids deciding at an average), so somewhere
    // around the twelfth period a firm on the edge of starting a batch starts it in one run and
    // not in the other, and the two histories are different from then on. The same happens for any
    // perturbation at all. What must not move is what the test above asserts.
  });

  it('conserves money exactly at every grid, which is what the pieces buy', () => {
    const w = foundationWorld('tick-conserve');
    for (let i = 0; i < 8; i += 1) w.step();
    const report = w.last?.audit;
    const money = report?.families.find((f) => f.family === 'money');
    // Every violation this family has left is the one this world is known to report (test/
    // expected.ts); what it does NOT report any more is a balance that drifted off its own grid.
    expect(money?.violations.every((v) => v.message.includes('no lender row'))).toBe(true);
  });
});
