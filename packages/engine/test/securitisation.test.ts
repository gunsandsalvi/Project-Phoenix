/**
 * Securitisation (13e): a loan leaves a bank's book, and a real party takes it.
 *
 * @spec Securitisation C1 Securitisation C2 Securitisation C2.a Securitisation C3 Securitisation C4 Securitisation C4.a Securitisation C5 Securitisation C6 Securitisation E1 Securitisation E2 XI-11 Law 2 Law 3
 */
import { describe, expect, it } from 'vitest';
import { isLoan } from '../src/index.js';
import {
  TRANCHE,
  faceToShed,
  securitisation,
  VEHICLE,
  isTranche,
  trancheTerms,
} from '../src/mechanisms/securitisation/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number) {
  const w = rigWorld('spv-a');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

describe('a vehicle is a real party (Securitisation C1, XI-11)', () => {
  it('has the shape XI-11 requires of a transferee: named, with a balance sheet', () => {
    const w = ran(4);
    for (const p of w.parties.ofKind(VEHICLE)) {
      // XI-11: there is no risk transfer without a transferee. A pool that is a number on a bank's
      // report and not a party holding rows has moved nothing.
      expect(p.representation).toBe('named');
      expect(String(p.id).startsWith('spv.')).toBe(true);
    }
  });

  it('holds only rows that name a borrower, its own notes and the money it collected', () => {
    const w = ran(4);
    for (const p of w.parties.ofKind(VEHICLE)) {
      for (const h of w.register.holdingsOf(p.id)) {
        const i = w.instruments.get(h.instrument);
        const money = w.registry.instrumentKind(i.kind).pricing === 'money';
        // E1: a pool of anonymous exposure is exactly what XI-11 forbids.
        expect(isLoan(i.terms) || isTranche(i.terms) || money).toBe(true);
      }
    }
  });
});

describe('a tranche states its boundaries and clears its price (C2.a, C3, Law 3)', () => {
  it('attaches below where it detaches, and never claims more than the pool', () => {
    const w = ran(4);
    for (const i of w.instruments.all()) {
      if (i.kind !== TRANCHE) continue;
      const t = trancheTerms(i);
      expect(t.attachment).toBeLessThan(t.detachment);
      expect(t.attachment).toBeGreaterThanOrEqual(0);
      expect(t.detachment).toBeLessThanOrEqual(1);
      // C6: the layers are cut FROM the pool, so what they claim can never exceed it.
      expect((t.detachment - t.attachment) * t.pool).toBeLessThanOrEqual(t.pool);
    }
  });

  it('is priced by a market and never off a spread, a rating or a table (Law 3)', () => {
    const w = ran(2);
    const kind = w.registry.instrumentKind(TRANCHE);
    expect(kind.pricing).toBe('cleared');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      if (!id.includes('tranche') && !id.includes('securitis')) continue;
      expect(id).not.toContain('spread');
      expect(id).not.toContain('attachment');
      expect(id).not.toContain('recovery');
      expect(id).not.toContain('correlation');
    }
  });

  it('declares NO number at all: every boundary is an outcome (Law 2)', () => {
    const w = ran(1);
    // The junior is what nobody would buy and the senior is what cleared. A world that declared
    // "the junior is 8%" would have written the answer down instead of finding it.
    const mine = w.params.all().filter((d) => String(d.id).startsWith('securitisation.'));
    expect(mine).toEqual([]);
  });
});

describe('the layers sum to the pool (Securitisation C6)', () => {
  it('cuts a senior and a junior that together are the whole of it, and no more', () => {
    const w = ran(4);
    const byVehicle = new Map<string, number[]>();
    for (const i of w.instruments.all()) {
      if (i.kind !== TRANCHE) continue;
      const t = trancheTerms(i);
      const list = byVehicle.get(String(t.vehicle)) ?? [];
      list.push((t.detachment - t.attachment) * t.pool);
      byVehicle.set(String(t.vehicle), list);
    }
    for (const [vehicle, faces] of byVehicle) {
      const first = w.instruments
        .all()
        .find((i) => i.kind === TRANCHE && isTranche(i.terms) && String(i.terms.vehicle) === vehicle);
      expect(first).toBeDefined();
      if (first === undefined || !isTranche(first.terms)) continue;
      const pool = first.terms.pool;
      const cut = faces.reduce((a, b) => a + b, 0);
      // Law 7: the tolerance is arithmetic dust on two terms, derived, never a band.
      expect(Math.abs(cut - pool)).toBeLessThanOrEqual(3 * Number.EPSILON * (cut + pool));
    }
  });
});

describe('when a bank sells, and how much (Securitisation D1, D2, Banks Capital B1.b)', () => {
  it('sheds the shortfall over what its own rule asks per unit, and nothing else', () => {
    // Two published numbers and a division. A bank 8 short, whose rule asks 0.08 of capital per
    // unit of weighted assets, has to get 100 of face off its book for the room to come back.
    expect(faceToShed({ headroom: -8, minWeighted: 0.08, binds: 'weighted' })).toBeCloseTo(100, 9);
    expect(faceToShed({ headroom: -1, minWeighted: 0.5, binds: 'weighted' })).toBeCloseTo(2, 9);
  });

  it('sells NOTHING for a bank with room, which is arithmetic and not a threshold (Law 6)', () => {
    expect(faceToShed({ headroom: 1, minWeighted: 0.08, binds: 'weighted' })).toBe(0);
    expect(faceToShed({ headroom: 0, minWeighted: 0.08, binds: 'weighted' })).toBe(0);
  });

  it('sells NOTHING when the LEVERAGE rule is the one binding, because it would relieve nothing', () => {
    // The whole of the point: a sale at a price is a swap. The rows go out and money of the same
    // value comes in, so total assets do not move and a leverage ratio does not move with them. A
    // bank that sold its book anyway would have shed its entire business for no relief at all.
    expect(faceToShed({ headroom: -1000, minWeighted: 0.08, binds: 'leverage' })).toBe(0);
    expect(faceToShed({ headroom: -1000, minWeighted: 0.08, binds: 'nothing' })).toBe(0);
  });
});

describe('what this world actually does with it (Law 11)', () => {
  it('cuts no deal, because every shortfall here binds on leverage — and says so', () => {
    /**
     * A FINDING, written as a test so it cannot be lost. Every bank in this world that runs out of
     * room runs out of it on the LEVERAGE backstop, never on the weighted rule — the banks carry
     * reserves many times their capital, and reserves weigh nothing under one rule and everything
     * under the other (Banks Capital B1.b). Securitisation relieves the rule that is not binding,
     * so it correctly does nothing, and the mechanism is unreached rather than wrong.
     *
     * When this stops being true the assertion below fails, which is the point of writing it: the
     * day a bank here is weighted-bound is the day the deal machinery starts running, and nobody
     * should have to notice that by accident.
     */
    const w = ran(8);
    const short = w.journal.ofKind('bank.capital').filter((e) => Number(e.data['headroom']) < 0);
    expect(short.length).toBeGreaterThan(0);
    for (const e of short) expect(e.data['binds']).toBe('leverage');
    expect(w.journal.ofKind('securitisation.cut')).toHaveLength(0);
  });
});

describe('a loss lands on a layer, from the bottom (C2, C6, D4, XI-1)', () => {
  it('keeps Σ layer face equal to the pool after every event, and says so in the audit', () => {
    const w = ran(8);
    // C6 is the identity the write-down keeps. The ownership family measures it every period and
    // never repairs it; a period where it fired would be a residual with no holder (Appendix B).
    const report = w.step();
    const ownership = report.audit.families.find((f) => f.family === 'ownership');
    expect(ownership).toBeDefined();
    for (const v of ownership?.violations ?? []) {
      expect(String(v.spec)).not.toContain('Securitisation C6');
    }
  });

  it('writes a layer down by an event on a date, never by moving a number (XI-1)', () => {
    const w = ran(8);
    for (const e of w.journal.ofKind('tranche.writtenDown')) {
      // A holder that lost something is NAMED, and what it lost is a quantity of a real row.
      expect(typeof e.data['holder']).toBe('string');
      expect(Number(e.data['units'])).toBeGreaterThan(0);
      expect(Number(e.data['left'])).toBeGreaterThanOrEqual(0);
    }
  });
});

describe('the audit families are built and say so', () => {
  it('contributes to ownership and names, and both run rather than reporting "not built"', () => {
    // Part II: a FORBID that holds is as valuable as a mechanism that works, and it breaks
    // silently — so an unbuilt family must say so and must never read green by omission (Audit C2).
    const m = securitisation();
    expect(m.families.map((f) => f.name).sort()).toEqual(['names', 'ownership']);
    for (const f of m.families) expect(f.built).toBe(true);
    const w = rigWorld('spv-a');
    w.step();
    const report = w.step();
    for (const name of ['ownership', 'names'] as const) {
      const family = report.audit.families.find((f) => f.family === name);
      expect(family).toBeDefined();
      expect(family?.built).toBe(true);
    }
  });
});
