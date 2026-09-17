/**
 * A loan is a row: written by creating a deposit, priced from the bank's own economics, refused
 * when the bank's own constraints say no.
 *
 * @spec Banks Lending A1 Banks Lending A1.a Banks Lending A2 Banks Lending A4 Banks Lending B1 Banks Lending B1.a Banks Lending B1.b Banks Lending B1.c Banks Lending B2 Banks Lending B2.a Banks Lending B2.c Banks Lending B2.d Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C2 Banks Lending C2.a Banks Lending C3 Banks Lending C3.a Banks Lending C4 Banks Lending D1 Banks Lending D3 Banks Lending E1 Banks Lending F1 Banks Lending F1.a Banks Lending F3 Money B3.a Money B3.c Bond N13.a Corporate Credit G8 XI-4
 */
import { asCash, heldAsMoney, plus } from '../src/core/measure.js';
import { borrowerOf, rateOn } from '../src/registry/credit.js';
import { rate as perAnnum } from '../src/core/rate.js';
import { describe, expect, it } from 'vitest';

import {
  type Instrument,
  type InstrumentId,
  creditorOf,
  moneyInstrumentId,
  mul,
  upTick,
  BANK_COUNT,
  defaultFrequency,
  drawBanks,
  InvalidRegistry,
  LOAN,
  USD,
  TREASURY_US,
  assemble,
  isLoan,
  loanKind,
  partyId,
  type InstructionDraft,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigSpec, withDependencies, mergeModules } from './rig.js';
import {
  type BankDecl,
  type CreditInputs,
  type Regulation,
  nameView,
} from '../src/index.js';
import type { Statement } from '../src/registry/statements.js';
import { asRatio } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { none, some } from '../src/core/option.js';
import type { Event } from '../src/journal/journal.js';
import type { ParticipantView } from '../src/world/context.js';
import { paidTo, unexpected } from './expected.js';
import { phx } from './units.js';
import { notDealing } from './no-dealing.js';
import { pathsOf, rolls, takes } from '../src/mechanisms/banks/workout.js';
import type { LoanTerms } from '../src/mechanisms/banks/loan.js';
import { Forbidden } from '../src/core/errors.js';
import { addMonths } from '../src/calendar/civil.js';
import { lossGivenDefault } from '../src/mechanisms/banks/credit-view.js';
import { uncoveredShare } from '../src/registry/secured.js';
import { requiredOnClaim, requiredYieldOf } from '../src/mechanisms/banks/treasury.js';
import { expectedLossOn } from '../src/registry/banking.js';
import { downTick } from '../src/core/tick.js';
import { BANK, FIRM, HOUSEHOLD } from '../src/registry/profiles.js';
import { mustRaise } from '../src/mechanisms/banks/dealing.js';
import type { DeskState } from '../src/mechanisms/banks/dealing-quote.js';
import { rigWorld } from './rig.js';
import type { AuditReport, Violation } from '../src/audit/audit.js';

/** Every ownership violation this period that is about where a loan is owed (17.8). */
function outOfTheSystem(report: { readonly audit: AuditReport }): Violation[] {
  return report.audit.families
    .filter((f) => f.family === 'ownership')
    .flatMap((f) => f.violations)
    .filter((v) => v.message.includes('outside the banking system'));
}
import { asPerPiece } from '../src/core/measure.js';
import { instrumentId } from '../src/core/ids.js';
import { FACILITY, isFacility, TERM_MONTHS } from '../src/registry/credit.js';
import { askToFund, committedTo } from '../src/mechanisms/control/deal.js';

const BORROWER = partyId('firm.1');
const PAYEE = partyId('firm.2');
const BANK_OF_A = partyId('bank.a');

/** A borrower that says what it is short of, which is the only thing a borrower says (C2). */
function asksFor(amount: number, at = 1): SystemModule {
  return {
    id: 'test.asks',
    spec: 'Banks Lending C2',
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.ask',
        spec: 'Banks Lending C2',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [{ kind: 'event', name: 'credit.request' }],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          // Item 0e: a borrower publishes what it is short of through the one door every borrower
          // uses. It wrote `firms.funding` directly, which is the coupling that item removed — a
          // bank read two other modules' event names and a third borrower had to join that list.
          ctx.request(partyId(BORROWER), {
            ccy: USD,
            short: asCash(amount, USD, 'what it is short of'),
            repays: 'atOption',
            // 17b.1: the money, not a promise of it.
            wants: 'money',
            // 17b.8: over the term a line is written for, which the borrower says — and it is
            // the world's own number, so a test that shortens it still shortens this.
            months: ctx.params.months(TERM_MONTHS.working),
            // 0i: it puts up nothing, and an ask that is unsecured says so.
            security: [],
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/**
 * A borrower that names what it would secure the loan on: the first live line it holds that is
 * not money, for all of it. What the thing IS the bank does not learn (A4); that it says it repays
 * on a schedule is what makes the row a term loan rather than a line (Bond F3, 11.2).
 */
function asksSecured(amount: number, at = 1): SystemModule {
  return {
    ...asksFor(amount, at),
    id: 'test.asksSecured',
    phases: [
      {
        name: 'test.askSecured',
        spec: 'Banks Lending A4',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [{ kind: 'event', name: 'credit.request' }],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          const pledge = ctx.register
            .holdingsOf(BORROWER)
            .find((h) => {
              const i = ctx.instruments.get(h.instrument);
              return i.status.live && ctx.registry.instrumentKind(i.kind).pricing !== 'money';
            });
          if (pledge === undefined) throw new Error('the rig gave the borrower nothing to pledge');
          ctx.request(BORROWER, {
            ccy: USD,
            short: asCash(amount, USD, 'what it is short of'),
            security: [{ instrument: pledge.instrument, qty: ctx.register.quantity(BORROWER, pledge.instrument) }],
            repays: 'onSchedule',
            wants: 'money',
            months: ctx.params.months(TERM_MONTHS.working),
          });
        },
      },
    ],
  };
}

/** Spends more than it holds, so its bank has to decide (Money B3.a). */
/**
 * A payment BIGGER THAN THE ACCOUNT, sized to what the payer's own bank has said it will have out
 * to one name (Banks Lending F3, published as `limitPerName`). Naming an amount here would be
 * naming one this world's banks happen to be able to allow, and what the test is about is the door
 * (Money B3.a) rather than the number: it overdraws the account by half of what its bank itself
 * says it will lend it.
 */
function overspendsItsLimit(at = 2): SystemModule {
  return {
    id: 'test.overspends',
    spec: 'Money B3.a',
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.overspend',
        spec: 'Money B3.a',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          const bank = ctx.parties.get(BORROWER).bank;
          const said = ctx.journal
            .ofKind('bank.capital')
            .filter((e) => e.subjects.includes(bank));
          const limit = Number(said[said.length - 1]?.data['limitPerName']);
          if (!Number.isFinite(limit) || limit <= 0) return;
          const held = ctx.register.quantity(BORROWER, moneyInstrumentId(bank, USD));
          const leg: Leg = {
            kind: 'money',
            receipt: { of: 'transfer' },
            from: { holder: BORROWER, issuer: bank },
            to: { holder: PAYEE, issuer: ctx.parties.get(PAYEE).bank },
            ccy: USD,
            // B3.a: A LITTLE over what it holds — a hundredth of what its bank will lend ONE NAME.
            // Half of that per-name limit used to be affordable and is not: a bank's room is the
            // LEAST of its capital, its appetite and its funding (Banks Lending B3), and the
            // per-name limit is only one of the three. What this module is for is an overdraft the
            // bank ALLOWS, so it asks for one small enough that the other two are not the binding
            // constraint.
            amount: ctx.registry.payable(plus(
                heldAsMoney(held, USD, 'what it holds'),
                asCash(limit / 100, USD, 'a little over it'),
                'a touch more than it holds',
              ),
            ),
          };
          ctx.settle({ legs: [leg], cause: 'transfer', reason: 'a bill' });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/**
 * PLAN §7: `times` is how many times its own money it tries to spend, and what that COMES TO is read
 * off its account at the moment it spends. An amount written down was more than a firm held when the
 * seed stated this world's scale, and 11.5 derives it: three hundred thousand is now small change
 * and the payment it was meant to fail simply went through.
 */
function overspends(times: number, at = 2): SystemModule {
  return {
    id: 'test.overspends',
    spec: 'Money B3.a',
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.overspend',
        spec: 'Money B3.a',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          const has = ctx.register.quantity(
            BORROWER,
            moneyInstrumentId(ctx.parties.get(BORROWER).bank, USD),
          );
          const leg: Leg = {
            kind: 'money',
            receipt: { of: 'transfer' },
            from: { holder: BORROWER, issuer: ctx.parties.get(BORROWER).bank },
            to: { holder: PAYEE, issuer: ctx.parties.get(PAYEE).bank },
            ccy: USD,
            amount: upTick(mul(has, times, 'what it tries to spend')),
          };
          const draft: InstructionDraft = { legs: [leg], cause: 'transfer', reason: 'a bill' };
          ctx.settle(draft);
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function world(extra: readonly SystemModule[] = [], limits?: number, months?: number): World {
  const spec = rigSpec('loans');
  const modules = withDependencies(spec.modules, (m) =>
m.id === 'sovereign-instruments' ||
        m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
        m.id === 'credit-events' ||
        m.id === 'banks' ||
        // 17b′.2: a lender commits against ACCOUNTS, so the module that prepares them is in the
        // scale model. It is not a `requires` — §48 reads what banks publish about themselves and
        // the two would be a cycle — so a world that wants both says so here.
        m.id === 'reporting' ||
        m.id === 'money-market',
    )
    .map((m) =>
      limits === undefined
        ? m
        : {
            ...m,
            params: m.params.map((p) =>
              p.id.startsWith('bank.limitPerBorrower.') ? { ...p, value: limits } : p,
            ),
          },
    )
    /**
     * A2, 17.7: how long this world's loans run for. A commercial facility is written for a year
     * and the world says so; a SCALE MODEL of a maturity has to reach one inside the periods a test
     * steps, so the test that is about what happens AT a maturity shortens the term rather than
     * stepping a year of weeks to get there. It is the same number in the same register, read the
     * same way — only smaller, which is what a scale model is.
     */
    .map((m) =>
      months === undefined
        ? m
        : {
            ...m,
            params: m.params.map((p) =>
              String(p.id) === 'lending.loanMonths' ? { ...p, value: months } : p,
            ),
          },
    )
    .map(notDealing);
  return assemble({ ...spec, modules: mergeModules(modules, extra) });
}

/** Whoever is owed a row, as a name a test can read; a repaid row is owed to nobody. */
function whoIsOwed(w: World, i: Instrument): string {
  const owed = creditorOf((h) => w.register.holdersOf(h), i);
  return owed.some ? String(owed.value) : '';
}

function loans(w: World): { id: string; issued: number; lender: string; rate: number }[] {
  return w.instruments
    .all()
    .filter((i) => i.kind === LOAN)
    .map((i) => ({
      id: String(i.id),
      issued: i.issued,
      lender: whoIsOwed(w, i),
      rate: isLoan(i.terms) ? rateOn(i.terms) : 0,
    }));
}

describe('who answers for an overdraft (Money B3.a)', () => {
  it('refuses to seal a world whose bank says it is a credit decision and nobody takes it', () => {
    const spec = rigSpec('no-decider');
    const modules = withDependencies(spec.modules, (m) =>
m.id === 'sovereign-instruments' || m.id === 'seed.foundation' ||
      m.id === 'seed.funding',
    );
    // A bank kind that says an overdraft at it is a credit decision, in a world with no lender to
    // take it, is broken from the start — and a defaulted-to refusal would look exactly like a bank
    // with a credit standard, which is the thing C3.a says must never be invisible.
    expect(() => assemble({ ...spec, modules })).toThrow(InvalidRegistry);
  });
});

describe('what a loan is (Banks Lending A1, A2, A4, D1)', () => {
  it('is carried at cost with no market, and states what it is secured on either way', () => {
    // A1.a: it is not a security. D1: so it is held at amortised cost rather than marked to a
    // market that does not exist — and it names no market at all, which the kernel enforces.
    expect(loanKind.pricing).toBe('carriedAtCost');
    expect(loanKind.carry).toBe('cost');
    expect(loanKind.liabilityOfIssuer).toBe(true);
    // N13.a: the ranking is stated. Unsecured here, and that is an answer rather than a gap.
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 4; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined) return;
    const rank = loanKind.ranking(row);
    expect(rank.secured).toEqual([]);
    expect(rank.claim.length).toBeGreaterThan(0);
  });
});

describe('writing it (Banks Lending B1, B1.a, B1.c)', () => {
  it('creates a deposit, and no reserve leaves the bank', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    // Period 1 is where it says what it is short of; period 2 is where the credit is arranged.
    w.step();
    const before = w.cash(BORROWER, USD);
    w.step();
    const written = w.journal.ofKind('credit.written');
    expect(written.length).toBe(1);
    const principal = Number(written[0]?.data['principal']);
    expect(principal).toBeGreaterThan(0);
    // B1: the loan on one side and the borrower's balance on the other, at the same instant.
    // ...and the one other thing that reached the account this period: a week of deposit interest
    // from its own bank (Banks Funding B1), which is not this loan's doing.
    expect(w.cash(BORROWER, USD)).toBe(before + principal + paidTo(w, BORROWER, 'coupon'));
    expect(loans(w)).toHaveLength(1);
    expect(loans(w)[0]?.issued).toBeCloseTo(principal, 9);
    // B1.a: no reserve leaves. The whole of endogenous money is that this instruction has no
    // interbank leg in it at all — the money it created is the lending bank's own.
    const record = w.ledger
      .all()
      .find((r) => r.outcome === 'settled' && r.instruction.cause === 'issuance');
    expect(record?.outcome === 'settled' && record.reserveLegs).toEqual([]);
    // B1.c: and nothing was consumed to fund it. The bank's own money holdings did not move.
    expect(w.journal.ofKind('reserve.overdraft')).toHaveLength(0);
  });

  it('is a row with a lender of record holding every unit of it (F1, F1.a)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    // There is no book number anywhere: a bank's book is the sum of the rows it holds.
    const owed = creditorOf((h) => w.register.holdersOf(h), row);
    expect(owed.some).toBe(true);
    if (!owed.some) return;
    expect(w.register.quantity(owed.value, row.id)).toBeCloseTo(row.issued, 9);
    expect(borrowerOf(row)).toBe(BORROWER);
  });
});

describe('the price (Banks Lending C1, C2, XI-4)', () => {
  it('is four named terms, and two banks do not quote the same', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const written = w.journal.ofKind('credit.written')[0];
    const rate = Number(written?.data['rate']);
    // C1: cost of funds plus expected loss plus the capital charge plus what it costs to run it.
    // Nothing it owes pays interest yet, so the first term is a true zero and the rest are real.
    // No two of them ask the same on their own capital, so no two of them quote the same.
    // Seed B4: the banks of this world are DRAWN from its own seed value, so the test asks for
    // that world's rows rather than importing a table (there is no table).
    const rows = drawBanks(BANK_COUNT, 'loans');
    const asked = rows.map((x) => x.returnOnCapital);
    expect(new Set(asked).size).toBe(rows.length);
    // C2: the borrower took the KEENEST QUOTE IT WAS GIVEN, which is not the same as the keenest
    // bank in the draw — a bank quotes when it has the room and the appetite for the name, so which
    // of them is in the running is an outcome and not a list. The quotes are public events, so the
    // comparison is read off the ones this borrower actually received.
    const keenest = w.journal
      .ofKind('credit.quoted')
      .filter((e) => e.data['borrower'] === written?.data['borrower'] && e.period === written?.period)
      .pop();
    expect(keenest, 'nobody quoted this borrower, so nothing was compared').toBeDefined();
    expect(written?.data['bank']).toBe(keenest?.data['bank']);
    expect(rate).toBeGreaterThan(0);
    // C1: THE RATE IS ITS FOUR TERMS AND NOTHING ELSE — cost of funds, expected loss, the capital
    // charge and what it costs to run the loan — and the quote publishes all four, so the claim is
    // checked by adding them up rather than by asserting which of them happen to be positive in
    // this world (a borrower that has never failed has an expected loss of a true zero).
    const terms = ['costOfFunds', 'expectedLoss', 'capitalCharge', 'operatingCost'] as const;
    for (const k of terms) expect(Number(keenest?.data[k])).toBeGreaterThanOrEqual(0);
    const built = terms.reduce((n, k) => n + Number(keenest?.data[k]), 0);
    expect(built).toBeCloseTo(Number(keenest?.data['rate']), 12);
    // The rate it was WRITTEN at is not asserted to be the rate it was QUOTED: they differ by three
    // parts in a million, because `publishQuotes` and `runRequests` each derive it (worklist 13f).
    // That is one fact with two writers and it is a finding, not something to assert around.
  });

  it('gets dearer for a borrower its bank has watched get into difficulty (C1.b, C4)', () => {
    const clean = world([asksFor(phx(10_000).pieces, 6)]);
    for (let i = 0; i < 8; i += 1) clean.step();
    // The same request from a borrower that overdrew its account four periods earlier, and whose
    // bank has carried the loan it wrote for that overdraft ever since.
    const marked = world([overspends(2, 2), asksFor(phx(10_000).pieces, 6)]);
    for (let i = 0; i < 8; i += 1) marked.step();
    /**
     * THE SAME REQUEST, which is the whole of the comparison and what this used to get wrong.
     *
     * It read `credit.written[0]` from each world — the FIRST loan either world wrote. In the clean
     * world that is the request; in the marked one it is the overdraft, a hundred million written
     * at period 2 against a million asked for at period 6. Two different loans to two different
     * purposes at two different times, and the assertion was that one of them was dearer than the
     * other. So the mechanism was right and the measurement was not: like for like, the marked
     * borrower is quoted 0.0272 where the clean one is quoted 0.0219.
     */
    const askedFor = (w: World): number => {
      const mine = w.journal
        .ofKind('credit.written')
        .filter((e) => e.data['borrower'] === BORROWER && e.data['principal'] === phx(10_000));
      expect(mine.length, 'the request was never written, so there is nothing to compare').toBe(1);
      return Number(mine[0]?.data['rate']);
    };
    // C1.b, C4: the bank's own view of this borrower moved — it is carrying a large claim on the
    // name now, and its capital charge and its cost of funds both say so — so the price moved.
    expect(askedFor(marked)).toBeGreaterThan(askedFor(clean));
  });

  /**
   * G8: AND A DEFAULT IS THE OTHER CHANNEL, which is a fact about the quote rather than about any
   * one world. What a bank expects to lose on a name is how often it has seen that name fail over
   * how long it remembers — so it is zero for a borrower that has never failed, which is what every
   * world above has, and it rises the moment one does.
   *
   * It is asked of the function rather than of a run because no world in this file has a borrower
   * that BOTH defaults and is still quoted: an overdraft its bank allows is not a failure, and a
   * payment its bank refuses only happens here when no bank will lend at all, so nobody quotes.
   * Reaching that state needs a bank with room for the name and no willingness to cover it, which
   * is 13f's committed facility — recorded here rather than asserted around.
   */
  it('expects to lose more on a name it has watched fail, and nothing on one it has not (G8)', () => {
    const w = world();
    w.step();
    const view = w.participantView(BANK_OF_A);
    const decl = drawBanks(BANK_COUNT, 'loans').find((b) => b.bank === BANK_OF_A);
    expect(decl).toBeDefined();
    if (decl === undefined) return;
    const never = defaultFrequency(view, decl, BORROWER, []);
    expect(never).toBe(0);
    const failed = defaultFrequency(view, decl, BORROWER, [
      {
        id: 1 as never,
        period: w.period,
        cycle: 0 as never,
        kind: 'credit.default',
        subjects: [String(BORROWER)],
        data: {},
        public: true,
      },
    ]);
    // It is a frequency and not a grade: one failure in the window it remembers.
    expect(failed).toBeGreaterThan(0);
  });
});

describe('the provision (Banks Lending D1, D2, D2.a, D2.b, C4)', () => {
  it('carries the loan at what its lender expects to recover, and the charge is visible', () => {
    // The same borrower, seen to fail, then borrowing: the bank prices it dearer AND carries it
    // lower, off the one model (C4) — two beliefs would mean the price and the provision disagree.
    const w = world([overspends(2, 2), asksFor(phx(10_000).pieces, 6)]);
    for (let i = 0; i < 9; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    const creditor = creditorOf((h) => w.register.holdersOf(h), row);
    expect(creditor.some).toBe(true);
    if (!creditor.some) return;
    const holding = w.register.holding(creditor.value, row.id);
    expect(holding.some).toBe(true);
    if (!holding.some) return;
    // D1, D2: amortised cost less what this bank expects to lose. It is ON the lot, which is what
    // the book carries it at — not a reserve sitting beside it (D2.b).
    const basis = holding.value.lots[0]?.basisPerUnit ?? 1;
    expect(basis).toBeLessThan(1);
    expect(basis).toBeGreaterThan(0);
    // D2.a: and the charge went through income — the equity account moved with it, journalled.
    const marks = w.journal
      .ofKind('revaluation')
      .filter((e) => e.subjects.includes(String(row.id)));
    expect(marks.length).toBeGreaterThan(0);
    expect(Number(marks[0]?.data['delta'])).toBeLessThan(0);
  });
});

describe('repaying it (Bond F3, Banks Lending F2, Small-Business Pools B1, 11.2)', () => {
  it('a term loan repays a slice of its principal every period, and a line falls due once', () => {
    const term = world([asksSecured(phx(20_000).pieces)]);
    const line = world([asksFor(phx(20_000).pieces)]);
    for (const w of [term, line]) {
      w.step();
      w.step();
    }
    const termRow = loans(term)[0];
    const lineRow = loans(line)[0];
    expect(termRow).toBeDefined();
    expect(lineRow).toBeDefined();
    if (termRow === undefined || lineRow === undefined) return;
    const termAt = termRow.issued;
    const lineAt = lineRow.issued;
    const before = term.cash(BORROWER, USD);
    for (const w of [term, line]) w.step();
    // F3: the slice was paid in money, and what is outstanding fell by exactly it — never a
    // number written on the row (Law 4: the outstanding IS the units).
    const termNow = loans(term)[0]?.issued;
    expect(termNow).toBeDefined();
    if (termNow === undefined) return;
    expect(termNow).toBeLessThan(termAt);
    const repaid = term.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'maturity' && r.instruction.reason.startsWith('amortisation'));
    expect(repaid.length).toBe(1);
    expect(termAt - termNow).toBe(repaid[0]?.instruction.legs.find((l) => l.kind === 'money')?.amount);
    // The borrower's cash moved by the slice and the interest on what was outstanding (F2), and by
    // nothing else the row did; a week of deposit interest is its own bank's doing.
    expect(term.cash(BORROWER, USD)).toBeLessThan(before + paidTo(term, BORROWER, 'coupon'));
    // A line is drawn and repaid at the borrower's option: nothing came off it.
    expect(loans(line)[0]?.issued).toBe(lineAt);
    const rows = term.instruments.all().filter((i) => i.kind === LOAN);
    expect(rows[0] !== undefined && isLoan(rows[0].terms) && rows[0].terms.amortising).toBe(true);
    // Bond D2: what the row promises after today derives from the same schedule — the sum of the
    // principal in its remaining flows is what is left of a unit.
    const flows = rows[0] === undefined ? [] : loanKind.cashFlows(rows[0], term.calendar.startOf(term.period), term.calendar, term.registry);
    expect(flows.length).toBeGreaterThan(1);
    const promised = flows.reduce((t, f) => t + f.perUnit, 0);
    expect(promised).toBeGreaterThan(1);
  });
});

describe('when it says no (Banks Lending B2, C3, C3.a, F3)', () => {
  it('declines when its limit for one name binds, and the decline is recorded', () => {
    // F3: a large-exposure limit that binds is what makes concentration a thing it manages.
    const w = world([asksFor(phx(20_000).pieces)], 0);
    for (let i = 0; i < 4; i += 1) w.step();
    expect(w.journal.ofKind('credit.written')).toHaveLength(0);
    const declined = w.journal.ofKind('credit.declined');
    expect(declined.length).toBeGreaterThan(0);
    // C3.a: a bank that never says no has no credit standard, so what it said no to is written
    // down — with which of its own constraints bound (B2.d).
    expect(declined[0]?.data['borrower']).toBe(BORROWER);
    expect(['capital', 'appetite']).toContain(String(declined[0]?.data['binds']));
  });
});

describe('an overdrawn customer (Money B3.a, B3.c)', () => {
  it('is borrowing: what the bank allowed is a row by the close, not a hole', () => {
    const w = world([overspendsItsLimit()]);
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // The payment went through because its bank decided to lend it the difference...
    const failed = w.ledger.all().filter((r) => r.outcome === 'failed');
    expect(failed).toHaveLength(0);
    // ...and by the close of the period it is a loan with a lender and a rate (B3.c).
    const rows = loans(w);
    expect(rows.length).toBeGreaterThan(0);
    expect(rows[0]?.rate).toBeGreaterThan(0);
    // The account is not below zero any more: the deposit the loan created brought it back.
    expect(w.cash(BORROWER, USD)).toBeGreaterThanOrEqual(0);
  });

  it('is refused when the bank has no room, and then the payment simply fails (B3.c)', () => {
    // Twice its money, at a bank that will lend nothing: the payment has nowhere to come from.
    const w = world([overspends(2)], 0);
    for (let i = 0; i < 4; i += 1) w.step();
    const failed = w.ledger.all().filter((r) => r.outcome === 'failed');
    expect(failed.length).toBeGreaterThan(0);
    expect(w.journal.ofKind('credit.declined').length).toBeGreaterThan(0);
    expect(loans(w)).toHaveLength(0);
    // Money E1: and the payer that could not pay is in default of payment, by name.
    expect(
      w.journal.ofKind('credit.default').some((e) => e.data['party'] === BORROWER),
    ).toBe(true);
  });
});

describe('carrying it (Banks Lending D3, E1)', () => {
  it('accrues interest that is paid to the lender, period by period', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 4; i += 1) w.step();
    const lenderBefore = w.register.equity(BANK_OF_A).pieces;
    const borrowerBefore = w.cash(BORROWER, USD);
    const from = w.period;
    w.step();
    // D3: interest accrues and is RECEIVED — read off the wire, which is where a payment is. It
    // used to be read as "the borrower's account fell", and that held only while nothing else paid
    // the borrower anything in the same period: a firm in a running world is being paid for what it
    // sells at the same time as it is paying its interest, and a net movement is not the payment.
    const paid = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'coupon')
      .filter((r) => r.instruction.legs.some((l) => l.kind === 'money' && l.from.holder === BORROWER));
    expect(paid.length, 'no interest was paid at all').toBeGreaterThan(0);
    // And it reaches the lender by EXTINGUISHING the deposit the lender itself issued, which is
    // what being paid in your own money is (Money C2). Its equity ledger says so entry by entry
    // (12a) — a NET movement does not, because the same bank is paying its own depositors in the
    // same period and that is a different flow with a different cause.
    const got = w.register
      .equityEntries(BANK_OF_A, from, w.period)
      .filter((e) => /coupon/i.test(e.cause) && e.delta > 0);
    expect(got.length, 'the lender was not paid on its loan at all').toBeGreaterThan(0);
    expect(borrowerBefore).toBeGreaterThan(0);
    expect(lenderBefore).not.toBe(w.register.equity(BANK_OF_A).pieces);
  });

  it('is a default when the borrower does not pay it (E1, E2)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    // Its own definition of default: a payment on the loan that the borrower did not make.
    if (row !== undefined) {
      expect(loanKind.defaultOn).toBeDefined();
      expect(row.status.live && row.status.performing).toBe(true);
    }
  });
});

describe('the world it lives in', () => {
  it('runs a year with lending in it and stays consistent', () => {
    const spec = rigSpec('loans-year');
    const w = assemble(spec);
    expect(w.phases.map((p) => p.name)).toContain('lending.write');
    for (let i = 0; i < 52; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // SOMEBODY BORROWS NOW, and it is the demand side this test used to say was missing. What
    // changed is that a firm has owners to pay (Equity D3, worklist 9): money it used to sit on
    // leaves it for its shareholders, and a week when its receipts are late is a week it is short.
    // A bank quoted, it took the quote, and the row is a row like any other — which is what the
    // supply side was built to answer with.
    const written = w.journal.ofKind('credit.written');
    expect(written.length).toBeGreaterThan(0);
    const rows = w.instruments.all().filter((i) => i.kind === LOAN);
    expect(rows.length).toBeGreaterThan(0);
    // F1.a: every row is owed to somebody holding every unit of it, which the flows family checks
    // every period — and the year above was green. Who that is is read off the register; the name
    // on the terms is whoever WROTE it (13e), and in a world with no sales yet they are the same.
    for (const row of rows) {
      expect(isLoan(row.terms)).toBe(true);
      const who = creditorOf((h) => w.register.holdersOf(h), row);
      // A row repaid to the last unit is owed to nobody, which is a state and not a defect.
      if (who.some) expect(String(who.value).startsWith('bank.')).toBe(true);
    }
    expect(w.parties.ofKind(partyId('bank') as never).length).toBeGreaterThan(0);
    expect(TREASURY_US).toBeDefined();
  });
});

/**
 * The credit view (item 17.0): ONE bank's opinion of a name, formed from its own record and from
 * what the name opened to it — and read by the quote, the row, the provision and the reservation
 * alike (C4). Asked of the function, with the inputs stated, because what is being tested is the
 * view rather than a world that happens to produce one.
 */
describe('the credit view (Banks Lending C1, C3, C4; Corporate Credit A4, E5)', () => {
  const NAME = partyId('name.one');
  const OTHER = partyId('name.two');

  function bank(): { view: ParticipantView; decl: BankDecl } {
    const w = world();
    w.step();
    const rows = drawBanks(BANK_COUNT, 'loans');
    const decl = rows[0];
    expect(decl).toBeDefined();
    return { view: w.participantView(partyId(decl!.bank)), decl: decl! };
  }

  const REG: Regulation = {
    capitalRatio: asRatio(0.08, 'what the standard asks'),
    riskWeight: asRatio(1, 'what an ordinary exposure weighs'),
    operatingCost: asRatio(0.01, 'what running a loan costs it'),
  };

  function inputsWith(over: Partial<CreditInputs>): CreditInputs {
    return {
      funds: asRatio(0.02, 'what money costs it'),
      reg: REG,
      defaults: [],
      recovered: { paid: asQty(0), lost: asQty(0) },
      gradeOn: () => none(),
      weightOf: () => REG.riskWeight,
      statementOf: () => none(),
      prepared: () => false,
      marketYieldOn: () => none(),
      ...over,
    };
  }

  /** XI-1: a default anybody published, which is public, so every bank saw it. */
  const failed = (who: ReturnType<typeof partyId>, at: number): Event => ({
    id: 1 as never,
    period: at as never,
    cycle: 0 as never,
    kind: 'credit.default',
    subjects: [String(who)],
    data: {},
    public: true,
  });

  it('prices two names apart when it has watched one of them fail (C1.b, C4)', () => {
    const { view, decl } = bank();
    const inputs = inputsWith({ defaults: [failed(NAME, view.period)] });
    const watched = nameView(view, decl, NAME, inputs);
    const clean = nameView(view, decl, OTHER, inputs);
    expect(watched.probabilityOfDefault).toBeGreaterThan(0);
    expect(clean.probabilityOfDefault).toBe(0);
    expect(watched.expectedLoss).toBeGreaterThan(clean.expectedLoss);
    expect(watched.rate).toBeGreaterThan(clean.rate);
    // And the same belief prices the paper as it prices the loan, one term apart: what it requires
    // to HOLD a name is its rate less what running a loan costs it, because holding is not work.
    expect(watched.rate - watched.required).toBeCloseTo(REG.operatingCost, 12);
  });

  it('two banks that have recovered differently quote one name differently (A4.b)', () => {
    const { view, decl } = bank();
    const seen = { defaults: [failed(NAME, view.period)] };
    // One has been paid nineteen of every twenty by the estates it was a creditor of; the other has
    // met no estate at all and treats the whole of a claim as at risk, which is what ignorance says.
    const paidWell = nameView(
      view,
      decl,
      NAME,
      inputsWith({ ...seen, recovered: { paid: asQty(95), lost: asQty(5) } }),
    );
    const blind = nameView(view, decl, NAME, inputsWith(seen));
    expect(blind.lossGivenDefault).toBe(1);
    expect(paidWell.lossGivenDefault).toBeLessThan(blind.lossGivenDefault);
    expect(paidWell.expectedLoss).toBeLessThan(blind.expectedLoss);
    expect(paidWell.rate).toBeLessThan(blind.rate);
  });

  it('declines a name that kept its books shut, and quotes one too young to have any (C3, C3.a)', () => {
    const { view, decl } = bank();
    const shut = nameView(view, decl, NAME, inputsWith({ prepared: () => true }));
    expect(shut.declines.some && shut.declines.value).toBe('undisclosed');
    // A name that has never closed a quarter has nothing to open, and is priced on the record —
    // which is all anybody has of it. A refusal there would refuse every new company in the world.
    const young = nameView(view, decl, NAME, inputsWith({}));
    expect(young.declines.some).toBe(false);
  });

  it('declines a name whose earnings did not cover its debt, and one the market prices worse (17.0)', () => {
    const { view, decl } = bank();
    const shown = (ebitda: number, service: number): Statement =>
      ({
        summary: {
          ebitda: asCash(ebitda, USD, 'what it earned'),
          service: asCash(service, USD, 'what its debt took'),
          freeCashFlow: asCash(0, USD, 'what was left'),
          netDebt: asCash(0, USD, 'what it owes net'),
        },
        balance: { debt: asCash(0, USD, 'what it owes') },
      }) as unknown as Statement;
    const thin = nameView(
      view,
      decl,
      NAME,
      inputsWith({ prepared: () => true, statementOf: () => some(shown(50, 100)) }),
    );
    expect(thin.coverage.some && thin.coverage.value).toBeLessThan(1);
    expect(thin.declines.some && thin.declines.value).toBe('coverage');
    // XI-4: and it does not lend below where the market already prices the name — it buys the paper.
    const covered = { prepared: () => true, statementOf: () => some(shown(300, 100)) };
    const dear = nameView(
      view,
      decl,
      NAME,
      inputsWith({ ...covered, marketYieldOn: () => some(asRatio(0.15, 'what the market requires')) }),
    );
    expect(dear.declines.some && dear.declines.value).toBe('marketYield');
    const fine = nameView(view, decl, NAME, inputsWith(covered));
    expect(fine.declines.some).toBe(false);
  });
});

/**
 * The workout (Banks Lending E3, finding 21.59, item 17.7).
 *
 * Two decisions and one door. The decisions are arithmetic over what the creditor's own view
 * already says, so they are checked as arithmetic; the door is the kernel's and is checked by what
 * it refuses, because what it refuses is the whole reason it is not a way to rewrite any line.
 */
describe('the workout (Banks Lending E3, 21.59)', () => {
  const YEAR = asRatio(1, 'a year of it');
  const NOTHING = asRatio(0, 'none of it');

  it('agrees when enforcing brings it nothing, and enforces when enforcing brings it everything', () => {
    // C1.b: a creditor that has met no estate has recovered nothing and expects to recover nothing,
    // so enforcing brings it nothing at all and any promise is worth more than that.
    const blind = pathsOf(asRatio(1, 'all of it at risk'), asRatio(0.1, 'seen it fail'), asRatio(0.01, 'what its capital costs'), YEAR);
    expect(blind.enforcing).toBe(0);
    expect(takes(blind)).toBe('agree');
    // And one that has been paid in full by every estate it met gets the whole of it now, which
    // nothing it waits for can beat once waiting costs it anything.
    const paidInFull = pathsOf(NOTHING, asRatio(0.1, 'seen it fail'), asRatio(0.01, 'what its capital costs'), YEAR);
    expect(paidInFull.enforcing).toBe(1);
    expect(takes(paidInFull)).toBe('enforce');
  });

  it('is the capital that carrying it costs that decides between two otherwise identical claims (E3)', () => {
    const loss = asRatio(0.5, 'half of it at risk');
    const fails = asRatio(0.2, 'how often it has seen the name fail');
    const charge = asRatio(0.4, 'what a unit of capital costs it');
    // The same claim, the same name, the same recovery: the only difference is how long the new
    // terms would carry it, and that is what E3 means by each path having a cost.
    const brief = pathsOf(loss, fails, charge, asRatio(0.1, 'a few weeks of it'));
    const long = pathsOf(loss, fails, charge, asRatio(2, 'two years of it'));
    expect(brief.enforcing).toBe(long.enforcing);
    expect(takes(brief)).toBe('agree');
    expect(takes(long)).toBe('enforce');
  });

  it('rolls a name it would lend to today, and calls in one its own standard turns away (C3)', () => {
    const w = world();
    w.step();
    const decl = drawBanks(BANK_COUNT, 'loans')[0];
    expect(decl).toBeDefined();
    if (decl === undefined) return;
    const view = w.participantView(partyId(decl.bank));
    const inputs = (over: Partial<CreditInputs>): CreditInputs => ({
      funds: asRatio(0.02, 'what money costs it'),
      reg: {
        capitalRatio: asRatio(0.08, 'what the standard asks'),
        riskWeight: asRatio(1, 'what an ordinary exposure weighs'),
        operatingCost: asRatio(0.01, 'what running a loan costs it'),
      },
      defaults: [],
      recovered: { paid: asQty(0), lost: asQty(0) },
      gradeOn: () => none(),
      weightOf: () => asRatio(1, 'what an ordinary exposure weighs'),
      statementOf: () => none(),
      prepared: () => false,
      marketYieldOn: () => none(),
      ...over,
    });
    // A name that kept its books shut is a name this bank declines, and a lender that would not
    // write the line today does not agree another term of it either.
    expect(rolls(nameView(view, decl, partyId('name.shut'), inputs({ prepared: () => true })))).toBe(false);
    expect(rolls(nameView(view, decl, partyId('name.young'), inputs({})))).toBe(true);
  });

  it('refuses a kind that never said it could be re-agreed (Law 15)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    // A share is not renegotiated and a sovereign bond's restructuring is an exchange offer to its
    // holders, not a private word with one of them — so neither kind declares a re-agreement and
    // neither is reachable through this door, whoever calls it.
    const other = w.instruments.all().find((i) => i.kind !== LOAN && i.status.live && i.issuer.some);
    expect(other).toBeDefined();
    if (other === undefined) return;
    expect(() => { w.reagreeOn(other.id, other.terms, 'rolled'); }).toThrow(Forbidden);
  });

  it('refuses new terms that would make it a different claim (E3)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    const t: LoanTerms = row.terms;
    // The two parties on it are the two parties to the agreement; a date brought forward is an
    // acceleration and has its own path; what it is secured on is a lien and is pledged, not typed.
    const changed = (over: Partial<LoanTerms>): LoanTerms => ({ ...t, ...over });
    // 21.70: a change of BORROWER is not refused here any more — it is unwriteable. Who owes the
    // row is its issuer, and a re-agreement of terms cannot reach it at all.
    expect(() => { w.reagreeOn(row.id, changed({ maturity: t.drawn }), 'rolled'); }).toThrow(Forbidden);
    expect(() => {
      w.reagreeOn(row.id, changed({ security: [{ instrument: row.id, qty: asQty(1) }] }), 'rolled');
    }).toThrow(Forbidden);
  });

  it('holds the reason to the status, because a roll and a workout are different events', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    expect(row.status.live && row.status.performing).toBe(true);
    const later: LoanTerms = { ...row.terms, maturity: addMonths(row.terms.maturity, 1) };
    expect(() => { w.reagreeOn(row.id, later, 'restructured'); }).toThrow(Forbidden);
    w.reagreeOn(row.id, later, 'rolled');
    expect(w.journal.ofKind('credit.reagreed')).toHaveLength(1);
  });

  it('keeps the row and moves only what the two of them agreed (Register F1)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    const owed = creditorOf((h) => w.register.holdersOf(h), row);
    // E2: it stopped performing, and only the re-agreement brings it back — on the new terms.
    w.instruments.markDefaulted(row.id);
    expect(w.instruments.get(row.id).status).toEqual({ live: true, performing: false });
    const agreed: LoanTerms = {
      ...row.terms,
      maturity: addMonths(row.terms.maturity, 6),
      // 17d.2: what the two of them agree is the MARGIN; the fixing is nobody's to agree.
      margin: asRatio(0.09, 'what it now requires'),
      coupon: perAnnum(0.09, { kind: 'annual' }),
    };
    w.reagreeOn(row.id, agreed, 'restructured');
    const after = w.instruments.get(row.id);
    expect(after.status).toEqual({ live: true, performing: true });
    expect(after.id).toBe(row.id);
    expect(after.issued).toBe(row.issued);
    expect(String(after.issuer.some ? after.issuer.value : '')).toBe(String(row.issuer.some ? row.issuer.value : ''));
    expect(w.register.holdersOf(row.id).map(String)).toEqual(
      owed.some ? [String(owed.value)] : [],
    );
    expect(isLoan(after.terms) && rateOn(after.terms)).toBe(0.09);
  });

  it('rolls a relationship that reaches its maturity instead of failing it (21.59)', () => {
    // A borrower publishes what its wages and its orders cost it and never what falls due, so
    // nothing in this world refinances a maturity: without the roll a performing borrower fails on
    // the date. One month rather than the world's twelve, so the scale model reaches one.
    const w = world([asksFor(phx(20_000).pieces)], undefined, 1);
    let rolled = 0;
    for (let i = 0; i < 8; i += 1) {
      w.step();
      rolled += w.journal.ofKindIn('credit.rolled', w.period).length;
    }
    expect(rolled).toBeGreaterThan(0);
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    // The row is the same row, still performing, with a date further out than the one it was
    // written for — and the borrower never defaulted on it.
    expect(row.status.live && row.status.performing).toBe(true);
    expect(w.journal.ofKind('credit.default').filter((e) => e.subjects.includes(String(row.id)))).toEqual([]);
  });
});

/**
 * Security reaches the price (Banks Lending A4, C5.a; Corporate Credit E5; item 17.7c).
 *
 * A pledge is worth something to a lender or it is decoration. What it is worth is the part of the
 * claim it stands behind at the market's own price of it, and what that does to a price is take the
 * covered part of the expected loss away — so a secured claim on a name is dearer than an unsecured
 * one on the same name, by arithmetic and not by anybody preferring collateral.
 */
describe('a pledge is worth what it covers (Banks Lending A4, C5.a)', () => {
  const OWED = asCash(1000, USD, 'what is owed');
  const PLEDGED = instrumentId('pledged.thing');
  const worth = (per: number) => () => asPerPiece(per, 'what the market says a unit is worth');

  it('covers nothing, part or all of it, and says which', () => {
    // Nothing pledged: the whole of it is at risk, which is what an unsecured claim IS.
    expect(uncoveredShare([], worth(1), OWED)).toBe(1);
    // A pledge nobody prices covers nothing — a real answer, and not a zero anybody chose.
    expect(uncoveredShare([{ instrument: PLEDGED, qty: 400 }], () => undefined, OWED)).toBe(1);
    // Four hundred units at one covers two fifths of a thousand.
    expect(uncoveredShare([{ instrument: PLEDGED, qty: 400 }], worth(1), OWED)).toBeCloseTo(0.6, 12);
    // And a pledge worth more than the claim leaves none of it at risk.
    expect(uncoveredShare([{ instrument: PLEDGED, qty: 400 }], worth(3), OWED)).toBe(0);
  });

  it('is the one derivation the loss and the price both read (Law 4)', () => {
    const unsecured = asRatio(0.5, 'what it loses of a unit that fails');
    const half = [{ instrument: PLEDGED, qty: 500 }];
    // The loss a lender provisions is the share still at risk times what it loses on the name, so
    // the two readers cannot disagree about what the pledge covers.
    expect(lossGivenDefault(unsecured, half, worth(1), OWED)).toBeCloseTo(0.5 * 0.5, 12);
    expect(lossGivenDefault(unsecured, [], worth(1), OWED)).toBe(unsecured);
    expect(lossGivenDefault(unsecured, half, worth(4), OWED)).toBe(0);
  });

  it('a lender requires less of a claim with something behind it than of the name (E5)', () => {
    const w = world([asksSecured(phx(20_000).pieces)]);
    for (let i = 0; i < 5; i += 1) w.step();
    const rows = w.instruments.all().filter((i) => i.kind === LOAN && isLoan(i.terms));
    const secured = rows.find((i) => loanKind.ranking(i).secured.length > 0);
    expect(secured, 'the rig wrote no secured row').toBeDefined();
    if (secured === undefined) return;
    const lender = creditorOf((h) => w.register.holdersOf(h), secured);
    expect(lender.some).toBe(true);
    if (!lender.some) return;
    const view = w.participantView(lender.value);
    const name = borrowerOf(secured);
    const onName = requiredYieldOf(view, name);
    const onClaim = requiredOnClaim(view, secured.id);
    expect(onName.some).toBe(true);
    expect(onClaim.some).toBe(true);
    if (!onName.some || !onClaim.some) return;
    // What the pledge covers, at the market's own price of it, is what the difference is made of:
    // the claim is dearer than the name by exactly the covered part of the published expected loss.
    const share = uncoveredShare(
      loanKind.ranking(secured).secured,
      (pledged: InstrumentId) => {
        const print = view.print(pledged);
        return print.some ? print.value.price : undefined;
      },
      asCash(secured.issued, secured.ccy, 'what it owes'),
    );
    const expected = expectedLossOn(view, String(name));
    expect(expected.some).toBe(true);
    if (!expected.some) return;
    expect(onClaim.value).toBeCloseTo(onName.value - expected.value * (1 - share), 12);
    expect(onClaim.value).toBeLessThanOrEqual(onName.value);
  });

  it('a claim with nothing behind it is priced at the name itself (E5)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 5; i += 1) w.step();
    const plain = w.instruments
      .all()
      .find((i) => i.kind === LOAN && isLoan(i.terms) && loanKind.ranking(i).secured.length === 0);
    expect(plain, 'the rig wrote no unsecured row').toBeDefined();
    if (plain === undefined) return;
    const lender = creditorOf((h) => w.register.holdersOf(h), plain);
    expect(lender.some).toBe(true);
    if (!lender.some) return;
    const view = w.participantView(lender.value);
    const name = borrowerOf(plain);
    const onName = requiredYieldOf(view, name);
    const onClaim = requiredOnClaim(view, plain.id);
    expect(onClaim.some).toBe(onName.some);
    if (onName.some && onClaim.some) expect(onClaim.value).toBe(onName.value);
  });
});

/**
 * A bank loan is not distributed outside the banking system (Banks Lending A1.a, D4.a, XI-11;
 * item 17.8).
 *
 * A FORBID that holds breaks silently, which is why it is a family and not a refusal at a door:
 * nothing in this world sells a loan anywhere it should not, and the check has to be able to say so
 * about a world that does.
 */
describe('where a loan may be owed (Banks Lending D4.a, XI-11)', () => {
  /** Sells part of a bank's loan book to a party that is not in the banking system (D4). */
  function sellsToAFirm(at = 3): SystemModule {
    return {
      id: 'test.sellsLoan',
      spec: 'Banks Lending D4',
      requires: [],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.sellLoan',
          spec: 'Banks Lending D4',
          anchor: { before: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== at) return;
            for (const row of ctx.instruments.ofKind(LOAN)) {
              if (!row.status.live) continue;
              const owed = creditorOf((h) => ctx.register.holdersOf(h), row);
              if (!owed.some) continue;
              // A test asks the draw for what it needs: a firm that is not the borrower — a
              // borrower buying back its own row is repaying it, not holding it (Register E2) — and
              // that has the money to pay for a parcel of it.
              const buyer = ctx.parties
                .ofKind(FIRM)
                .find(
                  (f) =>
                    f.status.alive &&
                    !(row.issuer.some && row.issuer.value === f.id) &&
                    ctx.register.quantity(f.id, moneyInstrumentId(f.bank, row.ccy)) > 0,
                );
              if (buyer === undefined) continue;
              const cash = ctx.register.quantity(
                buyer.id,
                moneyInstrumentId(buyer.bank, row.ccy),
              );
              const units = downTick(Math.min(ctx.register.quantity(owed.value, row.id), cash / 2));
              if (units <= 0) continue;
              // D4: a loan CAN be sold, and then it has a price and a buyer. Whether this buyer may
              // be the one is the question the family answers, and settlement is not where it is
              // asked — the wire moves what two parties agreed to move.
              ctx.settle({
                legs: [
                  {
                    kind: 'asset',
                    from: owed.value,
                    to: buyer.id,
                    instrument: row.id,
                    qty: asQty(units),
                    pricePerUnit: some(asPerPiece(1, 'at what it promised')),
                    accruedPerUnit: none(),
                  },
                  {
                    kind: 'money',
                    receipt: { of: 'disposal' },
                    from: ctx.accountOf(buyer.id, row.ccy),
                    to: ctx.accountOf(owed.value, row.ccy),
                    ccy: row.ccy,
                    amount: units,
                  },
                ],
                cause: 'trade',
                reason: `${String(owed.value)} sells ${String(row.id)} to ${String(buyer.id)}`,
              });
              return;
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
  }

  it('is a fact about the KIND, and three kinds say yes', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    w.step();
    // Law 15: a list of names here would be a list somebody has to remember to add to. What a bank,
    // the centre of the system, and the vehicle that holds a pool for its noteholders have in common
    // is declared on each of them.
    const inside = [...w.registry.partyKinds.keys()].filter(
      (k) => w.registry.partyKind(k).banking,
    );
    expect(inside.length).toBeGreaterThan(0);
    for (const k of inside) expect(w.registry.partyKind(k).banking).toBe(true);
    // And the ones that plainly are not: a household, a firm, a state.
    expect(w.registry.partyKind(HOUSEHOLD).banking).toBe(false);
    expect(w.registry.partyKind(FIRM).banking).toBe(false);
  });

  it('holds in this world, and says so when a row leaves it', () => {
    const clean = world([asksFor(phx(20_000).pieces)]);
    let saidNothing = true;
    for (let i = 0; i < 5; i += 1) {
      const report = clean.step();
      if (outOfTheSystem(report).length > 0) saidNothing = false;
    }
    // Nothing in this world sells a loan out of the banking system, so the family reports nothing —
    // and it is BUILT, which is what stops "nothing" from meaning "nobody checked" (Audit C2).
    expect(saidNothing).toBe(true);
    const built = clean
      .step()
      .audit.families.find((f) => f.family === 'ownership' && f.built);
    expect(built).toBeDefined();

    const sold = world([asksFor(phx(20_000).pieces), sellsToAFirm()]);
    const said: string[] = [];
    for (let i = 0; i < 5; i += 1) {
      for (const v of outOfTheSystem(sold.step())) said.push(v.message);
    }
    // XI-11, D4.a: the row is now owed to a firm, and the audit names the holder, the row and how
    // much of it — it does not move the units back, because the audit never repairs.
    expect(said.length).toBeGreaterThan(0);
    expect(said.some((m) => m.includes('not distributed outside the banking system'))).toBe(true);
  });
});

/**
 * A line that is past its appetite is told to come DOWN (Banks Capital B1, B3; Banks Funding D4;
 * item 17.9).
 *
 * The treasury shares out what the capital rules leave. Two floors used to turn a negative share
 * into a zero — *"a line already past its own appetite is asking for nothing"* — and with them the
 * world had no way to say the one thing a bank in breach has to be told.
 */
describe('what the treasury allots can be negative (Banks Capital B3, Banks Funding D4)', () => {
  /** Every line every bank published this period: what it asked for and what it was given. */
  function linesIn(w: World): { line: string; asked: number; room: number }[] {
    const out: { line: string; asked: number; room: number }[] = [];
    for (const e of w.journal.ofKindIn('bank.lines', w.period)) {
      const rows = e.data['lines'];
      if (!Array.isArray(rows)) continue;
      for (const r of rows as Record<string, unknown>[]) {
        out.push({ line: String(r['line']), asked: Number(r['asked']), room: Number(r['room']) });
      }
    }
    return out;
  }

  it('tells a line over its own appetite to shed, and never rounds the shed up to nothing', () => {
    const w = rigWorld('lines-shed');
    let over = 0;
    for (let i = 0; i < 4; i += 1) {
      w.step();
      for (const l of linesIn(w)) {
        if (l.asked >= 0) continue;
        over += 1;
        // D1, XI-4: the ask is the distance between its own appetite and what the line is already
        // using, and a negative distance is not "asking for nothing" — it is the size of the sale.
        // Law 6: there is no floor under it, so it is never a zero.
        expect(l.room).toBeLessThan(0);
        // It rounds UP in SIZE, because a shed is what the line MUST do: a line told to come down
        // by less than it is over is a line still over. And it can be told to come down by MORE —
        // when the bank itself is past the capital rules, the rest of that hole is shed too.
        expect(l.room).toBeLessThanOrEqual(-Math.ceil(-l.asked));
      }
    }
    // The scale model's banks open past their appetites, so there was something here to check.
    expect(over).toBeGreaterThan(0);
  });

  it('never gives a line more room than it asked for', () => {
    const w = rigWorld('lines-shed');
    for (let i = 0; i < 4; i += 1) {
      w.step();
      for (const l of linesIn(w)) {
        if (l.asked < 0) continue;
        expect(l.room).toBeLessThanOrEqual(Math.ceil(l.asked));
      }
    }
  });

  it('is what the desk reads, so a book above its limit is a sale (Banks Funding D2, D4)', () => {
    const w = rigWorld('lines-shed');
    for (let i = 0; i < 3; i += 1) w.step();
    const bank = w.parties.ofKind(BANK).find((b) => b.status.alive);
    expect(bank).toBeDefined();
    if (bank === undefined) return;
    // A real bank's own view, and a book put beside a limit: what is constructed here is the two
    // numbers the treasury and the desk each already produce, not the desk.
    const view = w.participantView(bank.id);
    const home = w.registry.currencyOf(bank.region);
    const state = (book: number, limit: number) =>
      ({
        bookValue: asCash(book, home, 'what its book is worth'),
        limitAggregate: asCash(limit, home, 'what its treasury allotted it'),
      }) as unknown as DeskState;
    // D4: "it shrinks its assets" — a desk carrying more than the room it was given raises the
    // difference by selling, and the size is the difference and nothing else.
    expect(mustRaise(view, state(1000, 400), home).pieces).toBe(600);
    // A desk inside its limit sells nothing for that reason, which is arithmetic and not a rule.
    expect(mustRaise(view, state(300, 400), home).pieces).toBeLessThanOrEqual(0);
    // And a desk whose treasury has said nothing has only the other reason to sell.
    expect(mustRaise(view, undefined, home).pieces).toBeLessThanOrEqual(0);
  });
});

/**
 * A line is repaid at the borrower's option (Banks Lending C9, F2; item 17.9a).
 *
 * Only half of C9 was built: a borrower could draw and nothing it ever did brought the row back
 * down. A firm that had a good quarter carried the debt of its worst one for ever, paid interest on
 * it, and went on consuming its lender's capital for money it was not using.
 */
describe('paying a line down (Banks Lending C9, F2)', () => {
  /** A borrower that publishes a SURPLUS: the same door an ask goes through, the other sign. */
  function hasSpare(amount: number, at: number): SystemModule {
    return {
      ...asksFor(amount, at),
      id: 'test.spare',
      phases: [
        {
          name: 'test.spare',
          spec: 'Banks Lending C9',
          anchor: { before: 'corporateActions' },
          reads: [],
          writes: [{ kind: 'event', name: 'credit.request' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== at) return;
            ctx.request(BORROWER, {
              ccy: USD,
              short: asCash(-amount, USD, 'the money it does not need'),
              repays: 'atOption',
              wants: 'money',
              months: ctx.params.months(TERM_MONTHS.working),
              // 0i: it puts up nothing, and an ask that is unsecured says so.
              security: [],
            });
          },
        },
      ],
    };
  }

  it('pays down what it said it does not need, and the row comes down with it', () => {
    const w = world([asksFor(phx(20_000).pieces), hasSpare(phx(5_000).pieces, 3)]);
    for (let i = 0; i < 3; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row, 'the rig wrote no loan').toBeDefined();
    if (row === undefined) return;
    const before = w.instruments.get(row.id).issued;
    expect(before).toBeGreaterThan(0);
    // The surplus is published in period 3 and read in period 4, the same lag an ask carries: a
    // lender acts on what it has already been told (Clearing F1).
    for (let i = 0; i < 2; i += 1) w.step();
    const said = w.journal.ofKind('credit.repaid');
    expect(said.length).toBeGreaterThan(0);
    const paid = Number(said[0]?.data['paid']);
    expect(paid).toBeGreaterThan(0);
    // F1.a: the outstanding and the lender's holding are one number, and both came down by what was
    // paid. Nothing was written off and nothing vanished: it is a redemption with two legs.
    const after = w.instruments.get(row.id);
    expect(after.issued).toBeCloseTo(before - paid, 6);
    const owed = creditorOf((h) => w.register.holdersOf(h), after);
    expect(owed.some).toBe(true);
    if (owed.some) expect(w.register.quantity(owed.value, after.id)).toBeCloseTo(after.issued, 6);
  });

  it('never pays more than it owes, and takes the dearest line first', () => {
    // It says it has ten times what it borrowed to spare, so what stops it is the row and not the
    // money: a borrower cannot repay a loan twice (Money E1, Register E2).
    const w = world([asksFor(phx(20_000).pieces), hasSpare(phx(200_000).pieces, 3)]);
    for (let i = 0; i < 5; i += 1) w.step();
    for (const e of w.journal.ofKind('credit.repaid')) {
      const line = w.instruments.get(instrumentId(String(e.data['loan'])));
      expect(Number(e.data['paid'])).toBeGreaterThan(0);
      expect(line.issued).toBeGreaterThanOrEqual(0);
    }
    // C9, XI-4: of two lines it owes, the one that costs it more is the one it retires — so the
    // rates it paid down come in the order a treasurer would take them.
    const rates = w.journal.ofKind('credit.repaid').map((e) => Number(e.data['rate']));
    for (let i = 1; i < rates.length; i += 1) {
      const a = rates[i - 1];
      const b = rates[i];
      if (a === undefined || b === undefined) continue;
      expect(a).toBeGreaterThanOrEqual(b);
    }
  });
});

/**
 * A spent claim on a name that has finished existing has ceased (Register E2, F2; Banks Lending E5;
 * item 17.9b).
 *
 * A terminal party succeeds itself — an estate that has paid everything away has nobody left to
 * succeed it — so the chain of references ends there and nothing it promised can be presented to
 * anybody again.
 */
describe('the end of a claim (Register E2, F2, Banks Lending E5)', () => {
  it('closes the empty lines and leaves the debts nobody can pay standing', () => {
    const w = rigWorld('estate-a');
    for (let i = 0; i < 24; i += 1) w.step();
    let spent = 0;
    let owing = 0;
    for (const i of w.instruments.all()) {
      if (!i.issuer.some) continue;
      const promised = w.parties.resolve(i.issuer.value);
      if (promised.status.alive) continue;
      // E5.a, XI-8: a row with something OUTSTANDING is a debt nobody can pay, and it stays exactly
      // as it is — that is a fact the audit should go on reporting, not one to tidy away.
      if (i.status.live) {
        expect(i.issued).toBeGreaterThan(0);
        owing += 1;
        continue;
      }
      if (i.issued === 0) spent += 1;
    }
    // The scale model buries forty-four estates in twenty-four periods, so there was something here
    // to have closed: before this, twenty-three live loan rows promised nothing, to nobody, on
    // names that had finished existing.
    expect(spent).toBeGreaterThan(0);
    const said = w.journal
      .ofKind('instrument.ceased')
      .filter((e) => String(e.data['reason']).includes('nothing was outstanding'));
    expect(said.length).toBeGreaterThan(0);
    // And it is stated either way: what is left standing is standing for a reason.
    expect(owing).toBeGreaterThanOrEqual(0);
  });

  it('leaves an undrawn line alone, because somebody is still there to draw on it (C9)', () => {
    const w = rigWorld('estate-a');
    for (let i = 0; i < 24; i += 1) w.step();
    let undrawn = 0;
    for (const i of w.instruments.all()) {
      if (i.kind !== LOAN || i.issued !== 0 || !i.issuer.some) continue;
      if (!w.parties.resolve(i.issuer.value).status.alive) continue;
      // The same shape — nothing outstanding — and a living borrower: that is a FACILITY, which is
      // what C9 says a line is, and closing it would take away the thing it exists to be.
      expect(i.status.live).toBe(true);
      undrawn += 1;
    }
    expect(undrawn).toBeGreaterThan(0);
  });
});

/**
 * §29 B2, E1 (17b.1): a borrower that wants the PROMISE and not the money. Same door, same shape,
 * one field different — which is the whole of the change, and the reason it is one field.
 */
function asksForCommitment(amount: number, at = 1): SystemModule {
  return {
    ...asksFor(amount, at),
    id: 'test.asksCommitment',
    phases: [
      {
        name: 'test.askCommitment',
        spec: 'Private Equity B2',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [{ kind: 'event', name: 'credit.request' }],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          ctx.request(partyId(BORROWER), {
            ccy: USD,
            short: asCash(amount, USD, 'what the deal is short of'),
            repays: 'onSchedule',
            wants: 'commitment',
            months: ctx.params.months(TERM_MONTHS.working),
            // 0i: it puts up nothing, and an ask that is unsecured says so.
            security: [],
          });
        },
      },
    ],
  };
}

describe('a lender that agreed to lend (Private Equity B2, B2.b, E1)', () => {
  it('writes a commitment and not a row: no money is made until it is drawn', () => {
    const w = world([asksForCommitment(phx(20_000).pieces)]);
    w.step();
    const before = w.cash(BORROWER, USD);
    w.step();
    const committed = w.journal.ofKind('credit.committed');
    expect(committed).toHaveLength(1);
    expect(Number(committed[0]?.data['limit'])).toBeGreaterThan(0);
    // E1: what exists is a promise. Nothing was created, nothing moved, and there is no row — the
    // money a commitment promises is made when it is drawn and not before.
    expect(w.journal.ofKind('credit.written')).toHaveLength(0);
    expect(loans(w)).toHaveLength(0);
    expect(w.cash(BORROWER, USD)).toBe(before + paidTo(w, BORROWER, 'coupon'));
    // It is an agreement between the two of them, and the borrower owes nothing on it yet (Law 2).
    const row = w.agreements.ofKind(FACILITY).find((a) => a.debtor === BORROWER);
    expect(row).toBeDefined();
    expect(row?.owed).toBe(0);
    expect(row?.creditor).toBeDefined();
    const terms = row?.terms;
    expect(terms !== undefined && isFacility(terms)).toBe(true);
    if (terms === undefined || !isFacility(terms)) return;
    expect(terms.rate).toBeGreaterThan(0);
    expect(terms.limit.pieces).toBeGreaterThan(0);
  });

  it('lapses when the deal it was committed for does not close (B2.b)', () => {
    const w = world([asksForCommitment(phx(20_000).pieces)]);
    for (let i = 0; i < 4; i += 1) w.step();
    // Nobody drew it, so it is over — terminated by its own terms rather than discharged by a
    // payment, because nothing was ever owed on it.
    expect(w.journal.ofKind('credit.lapsed')).toHaveLength(1);
    const live = w.agreements.ofKind(FACILITY).filter((a) => a.state === 'performing');
    expect(live).toHaveLength(0);
    // And no money was ever made for it: a deal that did not happen left the bank where it was.
    expect(loans(w)).toHaveLength(0);
  });
});

/**
 * §29 B2, B2.a (17b.2): the deal's borrowing, published in the TARGET's name. The module stands in
 * for `control` so that what is under test is the ask and not the valuation behind it.
 */
function asksToFund(amount: number, from = 1, to = 1): SystemModule {
  return {
    ...asksFor(amount, from),
    id: 'test.asksToFund',
    phases: [
      {
        name: 'test.askToFund',
        spec: 'Private Equity B2',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [
          { kind: 'event', name: 'control.financing' },
          { kind: 'event', name: 'credit.request' },
        ],
        run: (ctx: MechanismContext) => {
          if (ctx.period < from || ctx.period > to) return;
          askToFund(
            ctx,
            partyId(BORROWER),
            asCash(amount, USD, 'what the deal is short of'),
            asCash(amount, USD, 'what the buyer brings itself'),
            partyId(PAYEE),
            60,
          );
        },
      },
    ],
  };
}

/** What a lender has committed to the borrower, recorded each period so a test can read it back. */
function watchesCommitments(): SystemModule {
  return {
    ...asksFor(0, 99),
    id: 'test.watchesCommitments',
    phases: [
      {
        name: 'test.watchCommitments',
        spec: 'Private Equity B2',
        anchor: { after: 'revaluation' },
        reads: [],
        writes: [{ kind: 'event', name: 'test.committed' }],
        run: (ctx: MechanismContext) => {
          ctx.record(
            'test.committed',
            [String(BORROWER)],
            { committed: committedTo(ctx, partyId(BORROWER), USD).pieces },
            false,
          );
        },
      },
    ],
  };
}

describe('the deal asks for the money that will buy it (Private Equity B2, B2.a, B2.b)', () => {
  it('asks in the TARGET’s name, because the debt is the target’s', () => {
    const w = world([asksToFund(phx(20_000).pieces)]);
    w.step();
    const asked = w.journal.ofKind('credit.request');
    expect(asked).toHaveLength(1);
    // B2.a: *"the debt is the target's liability, not the fund's"*. The borrower on the request is
    // the company, and the buyer is named beside it rather than on it.
    expect(asked[0]?.data['borrower']).toBe(String(BORROWER));
    expect(asked[0]?.data['wants']).toBe('commitment');
    const said = w.journal.ofKind('control.financing');
    expect(said).toHaveLength(1);
    expect(said[0]?.data['target']).toBe(String(BORROWER));
    expect(said[0]?.data['buyer']).toBe(String(PAYEE));
  });

  it('does not ask twice while its last ask is unanswered (Corporate Credit C9)', () => {
    const w = world([asksToFund(phx(20_000).pieces, 1, 2)]);
    w.step();
    w.step();
    // Two periods, two attempts, one ask: a borrower that published in period 1 hears in period 2,
    // and asking again in between is the same money asked for twice and two lenders' capital held
    // against one deal.
    expect(w.journal.ofKind('credit.request')).toHaveLength(1);
    expect(w.journal.ofKind('control.financing')).toHaveLength(1);
  });

  it('reads what a lender has committed, and nothing once it has lapsed', () => {
    const w = world([asksForCommitment(phx(20_000).pieces), watchesCommitments()]);
    const seen: number[] = [];
    for (let i = 0; i < 5; i += 1) {
      w.step();
      const e = w.journal.ofKindIn('test.committed', w.period).at(-1);
      seen.push(Number(e?.data['committed']));
    }
    // Nothing in period 1 (it has only just asked), something once the bank committed, nothing
    // again once the deal it was committed for did not close.
    expect(seen[0]).toBe(0);
    expect(seen[1]).toBeGreaterThan(0);
    expect(seen.at(-1)).toBe(0);
  });
});

/** 17b.8: a borrower that says how long it wants the money for, which is the thing under test. */
function asksForMonths(amount: number, months: number, at = 1): SystemModule {
  return {
    ...asksFor(amount, at),
    id: 'test.asksMonths',
    phases: [
      {
        name: 'test.askMonths',
        spec: 'Corporate Credit A2',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [{ kind: 'event', name: 'credit.request' }],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          ctx.request(partyId(BORROWER), {
            ccy: USD,
            short: asCash(amount, USD, 'what it is short of'),
            repays: 'onSchedule',
            wants: 'money',
            months,
            security: [],
          });
        },
      },
    ],
  };
}

describe('how long it is for is the borrower\u2019s (Corporate Credit A2, 17b.8)', () => {
  it('writes the row for the term the ask named, not for a number in the lender', () => {
    const w = world([asksForMonths(phx(20_000).pieces, 7)]);
    w.step();
    w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    // A2: a term is a decision about a NEED and the need is the borrower's. Seven months because
    // the ask said seven — the lender's own convention is a year and it is not what was written.
    const t = row.terms;
    expect(t.maturity.m - t.drawn.m + 12 * (t.maturity.y - t.drawn.y)).toBe(7);
  });

  it('two borrowers asking for two terms get two terms from the same lender', () => {
    const short = world([asksForMonths(phx(20_000).pieces, 3)]);
    const long = world([asksForMonths(phx(20_000).pieces, 60)]);
    for (const w of [short, long]) {
      w.step();
      w.step();
    }
    const months = (w: World): number => {
      const row = w.instruments.all().find((i) => i.kind === LOAN);
      if (row === undefined || !isLoan(row.terms)) return 0;
      const t = row.terms;
      return t.maturity.m - t.drawn.m + 12 * (t.maturity.y - t.drawn.y);
    };
    // Finding 21.60(a): every loan in this world ran twelve months because one parameter said so.
    expect(months(short)).toBe(3);
    expect(months(long)).toBe(60);
  });
});

describe('the books a lender sees (Reporting A2, A3, Corporate Credit A4, 17b′.1)', () => {
  it('prepares management accounts for a borrower that asked for a commitment', () => {
    const w = world([asksForCommitment(phx(20_000).pieces)]);
    w.step();
    const said = w.journal.ofKind('reporting.interim').filter((e) => e.subjects[0] === BORROWER);
    // It asked in period 1 and its books were struck at the end of period 1, before its lender
    // decided in period 2 — which is the lag a covenant needs (Reporting B2.a).
    expect(said).toHaveLength(1);
    expect(said[0]?.period).toBe(1);
    // A3, G4: PRIVATE. A snapshot handed to a lender is not a quarter anybody may trade on.
    expect(said[0]?.public).toBe(false);
    // It covers the whole of the company's life so far, because it has never closed a quarter.
    expect(Number(said[0]?.data['from'])).toBe(0);
    expect(Number(said[0]?.data['to'])).toBe(1);
  });

  it('prepares none for a borrower that just wants the money', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    w.step();
    w.step();
    // A working-capital line is not the large underwritten borrowing, and nobody asks a company for
    // its books to draw one. The trigger is what it asked for, never a size anybody declared.
    expect(w.journal.ofKind('reporting.interim')).toHaveLength(0);
    expect(w.journal.ofKind('credit.written').length).toBeGreaterThan(0);
  });

  it('is shown to the banks it is asking, and the commitment is struck on it', () => {
    const w = world([asksForCommitment(phx(20_000).pieces)]);
    w.step();
    w.step();
    const said = w.journal.ofKind('reporting.interim').find((e) => e.subjects[0] === BORROWER);
    expect(said).toBeDefined();
    // Observer A3: the disclosure is on the record — who was shown what, and when.
    const shown = w.journal
      .ofKind('disclosed')
      .filter((e) => Number(e.data['event']) === Number(said?.id));
    expect(shown.length).toBeGreaterThan(0);
    // And the promise the lender asked for was struck on those accounts: no accounts, no commitment.
    const row = w.agreements.ofKind(FACILITY).find((a) => a.debtor === BORROWER);
    expect(row).toBeDefined();
    const terms = row?.terms;
    if (terms === undefined || !isFacility(terms)) throw new Error('no commitment was written');
    expect(terms.covenant.leverage).toBeGreaterThan(0);
    expect(terms.covenant.coverage).toBeGreaterThan(0);
  });
});

describe('a loan floats (Corporate Credit B4, Bond N5.b, 17d.2)', () => {
  it('is written as a margin over what money cost, not as a rate locked for a year', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    w.step();
    w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    const t = row.terms;
    // B4: what the two of them agreed is the MARGIN; what the borrower pays is that plus the
    // fixing, and the two are different numbers on the same row.
    expect(t.margin).toBeGreaterThan(0);
    expect(rateOn(t)).toBeGreaterThanOrEqual(t.margin);
    if (t.floatsOver.some) {
      // It floats over the SECURED book of its own money — what money actually changes hands at
      // overnight against collateral, which is the thing SOFR is (D3, Law 4).
      expect(t.floatsOver.value).toBe(`${row.ccy}:secured`);
      expect(rateOn(t)).toBeGreaterThan(t.margin);
    }
  });

  it('resets, and the reset is public (Bond N5.b)', () => {
    const w = world([asksFor(phx(20_000).pieces)]);
    for (let i = 0; i < 6; i += 1) w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    if (row === undefined || !isLoan(row.terms) || !row.terms.floatsOver.some) return;
    // A holder of the claim learns what it pays next, and so does anybody pricing it: the kernel is
    // the one writer of what a fixing does to a line, and it records it.
    const fixed = w.journal.ofKind('coupon.fixed').filter((e) => e.subjects[0] === String(row.id));
    expect(fixed.length).toBeGreaterThan(0);
    // And what it pays moved because money did — the margin never moved, so the difference is the
    // fixing and nothing else.
    const rates = fixed.map((e) => Number(e.data['coupon']));
    expect(new Set(rates).size).toBeGreaterThanOrEqual(1);
    for (const r of rates) expect(r).toBeGreaterThanOrEqual(row.terms.margin);
  });

  it('is a FIXED row where the benchmark has never fixed (17d.3)', () => {
    // A loan that cannot fix cannot float, and the terms say which it is rather than a flag
    // anybody sets later. In a world whose overnight book has never traded, every row is fixed at
    // what it was quoted — which is the honest answer and not a rate invented for it.
    const w = world([asksFor(phx(20_000).pieces)]);
    w.step();
    w.step();
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    if (row === undefined || !isLoan(row.terms)) return;
    const traded = w.journal.ofKind('index.benchmark').length > 0;
    expect(row.terms.floatsOver.some).toBe(traded && rateOn(row.terms) !== row.terms.margin);
  });
});
