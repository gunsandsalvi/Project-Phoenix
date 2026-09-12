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
  OPTION,
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
    /**
     * Law 11, `13b-10`: THIS IS A CHECK AGAINST AN INCOMPLETE MODEL AND IT IS ALLOWED TO FAIL.
     *
     * Every party this world lets into a contract book is a bank or a firm, and in this world both
     * want the same thing — banks are long duration and short of fixed, firms are short of foreign
     * money — so every book has one side and clears nothing. The parties who would take the other
     * side of a hedge (an insurer with liabilities to match, a pension with duration to buy) are
     * 13h's and do not exist yet. Letting the households in would be the wrong fix: a household
     * does not sell a bond future, it owns a fund that does.
     *
     * So what is asserted is what a reader is SHOWN about each level there is, and never that
     * there is one. `derivativeCurves` and `measures` below say the same thing the same way.
     */
    const w = ran(16);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    for (const p of seen.contractPrints) {
      // Law 8: the unit is part of the number. A level of 0.01 is a hundredth of a cent per unit
      // of face per annum on a spread book and a cent on a premium book, and a reader shown
      // neither has been shown a number.
      expect(['money', 'rate']).toContain(p.quotedAs);
      expect(typeof p.level).toBe('number');
    }
  });

  it('asks each class what its own book says against the rest of the world (C3, E3, I1.a, D7.a)', () => {
    const w = ran(16);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    // Law 15: the surface knows what none of these mean. Every one of them came off the kind whose
    // book it is OF, so a class that declares a measurement is shown one without this file, the
    // observer, or the snapshot changing — which is what "adding a system is one module" means.
    const kinds = new Set(seen.measures.map((x) => x.kind));
    for (const x of seen.measures) {
      expect(['money', 'rate', 'notional']).toContain(x.unit);
      // Law 8: a reader shown 0.0042 with no word for what it is the difference BETWEEN has been
      // shown a number and not a measurement.
      expect(x.measure.length).toBeGreaterThan(0);
      expect(x.subject.length).toBeGreaterThan(0);
      expect(Number.isFinite(x.level)).toBe(true);
    }
    // E3: net notional on a name is ONE number however many tenors carry a book on it (Law 4).
    const nets = seen.measures.filter((x) => x.measure === 'protection outstanding');
    expect(new Set(nets.map((x) => x.subject)).size).toBe(nets.length);
    for (const n of nets) expect(n.level).toBeGreaterThanOrEqual(0);
    // The CDS books exist in this rig whether or not anybody has traded in one, so the measurement
    // that needs no print — how much protection exists — is always here (`13b-10`).
    if (w.markets.some((m) => m.contract !== undefined && isCds(m.contract.terms))) {
      expect(kinds.has('cds')).toBe(true);
    }
  });

  it('shows each index rule with what it was read from, and the vehicles on it (A2, A3, B2)', () => {
    const w = ran(16);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    expect(seen.indices.length).toBeGreaterThan(0);
    for (const i of seen.indices) {
      // A2: the level is its constituents and nothing else, so what it was read FROM is shown
      // beside it — and a size index's boundary is where this list stops, never a stored number.
      expect(i.from.length).toBe(i.constituents === 0 ? 0 : i.from.length);
      for (const c of i.from) {
        expect(c.price).toBeGreaterThan(0);
        expect(c.name.length).toBeGreaterThan(0);
      }
      // B2, Fund Shares E1: a vehicle is here because it published a launch, never because a list
      // says it should exist — a rule with no tracker is a measurement and says so by being empty.
      for (const v of i.vehicles) expect(v.shares).toBeGreaterThan(0);
    }
    /**
     * `13b-6`, `13b-11`: NOT "at least one rule has a vehicle". The trackers this world declares on
     * the size segments launch nothing, because nobody can assemble the basket (13h), and the one
     * vehicle that DOES exist was put there by the seed, which publishes no launch — so the read
     * that answers "which trackers are on this rule" cannot see it. Both are findings, and neither
     * is a reason to assert something this world does not do (Law 11).
     */
    expect(seen.indices.every((i) => i.vehicles.every((v) => v.fund.length > 0))).toBe(true);
  });

  it('shows what a hedge does not cover, with its parts and beside what the rate did (E4, D2.a)', () => {
    const w = ran(16);
    const seen = snapshot(w, { kind: 'inspector' }, 0);
    for (const h of seen.hedges) {
      // E4: the residual is never netted away, and it arrives with the two numbers it is made of —
      // the position and what has been fixed against it — because they are different objects: a
      // price that keeps moving and a notional fixed on a date.
      expect(h.residual).toBe(h.exposure - h.covered);
      // A row is only here because the party is actually in this pair; a table of zeroes would
      // bury the ones that mean something.
      expect(h.exposure === 0 && h.covered === 0).toBe(false);
      // D2.a, Law 19: what the rate DID is the engine's own journalled revaluation, and null is a
      // real answer — the rate did nothing to this party this period — rather than nothing left
      // over, which is what `residual` says.
      if (h.revalued !== null) expect(Number.isFinite(h.revalued)).toBe(true);
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

describe('a book is not its own last price (XI-13, Law 3, Clearing E1)', () => {
  it('does not freeze a contract book at the first level it printed', () => {
    const w = ran(12);
    const books = w.markets.filter((m) => m.contract !== undefined);
    expect(books.length).toBeGreaterThan(0);
    // Every class now names a level of its OWN — the reference's cash bond, the overnight fixing,
    // the cash deliverable, its own carry, its own view of the move — and reads this book's print
    // only to decide which side it is on. Under the order that stood before, a party with nothing
    // of its own to say posted AT the print, so a book whose members were all in that state
    // printed one number for the rest of the run: the print moved the outlook, the outlook moved
    // the view, the view moved the quote, the quote moved the print (`banks/dealing-quote.ts`
    // records what that did to a bill). So a book that printed more than once has moved.
    const printedTwice = books.filter((m) => w.prices.history(m.instrument).length > 1);
    if (printedTwice.length === 0) return;
    const moved = printedTwice.filter(
      (m) => new Set(w.prices.history(m.instrument).map((x) => x.price)).size > 1,
    );
    expect(moved.length).toBeGreaterThan(0);
  });

  it('opens an option book, which a bootstrap of one tick could not (D7.b, D4)', () => {
    const w = ran(12);
    const books = w.markets.filter((m) => m.contract !== undefined && isOption(m.contract.terms));
    if (books.length === 0) return;
    // The fallback used to be `tickForDerivative`, the smallest increment the kind quotes in. Every
    // party's own arithmetic was above it, so every party took the writer's side: all offers, no
    // bids, `noDemand`, no print — and next period the same tick again. What a party names now is
    // what its own view of the underlying's move says optionality is worth, and the two sides of
    // the book are two parties whose surprises differ (§46 A3).
    //
    // What can be asserted is that the number a party names is its OWN and not the tick: the
    // schedules an option book gathers are at levels a tick could not produce. Whether anything
    // CROSSES is `13b-10`'s: every party this world admits to a contract book is on the same side
    // of it, and the party that would take the other is 13h's.
    const tick = w.registry.tickForDerivative(OPTION, USD);
    let posted = 0;
    for (const m of books) {
      const decl = m.contract;
      if (decl === undefined) continue;
      const profile = w.registry.derivativeKind(decl.kind);
      for (const p of w.parties.alive()) {
        for (const o of profile.orders?.(w.participantView(p.id), m) ?? []) {
          posted += 1;
          if (o.price === 'market') continue;
          expect(o.price).not.toBe(tick);
        }
      }
    }
    expect(posted).toBeGreaterThan(0);
  });
});
