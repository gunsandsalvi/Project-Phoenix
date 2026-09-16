/**
 * The corporate bond (13f): a firm borrows from the market, and promises something for it.
 *
 * @spec Corporate Credit A1 Corporate Credit B2 Corporate Credit B2.a Corporate Credit B3 Corporate Credit G2 Bond N4 Bond N6 Bond N13 Bond N13.a Reporting A2 Law 2 Law 4 Law 6 Law 9
 */
import { USD } from '../src/seeds/foundation.js';
import { asCash, asRatio } from '../src/core/measure.js';
import { describe, expect, it } from 'vitest';
import {
  annualCostOf,
  headroomOn,
  partyId,
  instrumentId,
  isCorporateBond,
  period,
  CORPORATE_BOND,
  corporateBondModule,
  type Event,
  type World,
} from '../src/index.js';
import { ranWorld, rigWorld } from './rig.js';
import { ANNUAL, rate } from '../src/core/rate.js';
import { asPerPiece } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { syndicate } from '../src/mechanisms/corporate-bond/arranger.js';
import {
  couponAt,
  floatsRatherThanFixes,
  LEVERAGED_LOAN,
  leveragedLoanTerms,
} from '../src/mechanisms/corporate-bond/floating.js';

describe('what a corporate bond IS, beside a sovereign one', () => {
  it('can default, cross-defaults, and says where it ranks (N12, N13.a, G2)', () => {
    const w = rigWorld('bond-a');
    const k = w.registry.instrumentKind(CORPORATE_BOND);
    // G2: a firm that misses one line has missed them all — its lenders do not wait their turn
    // while the estate empties. A sovereign has no cross-default and says so (Sovereign G3).
    expect(k.accelerates).toBe(true);
    expect(k.defaultOn).toBeDefined();
    expect(k.liabilityOfIssuer).toBe(true);
    // Register B3, XI-3: its issuer owes the face, so its paper falling is never the firm's gain.
    expect(k.owes).toBe('face');
  });

  it('is priced by a book and never off a spread or a rating (Law 3)', () => {
    const w = rigWorld('bond-a');
    expect(w.registry.instrumentKind(CORPORATE_BOND).pricing).toBe('cleared');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      if (!id.includes('corporate')) continue;
      expect(id).not.toContain('spread');
      expect(id).not.toContain('covenant');
      expect(id).not.toContain('leverage');
    }
  });

  it('declares no covenant, no spread and no leverage (Law 2)', () => {
    // B2: what a given firm promised is an outcome of what it had to promise to be lent to. A
    // world with a `corporate.covenant.leverage` in it would have every issuer promising the same
    // thing, which is one issuer with many names. So: no covenant number, no spread, no leverage
    // line. What IS declared is how long a firm's paper runs for — a TECHNOLOGY of the market, in
    // the unit the calendar takes (Law 8) — and, since 17.1, the margin a floating line promises
    // over its reference, which is a PLACEHOLDER with a scheduled death: 17.2 builds the book that
    // strikes a margin, and a struck margin is a cleared level rather than a number anybody typed.
    const declared = corporateBondModule().params;
    expect(declared.map((d) => String(d.id))).toEqual(['corporateBond.tenor']);
    expect(declared[0]?.kind).toBe('technology');
    expect(declared[0]?.dimension).toBe('months');
  });
});

describe('the covenant is tested on what was PUBLISHED (B2.a, Reporting A2)', () => {
  it('reads the issuer’s own accounts and never a second set computed here (Law 4)', () => {
    const w = rigWorld('bond-a');
    for (let i = 0; i < 6; i += 1) w.step();
    for (const e of w.journal.ofKind('covenant.breached')) {
      // A breach names the quarter it was found in, because it is a fact about a report.
      expect(typeof e.data['quarter']).toBe('string');
      expect(typeof e.data['issuer']).toBe('string');
      expect(String(e.data['broke'])).toMatch(/leverage|coverage/);
    }
  });

  it('breaches when the accounts are the wrong side of the promise, and not before', () => {
    const c = { leverage: asRatio(0.6, 'what it promised'), coverage: asRatio(2, 'what it promised') };
    // Headroom is what is left of the promise. Negative is a breach, and it is arithmetic on two
    // published numbers rather than a threshold anybody tuned.
    expect(headroomOn({ assets: asCash(100, USD, 'what it holds'), liabilities: asCash(50, USD, 'what it owes') }, c)).toBeCloseTo(0.1, 12);
    expect(headroomOn({ assets: asCash(100, USD, 'what it holds'), liabilities: asCash(70, USD, 'what it owes') }, c)).toBeLessThan(0);
    // A firm with no assets has no ratio that means anything and HAS breached — which is what the
    // worst case is, rather than a number pushed back inside a range (Law 6).
    expect(headroomOn({ assets: asCash(0, USD, 'what it holds'), liabilities: asCash(1, USD, 'what it owes') }, c)).toBeLessThan(0);
  });

  it('never repairs or accelerates by itself: a breach is an event and that is all', () => {
    const m = corporateBondModule();
    // N5.b (17.1): and a floating line FIXES before anything it pays or is priced at — which is a
    // third phase and still not a repair: what a fixing does is set what the line owes next.
    expect(m.phases.map((f) => f.name)).toEqual([
      'loan.fix',
      'bond.issue',
      'bond.takeUp',
      'covenant.test',
    ]);
    // What happens after a breach is the holders' decision, and a decision is not a rule.
    expect(m.families.every((f) => f.built)).toBe(true);
  });
});

describe('the schedule is the kernel’s, because it is not a corporate fact (Law 4)', () => {
  it('costs its issuer its coupon a year per unit of face, read off its own terms', () => {
    const t = {
      coupon: { amount: 0.05, per: { kind: 'annual' as const } },
    } as Parameters<typeof annualCostOf>[0];
    expect(annualCostOf(t)).toBeCloseTo(0.05, 12);
  });
});

describe('a firm issues because a market was cheaper than its bank (A1, B1, E5.d)', () => {
  /** Every issue this world brought, oldest first. A read of what was announced (Observer A3). */
  const offers = (w: World): readonly Event[] => w.journal.ofKind('bond.offered');

  it('brings paper at all, which is the whole of what item 10 was for', () => {
    const w = ranWorld('bond-issue', 10);
    // B1: it needed the banks to have quoted and the holders to have published what they require,
    // and both are published in `lending.write` — so nothing can come before the first of those.
    expect(offers(w).length).toBeGreaterThan(0);
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND) continue;
      // Law 9: named as a market names it — the issuer and the maturity, never an internal id.
      expect(String(i.id)).toMatch(/^bond:firm\.\d+:/);
      expect(i.market.some).toBe(true);
    }
  });

  it('says which of B1’s two reasons brought it, with both prices on the record', () => {
    const w = ranWorld('bond-issue', 10);
    for (const e of offers(w)) {
      const quoted = e.data['quoted'];
      const required = e.data['requiredByHolders'];
      const lendable = e.data['lendable'];
      expect(typeof required).toBe('number');
      // Either the market was cheaper than the bank, or the bank would not lend it enough, or
      // nobody quoted it at all. A third reason would be a reason nothing published.
      if (typeof quoted === 'number' && typeof lendable === 'number') {
        const short = e.data['short'];
        expect(
          (required as number) < quoted || (typeof short === 'number' && lendable < short),
        ).toBe(true);
      } else {
        expect(quoted).toBeNull();
      }
    }
  });

  it('never prices its own issue: the reservation is its alternative, and the price is the book’s', () => {
    const w = ranWorld('bond-issue', 10);
    for (const e of offers(w)) {
      const line = String(e.data['line']);
      const reservation = e.data['reservation'];
      expect(typeof reservation).toBe('number');
      expect(reservation as number).toBeGreaterThan(0);
      // C4, Law 3: what it brought is a size and a walk-away. Whatever it got for the paper is a
      // PRINT, and a print is the auction's, so nothing here may equal the reservation by
      // construction — it may only be at or above it, because below it the paper is withdrawn.
      for (const p of w.prices.history(instrumentId(line))) {
        expect(p.price).toBeGreaterThanOrEqual(reservation as number);
      }
    }
  });

  it('taps the line it has rather than minting a second one at the same date (C8, Law 4)', () => {
    const w = ranWorld('bond-issue', 12);
    const seen = new Set<string>();
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND) continue;
      expect(seen.has(String(i.id))).toBe(false);
      seen.add(String(i.id));
    }
    // And a second offer on a line that already exists is a TAP: same instrument, more face.
    const byLine = new Map<string, number>();
    for (const e of offers(w)) {
      const line = String(e.data['line']);
      byLine.set(line, (byLine.get(line) ?? 0) + 1);
    }
    for (const [line] of byLine) expect(seen.has(line)).toBe(true);
  });

  it('promises its own published accounts as this borrowing leaves them (B2, Reporting A2)', () => {
    const w = ranWorld('bond-issue', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== CORPORATE_BOND || !isCorporateBond(i.terms)) continue;
      // B2: both lines are real promises, and neither is a parameter anybody declared.
      expect(i.terms.covenants.leverage).toBeGreaterThan(0);
      expect(i.terms.covenants.coverage).toBeGreaterThan(0);
      // N13.a: senior unsecured, which is what a firm's first market borrowing is.
      expect(i.terms.seniority).toBe(1);
      // B2.a: a promise the issuer could actually make — its own published accounts, so the firm
      // that has not deteriorated since is inside it and only the one that has is not.
      const said = w.published.lastStatement(i.terms.issuer);
      if (said === undefined || said.balance.assets.pieces <= 0) continue;
      expect(i.terms.covenants.leverage).toBeGreaterThanOrEqual(
        said.balance.liabilities.pieces / said.balance.assets.pieces,
      );
    }
  });

  it('funds one shortfall through one channel, never both (Law 4, Law 5)', () => {
    const w = ranWorld('bond-issue', 12);
    for (const e of w.journal.ofKind('bond.offered')) {
      const issuer = e.subjects[0];
      if (issuer === undefined) continue;
      // Corporate Credit A1, C7: a bank writing a loan against the same published number would
      // fund the same hole twice, and the second one is a residual with no holder.
      const wrote = w.journal
        .ofKindIn('credit.written', period(e.period + 1))
        .filter((c) => c.subjects.includes(issuer));
      expect(wrote).toEqual([]);
    }
  });

  it('has something to test, which is why the covenant table exists (10.3)', () => {
    const w = ranWorld('bond-issue', 12);
    const live = w.instruments.all().filter((i) => i.kind === CORPORATE_BOND && i.status.live);
    expect(live.length).toBeGreaterThan(0);
    // 10.4, G2: and every one of them cross-defaults, which a sovereign line does not (Sovereign G3).
    expect(w.registry.instrumentKind(CORPORATE_BOND).accelerates).toBe(true);
  });
});

/**
 * B4, N5.b (17.1): the leveraged loan — a corporate line whose coupon is a margin over a rate the
 * money market actually cleared, rather than one locked at issuance.
 */
describe('fixed or floating is a decision (Corporate Credit B4, Bond N5.b, N6)', () => {
  it('pays the reference plus what the issuer promised over it, and nothing else', () => {
    // N5.b: the coupon in force IS the fixing plus the margin. Two numbers, added once.
    const fixing = asRatio(0.04, 'what the overnight book cleared at');
    const margin = rate(asRatio(0.03, 'what it promised over it'), ANNUAL);
    expect(couponAt(fixing, margin).amount).toBeCloseTo(0.07, 12);
    expect(couponAt(fixing, margin).per.kind).toBe('annual');
  });

  it('floats when floating is cheaper on its OWN view of the rate, and fixes when it is not (A2.c)', () => {
    const fixed = asRatio(0.09, 'the coupon it would have to lock');
    const fixing = asRatio(0.04, 'what the rate is now');
    const margin = asRatio(0.03, 'what it would promise over it');
    // Four plus three against nine: floating, on what it knows now.
    expect(floatsRatherThanFixes(fixed, fixing, margin, undefined)).toBe(true);
    // A firm that expects the rate at seven does not float at three over it, on the same day and
    // the same market — which is A2.c: the structure is an outcome of a decision meeting a price.
    expect(
      floatsRatherThanFixes(fixed, fixing, margin, asRatio(0.07, 'where it thinks the rate goes')),
    ).toBe(false);
  });

  it('brings floating lines named by their MARGIN, and fixes them from the published benchmark', () => {
    const w = ranWorld('bond-issue', 20);
    const loans = w.instruments.all().filter((i) => i.kind === LEVERAGED_LOAN);
    // A world whose overnight book never cleared has no benchmark and can bring none of these,
    // which is the honest state and not a failure — so what is asserted is what a brought one IS.
    for (const i of loans) {
      // Law 9: the issuer and the maturity, and the display name carries the margin rather than
      // the coupon — the margin is what it promised; the coupon is what that comes to this quarter.
      expect(String(i.id)).toMatch(/^loan:firm\.\d+:/);
      const t = leveragedLoanTerms(i);
      expect(t.benchmark.endsWith(':secured')).toBe(true);
      expect(t.margin.amount).toBeGreaterThan(0);
      expect(t.coupon.amount).toBeGreaterThan(t.margin.amount);
    }
    // N5.b, Indices A1: every fixing on the record is a rate somebody PUBLISHED plus that line's
    // own margin — read, never re-derived.
    for (const e of w.journal.ofKind('coupon.fixed')) {
      const line = w.instruments.get(instrumentId(String(e.data['instrument'])));
      const t = leveragedLoanTerms(line);
      const named = String(e.data['benchmark']);
      expect(named).toBe(t.benchmark);
      const published = w.journal
        .ofKind('index.benchmark')
        .filter((b) => b.subjects.includes(named) && b.period <= e.period)
        .at(-1);
      expect(published, `${named} fixed with nothing published`).toBeDefined();
      expect(Number(e.data['coupon'])).toBeCloseTo(
        Number(published!.data['rate']) + t.margin.amount,
        12,
      );
    }
  });

  it('says which of the two kinds resets, so the kernel can refuse the other (N5.a against N5.b)', () => {
    const w = ranWorld('bond-issue', 20);
    // The kernel's fixing door asks the KIND and not the caller (`ctx.fixCoupon` → `floats`): a
    // fixed line has no reference to fix on, and a module that thought it did would be quietly
    // rewriting what an issuer promised. The declaration is what makes that refusal possible.
    expect(w.registry.instrumentKind(LEVERAGED_LOAN).floats).toBe(true);
    expect(w.registry.instrumentKind(CORPORATE_BOND).floats).not.toBe(true);
  });
});

/**
 * C1, C6, C7, C10, C11 (17.2): a deal is BROUGHT by somebody, who is paid, and who is left holding
 * what the book did not take — or is not, which is the other basis and a different price.
 */
describe('the arranger (Corporate Credit C1, C6, C7, C10, C11)', () => {
  const brought = (w: World): readonly Event[] => w.journal.ofKind('bond.offered');

  it('names who brought it, on which basis, and who agreed to carry what (C1, C10.a, C11)', () => {
    const w = ranWorld('bond-issue', 20);
    for (const e of brought(w)) {
      const mandate = e.data['mandate'] as {
        lead: string;
        basis: string;
        size: number;
        members: { bank: string; commits: number; perUnit: number }[];
      };
      expect(mandate, 'an issue nobody brought').toBeDefined();
      // C1: a named party, alive, and not the issuer itself.
      expect(w.parties.has(partyId(mandate.lead))).toBe(true);
      expect(mandate.lead).not.toBe(String(e.data['issuer']));
      expect(['bestEffort', 'backstopped']).toContain(mandate.basis);
      expect(mandate.members.length).toBeGreaterThan(0);
      // C10.a: the shares are struck BEFORE the book opens and they come to the deal's size.
      const committed = mandate.members.reduce((t, m) => t + m.commits, 0);
      expect(committed).toBe(mandate.size);
      // C10.c: a syndicate that could not carry the whole of it brings a smaller deal, and the
      // record says what the issuer wanted beside what it brought.
      expect(mandate.size).toBeLessThanOrEqual(Number(e.data['wanted']));
      for (const m of mandate.members) {
        expect(w.parties.has(partyId(m.bank))).toBe(true);
        expect(m.perUnit).toBeGreaterThan(0);
      }
    }
  });

  it('pays the fee out of the proceeds, to the banks that brought it and to nobody else (C6)', () => {
    const w = ranWorld('bond-issue', 20);
    for (const fee of w.journal.ofKind('bond.fee')) {
      const line = String(fee.data['line']);
      const offered = w.journal
        .ofKind('bond.offered')
        .filter((e) => String(e.data['line']) === line && e.period === fee.period)
        .at(-1);
      expect(offered, `a fee on ${line} that no deal brought`).toBeDefined();
      const mandate = offered!.data['mandate'] as { members: { bank: string }[] };
      expect(mandate.members.map((m) => m.bank)).toContain(String(fee.data['bank']));
      // C6: it is money, it moved, and it moved from the issuer to the bank — one instruction with
      // both legs, so the issuer is left with the proceeds NET of it.
      expect(Number(fee.data['units'])).toBeGreaterThan(0);
    }
  });

  it('leaves the underwriter holding what the book did not take, and the agent holding nothing (C7.a, C11.d)', () => {
    const w = ranWorld('bond-issue', 20);
    for (const took of w.journal.ofKind('bond.underwritten')) {
      const line = String(took.data['line']);
      const offered = w.journal
        .ofKind('bond.offered')
        .filter((e) => String(e.data['line']) === line && e.period === took.period)
        .at(-1);
      expect(offered).toBeDefined();
      const mandate = offered!.data['mandate'] as { basis: string; members: { bank: string }[] };
      // C11.d: NO BEST-EFFORT DEAL LEAVES THE AGENT HOLDING PAPER. An agent commits nothing, so
      // what the book did not take is simply not issued.
      expect(mandate.basis).toBe('backstopped');
      expect(mandate.members.map((m) => m.bank)).toContain(String(took.data['bank']));
      expect(Number(took.data['units'])).toBeGreaterThan(0);
      // C7.a: and it is the underwriter's own position now — units it holds, bought at the strike.
      const held = w.register.quantity(partyId(String(took.data['bank'])), instrumentId(line));
      expect(held).toBeGreaterThan(0);
    }
  });

  it('charges more for a backstop than for best effort, because of the risk behind it (C7.b, C11.e)', () => {
    // C11.e is a CONSEQUENCE and not a rule: the backstop fee is the work fee plus what carrying
    // the paper costs the bank — its own published credit view of the name over the placement — so
    // it exceeds the best-effort fee by exactly the risk, and by nothing else.
    const w = ranWorld('bond-issue', 20);
    const byBasis = new Map<string, number[]>();
    for (const e of brought(w)) {
      const mandate = e.data['mandate'] as {
        basis: string;
        members: { perUnit: number }[];
      };
      const reservation = Number(e.data['reservation']);
      if (reservation <= 0) continue;
      for (const m of mandate.members) {
        const share = byBasis.get(mandate.basis) ?? [];
        // Per unit of par, so two deals at different prices are comparable.
        share.push(m.perUnit / reservation);
        byBasis.set(mandate.basis, share);
      }
    }
    const best = byBasis.get('bestEffort');
    const back = byBasis.get('backstopped');
    if (best !== undefined && back !== undefined && best.length > 0 && back.length > 0) {
      const mean = (xs: number[]): number => xs.reduce((t, x) => t + x, 0) / xs.length;
      expect(mean(back)).toBeGreaterThan(mean(best));
    }
  });

  it('never gives a member more than its own limit, and downsizes rather than exceed one (C10.b, C10.c)', () => {
    const willing = (bank: string, fee: number, room: number) => ({
      bank: partyId(bank),
      feePerUnit: asPerPiece(fee, 'what it charges a unit'),
      canCommit: asQty(room),
    });
    // Three banks with 40, 30 and 20 units of room, keenest first, against a deal of 100.
    const banks = [willing('bank.1', 1, 40), willing('bank.2', 2, 30), willing('bank.3', 3, 20)];
    const backed = syndicate(banks, asQty(100), 'backstopped');
    // C10.b: each takes its own room and not a unit more — the lead cannot lend a member capacity
    // it does not have, and a member cannot be assigned what it did not agree to carry.
    expect(backed.map((m) => m.commits)).toEqual([40, 30, 20]);
    // C10.c: ninety is what the willing members can carry, so ninety is what the deal is brought
    // at. The largest deal this market can bring is the sum of the willing limits, and a deal
    // bigger than that is an observable downsizing rather than one silently carried past a limit.
    expect(backed.reduce((t, m) => t + m.commits, 0)).toBe(90);
    // A deal inside the first member's room needs no syndicate at all.
    expect(syndicate(banks, asQty(30), 'backstopped').map((m) => String(m.bank))).toEqual([
      'bank.1',
    ]);
    // C11.a: on best effort there is no risk to share, so the agent brings the whole of it alone
    // and commits nothing — the commitment here is the size it agreed to place, not to take.
    const agent = syndicate(banks, asQty(100), 'bestEffort');
    expect(agent).toHaveLength(1);
    expect(agent[0]?.commits).toBe(100);
  });

  it('declares no fee of its own: what a bank charges is the BANK’s number (Law 2, Law 4)', () => {
    const declared = corporateBondModule().params.map((d) => String(d.id));
    // 17.2: `corporateBond.margin` is dead — a floating line's margin is what the keenest holder
    // requires of the name less what the reference is fixing at, both published. What an arranger
    // charges is a preference of the arranger, declared by the module that owns banks.
    expect(declared).toEqual(['corporateBond.tenor']);
    expect(declared.some((id) => id.includes('margin') || id.includes('fee'))).toBe(false);
  });
});
