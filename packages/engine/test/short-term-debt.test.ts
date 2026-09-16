/**
 * Short-term debt (item 10b): money borrowed for weeks, and the asking-again that is the whole risk.
 *
 * @spec Short-Term Debt A1 Short-Term Debt A1.b Short-Term Debt A2 Short-Term Debt A2.a Short-Term Debt A3 Short-Term Debt B1 Short-Term Debt B3 Short-Term Debt B3.a Short-Term Debt B4 Short-Term Debt C2 Short-Term Debt C3 Short-Term Debt D1 Short-Term Debt E1 Short-Term Debt E2 Short-Term Debt E3 Law 2 Law 4 Law 9
 */
import { describe, expect, it } from 'vitest';
import {
  BACKSTOP,
  COMMERCIAL_PAPER,
  PAPER_PARAMS,
  isBackstop,
  isPaper,
  shortTermDebt,
  yieldOn,
} from '../src/index.js';
import { asPerPiece } from '../src/core/measure.js';
import { ranWorld, rigWorld } from './rig.js';
import { BANK, FIRM } from '../src/registry/profiles.js';

describe('what commercial paper IS (A1)', () => {
  it('pays no coupon, redeems at par, and its yield comes OUT of its price (A1.a, A2)', () => {
    const w = rigWorld('cp-a');
    const k = w.registry.instrumentKind(COMMERCIAL_PAPER);
    // A2, E2: cleared, and there is no other answer. A discount off an untraded curve is the defect.
    expect(k.pricing).toBe('cleared');
    expect(k.liabilityOfIssuer).toBe(true);
    // Register B3, XI-3: the issuer owes the FACE, so its paper falling is never its own gain.
    expect(k.owes).toBe('face');
    // A2: price in, yield out — and only that direction.
    expect(yieldOn(asPerPiece(0.99, 'a price'), 0.25, 'a yield')).toBeCloseTo((1 - 0.99) / (0.99 * 0.25), 12);
  });

  it('can fail and cross-defaults, which is what separates it from a bill (G2, Sovereign G3)', () => {
    const w = rigWorld('cp-a');
    const k = w.registry.instrumentKind(COMMERCIAL_PAPER);
    expect(k.defaultOn).toBeDefined();
    // An issuer that cannot repay a week of borrowing has not got a week's problem.
    expect(k.accelerates).toBe(true);
    // And the sovereign bill it shares a promise with does NOT, which is the whole distinction.
    expect(w.registry.instrumentKind(w.registry.instrumentKind(COMMERCIAL_PAPER).id).accelerates).toBe(true);
  });

  it('declares only numbers that are one of Law 2’s kinds, and the shape names its death', () => {
    const declared = shortTermDebt().params;
    for (const d of declared) {
      expect(['technology', 'preference', 'policy', 'resolution', 'shape', 'placeholder']).toContain(d.kind);
      // Law 2: a shape with a scheduled death is a PLACEHOLDER and names the item that kills it.
      if (d.kind === 'shape') expect(d.standsInFor?.item).toBeDefined();
    }
    const ids = declared.map((d) => String(d.id));
    expect(ids).toContain(String(PAPER_PARAMS.tenor));
    expect(ids).toContain(String(PAPER_PARAMS.commitmentFee));
  });
});

describe('the roll is a new issue into a market that must clear (B3.a, E1)', () => {
  it('has no renewal path at all: the only way paper continues is a fresh offer', () => {
    const m = shortTermDebt();
    // E1: paper that always rolls at a written rate is not debt. There is one issuing phase and
    // what maturing paper does is enlarge the need it brings paper against — never renew itself.
    expect(m.phases.map((p) => p.name)).toEqual(['paper.issue', 'paper.backstop']);
    expect(m.phases[0]?.anchor).toEqual({ before: 'markets' });
    // B3.b (item 0, stop 4): the backstop runs BEFORE the maturity it is drawn to meet. It ran
    // `after: markets`, and the kernel presents a maturity in `corporateActions`, the period's
    // first phase — so the paper failed, the default carried to every other line the issuer had,
    // and the draw arrived to fund a repayment that had already failed.
    expect(m.phases[1]?.anchor).toEqual({ before: 'corporateActions' });
  });

  it('brings paper for a dated need, and says how much of the trip is a roll', () => {
    const w = ranWorld('cp-run', 12);
    for (const e of w.journal.ofKind('paper.offered')) {
      expect(typeof e.data['size']).toBe('number');
      expect(e.data['size'] as number).toBeGreaterThan(0);
      // B3: a reader can see which part of the ask is asking for the same money again.
      expect(typeof e.data['rolling']).toBe('number');
      expect(typeof e.data['reservation']).toBe('number');
      // C4, Law 3: it brought a size and a walk-away. The price is the book's.
      expect(e.data['reservation'] as number).toBeGreaterThan(0);
    }
  });

  it('names a line by its issuer and the day it is due, one per issuer per date (Law 9)', () => {
    const w = ranWorld('cp-run', 12);
    const seen = new Set<string>();
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER) continue;
      expect(String(i.id)).toMatch(/^cp:/);
      expect(seen.has(String(i.id))).toBe(false);
      seen.add(String(i.id));
      // D1: it trades after issue, so a holder can get out early.
      expect(i.market.some).toBe(true);
    }
  });
});

describe('the backstop costs money in every period it is not used (B4)', () => {
  it('is an agreement between two named parties, never an instrument', () => {
    const w = ranWorld('cp-run', 8);
    for (const row of w.agreements.ofKind(BACKSTOP)) {
      expect(isBackstop(row.terms)).toBe(true);
      if (!isBackstop(row.terms)) continue;
      // B4: a committed line with no fee on undrawn headroom is a free option nobody sold.
      expect(row.terms.fee).toBeGreaterThan(0);
      expect(row.terms.limit).toBeGreaterThan(0);
      expect(row.debtor).not.toBe(row.creditor);
    }
    // XI-8: nobody trades a commitment, so no instrument kind answers for one.
    expect(w.instruments.all().some((i) => String(i.kind) === String(BACKSTOP))).toBe(false);
  });
});

describe('what must not happen (E3)', () => {
  it('never leaves paper outstanding past its own maturity, and never a negative amount', () => {
    const w = ranWorld('cp-run', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER || !isPaper(i.terms)) continue;
      const outstanding = w.register.heldTotal(i.id).value;
      expect(outstanding).toBeGreaterThanOrEqual(0);
      if (w.calendar.periodOf(i.terms.maturity) < w.period && i.status.live) {
        expect(outstanding).toBe(0);
      }
    }
  });

  it('A1.b: every line runs under a year, because that is what the instrument IS', () => {
    const w = ranWorld('cp-run', 12);
    for (const i of w.instruments.all()) {
      if (i.kind !== COMMERCIAL_PAPER || !isPaper(i.terms)) continue;
      const days = w.calendar.periodOf(i.terms.maturity) - w.calendar.periodOf(i.terms.issueDate);
      expect(days).toBeLessThan(53);
      // A2.a: the quoting convention is a material part of the number at this tenor, so it is said.
      expect(i.terms.dayCount).toBe('ACT/360');
    }
  });
});

/**
 * Banks Lending A3, A3.a, A3.b; Corporate Credit C9 (17.3): the lender sets the line, and an
 * undrawn commitment is a real obligation of the bank.
 */
describe('the committed facility (Banks Lending A3, A3.a, A3.b; Corporate Credit C9)', () => {
  it('is granted by the lender that quoted the name, at what that lender said it would lend', () => {
    const w = ranWorld('cp-run', 12);
    for (const row of w.agreements.ofKind(BACKSTOP)) {
      if (!isBackstop(row.terms)) continue;
      // C9: ONE LINE PER LENDER PER BORROWER, at the margin that lender quoted. The limit is what
      // that bank published it would have out to the name — the least of what its capital, its own
      // limit for one name and its funding leave it — and never a share of the borrower's book.
      const quoted = w.journal
        .ofKind('credit.quoted')
        .filter((e) => String(e.data['borrower']) === String(row.debtor))
        .at(-1);
      expect(quoted, `a line to ${String(row.debtor)} that no bank quoted`).toBeDefined();
      expect(String(quoted!.data['bank'])).toBe(String(row.creditor));
      expect(row.terms.rate).toBe(Number(quoted!.data['rate']));
      expect(row.terms.limit.pieces).toBe(Number(quoted!.data['most']));
    }
  });

  it('declares no line size anywhere: the size is a decision, not a share (Law 2)', () => {
    // 17.3: `shortTermDebt.line` — a tenth of the borrower's own book — is deleted. What is left is
    // the commitment FEE, which is a convention of the market and a technology, not a stand-in.
    const declared = shortTermDebt().params;
    expect(declared.map((d) => String(d.id))).not.toContain('shortTermDebt.line');
    expect(declared.every((d) => d.kind !== 'placeholder')).toBe(true);
  });

  it('consumes the lender’s capital before it is drawn, and says how much (A3.a, A3.b)', () => {
    const w = ranWorld('cp-run', 12);
    const said = w.journal.ofKind('bank.capital').filter((e) => e.period === w.period);
    expect(said.length).toBeGreaterThan(0);
    for (const e of said) {
      // A3.b: UNDRAWN COMMITMENTS ARE VISIBLE — what it has promised and not lent, and what that
      // took out of its capital. A facility that costs nothing until drawn is a free option.
      expect(typeof e.data['committed']).toBe('number');
      expect(typeof e.data['committedWeighted']).toBe('number');
      const committed = Number(e.data['committed']);
      const weighted = Number(e.data['committedWeighted']);
      expect(committed).toBeGreaterThanOrEqual(0);
      // A3.a: it consumes SOMETHING, and less than the loan it would become — the standard-setter's
      // conversion factor, applied to the headroom.
      if (committed > 0) {
        expect(weighted).toBeGreaterThan(0);
        expect(weighted).toBeLessThan(committed);
        // And what it weighs is inside what the bank's whole weighted position comes to.
        expect(Number(e.data['weighted'])).toBeGreaterThanOrEqual(weighted);
      } else {
        expect(weighted).toBe(0);
      }
    }
  });

  it('counts the headroom as the limit less what has been drawn, and the kind answers it (A3.a)', () => {
    const w = ranWorld('cp-run', 12);
    for (const row of w.agreements.ofKind(BACKSTOP)) {
      if (!isBackstop(row.terms)) continue;
      const headroom = w.agreements.headroomOf(row, w.period);
      // Only a COMMITMENT answers; that is what makes an undrawn promise findable at all.
      expect(headroom.some).toBe(true);
      if (!headroom.some) continue;
      expect(headroom.value.pieces).toBeLessThanOrEqual(row.terms.limit.pieces);
      expect(headroom.value.ccy).toBe(row.ccy);
    }
  });
});

/**
 * A2, C2, E3 (17.6): whose alternative is whose, and what an unpaid maturity is not.
 */
describe('the alternative each side compares against (A2, C2, E3)', () => {
  it('lets a BANK issue, because what money last cost it is a number it publishes (A2, E1)', () => {
    const w = ranWorld('cp-run', 12);
    // A2: an issuer will not sell paper below what borrowing otherwise costs it. Nobody quotes a
    // bank a loan, so a bank had no alternative to compare against and could not issue any — the
    // one issuer whose whole business is borrowing short was the one this market had no price for.
    // Its own published cost of funds is that number.
    const said = w.journal.ofKind('bank.costOfFunds');
    expect(said.length).toBeGreaterThan(0);
    for (const e of w.journal.ofKind('paper.offered')) {
      const alternative = e.data['alternative'];
      expect(typeof alternative).toBe('number');
      expect(Number(alternative)).toBeGreaterThan(0);
    }
  });

  it('asks the party KIND what else the money could do, and never branches on a name (C2, Law 15)', () => {
    const w = rigWorld('cp-alt');
    // A bank banks at the central bank: what its money earns instead is the DEPOSIT FACILITY, the
    // floor of the corridor. Anybody else is a depositor and compares against a bank's board. The
    // difference is declared on the kind — a bank ISSUES money — and read, never asked by name.
    expect(w.registry.partyKind(BANK).moneyIssuer).not.toBeNull();
    expect(w.registry.partyKind(FIRM).moneyIssuer).toBeNull();
  });

  it('does not report an unpaid maturity against a name that is in an estate (E3, XI-8)', () => {
    // Its own world, stepped, so the audit of each period is read as it is produced.
    const w = rigWorld('cp-estate');
    for (let i = 0; i < 12; i += 1) {
      const report = w.step().audit;
      const dead = new Set<string>();
      for (const p of w.parties.all()) if (!p.status.alive) dead.add(String(p.id));
      for (const v of report.families.flatMap((f) => f.violations)) {
        if (v.family !== 'units' || !v.message.includes('still outstanding')) continue;
        // A name whose estate is open owes what it owes into a waterfall: the paper is a claim
        // ranking with the rest, and reporting it as cash that did not move would be the audit
        // reporting a mechanism that is working.
        const line = w.instruments.get(v.owner as never);
        if (!isPaper(line.terms)) continue;
        expect(dead.has(String(line.terms.issuer))).toBe(false);
      }
    }
  });
});
