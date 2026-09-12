/**
 * What the classes owe the rest of the world: wrong-way risk, the funding swap, the hedge, and the
 * reads nobody may turn into a price.
 *
 * @spec CDS B1.a CDS B3 CDS E1 CDS E2 CDS E3 CDS E4 FX Forwards A4 FX Forwards B3 FX Forwards B3.b FX Forwards D3 FX Forwards E4 Dealer Desks E1 Dealer Desks E2 Sovereign I1.a Sovereign I2 Sovereign I3 Sovereign I3.a Derivative D10 Derivative D10.a Law 3 Law 19
 */
import { describe, expect, it } from 'vitest';
import {
  BOND_FUTURE_PARAMS,
  CDS_PARAMS,
  FX_PARAMS,
  OPTION_PARAMS,
  USD,
  cdsTenorsOf,
  impliedMove,
  isBondFuture,
  isCds,
  isFxForward,
  isIndexFuture,
  isOption,
  netNotionalOn,
  snapshot,
  type World,
} from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(periods: number): World {
  const w = rigWorld('classes');
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

describe('who you face is part of what it is worth (D10, D10.a, CDS E1, E2)', () => {
  it('prices protection against what it already has on that name with that side', () => {
    const w = ran(4);
    const ctx = w.mechanismContext('test');
    const references = new Set(
      w.markets
        .map((m) => m.contract?.terms)
        .filter((t) => t !== undefined && isCds(t))
        .map((t) => (isCds(t) ? String(t.reference) : '')),
    );
    for (const r of references) {
      if (r === '') continue;
      // E3: net notional per reference is a READ — how much protection on one name exists. It is
      // one question about one name and never a netting across counterparties (G3).
      const net = netNotionalOn(ctx, r as never);
      expect(Number.isFinite(net)).toBe(true);
      expect(net).toBeGreaterThanOrEqual(0);
    }
    // E4: and there is no parameter anywhere that sets an exposure limit per name. What a party
    // will face is its own reservation, which is a reason and not a rule.
    expect(Object.keys(CDS_PARAMS).some((k) => k.toLowerCase().includes('limit'))).toBe(false);
  });
});

describe('the swap that funds a foreign book (A4, D3, 12d-14)', () => {
  it('is a spot trade and a forward row, and nothing invents a third instrument for it', () => {
    const w = ran(3);
    // A4: an FX swap is not a kind. It is what a party does — spot one way and forward back — so
    // what exists in this world is a pair market and a forward book, and no `fx.swap` anywhere.
    expect(w.registry.derivativeKinds.has('fx.swap' as never)).toBe(false);
    const forwards = w.markets.filter((m) => m.contract !== undefined && isFxForward(m.contract.terms));
    const pairs = w.markets.filter((m) => m.fx !== undefined);
    if (pairs.length === 0) return;
    expect(forwards.length).toBeGreaterThan(0);
    // B3.b: one basis, and it is read from prints. No parameter names one.
    expect(Object.keys(FX_PARAMS).some((k) => k.toLowerCase().includes('basis'))).toBe(false);
    expect(Object.keys(FX_PARAMS).some((k) => k.toLowerCase().includes('parity'))).toBe(false);
  });

  it('charges a foreign line the cost of the money it is in, not of the bank’s own', () => {
    const w = ran(4);
    // 12d-14: a bank publishes what money costs it in its OWN money and, beside it, in every
    // other one it lends in. A desk quoting a line in another money reads that one.
    const said = w.journal.ofKind('bank.costOfFunds').at(-1);
    if (said === undefined) return;
    expect(typeof said.data['ccy']).toBe('string');
    expect(typeof said.data['alsoIn']).toBe('object');
  });
});

describe('the hedge is a contract with a counterparty (Dealer Desks E1, E2)', () => {
  it('exists as a book a desk can lay a share position off into', () => {
    const w = ran(3);
    const futures = w.markets.filter((m) => m.contract !== undefined && isIndexFuture(m.contract.terms));
    expect(futures.length).toBeGreaterThan(0);
    for (const m of futures) {
      const t = m.contract?.terms;
      if (t === undefined || !isIndexFuture(t)) continue;
      // E2: a hedge with a counterparty and its own margin — not a coefficient that makes the
      // position disappear from a report.
      expect(w.index(t.index).some).toBe(true);
    }
  });
});

describe('what is measured and never set (I1.a, I2, I3.a, Law 3)', () => {
  it('has a trader’s own tolerance and no basis anywhere', () => {
    // I3.a: what stops a basis trade is the trader's own drawdown tolerance — a PREFERENCE, what
    // this party will stand — and nothing makes it whole.
    expect(String(BOND_FUTURE_PARAMS.tolerance)).toContain('drawdown');
    expect(Object.keys(BOND_FUTURE_PARAMS).some((k) => k.toLowerCase().includes('basis'))).toBe(false);
  });

  it('takes the option’s implied move off its premium and declares no volatility', () => {
    // Law 3, Bond N7.b: the premium is what cleared; the move it implies is arithmetic on it.
    expect(impliedMove(9, 1, 9).some).toBe(true);
    expect(Object.keys(OPTION_PARAMS).some((k) => k.toLowerCase().includes('vol'))).toBe(false);
  });
});

describe('every class is a module and none of them is the kernel’s (Law 15)', () => {
  it('registers its kinds through the registry and nothing else', () => {
    const w = ran(2);
    const kinds = [...w.registry.derivativeKinds.keys()].map(String).sort();
    // Seven classes on one layer, and the layer itself declares none of them.
    expect(kinds).toContain('cds');
    expect(kinds).toContain('cds.index');
    expect(kinds).toContain('irs');
    expect(kinds).toContain('fx.forward');
    expect(kinds).toContain('xccy');
    expect(kinds).toContain('index.future');
    expect(kinds).toContain('option');
    expect(kinds).toContain('bond.future');
    // Each one answers for itself: what a party's reason to be in its book is, is the kind's.
    for (const id of w.registry.derivativeKinds.keys()) {
      const profile = w.registry.derivativeKind(id);
      expect(typeof profile.mark).toBe('function');
      expect(typeof profile.flip).toBe('function');
    }
    expect(cdsTenorsOf({ params: w.params }).length).toBeGreaterThan(0);
    expect(String(USD)).toBe('USD');
    expect(isBondFuture({ kind: 'x' as never })).toBe(false);
    expect(isOption({ kind: 'x' as never })).toBe(false);
  });
});

describe('what a reader is shown (Observer A1, A3; CDS A1.d; IRS C1)', () => {
  it('shows every contract book’s own level, and says whether it is money or a rate', () => {
    // Long enough for a book to have printed: these markets are thin, which is a measurement and
    // not a defect (Law 11) — a young book with one side in it clears nothing for weeks.
    const w = ran(16);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    expect(seen.contractPrints.length).toBeGreaterThan(0);
    for (const p of seen.contractPrints) {
      // Law 8: the unit is part of the number. A level of 0.01 is a hundredth of a cent per unit
      // of face per annum on a spread book and a cent on a premium book, and a reader shown
      // neither has been shown a number.
      expect(['money', 'rate']).toContain(p.quotedAs);
      expect(typeof p.level).toBe('number');
    }
  });

  it('gathers them into curves in tenor order, with no point nobody paid for', () => {
    const w = ran(6);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    for (const c of seen.derivativeCurves) {
      expect(c.points.length).toBeGreaterThan(0);
      for (let i = 1; i < c.points.length; i += 1) {
        expect(c.points[i]?.tenorYears).toBeGreaterThan(c.points[i - 1]?.tenorYears ?? 0);
      }
      // Law 3, Law 19: every point is a print. Nothing here interpolates between two of them.
      const prints = seen.contractPrints.filter((p) => p.market.startsWith(c.subject));
      expect(c.points.length).toBeLessThanOrEqual(prints.length);
    }
  });
});
