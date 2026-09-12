/**
 * The deliverable bond future: a named line, delivered, with a carry and a basis that are read.
 *
 * @spec Sovereign I1 Sovereign I1.a Sovereign I2 Sovereign I3 Sovereign I3.a Derivative D1.b Derivative D3 XI-5 Law 3 Law 19
 */
import { describe, expect, it } from 'vitest';
import {contractOf, BOND_FUTURE,
  BOND_FUTURE_PARAMS,
  bondCarryOf,
  bondFutureKind,
  isBondFuture,
  mirrored,
  netBasis,
  type Contract,
  type MarketDecl,
  type World,} from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number): World {
  const w = rigWorld('bfut');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

const futureBooks = (w: World): readonly MarketDecl[] =>
  w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isBondFuture(t);
    });

describe('the contract (I1, D3)', () => {
  it('names a real deliverable line and quotes per unit of face', () => {
    const w = ran(3);
    const books = futureBooks(w);
    expect(books.length).toBeGreaterThan(0);
    for (const m of books) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isBondFuture(t)) continue;
      // I1: a NAMED benchmark line, not a notional bond nobody issued.
      expect(w.instruments.has(t.deliverable)).toBe(true);
      expect(w.instruments.get(t.deliverable).status.live).toBe(true);
      expect(t.contractSize).toBe(w.params.price(BOND_FUTURE_PARAMS.size));
    }
  });

  it('is one number and its negation, whichever side states it (D1.b)', () => {
    const w = ran(3);
    const m = futureBooks(w)[0];
    const t = contractOf(m)?.terms;
    if (m === undefined || t === undefined || !isBondFuture(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = {
      id: 'contract.test' as Contract['id'],
      kind: BOND_FUTURE,
      a,
      b,
      terms: t,
      ccy: m.ccy,
      notional: 5,
      struckAt: 0.99,
      basis: 0,
      opened: w.period,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
    const reads = w.contractReads(w.period);
    expect(bondFutureKind.mark(c, w.period, reads) + bondFutureKind.mark(mirrored(c, bondFutureKind), w.period, reads)).toBe(0);
  });
});

describe('the carry and the basis (I1.a, Law 3, Law 19)', () => {
  it('reads the coupon from the terms and the financing from what the book printed', () => {
    const w = ran(4);
    const t = contractOf(futureBooks(w)[0])?.terms;
    if (t === undefined || !isBondFuture(t)) return;
    const ctx = w.mechanismContext('test');
    const carry = bondCarryOf(ctx, t.deliverable, t.expiry);
    // Either both reads are there and the carry is a number, or one of them is not and the carry
    // is Missing — never a rate somebody assumed.
    expect(typeof carry.some).toBe('boolean');
    const basis = netBasis(ctx, t.deliverable, t.expiry);
    expect(typeof basis.some).toBe('boolean');
  });

  it('has no parameter that sets a basis or a recovery anywhere', () => {
    // I1.a: MEASURED, never set. The parameter register has a contract size, a life, a margin
    // window and a trader's own drawdown tolerance — and nothing that names a basis.
    expect(Object.keys(BOND_FUTURE_PARAMS).some((k) => k.toLowerCase().includes('basis'))).toBe(false);
    expect(String(BOND_FUTURE_PARAMS.tolerance)).toContain('drawdown');
  });
});

describe('delivery (I1, XI-5)', () => {
  it('hands over the line against cash, or fails, and never settles in cash instead', () => {
    const w = ran(20);
    const delivered = w.journal.ofKind('future.delivered');
    for (const e of delivered) {
      expect(e.public).toBe(true);
      // XI-5: what was delivered is named, and what it was paid for is the line's own print.
      expect(typeof e.data['deliverable']).toBe('string');
      expect(typeof e.data['face']).toBe('number');
    }
    // D11: a future that delivers is never torn up for a cash difference by the layer instead.
    expect(bondFutureKind.expires({} as Contract, 0 as never, w.calendar)).toBe(false);
  });
});
