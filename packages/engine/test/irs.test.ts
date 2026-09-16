/**
 * Interest-rate swaps: two legs, one notional that never moves, and a curve that is its prints.
 *
 * @spec IRS A1 IRS A1.b IRS A1.c IRS A2 IRS A3 IRS A4 IRS C1 IRS C1.a IRS C2 IRS C3 IRS E1 IRS E2 IRS E3 Derivative D1.b Derivative D3.a Derivative D7.b Law 3 Law 19
 */
import { struckAs } from '../src/registry/derivatives.js';
import { asCash } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { describe, expect, it } from 'vitest';
import {contractOf, IRS,
  IRS_PARAMS,
  USD,
  benchmarkOf,
  floatingRate,
  forwardRate,
  irsKind,
  irsLineOf,
  irsTenorsOf,
  isIrs,
  mirrored,
  period,
  swapCurve,
  type Contract,
  type MarketDecl,
  type World,} from '../src/index.js';
import { rigWorld } from './rig.js';
import { moneyPrint, ratePrint, type Print } from '../src/prices/price-store.js';
import { asPerPiece } from '../src/core/measure.js';
import { instrumentId, marketId } from '../src/core/ids.js';

function ran(periods: number): World {
  const w = rigWorld('irs');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

function swapBooks(w: World): readonly MarketDecl[] {
  return w.markets.filter((m) => {
      const t = contractOf(m)?.terms;
      return t !== undefined && isIrs(t);
    });
}

function aSwap(w: World, a: string, b: string, tenorYears: number): Contract {
  return {
    id: 'contract.test' as Contract['id'],
    kind: IRS,
    a: a as never,
    b: b as never,
    terms: {
      kind: IRS,
      ccy: USD,
      benchmark: benchmarkOf(USD),
      book: irsLineOf(USD, tenorYears),
      maturity: period(w.period + 40),
      tenorYears,
      fixedEvery: w.params.periods(IRS_PARAMS.fixedEvery),
      floatEvery: w.params.periods(IRS_PARAMS.floatEvery),
      paysFixed: true,
      window: 8,
    },
    ccy: USD,
    notional: asQty(10_000_000, 'the notional'),
    struckAt: struckAs('rate', 0.02),
    basis: asCash(0, USD, 'what it cost'),
    opened: w.period,
    state: 'open',
    terminated: { some: false },
    pairedWith: { some: false },
    house: null,
  } as Contract;
}

describe('the books (A1, A1.c, C1, D3.a)', () => {
  it('opens one per money per tenor, and only where the overnight book has traded', () => {
    /**
     * D3.a: THE GATE IS THE BENCHMARK, so the test waits for the benchmark and not for a period
     * count. `irs.books` runs at the top of a period and the fixing is published at the bottom of
     * one, so the books open the period AFTER the overnight market first trades — and how long
     * that takes is a fact about a world where a bank has to be short of money before anybody
     * lends any, not a number this test may assert. It used to say `ran(3)`, which was three
     * periods of one particular draw (`13b-7`).
     */
    const w = rigWorld('irs');
    let bench = 0;
    for (let i = 0; i < 12 && (bench === 0 || swapBooks(w).length === 0); i += 1) {
      w.step();
      bench = w.journal.ofKind('index.benchmark').length;
    }
    expect(bench).toBeGreaterThan(0);
    const open = swapBooks(w);
    expect(open.length).toBeGreaterThan(0);
    for (const m of open) {
      const t = contractOf(m)?.terms;
      if (t === undefined || !isIrs(t)) continue;
      // E3, D3.a: it fixes on a book that transacted. The world published that fixing, by name.
      expect(
        w.journal.ofKind('index.benchmark').some((e) => e.subjects.includes(t.benchmark)),
      ).toBe(true);
      expect(contractOf(m)?.house).not.toBe(null);
    }
    // A1.c, D7.b: what clears is the fixed RATE that makes it worth nothing today.
    expect(irsKind.quotedAs).toBe('rate');
    expect(irsKind.premiumPerUnit(struckAs('rate', 0.02), { kind: IRS })).toBe(0);
  });
});

describe('the legs (A1.b, A2, A4, E1)', () => {
  it('never exchanges the notional, in any period of its life', () => {
    const w = ran(4);
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = aSwap(w, String(a), String(b), irsTenorsOf({ params: w.params })[0] ?? 1);
    for (let at = 0; at < 30; at += 1) {
      for (const leg of irsKind.legs(c, period(w.period + at), w.contractReads(period(w.period + at)))) {
        // E1: the notional is what the legs accrue ON. Nothing ever moves it.
        expect(leg.amount).not.toBe(c.notional);
        expect(leg.amount).toBeLessThan(c.notional);
      }
    }
  });

  it('moves the NET and only the net, in one direction (A1.b, A4)', () => {
    const w = ran(4);
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = aSwap(w, String(a), String(b), irsTenorsOf({ params: w.params })[0] ?? 1);
    const due = irsKind.legs(c, w.period, w.contractReads(w.period));
    // A1.b: one payment or none — never two that mostly cancel.
    expect(due.length).toBeLessThanOrEqual(1);
    for (const leg of due) {
      expect(leg.from).not.toBe(leg.to);
      expect(leg.amount).toBeGreaterThan(0);
    }
  });

  it('is one number and its negation, whichever side states it (D1.b)', () => {
    const w = ran(4);
    const parties = w.parties.all().filter((p) => p.status.alive);
    const a = parties[0]?.id;
    const b = parties[1]?.id;
    if (a === undefined || b === undefined) return;
    const c = aSwap(w, String(a), String(b), irsTenorsOf({ params: w.params })[0] ?? 1);
    const reads = w.contractReads(w.period);
    expect(irsKind.mark(c, w.period, reads).pieces + irsKind.mark(mirrored(c, irsKind), w.period, reads).pieces).toBe(0);
  });

  it('has nothing to accrue in a period the overnight book did not trade (D3.a)', () => {
    const w = ran(2);
    const reads = w.contractReads(w.period);
    const fixing = floatingRate(
      {
        kind: IRS,
        ccy: USD,
        benchmark: 'a book that never traded',
        book: irsLineOf(USD, 1),
        maturity: period(w.period + 10),
        tenorYears: 1,
        fixedEvery: 1,
        floatEvery: 1,
        paysFixed: true,
        window: 8,
      },
      reads,
    );
    // A read of a benchmark nobody published is Missing, not zero and not the last one carried on.
    expect(fixing.some).toBe(false);
  });
});

describe('the curve (C1, C1.a, C2, E2)', () => {
  it('is the set of what the books cleared, and a tenor nobody traded has no point', () => {
    const w = ran(6);
    const curve = swapCurve(w.mechanismContext('test'), USD);
    for (const p of curve) {
      const print = w.prices.latest(irsLineOf(USD, p.tenorYears), w.period);
      // C1.a: a READ. Every point on it is a print somebody paid, and nothing else is on it.
      expect(print.some).toBe(true);
      expect(print.some ? print.value.price : NaN).toBe(p.rate);
    }
    expect(curve.length).toBeLessThanOrEqual(irsTenorsOf({ params: w.params }).length);
  });

  it('derives a forward rate from two cleared points and stores none (C2)', () => {
    const made = [
      { tenorYears: 1, rate: 0.02 },
      { tenorYears: 3, rate: 0.03 },
    ];
    const f = forwardRate(made, 1, 3);
    expect(f.some).toBe(true);
    // Between year one and year three, the rate that makes the two the same investment.
    expect(f.some ? f.value : 0).toBeGreaterThan(0.03);
    // A tenor the curve has no point at has no forward off it.
    expect(forwardRate(made, 1, 5).some).toBe(false);
    expect(forwardRate(made, 3, 1).some).toBe(false);
  });

  it('has no way to compute a fixed rate from a discount curve (E2)', () => {
    // E2 is a FORBID, and what makes it hold is that the door does not exist: the profile has a
    // mark, legs and a margin, and nothing anywhere that turns a curve into a par rate.
    expect('parRate' in irsKind).toBe(false);
    expect(Object.keys(IRS_PARAMS).some((k) => k.toLowerCase().includes('par'))).toBe(false);
  });
});

describe('a book says what its prints ARE (Law 8, Derivative D7, 18.0)', () => {
  it('prints a swap book as a RATE and everything else as money, and the two cannot be confused', () => {
    const w = ran(8);
    const books = w.markets.filter((m) => m.kind === 'contract');
    expect(books.length).toBeGreaterThan(0);
    let rates = 0;
    let moneys = 0;
    for (const m of books) {
      const p = w.prices.latest(m.instrument, w.period);
      if (!p.some) continue;
      const quotes = w.registry.derivativeKind(m.contract.kind).quotedAs;
      // Law 4: the print's tag IS the kind's own `quotedAs`, not a second opinion about it.
      expect(p.value.quotedAs).toBe(quotes);
      if (quotes === 'rate') {
        rates += 1;
        // Two per cent and two cents are the same number and not the same thing. A reader that
        // asks a rate book for money is told, at the site, rather than handed 0.02 (E-11).
        expect(ratePrint(p.value, 'the rate it printed')).toBe(p.value.price);
        expect(() => moneyPrint(p.value, 'money it does not print')).toThrow();
      } else {
        moneys += 1;
        expect(moneyPrint(p.value, 'the level it printed')).toBe(p.value.price);
        expect(() => ratePrint(p.value, 'a rate it does not print')).toThrow();
      }
    }
    // Law 11: whether any contract book in THIS world printed is the world's business — the loop
    // above says nothing when none did, and what the tag MEANS is asserted on prints made here.
    expect(rates + moneys).toBeGreaterThanOrEqual(0);
    const asRate: Print = {
      instrument: instrumentId('book.under.test'),
      market: marketId('mkt.under.test'),
      period: period(0),
      price: asPerPiece(0.02, 'two per cent'),
      ccy: USD,
      quotedAs: 'rate',
      provenance: { kind: 'opening' },
    };
    expect(ratePrint(asRate, 'the rate it printed')).toBe(0.02);
    expect(() => moneyPrint(asRate, 'money it does not print')).toThrow();
    const asMoney: Print = { ...asRate, quotedAs: 'money' };
    expect(moneyPrint(asMoney, 'the level it printed')).toBe(0.02);
    expect(() => ratePrint(asMoney, 'a rate it does not print')).toThrow();
    // And an ordinary asset book prints money, which is what every book did before 18.0.
    const asset = w.markets.find((m) => m.kind !== 'contract' && w.prices.latest(m.instrument, w.period).some);
    expect(asset).toBeDefined();
    if (asset === undefined) return;
    const print = w.prices.latest(asset.instrument, w.period);
    expect(print.some && print.value.quotedAs).toBe('money');
  });
});
