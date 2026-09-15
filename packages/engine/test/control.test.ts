/**
 * The market for control (13g): what somebody will pay for a whole firm, and what its owners take.
 *
 * @spec M&A A1 M&A A3 M&A A4 M&A A5 M&A B1 M&A B2.a M&A B4 M&A C1 M&A C2 M&A E2 Equity A1 Equity B1 Equity E3 Labour A3 Private Equity A5 Law 2 Law 3 Law 5 Law 11 Law 15
 */
import { asPerPiece } from '../src/core/measure.js';
import { describe, expect, it } from 'vitest';
import { control, instrumentId, paramId, partyId, period, premiumOver } from '../src/index.js';
import { ranWorld, rigWorld } from './rig.js';

describe('a premium is a distance between two numbers (B2.a, Law 3)', () => {
  it('is what the book produced over what a share was trading at, and never a percentage', () => {
    expect(premiumOver(asPerPiece(120, 'what it paid'), asPerPiece(100, 'what it printed'))).toBe(20);
    // A bid that clears BELOW the last print is a real outcome — a firm whose holders wanted out
    // more than the buyer wanted in — and nothing bounds it away (Law 6).
    expect(premiumOver(asPerPiece(90, 'what it paid'), asPerPiece(100, 'what it printed'))).toBeLessThan(0);
  });

  it('declares no number at all: there is no control premium and no synergy term (Law 2)', () => {
    const m = control();
    expect(m.params).toEqual([]);
    // And nothing anywhere in this world declares what a takeover is worth. (A liquidity premium
    // and an insurance premium are different words for a different thing and are somebody else's.)
    const w = rigWorld('ctrl-p');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('control.');
      expect(id).not.toContain('synergy');
      expect(id).not.toContain('takeover');
      expect(id).not.toContain('acquisition');
    }
  });
});

describe('what a bidder looks for is the RESIDUAL claim (Equity A1, Law 15)', () => {
  it('reads it off the kind profile rather than branching on a kind id', () => {
    // Buying every liability of a firm buys you nothing; buying the residual buys you the firm. So
    // the test is "a claim ON somebody that its issuer does not owe", which the register already
    // says — and a module that named the share kind would be a module importing another module.
    const w = rigWorld('ctrl-p');
    const residual = w.instruments
      .all()
      .filter((i) => i.issuer.some && !w.registry.instrumentKind(i.kind).liabilityOfIssuer);
    for (const i of residual) expect(w.registry.instrumentKind(i.kind).owes).toBe('face');
    expect(residual.length).toBeGreaterThan(0);
  });
});

describe('what this world actually does with it (Law 11)', () => {
  it('bids for nothing, because every listed firm here publishes a LOSS — and says so', () => {
    /**
     * A FINDING, written as a test so it cannot be lost. An acquirer values a target the way it
     * values a machine: what it would get, against what it requires. What it would get is what the
     * target PUBLISHED, and in this world every listed firm publishes negative earnings — so there
     * is no stream to capitalise and no bid to make, which is the correct answer to the question
     * rather than a mechanism that does not work.
     *
     * The day a listed firm here earns something, this assertion fails and the tender machinery
     * starts running. Nobody should have to notice that by accident.
     */
    const w = ranWorld('ctrl-c', 30, 4, 40);
    const reports = w.journal.ofKind('reporting.report');
    expect(reports.length).toBeGreaterThan(0);
    for (const e of reports) expect(Number(e.data['earned'])).toBeLessThan(0);
    expect(w.journal.ofKind('control.acquired')).toHaveLength(0);
  });

  it('holds its own family over a run: nobody acquired is still trading (E2)', () => {
    const w = rigWorld('ctrl-p');
    for (let i = 0; i < 6; i += 1) {
      const report = w.step();
      const names = report.audit.families.find((f) => f.family === 'names');
      for (const v of names?.violations ?? []) expect(v.spec).not.toContain('M&A');
    }
  });
});

/**
 * 10f.3: THE FOUR SHAPES. *"m&a, acquisitions, mergers and disposals should all exist, with the PE
 * case being only an application"* — one layer, and which outcome a deal has is read off the
 * register and off what the buyer is, never declared.
 */
describe('a deal has four shapes and one mechanism (M&A A4, A5, Equity E3, §29 A5)', () => {
  it('leaves the target standing when the buyer employs nobody, and combines when it does', () => {
    const w = ranWorld('ctrl-shapes', 26);
    for (const e of w.journal.ofKind('control.owned')) {
      const target = String(e.data['target']);
      // §29 A5, B2.a: it goes on being a company with its own balance sheet. That is the whole of
      // why a buyout that fails kills the firm and not the buyer.
      expect(w.parties.has(partyId(target))).toBe(true);
      expect(w.parties.get(partyId(target)).status.alive).toBe(true);
      // And the buyer never paid anybody a wage: that is the test, and it is not a kind.
      expect(w.employment.everEmployed(String(e.data['buyer']) as never)).toBe(false);
    }
    for (const e of w.journal.ofKind('control.combined')) {
      if (e.data['combined'] !== true) continue;
      // §35 A4: the two balance sheets are one party now, and the target has a successor.
      expect(w.parties.get(partyId(String(e.data['target']))).status.alive).toBe(false);
    }
  });

  it('closes the book behind a take-private, and only where there was one (E3)', () => {
    const w = ranWorld('ctrl-shapes', 26);
    for (const e of w.journal.ofKind('control.owned')) {
      const line = instrumentId(String(e.data['line']));
      // Either way the line does not trade afterwards: one of them had a market this morning and
      // does not now, and the other never had one. The record says which it was.
      expect(w.instruments.get(line).market.some).toBe(false);
      if (e.data['tookPrivate'] === true) {
        // Clearing E4, §29 C5: the prints it made stay where they are and go visibly stale. The
        // holding is a MARK from here and it is never mistaken for a cleared price.
        expect(w.prices.latest(line, w.period).some).toBe(true);
      }
    }
  });

  it('lets the owners of a company nobody trades answer a bid at all (C1, C2)', () => {
    /**
     * F-6, and it is what made a private company unbuyable: a holder answered from its OUTLOOK of
     * the price, an outlook is formed from prints, and a private line makes none — so every bid for
     * one failed with "nobody tendered". A holder with no outlook answers from its own books now.
     */
    const w = ranWorld('ctrl-shapes', 26);
    const failed = w.journal
      .ofKind('control.failed')
      .filter((e) => String(e.data['why']) === 'nobody tendered');
    for (const e of failed) {
      // Whatever else is true of a refusal, it is not that the holders could not speak: every
      // holder of a live line has either an outlook or a basis, so a silence here means the line
      // has no holder at all — which 10f.1 says is a defect and F-1 says is a resolution floor.
      const target = partyId(String(e.data['target']));
      const lines = w.instruments.issuedBy(target).filter((i) => i.status.live);
      for (const i of lines) {
        if (w.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
        if (i.issued > 0) expect(w.register.holdersOf(i.id).length).toBeGreaterThan(0);
      }
    }
  });
});

/**
 * 10f.4: THE FORMAL PROCESS. *"Formal exit processes and m&a processes lead by IBD departments."*
 */
describe('a bank runs the sale, and what it can run is its people (M&A B4, Labour A3)', () => {
  it('declares no fee and no capacity: both are reads of a payroll', () => {
    const w = rigWorld('ctrl-ibd');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      // Law 2: no percentage of a deal anywhere, and no count of processes anybody stated.
      expect(id).not.toContain('advisoryfee');
      expect(id).not.toContain('processesperbank');
    }
    // What IS declared is the hours one sale takes, which is a technology and is what makes the
    // capacity a count of people rather than a policy.
    const hours = w.params.decl(paramId('bank.hoursPerProcess'));
    expect(hours.kind).toBe('technology');
    expect(hours.value).toBeGreaterThan(0);
  });

  it('never appoints a bank to more sales than its people can run', () => {
    const w = ranWorld('ctrl-ibd', 26);
    for (let p = 0; p <= Number(w.period); p += 1) {
      const quoted = new Map<string, number>();
      for (const e of w.journal.ofKindIn('advisory.quoted', period(p))) {
        const bank = String(e.data['bank']);
        const capacity = e.data['capacity'];
        if (typeof capacity === 'number') quoted.set(bank, capacity);
      }
      const ran = new Map<string, number>();
      for (const e of w.journal.ofKindIn('advisory.ran', period(p))) {
        // The event carries the RUNNING count, so the last one for a bank is how many it ran.
        ran.set(String(e.data['bank']), Number(e.data['processes']));
      }
      for (const [bank, count] of ran) {
        expect(count).toBeLessThanOrEqual(quoted.get(bank) ?? 0);
      }
    }
  });

  it('pays the bank that ran it, out of the deal, in one instruction with two sides (Law 5)', () => {
    const w = ranWorld('ctrl-ibd', 26);
    for (const e of w.journal.ofKind('advisory.fee')) {
      expect(Number(e.data['fee'])).toBeGreaterThan(0);
      // The payer is named and it is the buyer in the deal the bank ran: a fee with no payer is a
      // flow with one leg, which is the thing Law 5 exists to catch.
      expect(String(e.data['payer'])).not.toBe('');
      expect(String(e.data['bank'])).not.toBe(String(e.data['payer']));
    }
  });

  it('puts every bidder in one book, so a second bidder moves the level (B4)', () => {
    const w = ranWorld('ctrl-ibd', 26);
    for (const e of w.journal.ofKind('control.contested')) {
      const bids = e.data['bids'];
      expect(Array.isArray(bids)).toBe(true);
      // More than one, or it would not have been recorded as contested at all.
      expect((bids as unknown[]).length).toBeGreaterThan(1);
    }
  });
});

/**
 * 10f.6: §29 B, C and D AS CALLERS. *"The PE case being only an application"* — there is no
 * private-equity mechanism here and there is not going to be one.
 */
describe('a pool buys a company through the same layer a company does (§29 B1, D1)', () => {
  it('qualifies a buyer by what it PUBLISHED, not by what kind of party it is (B3, Law 15)', () => {
    const w = ranWorld('ctrl-pe', 26);
    for (const e of w.journal.ofKind('control.acquired')) {
      const buyer = partyId(String(e.data['buyer']));
      // M&A B3: it must be able to fund it, so it had a cost of money — either a bank quoted it or
      // its own investors require something of it. A party with neither cannot value a company.
      const quoted = w.journal.lastOf('credit.quoted', buyer);
      const struck = w.journal.lastOf('fund.struck', buyer);
      const hasRate =
        typeof quoted?.data['rate'] === 'number' || typeof struck?.data['requires'] === 'number';
      expect(hasRate).toBe(true);
    }
  });

  it('declares nothing about private equity anywhere in the control module (Law 15)', () => {
    // §29 B, C and D are callers of this layer. If any of it had needed a private-equity branch,
    // it would be here — and there is no kind test, no vehicle type and no buyout flag.
    const m = control();
    expect(m.params).toEqual([]);
    expect(m.instrumentKinds).toEqual([]);
    expect(m.partyKinds).toEqual([]);
  });
});
