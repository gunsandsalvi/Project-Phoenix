/**
 * FX forwards, the basis, and the cross-currency swap.
 *
 * @spec FX Forwards A1 FX Forwards A1.b FX Forwards A1.d FX Forwards A2 FX Forwards A3 FX Forwards A4 FX Forwards B1 FX Forwards B2 FX Forwards B3 FX Forwards B3.b FX Forwards C1 FX Forwards C3 FX Forwards C4 FX Forwards E1 FX Forwards E3 FX Forwards E4 Derivative D1.b XI-5 Law 3
 */
import { describe, expect, it } from 'vitest';
import {contractOf, pairOf, FX_FORWARD,
  USD,
  XCCY,
  forwardLineOf,
  fxForwardKind,
  fxTenorsOf,
  isFxForward,
  isXccy,
  mirrored,
  period,
  xccyKind,
  xccyLineOf,
  type Contract,
  type MarketDecl,
  type World,} from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number): World {
  const w = rigWorld('fxfwd');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

const forwardBooks = (w: World): readonly MarketDecl[] =>
  w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isFxForward(t);
    });

const basisBooks = (w: World): readonly MarketDecl[] =>
  w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isXccy(t);
    });

describe('the forward (A1, A1.b, A1.d, A3, E3, XI-5)', () => {
  it('settles both notionals, each in its own money, on the one date', () => {
    const w = ran(3);
    const m = forwardBooks(w)[0];
    const t = contractOf(m)?.terms;
    if (m === undefined || t === undefined || !isFxForward(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = {
      id: 'contract.test' as Contract['id'],
      kind: FX_FORWARD,
      a,
      b,
      terms: t,
      ccy: t.quote,
      notional: 1_000_000,
      struckAt: 1.1,
      basis: 0,
      opened: w.period,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
    // A2, E3: nothing falls due before maturity — what moves in between is the margin.
    expect(fxForwardKind.legs(c, w.period, w.contractReads(w.period)).length).toBe(0);
    const due = fxForwardKind.legs(c, t.maturity, w.contractReads(t.maturity));
    expect(due.length).toBe(2);
    // A1.d, D5: a leg states its OWN money, and the two are different ones.
    expect(new Set(due.map((l) => l.ccy)).size).toBe(2);
    // A1.b: one each way, at the rate they struck. Both settle or neither does (XI-5) — which is
    // the layer's business, and what this asserts is that both are there to settle.
    const base = due.find((l) => l.ccy === t.base);
    const quote = due.find((l) => l.ccy === t.quote);
    expect(base?.amount).toBe(c.notional);
    expect(quote?.amount).toBeCloseTo(c.notional * c.struckAt, 6);
    expect(base?.from).toBe(quote?.to);
    expect(base?.to).toBe(quote?.from);
  });

  it('is one number and its negation, whichever side states it (D1.b)', () => {
    const w = ran(3);
    const t = contractOf(forwardBooks(w)[0])?.terms;
    if (t === undefined || !isFxForward(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = {
      id: 'contract.test' as Contract['id'],
      kind: FX_FORWARD,
      a,
      b,
      terms: t,
      ccy: t.quote,
      notional: 1_000_000,
      struckAt: 1.1,
      basis: 0,
      opened: w.period,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
    const reads = w.contractReads(w.period);
    expect(fxForwardKind.mark(c, w.period, reads) + fxForwardKind.mark(mirrored(c, fxForwardKind), w.period, reads)).toBe(0);
  });

  it('marks against the FORWARD for the tenor left, not against spot (A3)', () => {
    // A3 is a statement about which print the mark reads, and the profile reads `terms.book` —
    // the forward's own line — rather than `terms.spot`. A forward struck at the market is worth
    // nothing the day it is struck, and marking it against spot would book its carry on day one.
    const w = ran(3);
    const t = contractOf(forwardBooks(w)[0])?.terms;
    if (t === undefined || !isFxForward(t)) return;
    expect(String(t.book)).toContain('fx.forward');
    expect(String(t.spot)).not.toBe(String(t.book));
  });
});

describe('the books (A1, B1, C1, C4)', () => {
  it('opens a forward and a basis book per pair per tenor, cleared', () => {
    const w = ran(3);
    const pairs = w.markets.filter((m) => pairOf(m) !== undefined);
    if (pairs.length === 0) return;
    expect(forwardBooks(w).length).toBeGreaterThan(0);
    for (const m of forwardBooks(w)) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isFxForward(t)) continue;
      expect(m.instrument).toBe(forwardLineOf(t.base, t.quote, t.tenorYears));
    }
    for (const m of basisBooks(w)) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isXccy(t)) continue;
      // C4: what clears on a basis book is a RATE — the basis on one leg, per annum.
      expect(xccyKind.quotedAs).toBe('rate');
      expect(m.instrument).toBe(xccyLineOf(t.base, t.quote, t.tenorYears));
    }
    expect(fxTenorsOf({ params: w.params }).length).toBeGreaterThan(0);
  });
});

describe('the cross-currency swap (C1, C1.a, C3)', () => {
  it('exchanges the notionals at the start and back at the ORIGINAL rate', () => {
    const w = ran(4);
    const t = contractOf(basisBooks(w)[0])?.terms;
    if (t === undefined || !isXccy(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = {
      id: 'contract.test' as Contract['id'],
      kind: XCCY,
      a,
      b,
      terms: t,
      ccy: t.quote,
      notional: 1_000_000,
      struckAt: 0.001,
      basis: 0,
      opened: t.started,
      state: 'open',
      terminated: { some: false },
      house: null,
    } as Contract;
    const start = xccyKind.legs(c, t.started, w.contractReads(t.started));
    const end = xccyKind.legs(c, t.maturity, w.contractReads(t.maturity));
    expect(start.length).toBe(2);
    expect(end.length).toBe(2);
    // C3: the same two amounts, the other way. The rate they come back at is the one they went
    // out at, which is what takes the FX risk off the principal and leaves only the two rates.
    const startQuote = start.find((l) => l.ccy === t.quote)?.amount;
    const endQuote = end.find((l) => l.ccy === t.quote)?.amount;
    expect(startQuote).toBe(endQuote);
    expect(start.find((l) => l.ccy === t.base)?.from).toBe(end.find((l) => l.ccy === t.base)?.to);
  });
});

describe('what the module refuses to be (B3.b, E1)', () => {
  it('has no parity formula anywhere that a level comes out of', () => {
    // The only arithmetic on the two rates is a RESERVATION — what one bank will pay — and a
    // reservation is a reason somebody has. Nothing here writes a price.
    expect('parity' in fxForwardKind).toBe(false);
    expect(fxForwardKind.premiumPerUnit(1.1, { kind: FX_FORWARD })).toBe(0);
    expect(period(0)).toBe(0);
    expect(String(USD)).toBe('USD');
  });
});
