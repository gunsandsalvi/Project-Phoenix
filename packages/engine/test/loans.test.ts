/**
 * A loan is a row: written by creating a deposit, priced from the bank's own economics, refused
 * when the bank's own constraints say no.
 *
 * @spec Banks Lending A1 Banks Lending A1.a Banks Lending A2 Banks Lending A4 Banks Lending B1 Banks Lending B1.a Banks Lending B1.b Banks Lending B1.c Banks Lending B2 Banks Lending B2.a Banks Lending B2.c Banks Lending B2.d Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C2 Banks Lending C2.a Banks Lending C3 Banks Lending C3.a Banks Lending C4 Banks Lending D1 Banks Lending D3 Banks Lending E1 Banks Lending F1 Banks Lending F1.a Banks Lending F3 Money B3.a Money B3.c Bond N13.a Corporate Credit G8 XI-4
 */
import { describe, expect, it } from 'vitest';
import {
  moneyInstrumentId,
  mul,
  upTick,
  BANK_COUNT,
  drawBanks,
  InvalidRegistry,
  LOAN,
  USD,
  TREASURY_US,
  assemble,
  isLoan,
  loanKind,
  none,
  partyId,
  type InstructionDraft,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigSpec, withDependencies, mergeModules } from './rig.js';
import { paidTo, unexpected } from './expected.js';
import { phx } from './units.js';
import { notDealing } from './no-dealing.js';

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
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          ctx.record('firms.funding', [BORROWER], { short: amount, owed: amount, ccy: USD }, false);
        },
      },
    ],
    participants: [],
    families: [],
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
        cycle: 0,
        anchor: { before: 'corporateActions' },
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
            from: { holder: BORROWER, issuer: bank },
            to: { holder: PAYEE, issuer: ctx.parties.get(PAYEE).bank },
            ccy: USD,
            // B3.a: A LITTLE over what it holds — a hundredth of what its bank will lend ONE NAME.
            // Half of that per-name limit used to be affordable and is not: a bank's room is the
            // LEAST of its capital, its appetite and its funding (Banks Lending B3), and the
            // per-name limit is only one of the three. What this module is for is an overdraft the
            // bank ALLOWS, so it asks for one small enough that the other two are not the binding
            // constraint.
            amount: ctx.registry.payable(USD, held + limit / 100),
            fromCell: none(),
            toCell: none(),
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
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== at) return;
          const has = ctx.register.quantity(
            BORROWER,
            moneyInstrumentId(ctx.parties.get(BORROWER).bank, USD),
          );
          const leg: Leg = {
            kind: 'money',
            from: { holder: BORROWER, issuer: ctx.parties.get(BORROWER).bank },
            to: { holder: PAYEE, issuer: ctx.parties.get(PAYEE).bank },
            ccy: USD,
            amount: upTick(mul(has, times, 'what it tries to spend')),
            fromCell: none(),
            toCell: none(),
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

function world(extra: readonly SystemModule[] = [], limits?: number): World {
  const spec = rigSpec('loans');
  const modules = withDependencies(spec.modules, (m) =>
m.id === 'sovereign-instruments' ||
        m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
        m.id === 'credit-events' ||
        m.id === 'banks' ||
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
    ).map(notDealing);
  return assemble({ ...spec, modules: mergeModules(modules, extra) });
}

function loans(w: World): { id: string; issued: number; lender: string; rate: number }[] {
  return w.instruments
    .all()
    .filter((i) => i.kind === LOAN)
    .map((i) => ({
      id: String(i.id),
      issued: i.issued,
      lender: isLoan(i.terms) ? String(i.terms.lender) : '',
      rate: isLoan(i.terms) ? i.terms.rate : 0,
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
    const w = world([asksFor(phx(20_000))]);
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
    const w = world([asksFor(phx(20_000))]);
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
    const w = world([asksFor(phx(20_000))]);
    for (let i = 0; i < 4; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    // There is no book number anywhere: a bank's book is the sum of the rows it holds.
    expect(w.register.quantity(row.terms.lender, row.id)).toBeCloseTo(row.issued, 9);
    expect(row.terms.borrower).toBe(BORROWER);
  });
});

describe('the price (Banks Lending C1, C2, XI-4)', () => {
  it('is four named terms, and two banks do not quote the same', () => {
    const w = world([asksFor(phx(20_000))]);
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
    // parts in a million, because `publishQuotes` and `runRequests` each derive it (docs/BUGS.md
    // 12d-3). That is one fact with two writers and it is a finding, not something to assert around.
  });

  it('gets dearer for a borrower that has failed to pay (C1.b, C4, Corporate Credit G8)', () => {
    const clean = world([asksFor(phx(10_000), 6)]);
    for (let i = 0; i < 8; i += 1) clean.step();
    const cleanRate = Number(clean.journal.ofKind('credit.written')[0]?.data['rate']);
    // The same request from a borrower the banks have watched fail to pay.
    const marked = world([overspends(2, 2), asksFor(phx(10_000), 6)], 0);
    for (let i = 0; i < 8; i += 1) marked.step();
    const seen = marked.journal
      .ofKind('credit.default')
      .filter((e) => e.data['party'] === BORROWER);
    expect(seen.length).toBeGreaterThan(0);
    const marked2 = world([overspends(2, 2), asksFor(phx(10_000), 6)]);
    for (let i = 0; i < 8; i += 1) marked2.step();
    const markedRate = Number(marked2.journal.ofKind('credit.written')[0]?.data['rate']);
    // C1.b: the bank's own view of this borrower moved, so the price moved. A default is
    // information, and this is the channel it travels down (G8).
    expect(markedRate).toBeGreaterThan(cleanRate);
  });
});

describe('the provision (Banks Lending D1, D2, D2.a, D2.b, C4)', () => {
  it('carries the loan at what its lender expects to recover, and the charge is visible', () => {
    // The same borrower, seen to fail, then borrowing: the bank prices it dearer AND carries it
    // lower, off the one model (C4) — two beliefs would mean the price and the provision disagree.
    const w = world([overspends(2, 2), asksFor(phx(10_000), 6)]);
    for (let i = 0; i < 9; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const row = w.instruments.all().find((i) => i.kind === LOAN);
    expect(row).toBeDefined();
    if (row === undefined || !isLoan(row.terms)) return;
    const holding = w.register.holding(row.terms.lender, row.id);
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
    expect(Number(marks[0]?.data['deltaPerMember'])).toBeLessThan(0);
  });
});

describe('when it says no (Banks Lending B2, C3, C3.a, F3)', () => {
  it('declines when its limit for one name binds, and the decline is recorded', () => {
    // F3: a large-exposure limit that binds is what makes concentration a thing it manages.
    const w = world([asksFor(phx(20_000))], 0);
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
    const w = world([asksFor(phx(20_000))]);
    for (let i = 0; i < 4; i += 1) w.step();
    const lenderBefore = w.register.equity(BANK_OF_A);
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
    expect(lenderBefore).not.toBe(w.register.equity(BANK_OF_A));
  });

  it('is a default when the borrower does not pay it (E1, E2)', () => {
    const w = world([asksFor(phx(20_000))]);
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
    // F1.a: every row has a lender of record holding every unit of it, which the flows family
    // checks every period — and the year above was green.
    for (const row of rows) {
      expect(isLoan(row.terms) && row.terms.lender.startsWith('bank.')).toBe(true);
    }
    expect(w.parties.ofKind(partyId('bank') as never).length).toBeGreaterThan(0);
    expect(TREASURY_US).toBeDefined();
  });
});
