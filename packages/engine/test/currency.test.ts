/**
 * The currency layer: four moneys, the pairs between them, and what a rate move does to a book.
 *
 * @spec Currency A1 Currency A3 Currency A5 Currency B1 Currency B4 Currency C1 Currency C5 Currency D1 Currency D2 Currency D4 Spot FX A1 Spot FX A3 Spot FX B1 Spot FX B2 Spot FX C1 Spot FX C3 Spot FX E1 Spot FX F1 XI-12 Law 3 Law 8
 *
 * A test here NAMES NO PARTY. Which bank is which is a draw (Seed B1.a); what a test may ask for is
 * a world with four moneys in it and then a party of a kind, which is what the rig is for.
 */
import { describe, expect, it } from 'vitest';
import {pairOf, ABROAD,
  USD,
  currencyUnit,
  fxPairId,
  moneyInstrumentId,
  partyId,
  type CurrencyCode,} from '../src/index.js';
import { rigWorld } from './rig.js';

const FOREIGN = ABROAD.map((c) => c.ccy);

describe('the currencies a world has (Currency A1, A3, A5)', () => {
  it('declares four moneys, each a named central bank’s liability in its own region and its own unit', () => {
    const w = rigWorld('ccy-A');
    const codes = [...w.registry.currencies.keys()];
    expect(codes).toEqual([USD, ...FOREIGN]);
    for (const code of codes) {
      const ccy = w.registry.currency(code);
      const issuer = w.parties.get(ccy.centralBank);
      // A2: a currency is a named issuer's promise, and the issuer is a party that exists.
      expect(issuer.status.alive).toBe(true);
      // A1: and it issues money in that currency and no other.
      expect(w.instruments.has(moneyInstrumentId(issuer.id, code))).toBe(true);
      // A3, Law 8: each money has its OWN unit, so two of them are never added.
      expect(w.registry.subdivision(currencyUnit(code))).toBeGreaterThan(0);
    }
    // A3: one region per money and one money per region — a region banks in one thing.
    const regions = [...w.registry.regions.values()];
    expect(new Set(regions.map((r) => r.ccy)).size).toBe(codes.length);
  });
});

describe('the pairs (Spot FX A1, A3, C1, F1; XI-12)', () => {
  it('opens a market for every pair, prices it in the quote, and clears it before anything else', () => {
    const w = rigWorld('ccy-B');
    const pairs = w.markets.filter((m) => pairOf(m) !== undefined);
    const n = 1 + FOREIGN.length;
    // A3: every unordered pair of moneys, once. Six for four moneys — including the three that are
    // not the dollar's, which is what makes a triangle exist at all (XI-12).
    expect(pairs.length).toBe((n * (n - 1)) / 2);
    for (const m of pairs) {
      const fx = pairOf(m);
      if (fx === undefined) throw new Error('a pair market with no pair');
      expect(fx.base).not.toBe(fx.quote);
      // C1: the price is what one unit of the base costs in the quote, so the market is in the quote.
      expect(m.ccy).toBe(fx.quote);
      expect(m.instrument).toBe(fxPairId(fx.base, fx.quote));
      // A1: nobody holds a pair — the id names a price, not a thing.
      expect(w.instruments.has(m.instrument)).toBe(false);
      // F1: and it runs before the markets that need the money it produces.
      expect(m.order).toBe(0);
    }
  });

  it('prints a rate for every pair from period one, and the print says how it was made', () => {
    const w = rigWorld('ccy-C');
    w.step();
    for (const m of w.markets) {
      if (pairOf(m) === undefined) continue;
      const p = w.prices.latest(m.instrument, w.period);
      expect(p.some).toBe(true);
      if (!p.some) continue;
      expect(p.value.price).toBeGreaterThan(0);
      expect(p.value.ccy).toBe(m.ccy);
      // Clearing E1: a print is either this session's trade or the last real one, carried and
      // visibly stale. It is never invented and never silent about which it is.
      expect(['traded', 'stale', 'opening']).toContain(p.value.provenance.kind);
    }
  });
});

describe('the rate in force (Currency C5, D1)', () => {
  it('is one number: what a payment settles at and what a book is valued at are the same read', () => {
    const w = rigWorld('ccy-D');
    w.step();
    for (const ccy of FOREIGN) {
      const rate = w.valuation.rateInForce(ccy, USD, w.period);
      expect(rate).toBeGreaterThan(0);
      // C5: and the other way round is the same rate, not a second one.
      const back = w.valuation.rateInForce(USD, ccy, w.period);
      expect(rate * back).toBeCloseTo(1, 12);
      // A money against itself is one, with no market and no print involved.
      expect(w.valuation.rateInForce(ccy, ccy, w.period)).toBe(1);
    }
  });
});

describe('a foreign balance is findable, and it is at that money’s own issuer (Money A1, Currency D2)', () => {
  it('holds a party’s foreign money at that currency’s central bank, not at its own bank', () => {
    const w = rigWorld('ccy-E');
    w.step();
    const bank = w.parties.ofKind(partyId('bank') as never)[0];
    expect(bank).toBeDefined();
    if (bank === undefined) return;
    for (const ccy of FOREIGN) {
      const at = w.accountOf(bank.id, ccy);
      // D2: a claim in a money its own bank does not issue is a claim on that money's issuer.
      expect(at.issuer).toBe(w.registry.centralBankOf(ccy));
      expect(at.holder).toBe(bank.id);
      // And its own money is at its own bank, which for a bank is itself (Money B1).
      expect(w.accountOf(bank.id, USD).issuer).toBe(bank.bank);
      // `cash` reads through the same rule, or every foreign balance in the world is invisible.
      expect(w.cash(bank.id, ccy)).toBe(
        w.register.quantity(bank.id, moneyInstrumentId(at.issuer, ccy)),
      );
    }
  });
});

describe('a rate move lands on somebody (Currency D2, D4)', () => {
  it('books every revaluation to a holder, and what was booked is what the rate did', () => {
    const w = rigWorld('ccy-F');
    for (let i = 0; i < 6; i += 1) w.step();
    const booked = new Map<string, number>();
    const implied = new Map<string, number>();
    for (const e of w.journal.ofKind('revaluation.fx')) {
      const ccy = e.data['ccy'];
      const delta = e.data['deltaPerMember'];
      const carried = e.data['carried'];
      const was = e.data['was'];
      const now = e.data['now'];
      if (typeof ccy !== 'string') continue;
      if (typeof delta !== 'number' || typeof carried !== 'number') continue;
      if (typeof was !== 'number' || typeof now !== 'number') continue;
      // D2: a revaluation names its holder. A gain with nobody holding it is a residual (Law 2).
      expect(e.subjects[0]).toBeTruthy();
      booked.set(ccy, (booked.get(ccy) ?? 0) + delta);
      implied.set(ccy, (implied.get(ccy) ?? 0) + carried * (now - was));
    }
    // D4: reached from the events rather than from the register, which is the audit's own check.
    for (const [ccy, total] of booked) {
      expect(total).toBeCloseTo(implied.get(ccy) ?? 0, 6);
    }
    // And the family that says so is built and green.
    const family = w.last?.audit.families.find((f) => f.family === 'money');
    expect(family?.contributions).toContain('currency');
    expect(family?.count).toBe(0);
  });
});

describe('a central bank lends to its own system (Currency D4, Central Bank D1)', () => {
  it('refuses a bank that books abroad, so a foreign money must be bought and not overdrawn', () => {
    const w = rigWorld('ccy-G');
    for (let i = 0; i < 6; i += 1) w.step();
    const home = (party: string): CurrencyCode =>
      w.registry.region(w.parties.get(partyId(party)).region).ccy;
    for (const e of w.journal.ofKind('centralBank.refused')) {
      if (e.data['foreign'] !== true) continue;
      const bank = e.data['bank'];
      const ccy = e.data['ccy'];
      if (typeof bank !== 'string' || typeof ccy !== 'string') continue;
      // The refusal is exactly and only for a money the bank does not book in.
      expect(home(bank)).not.toBe(ccy);
    }
    // Money B3.c: and nothing is left below zero at an issuer that never lent it.
    const family = w.last?.audit.families.find((f) => f.family === 'money');
    expect(family?.count).toBe(0);
  });
});
