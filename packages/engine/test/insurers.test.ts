/**
 * Insurers and pensions (13h): the sector whose liability is a schedule, and therefore has duration.
 *
 * @spec Insurers A1 Insurers A2.a Insurers A3 Insurers A4.b Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers D1 Insurers D2 Insurers E1 XI-3 Law 2 Law 3 Law 6
 */
import { asCash, asRatio } from '../src/core/measure.js';
import { asPerPiece } from '../src/core/measure.js';
import { describe, expect, it } from 'vitest';
import {
  INSURANCE,
  POLICY,
  coverPrice,
  gapOf,
  insurers,
  presentValueOf,
  wantsDuration,
} from '../src/index.js';
import { rigWorld } from './rig.js';

describe('the liability is a SCHEDULE, which is the whole clause (B1, B2.b)', () => {
  it('refuses a policy with nothing owed on any date, because that is a cash balance', () => {
    const w = rigWorld('ins-a');
    const k = w.registry.instrumentKind(POLICY);
    // B2.b: a liability that accumulates contributions minus benefits plus investment income has no
    // schedule, no discount rate and no discounting — so it never moves when rates move, and the
    // sector's defining risk disappears. It is refused where it would be constructed.
    expect(() => {
      k.validateTerms({ kind: POLICY, insurer: 'x', schedule: [], discountedAt: 'y' } as never);
    }).toThrow(/B2\.b/);
  });

  it('refuses a scheduled payment of nothing, because that is not a promise (B1)', () => {
    const w = rigWorld('ins-a');
    const k = w.registry.instrumentKind(POLICY);
    expect(() => {
      k.validateTerms({
        kind: POLICY,
        insurer: 'x',
        discountedAt: 'y',
        schedule: [{ date: { y: 2030, m: 1, d: 1 }, perUnit: 0 }],
      } as never);
    }).toThrow(/Insurers B1/);
  });
});

describe('falling rates raise the liability (B2, B2.a, D2)', () => {
  it('is arithmetic on the schedule and the discount, not a rule anybody wrote', () => {
    const schedule = [
      { date: { y: 2030, m: 1, d: 1 }, perUnit: asPerPiece(100, 'what a unit pays') },
      { date: { y: 2040, m: 1, d: 1 }, perUnit: asPerPiece(100, 'what a unit pays') },
    ];
    const dear = presentValueOf(schedule, (d) =>
      asRatio(d.y === 2030 ? 0.9 : 0.5, 'what money then is worth now'),
    );
    const cheap = presentValueOf(schedule, (d) =>
      asRatio(d.y === 2030 ? 0.95 : 0.7, 'what money then is worth now'),
    );
    // The same promise is worth MORE when money later is worth more — which is what a rate falling
    // means. That is why a rate move is a solvency event for this sector (B2.a) and a P&L event for
    // everybody else, and nothing anywhere had to say so.
    expect(cheap).toBeGreaterThan(dear);
    expect(dear).toBeCloseTo(140, 9);
  });

  it('makes the INSTITUTION wear it, which is the difference from a fund (A2.a)', () => {
    const w = rigWorld('ins-a');
    // The beneficiary does not absorb the investment result: the promise is fixed and the
    // institution's equity is what moves. One declared fact does it, and the revaluation does
    // the rest — a sector that passed the result through would be a fund wearing an insurer's name.
    expect(w.registry.instrumentKind(POLICY).owes).toBe('value');
    expect(w.registry.instrumentKind(POLICY).liabilityOfIssuer).toBe(true);
  });

  it('is priced off a market and never at a fixed rate (B2.b, Law 3)', () => {
    const w = rigWorld('ins-a');
    expect(w.registry.instrumentKind(POLICY).pricing).toBe('derived');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      if (!id.startsWith('insur')) continue;
      expect(id).not.toContain('discount');
      expect(id).not.toContain('lossratio');
    }
    // And the module states no numbers at all: what it charges and what it owes are both outcomes.
    expect(insurers().params).toEqual([]);
  });
});

describe('it can fail, and the gap is what does it (A3, D1, XI-3)', () => {
  it('says so on the party kind, both ways', () => {
    const w = rigWorld('ins-a');
    const k = w.registry.partyKind(INSURANCE);
    // A3: equity is assets minus liabilities, it is a read, and it can go negative. XI-3: nothing
    // is immortal, and an institution that could not fail would be the exception nobody named.
    expect(k.fails).toContain('solvency');
    expect(k.fails).toContain('cash');
  });

  it('measures the gap as what it has against what it owes, and lets it be negative (Law 6)', () => {
    expect(gapOf(asCash(100, 'what it has'), asCash(60, 'what it owes'))).toBe(40);
    expect(gapOf(asCash(60, 'what it has'), asCash(100, 'what it owes'))).toBe(-40);
  });
});

describe('what it charges is its own experience and its own capital (A4.b)', () => {
  it('is those two things and nothing else — no loss ratio, no industry number', () => {
    const seen = (x: number) => asPerPiece(x, 'what its own claims have cost it');
    const costs = (x: number) => asRatio(x, 'what its capital costs');
    expect(coverPrice(seen(0.03), costs(0.02))).toBeCloseTo(0.05, 12);
    // Worse experience or dearer capital quotes higher, which is the whole of A4.b.
    expect(coverPrice(seen(0.06), costs(0.02))).toBeGreaterThan(coverPrice(seen(0.03), costs(0.02)));
    expect(coverPrice(seen(0.03), costs(0.05))).toBeGreaterThan(coverPrice(seen(0.03), costs(0.02)));
  });

  it('wants long assets against long liabilities, which is a read of its own book (C2.a)', () => {
    const long = [{ date: { y: 2045, m: 1, d: 1 }, perUnit: asPerPiece(1, 'what a unit pays') }];
    expect(wantsDuration(long, { y: 2030, m: 1, d: 1 })).toBe(true);
    expect(wantsDuration(long, { y: 2050, m: 1, d: 1 })).toBe(false);
  });
});
