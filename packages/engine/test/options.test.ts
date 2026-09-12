/**
 * Options: the premium clears, the volatility is read back off it, and exercise is a decision.
 *
 * @spec Derivative D1 Derivative D1.b Derivative D2 Derivative D3 Derivative D3.a Derivative D7 Derivative D7.a Derivative D7.b Derivative D8 Derivative D8.a Derivative D9 Derivative D11 Derivative D11.a Bond N7.b Law 3 Law 6 §46 A3
 */
import { describe, expect, it } from 'vitest';
import {
  OPTION,
  OPTION_PARAMS,
  impliedMove,
  intrinsic,
  isOption,
  mirrored,
  optionKind,
  period,
  type Contract,
  type MarketDecl,
  type World,
} from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number): World {
  const w = rigWorld('opt');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

const optionBooks = (w: World): readonly MarketDecl[] =>
  w.markets.filter((m) => m.contract !== undefined && isOption(m.contract.terms));

function rowOn(w: World, m: MarketDecl, a: string, b: string, notional: number): Contract {
  return {
    id: 'contract.test' as Contract['id'],
    kind: OPTION,
    a: a as never,
    b: b as never,
    terms: m.contract?.terms as never,
    ccy: m.ccy,
    notional,
    struckAt: 1,
    basis: notional,
    opened: w.period,
    state: 'open',
    terminated: { some: false },
    house: null,
  };
}

describe('the books (D7, D7.a, D3.a)', () => {
  it('lists a ladder on lines this world already clears, and prices the PREMIUM', () => {
    const w = ran(3);
    const books = optionBooks(w);
    expect(books.length).toBeGreaterThan(0);
    for (const m of books) {
      const t = m.contract?.terms;
      if (t === undefined || !isOption(t)) continue;
      // D3.a: the underlying exists outside the derivative — it is a line with its own market.
      expect(w.instruments.has(t.underlying)).toBe(true);
      expect(w.markets.some((x) => x.id === t.market)).toBe(true);
      expect(t.strike).toBeGreaterThan(0);
    }
    // D7, D7.a: unlike every par-struck contract in this tree, an option is BOUGHT — the cleared
    // price IS the premium, and it is what the holder pays the writer when the row is written.
    const some = books[0]?.contract?.terms;
    if (some === undefined || !isOption(some)) return;
    expect(optionKind.premiumPerUnit(3, some)).toBe(3 * some.multiplier);
  });
});

describe('exercise is a decision, not a clamp (Law 6, D11)', () => {
  it('pays what exercising came to, and nothing when nobody would', () => {
    const w = ran(3);
    const m = optionBooks(w).find((x) => {
      const t = x.contract?.terms;
      return t !== undefined && isOption(t) && t.right === 'call';
    });
    const t = m?.contract?.terms;
    if (m === undefined || t === undefined || !isOption(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = rowOn(w, m, String(a), String(b), 10);
    const paid = intrinsic(c, w.period, w.contractReads(w.period));
    // Not a floor: either exercising pays and this is what it paid, or nobody exercised and it is
    // nothing. There is no Math.max anywhere in this module and the lint would say so.
    expect(paid).toBeGreaterThanOrEqual(0);
    const print = w.prices.latest(t.underlying, w.period);
    if (print.some && print.value.price <= t.strike) expect(paid).toBe(0);
  });

  it('is one number and its negation, whichever side states it (D1.b, D8.a)', () => {
    const w = ran(3);
    const m = optionBooks(w)[0];
    const t = m?.contract?.terms;
    if (m === undefined || t === undefined || !isOption(t)) return;
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = rowOn(w, m, String(a), String(b), 10);
    const reads = w.contractReads(w.period);
    expect(optionKind.mark(c, w.period, reads) + optionKind.mark(mirrored(c, optionKind), w.period, reads)).toBe(0);
  });
});

describe('what is derived and what does not exist (Law 3, Bond N7.b)', () => {
  it('takes the implied move OFF the premium and stores no volatility anywhere', () => {
    // Law 3: the premium is what cleared; the move it implies is arithmetic on it, at the read.
    const m = impliedMove(4, 1, 4);
    expect(m.some).toBe(true);
    expect(m.some ? m.value : 0).toBe(2);
    // A premium for no time at all implies nothing, which is Missing and not zero.
    expect(impliedMove(4, 1, 0).some).toBe(false);
    // And there is no parameter, store or field named for a volatility in this module.
    expect(Object.keys(OPTION_PARAMS).some((k) => k.toLowerCase().includes('vol'))).toBe(false);
    expect('volatility' in optionKind).toBe(false);
    expect(period(0)).toBe(0);
  });
});
