import { describe, expect, it } from 'vitest';
import fc from 'fast-check';
import { clear, partyId, sum, type Order } from '../src/index.js';
import { asQty } from '../src/core/tick.js';

/** Law 8: a size in this test is a count of the unit's own pieces, like every size anywhere. */
const q = asQty;

const P = (s: string) => partyId(s);

describe('clearing solver (Clearing C1-C5)', () => {
  it('reports the non-clearing outcomes distinctly (C4.b)', () => {
    expect(clear([], 'proRata').kind).toBe('noDemand');
    expect(clear([{ party: P('a'), side: 'buy', price: 1, qty: q(1) }], 'proRata').kind).toBe(
      'noSupply',
    );
    const r = clear(
      [
        { party: P('a'), side: 'buy', price: 0.9, qty: q(10) },
        { party: P('b'), side: 'sell', price: 1.0, qty: q(10) },
      ],
      'proRata',
    );
    expect(r.kind).toBe('noOverlap');
  });

  it('clears at a posted price, never a bracket (C4.c), and rations pro rata (C3)', () => {
    const r = clear(
      [
        { party: P('b1'), side: 'buy', price: 1.0, qty: q(60) },
        { party: P('b2'), side: 'buy', price: 1.0, qty: q(40) },
        { party: P('b3'), side: 'buy', price: 0.95, qty: q(100) },
        { party: P('s1'), side: 'sell', price: 0.97, qty: q(50) },
      ],
      'proRata',
    );
    expect(r.kind).toBe('cleared');
    if (r.kind !== 'cleared') return;
    expect([0.95, 0.97, 1.0]).toContain(r.price);
    expect(r.volume).toBe(50);
    const buys = r.fills.filter((f) => f.side === 'buy');
    expect(buys.map((f) => f.party).sort()).toEqual(['b1', 'b2']);
    expect(buys.find((f) => f.party === 'b1')?.qty).toBeCloseTo(30, 12);
    expect(buys.find((f) => f.party === 'b2')?.qty).toBeCloseTo(20, 12);
    expect(r.rationed).toBe('buy');
  });

  const orderArb: fc.Arbitrary<Order> = fc.record({
    party: fc.integer({ min: 0, max: 9 }).map((n) => P(`p${n}`)),
    side: fc.constantFrom<'buy' | 'sell'>('buy', 'sell'),
    price: fc.integer({ min: 80, max: 120 }).map((n) => n / 100),
    qty: fc.integer({ min: 1, max: 1000 }).map(q),
  });

  it('is a pure function of the schedules (C5) and bought equals sold (D5)', () => {
    fc.assert(
      fc.property(fc.array(orderArb, { maxLength: 40 }), (orders) => {
        const a = clear(orders, 'proRata');
        const b = clear(orders, 'proRata');
        expect(a).toEqual(b);
        if (a.kind !== 'cleared') return true;
        const bought = sum(a.fills.filter((f) => f.side === 'buy').map((f) => f.qty));
        const sold = sum(a.fills.filter((f) => f.side === 'sell').map((f) => f.qty));
        expect(Math.abs(bought.value - sold.value)).toBeLessThanOrEqual(
          bought.dust + sold.dust + 1e-9,
        );
        expect(orders.some((o) => o.price === a.price)).toBe(true);
        for (const f of a.fills) {
          const o = orders.filter((x) => x.party === f.party && x.side === f.side);
          expect(f.qty).toBeGreaterThan(0);
          expect(f.qty).toBeLessThanOrEqual(sum(o.map((x) => x.qty)).value + 1e-9);
        }
        return true;
      }),
    );
  });

  it('adds no demand of its own (B5): volume never exceeds either side', () => {
    fc.assert(
      fc.property(fc.array(orderArb, { maxLength: 40 }), (orders) => {
        const r = clear(orders, 'proRata');
        if (r.kind !== 'cleared') return true;
        const demand = sum(orders.filter((o) => o.side === 'buy').map((o) => o.qty)).value;
        const supply = sum(orders.filter((o) => o.side === 'sell').map((o) => o.qty)).value;
        return r.volume <= demand + 1e-9 && r.volume <= supply + 1e-9;
      }),
    );
  });
});
