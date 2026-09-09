/**
 * Banks lending: a loan is a row, written by creating a deposit, priced from the bank's own
 * economics, and refused when the bank's own constraints say no.
 *
 * @spec Banks Lending A1 Banks Lending A1.a Banks Lending A2 Banks Lending A4 Banks Lending B1 Banks Lending B1.a Banks Lending B1.b Banks Lending B1.c Banks Lending B2 Banks Lending B2.a Banks Lending B2.c Banks Lending B2.d Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C2 Banks Lending C2.a Banks Lending C3 Banks Lending C3.a Banks Lending C4 Banks Lending D1 Banks Lending D3 Banks Lending F1 Banks Lending F1.a Banks Lending F2 Banks Lending F3 Money B3.a Money B3.c XI-4 Law 2 Law 15
 *
 * B1 IS THE WHOLE OF ENDOGENOUS MONEY, and it is one instruction: the loan is issued by the
 * borrower to the bank, and the bank's own money is created into the borrower's account, in the
 * same instant. Both sides of both parties move at once and neither is left over. **No reserve
 * leaves the bank** (B1.a) — the money leg has the same issuer on both ends, so settlement makes no
 * interbank leg at all. Reserves move later, when the borrower spends it to somebody at another
 * bank (B1.b), and that is an ordinary payment the wire already knows how to make. There is
 * nowhere in this module where a deposit or a reserve is consumed to fund a loan (B1.c).
 *
 * A customer overdrawn is borrowing, and it is this bank's decision (Money B3.a). The kernel asks
 * this module the moment a payment would take an account below zero, and the answer is the same
 * credit decision as any other: the room its own capital supports, and its own limit for that name.
 * What it allows becomes a row before the period closes, so the negative balance is a drawing on a
 * loan and never a silent hole (B3.c).
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { Event } from '../../journal/journal.js';
import { period } from '../../calendar/calendar.js';
import { civil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { paramId } from '../../core/ids.js';
import { div, dustOf, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { Instrument } from '../../register/instruments.js';
import type { OverdraftContext, OverdraftDecision } from '../../registry/kinds.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { BANKS, bankParam, type BankDecl } from './data.js';
import { LOAN, loanId, loanKind, isLoan, type LoanTerms } from './loan.js';
import { lossGivenDefault, probabilityOfDefault, quote, room, type Quote, type Regulation } from './quote.js';

export * from './data.js';
export * from './loan.js';
export { quote, room, probabilityOfDefault, lossGivenDefault, exposureTo } from './quote.js';
export type { Quote, Regulation, Room } from './quote.js';

export const LENDING_PARAMS = {
  capitalRatio: paramId('regulation.capitalRatio'),
  riskWeight: paramId('regulation.riskWeight.loan'),
  operatingCost: paramId('loan.operatingCost'),
} as const;

/** What a bank was asked for, by whom, and what it said (C3.a: a decline is an answer). */
interface Book {
  next: number;
  /** Money B3.a: what the kernel allowed as a drawing this period, waiting to become a row. */
  draws: { holder: string; issuer: string; ccy: string; amount: number }[];
}

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('book', () => ({ next: 1, draws: [] }));
}

function regulationOf(view: ParticipantView): Regulation {
  return {
    capitalRatio: view.params.get(LENDING_PARAMS.capitalRatio),
    riskWeight: view.params.get(LENDING_PARAMS.riskWeight),
    operatingCost: view.params.get(LENDING_PARAMS.operatingCost),
  };
}

/** C1.b: every default anybody published — public, so every bank saw them (Expectations A2). */
function seenDefaults(ctx: MechanismContext): readonly Event[] {
  return ctx.journal.ofKind('credit.default');
}

function declOf(bank: PartyId): BankDecl | undefined {
  return BANKS.find((b) => b.bank === bank);
}

/**
 * C1.a, XI-4 joint one: what this bank actually paid for what it owed, annualised — read off the
 * wire rather than assumed. Nothing a bank issues pays interest in this world yet, so it is zero;
 * the moment a deposit or a wholesale line does (worklist 11), this reads it without changing.
 */
function costOfFunds(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  if (ctx.period === 0) return 0;
  const previous = period(ctx.period - 1);
  const paid: number[] = [];
  for (const r of ctx.ledger.inPeriod(previous)) {
    if (r.outcome !== 'settled' || r.instruction.cause !== 'coupon') continue;
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg) || leg.from.holder !== bank || leg.ccy !== ccy) continue;
      paid.push(leg.amount);
    }
  }
  const owed = owedBy(ctx, bank, ccy);
  if (owed <= 0) return 0;
  // Law 8: a rate is per annum, so what it paid over this period is divided by the fraction of a
  // year the period actually was — read off the calendar's own dates, never a periods-per-year.
  const year = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(previous),
    ctx.calendar.startOf(ctx.period),
  );
  if (year <= 0) return 0;
  return div(div(sum(paid).value, owed, 'what it paid on what it owed'), year, 'per annum');
}

/** What this bank owes: every liability of its own that anybody holds. */
function owedBy(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.issuer.some || i.issuer.value !== bank || i.ccy !== ccy) continue;
    if (!ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    terms.push(i.issued);
  }
  return sum(terms).value;
}

/**
 * C1, C2: every bank quotes from its own state, and the borrower takes the keenest that will have
 * it. A bank with no room does not quote — declining IS the credit decision (C3) — and what it
 * declined is recorded, because a bank that never says no has no credit standard (C3.a).
 */
function shop(ctx: MechanismContext, borrower: PartyId, want: number, ccy: CurrencyCode): {
  readonly best: Quote | undefined;
  readonly lend: number;
} {
  let best: Quote | undefined;
  let lend = 0;
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(b.id);
    if (decl === undefined || !b.status.alive) continue;
    const view = ctx.participant(b.id);
    const reg = regulationOf(view);
    const r = room(view, decl, borrower, reg);
    if (r.most <= 0) {
      ctx.record(
        'credit.declined',
        [b.id, borrower],
        { bank: b.id, borrower, asked: want, binds: r.binds, capitalRoom: r.capital, appetiteRoom: r.appetite },
        false,
      );
      continue;
    }
    const q = quote(view, decl, borrower, reg, costOfFunds(ctx, b.id, ccy), seenDefaults(ctx));
    const takeable = r.most < want ? r.most : want;
    if (best === undefined || q.rate < best.rate) {
      best = q;
      lend = takeable;
    }
  }
  return { best, lend };
}

/**
 * B1: the loan is written. One instruction, two legs, both sides of both parties at once — the
 * borrower issues the loan to the bank, and the bank creates its own money into the borrower's
 * account. The money leg has the same issuer at both ends, so settlement generates NO reserve leg,
 * which is B1.a exactly: nothing left the bank to make this loan.
 */
function write(
  ctx: MechanismContext,
  bank: PartyId,
  borrower: PartyId,
  principal: number,
  rate: number,
  ccy: CurrencyCode,
  /**
   * Corporate Credit C9, A3: whether this is a drawing on the borrower's LINE at this bank. A line
   * is drawn and repaid at the borrower's option, so it is ONE row that its outstanding moves on —
   * never a new loan every period, which would turn a facility into a pile of term loans and make
   * the borrower's exposure a thing you have to add up rather than a thing you can look at.
   */
  onTheLine = false,
): InstrumentId | undefined {
  const b = book(ctx);
  const existing = onTheLine ? lineOf(ctx, bank, borrower) : undefined;
  if (existing !== undefined) return draw(ctx, existing, principal, ccy);
  const id = loanId(bank, borrower, b.next);
  const drawn = ctx.calendar.startOf(ctx.period);
  const terms: LoanTerms = {
    kind: LOAN,
    lender: bank,
    borrower,
    rate,
    drawn,
    // A2: a year, placed by date like every other maturity in this world (Money G3.a).
    maturity: civil(drawn.y + 1, drawn.m, drawn.d),
    dayCount: 'ACT/365F',
    // A4: unsecured, and it is stated. There is nothing in this world a bank could take security
    // over that it could realise (worklist 7 gives an estate, 13d a dwelling).
    security: [],
  };
  ctx.issue({ id, kind: LOAN, issuer: some(borrower), ccy, terms, market: none() });
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: borrower,
      to: bank,
      instrument: id,
      qty: principal,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: { holder: bank, issuer: bank },
      to: { holder: borrower, issuer: bank },
      ccy,
      amount: principal,
      fromCell: none(),
      toCell: none(),
    },
  ];
  const r = ctx.settle({ legs, cause: 'issuance', reason: `${bank} lends ${principal} to ${borrower}` });
  if (r.outcome !== 'settled') return undefined;
  b.next += 1;
  ctx.record(
    'credit.written',
    [bank, borrower, id],
    { bank, borrower, loan: id, principal, rate },
    false,
  );
  return id;
}

/** C9: the borrower's live line at this bank, if it has one. One row, whatever it has drawn. */
function lineOf(ctx: MechanismContext, bank: PartyId, borrower: PartyId): Instrument | undefined {
  return ctx.instruments
    .all()
    .find(
      (i) =>
        i.status.live &&
        isLoan(i.terms) &&
        i.terms.lender === bank &&
        i.terms.borrower === borrower,
    );
}

/**
 * C9, A3: a further drawing on a line that already exists. More of the same instrument is issued
 * and more of the bank's money is created against it — the outstanding moves, the row does not
 * multiply, and what the borrower owes this bank stays one number you can look at.
 */
function draw(
  ctx: MechanismContext,
  line: Instrument,
  amount: number,
  ccy: CurrencyCode,
): InstrumentId | undefined {
  if (!isLoan(line.terms)) return undefined;
  const { lender, borrower } = line.terms;
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: borrower,
      to: lender,
      instrument: line.id,
      qty: amount,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: { holder: lender, issuer: lender },
      to: { holder: borrower, issuer: lender },
      ccy,
      amount,
      fromCell: none(),
      toCell: none(),
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${borrower} draws ${amount} on its line at ${lender}`,
  });
  if (r.outcome !== 'settled') return undefined;
  ctx.record('credit.draw', [lender, borrower, line.id], { bank: lender, borrower, loan: line.id, amount }, false);
  return line.id;
}

/**
 * Money B3.a: the credit decision behind an overdraft. It is the same decision as any other loan —
 * the room this bank's own capital supports and its own limit for that name — and what it allows is
 * a DRAWING, which becomes a row before the period closes. A bank with no room refuses, and the
 * payment fails: that is the refusal B3.c requires and it is recorded by settlement.
 */
function overdraft(ctx: MechanismContext, o: OverdraftContext): OverdraftDecision {
  const decl = declOf(o.issuer);
  if (decl === undefined) return { allow: false };
  const view = ctx.participant(o.issuer);
  // A1, XI-8: a loan is a contract with somebody, and some parties are nobody to contract with —
  // an estate is being wound up, and a household has no lender in this world at all (Households
  // C1.d). The kind says so and the bank reads it (Law 15); the refusal is the answer, recorded.
  const borrows = ctx.registry.partyKind(ctx.parties.get(o.holder).kind).borrows;
  const r = room(view, decl, o.holder, regulationOf(view));
  if (!borrows || r.most < o.shortfall) {
    ctx.record(
      'credit.declined',
      [o.issuer, o.holder],
      {
        bank: o.issuer,
        borrower: o.holder,
        asked: o.shortfall,
        binds: borrows ? r.binds : 'nobody lends to a party of this kind',
        overdraft: true,
      },
      false,
    );
    return { allow: false };
  }
  book(ctx).draws.push({ holder: o.holder, issuer: o.issuer, ccy: o.ccy, amount: o.shortfall });
  return { allow: true, recordedAs: 'facilityDraw' };
}

/**
 * Money B3.c: every drawing the kernel allowed becomes a row before the audit sees the period. The
 * loan creates a deposit, which is what brings the account back from below zero — so what looked
 * like an overdraft during the period is a loan by the end of it, with a rate and a lender.
 */
function bookDraws(ctx: MechanismContext): void {
  const b = book(ctx);
  const draws = [...b.draws];
  b.draws = [];
  for (const d of draws) {
    const bank = d.issuer as PartyId;
    const decl = declOf(bank);
    if (decl === undefined) continue;
    const view = ctx.participant(bank);
    const q = quote(
      view,
      decl,
      d.holder as PartyId,
      regulationOf(view),
      costOfFunds(ctx, bank, d.ccy as CurrencyCode),
      seenDefaults(ctx),
    );
    // C9: an overdraft is a drawing on the borrower's line, not a new loan every week.
    write(ctx, bank, d.holder as PartyId, d.amount, q.rate, d.ccy as CurrencyCode, true);
  }
}

/**
 * F2: new lending, amortisation, prepayment and write-off account for the change in the book. The
 * book is the sum of the rows a bank holds (F1), so what it was plus what the wire did to it is
 * what it is — and a book that moved with no instruction behind it is the scalar F1.a forbids,
 * arrived at by another route.
 */
function bookMoves(): Family {
  return {
    name: 'flows',
    contributor: 'bank-lending',
    spec: 'Banks Lending F1 Banks Lending F1.a Banks Lending F2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!isLoan(i.terms)) continue;
        const held = view.register.quantity(i.terms.lender, i.id);
        const holding = view.register.holding(i.terms.lender, i.id);
        // Law 7: `issued` is a running total that carries the dust of every drawing it has taken,
        // and what the lender holds is a sum over the lots those drawings made. The comparison is
        // entitled to both walks and to nothing else.
        const lots = holding.some ? holding.value.lots.length : 0;
        const dust =
          i.issuedDust + dustOf(lots + 2, Math.abs(i.issued) + Math.abs(held));
        // F1.a: the lender of record holds every unit of it. A loan somebody else is holding is a
        // loan that was sold, and selling one is worklist 13f — so until then this must be true.
        if (withinDust(held, i.issued, dust)) continue;
        out.push({
          family: 'flows',
          spec: 'Banks Lending F1.a',
          owner: i.id,
          size: sub(i.issued, held, 'units not with the lender of record'),
          unit: i.unit,
          period: view.period,
          message: `${i.id}: ${i.issued} outstanding and its lender of record holds ${held}`,
        });
      }
      return out;
    },
  };
}

export const bankLending: SystemModule = {
  id: 'bank-lending',
  spec: 'Banks Lending',
  // It needs nobody. What a borrower is short of and what a borrower has failed to pay both reach
  // it as journal events, which are the kernel's — so a world with banks in it can lend whether or
  // not it has firms, and a bank's answer to Money B3.a exists as soon as there is a bank.
  requires: [],
  instrumentKinds: [loanKind],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: LENDING_PARAMS.capitalRatio,
      value: 0.08,
      unit: 'ratio of risk-weighted assets',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Lending B2.a: the capital a bank must hold against what it lends. It is a rule somebody wrote, not a fact about the world, and it is the number a downturn makes bind.',
    },
    {
      id: LENDING_PARAMS.riskWeight,
      value: 1,
      unit: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Lending B2.a: how much of the requirement a unit of unsecured lending consumes. One, because an unsecured loan to a firm is the thing the requirement was written about; a weight per security arrives when there is security to weigh (worklist 13d).',
    },
    {
      id: LENDING_PARAMS.operatingCost,
      value: 0.005,
      unit: 'per annum on the principal',
      kind: 'technology',
      owner: 'model',
      why: 'Banks Lending C1.d: what it costs a bank to make and keep a loan — the people, the assessment, the collecting. It is a real cost of doing the thing, which is what technology means.',
    },
    ...BANKS.flatMap((b) => [
      {
        id: bankParam(b.bank, 'credit.memory'),
        value: b.memoryPeriods,
        unit: 'periods',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending C1.b, C4: how far back ${b.bank} looks when it judges a borrower. ${b.why}`,
      },
      {
        id: bankParam(b.bank, 'returnOnCapital'),
        value: b.returnOnCapital,
        unit: 'per annum',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending C1.c: what ${b.bank} needs to earn on the capital a loan consumes.`,
      },
      {
        id: bankParam(b.bank, 'capitalBuffer'),
        value: b.capitalBuffer,
        unit: 'ratio of risk-weighted assets',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending B2.a: how far above the requirement ${b.bank} insists on running. Its own caution, which is why two banks stop lending at different moments.`,
      },
      {
        id: bankParam(b.bank, 'limitPerBorrower'),
        value: b.limitPerBorrower,
        unit: 'ratio of its own capital',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending F3, B2.c: the most ${b.bank} will have out to one name. A limit that binds is what makes concentration a thing it manages rather than a thing it reports.`,
      },
    ]),
  ],
  phases: [
    {
      name: 'lending.write',
      spec: 'Banks Lending B1 Banks Lending C1 Banks Lending C2 Banks Lending C3',
      cycle: 0,
      // Clearing F1: it acts on what it has already been told. A borrower says what it is short of
      // in the period it finds out, and the credit is arranged in the next one — which is a lag
      // and is stated as one, because arranging a loan takes longer than noticing you need it.
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        runRequests(ctx);
      },
    },
    {
      name: 'lending.book',
      spec: 'Money B3.a Money B3.c Banks Lending B1',
      cycle: 'anchor',
      // Before the audit sees the period, and before anything can DIE of it: an overdraft the bank
      // allowed is a drawing, and a drawing is a row. A party that ceased still carrying a raw
      // negative balance would leave its bank holding a claim with no instrument behind it, and the
      // estate nothing to assume — so this runs first and what is left is always a loan.
      anchor: { before: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        bookDraws(ctx);
        publishStandard(ctx);
        publishCostOfFunds(ctx);
      },
    },
  ],
  participants: [],
  marks: [{ instrumentKind: LOAN, value: worthToItsLender }],
  creditDecisions: [{ partyKind: BANK, decide: overdraft }],
  families: [bookMoves()],
};

/**
 * D1, D2: what a unit of this loan is worth to the bank that holds it. Amortised cost less what
 * that bank expects to lose on it — its OWN assessment, from the SAME model it priced the loan
 * with (C4: two models that disagree mean the price and the provision are struck against different
 * beliefs). The kernel books the difference against the lot and the equity account together, so the
 * provision is a charge to income that is visible (D2.a) and never a reserve absorbing things
 * quietly (D2.b); and it moves back up when the assessment does, because a claim is not inventory.
 *
 * A bank that has seen this borrower fail half the time carries the loan at half. That is the whole
 * of it: no coverage ratio, no stage, no through-the-cycle anything.
 */
function worthToItsLender(ctx: MechanismContext, i: Instrument): Option<number> {
  if (!isLoan(i.terms)) return none<number>();
  const decl = declOf(i.terms.lender);
  if (decl === undefined) return none<number>();
  const view = ctx.participant(i.terms.lender);
  const pd = probabilityOfDefault(view, decl, i.terms.borrower, seenDefaults(ctx));
  const loss = mul(pd, lossGivenDefault(i.terms.security), 'what it expects to lose per unit');
  return some(sub(1, loss, 'what a unit is worth to it'));
}

/** C2: a borrower that said what it is short of gets quotes, and takes the keenest that will have it. */
function runRequests(ctx: MechanismContext): void {
  if (ctx.period === 0) return;
  const said = period(ctx.period - 1);
  for (const e of ctx.journal.ofKind('firms.funding')) {
    if (e.period !== said) continue;
    const borrower = e.subjects[0];
    const want = e.data['short'];
    if (borrower === undefined || typeof want !== 'number' || want <= 0) continue;
    const party = ctx.parties.get(borrower as PartyId);
    // XI-8, Firm Birth D5: what it asked for last period it asked for as a going concern. It has
    // since ceased, and an estate is winding it up rather than borrowing: there is nobody left to
    // sign, so the request dies with the borrower.
    if (!party.status.alive) continue;
    const ccy = ctx.registry.region(party.region).ccy;
    const { best, lend } = shop(ctx, borrower as PartyId, want, ccy);
    if (best === undefined || lend <= 0) continue;
    // C9, F1.a: one row per (lender, borrower). A borrower that comes back to the same bank is
    // drawing on what it already has there, not taking a new loan every week — and the margin it
    // draws at is the one that was struck when the line was agreed (A2, A3).
    write(ctx, best.bank, borrower as PartyId, lend, best.rate, ccy, true);
  }
}

/**
 * C3.a: declined volume is visible. A bank that never says no has no credit standard, so what the
 * banks between them turned away this period is published as a count and a volume — an aggregate
 * read of events that already happened, causing nothing (Observer A5). Who was refused stays
 * between the two of them; that it happened, and how much of it, does not.
 */
function publishStandard(ctx: MechanismContext): void {
  const declined = ctx.journal
    .ofKind('credit.declined')
    .filter((e) => e.period === ctx.period);
  const written = ctx.journal.ofKind('credit.written').filter((e) => e.period === ctx.period);
  const volume = (rows: readonly Event[], key: string): number =>
    sum(rows.map((e) => (typeof e.data[key] === 'number' ? (e.data[key]) : 0))).value;
  ctx.record(
    'credit.standard',
    [],
    {
      declined: declined.length,
      declinedVolume: volume(declined, 'asked'),
      written: written.length,
      writtenVolume: volume(written, 'principal'),
    },
    true,
  );
}

/**
 * C1.a, XI-4 joint one: what each bank actually paid for what it owed, published under its own name.
 *
 * A bank's cost of funds is a FACT ABOUT THE BANK with one writer (Law 4), and it is public because
 * something else in this world prices off it: a trading desk inside a bank pays that bank for the
 * money its inventory ties up, every period it holds it (Dealer Desks D3), and a desk that read a
 * different number from the one its bank pays would be two prices for one thing. It is a read of
 * what already left the bank (Observer A5) and it causes nothing by itself.
 */
function publishCostOfFunds(ctx: MechanismContext): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    if (declOf(b.id) === undefined || !b.status.alive) continue;
    const ccy = ctx.registry.region(b.region).ccy;
    ctx.record(
      'bank.costOfFunds',
      [b.id],
      { bank: b.id, ccy, perAnnum: costOfFunds(ctx, b.id, ccy), owed: owedBy(ctx, b.id, ccy) },
      true,
    );
  }
}

/** Re-exported so the observer and the tests can read a bank's own book as the sum of its rows. */
export function loanBook(view: ParticipantView, bank: PartyId): number {
  return exposureToAll(view, bank);
}

function exposureToAll(view: ParticipantView, bank: PartyId): number {
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!isLoan(i.terms) || i.terms.lender !== bank) continue;
    terms.push(view.quantity(h.instrument));
  }
  return sum(terms).value;
}
