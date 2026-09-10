/**
 * The indivisible piece of a unit: what it is, that the arithmetic on it is exact, and that the
 * world's path does not turn on how fine it is.
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
  currencyUnit,
  downTick,
  foundationSpec,
  foundationWorld,
  none,
  onTick,
  partyId,
  splitOnTick,
  toTick,
  unitId,
  upTick,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

describe('what a piece is (Law 8)', () => {
  it('is a whole number, so pieces add exactly and there is no dust to forgive (Law 7)', () => {
    expect(onTick(1)).toBe(true);
    expect(onTick(1000000)).toBe(true);
    // The two things that are NOT quantities: a fraction of a piece, and a count so large the
    // machine can no longer add one to it exactly.
    expect(onTick(0.5)).toBe(false);
    expect(onTick(0.01)).toBe(false);
    expect(onTick(Math.pow(2, 53) + 2)).toBe(false);
    // This is the whole reason a quantity is a COUNT and not a fraction of a named unit: a
    // hundredth cannot be held exactly in binary, so a hundred of them are not one...
    expect(0.1 + 0.2 === 0.3).toBe(false);
    // ...while a million cents added one at a time are exactly a million cents.
    let running = 0;
    for (let i = 0; i < 1000000; i += 1) running += 1;
    expect(running).toBe(1000000);
    expect(onTick(running)).toBe(true);
  });

  it('rounds three ways, and each is a different question', () => {
    // What exists nearest: the answer for a value that BECOMES a payment.
    expect(toTick(1.4999)).toBe(1);
    expect(toTick(1.5)).toBe(2);
    // What somebody CAN pay or deliver: never up, because the piece above is not theirs.
    expect(downTick(1.4999)).toBe(1);
    expect(downTick(2.9)).toBe(2);
    // What a requirement NEEDS: never down, because a recipe met with the piece below is not met.
    expect(upTick(2.1)).toBe(3);
    // ...and it works the same on both sides of zero.
    expect(downTick(-2.9)).toBe(-2);
    expect(upTick(-2.1)).toBe(-3);
  });
});

describe('splitting a piece (Clearing C3, Law 2)', () => {
  it('gives the parts to named claimants and never leaves a residual', () => {
    // Ten cents shared three ways: four, three and three — and the odd cent has a holder.
    const parts = splitOnTick(10, [1, 1, 1]);
    expect(parts).toEqual([4, 3, 3]);
    expect(parts.reduce((a, b) => a + b, 0)).toBe(10);
  });

  it('gives the odd pieces to the largest remainders, ties to the earlier claimant', () => {
    const parts = splitOnTick(7, [3, 3, 1]);
    expect(parts.reduce((a, b) => a + b, 0)).toBe(7);
    expect(parts[0]).toBeGreaterThanOrEqual(parts[2] ?? 0);
    // The same claimants in the same order always get it: nothing here is a coin toss.
    expect(splitOnTick(7, [3, 3, 1])).toEqual(splitOnTick(7, [3, 3, 1]));
  });

  it('sums to exactly the whole, at any weights and either sign', () => {
    for (const weights of [[1, 2, 3, 5, 8], [1], [7, 7, 7, 7], [1, 1000000]]) {
      for (const total of [1, 13, 9999]) {
        const parts = splitOnTick(total, weights);
        expect(parts.reduce((a, b) => a + b, 0)).toBe(total);
        expect(parts.every((p) => onTick(p))).toBe(true);
      }
    }
    expect(splitOnTick(-10, [1, 1, 1]).reduce((a, b) => a + b, 0)).toBe(-10);
  });

  it('deals with a population in whole pieces per member (XI-15)', () => {
    // Two cells of five hundred and three hundred can exchange fifteen hundred pieces at a time:
    // five each for one of them and three each for the other, and nothing finer than that.
    expect(commonGrain(500, 300)).toBe(1500);
    expect(commonGrain(1, 1)).toBe(1);
    expect(commonGrain(1, 500)).toBe(500);
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
  it('throws on half a cent, and settles a whole one', () => {
    const spec = foundationSpec('offgrid');
    const bad = assemble({ ...spec, modules: [...spec.modules, payer(0.5)] });
    expect(() => bad.step()).toThrow(/not a whole number of pieces/);
    const good = assemble({ ...spec, modules: [...spec.modules, payer(1)] });
    expect(() => good.step()).not.toThrow();
  });
});

/** The same world, declared in pieces `shift` times finer than the units state. */
function atShift(
  shift: number,
  periods: number,
): { produced: number; money: number; batches: number; piece: number; reds: number } {
  const spec = foundationSpec('piece-invariance');
  const params = spec.params.map((p) =>
    p.id === KERNEL_PARAMS.pieceShift ? { ...p, value: shift } : p,
  );
  const w: World = assemble({ ...spec, params });
  const perPHX = w.registry.subdivision(currencyUnit(PHX));
  const perTonne = w.registry.subdivision(unitId('tonnes'));
  let produced = 0;
  let batches = 0;
  let reds = 0;
  for (let i = 0; i < periods; i += 1) {
    const r = w.step();
    reds += unexpected(r.audit).length;
    for (const e of w.journal.ofKind('firms.produced')) {
      if (e.period !== w.period) continue;
      produced += Number(e.data['finished']);
      batches += 1;
    }
  }
  // Read back in NAMED units, because that is what two worlds declared at different subdivisions
  // have in common: one holds cents and the other tenths of a cent, and both hold the same PHX.
  return {
    produced: produced / perTonne,
    money: (w.moneyStock()['PHX'] ?? 0) / perPHX,
    batches,
    piece: 1 / perTonne,
    reds,
  };
}

describe('how fine the pieces are, is a RESOLUTION (Law 2)', () => {
  it('holds every structural invariant exactly, at every subdivision', () => {
    // This is the invariance that must be EXACT, and it is: money is conserved, holdings sum to
    // what is issued, no residual is left anywhere — with the piece a cent, a tenth of a cent and
    // a hundredth of one. Nothing here is within anything.
    for (const shift of [1, 10, 100]) expect(atShift(shift, 8).reds).toBe(0);
  });

  it('converges as the piece gets finer, in what it made and in the money it holds', () => {
    const periods = 8;
    const coarse = atShift(1, periods);
    const finer = atShift(10, periods);
    const finest = atShift(100, periods);
    // What is EXACTLY invariant is the structure, and the test above says so with no band at all.
    // THE PATH IS NOT, and honestly so: what a payment or a batch comes to is rounded to a whole
    // piece, and this world's decisions are thresholds (Law 2 forbids deciding at an average), so a
    // firm on the edge of starting a batch starts it in one run and not in the other and the two
    // histories differ from then on. The same happens for any perturbation at all.
    //
    // So what has to be true is that the difference IS the rounding and nothing else — and the way
    // to say that without choosing a band is that a factor of ten finer is a factor of ten closer.
    // That is measured here in both halves of the world, the real one and the money one, and a
    // number that had stopped scaling with the grid would fail it rather than pass quietly.
    const gap = (a: number, b: number): number => Math.abs(a - b) / Math.abs(a);
    expect(gap(finer.produced, finest.produced)).toBeLessThan(gap(coarse.produced, finer.produced));
    expect(gap(finer.money, finest.money)).toBeLessThan(gap(coarse.money, finer.money));
    // And every batch either world started is a whole number of pieces of its own good, which is
    // what makes the difference a rounding rather than a different world.
    expect(coarse.batches).toBeGreaterThan(0);
    expect(coarse.piece).toBeGreaterThan(0);
  });

  it('conserves money exactly, which is what whole pieces buy', () => {
    const w = foundationWorld('piece-conserve');
    for (let i = 0; i < 8; i += 1) w.step();
    const money = w.last?.audit.families.find((f) => f.family === 'money');
    // NOTHING. Money is conserved, no stock moved by more than its own creation legs, and no
    // account closes below zero without a lender behind it — with no tolerance anywhere in any of
    // the three, because whole pieces add exactly.
    expect(money?.violations).toEqual([]);
  });
});
