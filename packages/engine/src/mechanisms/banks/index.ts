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
import { type Qty } from '../../core/tick.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { Event } from '../../journal/journal.js';
import { period } from '../../calendar/calendar.js';
import { civil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { currencyUnit, paramId, partyId } from '../../core/ids.js';
import { add, atLeast, atMost, div, dustOf, mul, sub, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { Instrument } from '../../register/instruments.js';
import type { OverdraftContext, OverdraftDecision } from '../../registry/kinds.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { ParamDecl } from '../../registry/params.js';
import type { SystemModule } from '../../world/module.js';
import {
  bankParam,
  lineParam,
  DEALING,
  TRADING_BOOK_RISK_WEIGHT,
  type BankDecl,
} from './data.js';
import { capitalOf, publish, type CapitalRules } from './capital.js';
import { arbitrage, dealingOrders, publishDealing } from './dealing.js';
import {
  classesSeen,
  liquidityPlan,
  liquidityTargets,
  publishBuffer,
  P_COVERAGE,
  ownDeposits,
  sessionOrders,
  setBoard,
  type ReserveMemory,
} from './treasury.js';
import {
  bidsFor,
  runRaise,
  subordinatedKind,
  SUB_PARAMS,
} from './subordinated.js';
import { publishLines } from './lines.js';
import { LOAN, loanId, loanKind, isLoan, type LoanTerms } from './loan.js';
import {
  holderReservation,
  lossGivenDefault,
  probabilityOfDefault,
  quote,
  room,
  type Quote,
  type Regulation,
} from './quote.js';

export * from './data.js';
export { DEALING, LENDING, roomFor } from './lines.js';
export * from './loan.js';
export * from './capital.js';
export * from './subordinated.js';
export * from './treasury.js';
export * from './dealing.js';
export * from './dealing-quote.js';
export { quote, holderReservation, room, probabilityOfDefault, lossGivenDefault, exposureTo } from './quote.js';
export type { Quote, Regulation, Room } from './quote.js';

export const LENDING_PARAMS = {
  capitalRatio: paramId('regulation.capitalRatio'),
  riskWeight: paramId('regulation.riskWeight.loan'),
  sovereignWeight: paramId('regulation.riskWeight.sovereign'),
  leverageRatio: paramId('regulation.leverageRatio'),
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
    capitalRatio: view.params.ratio(LENDING_PARAMS.capitalRatio),
    riskWeight: view.params.ratio(LENDING_PARAMS.riskWeight),
    operatingCost: view.params.perAnnum(LENDING_PARAMS.operatingCost),
  };
}

/**
 * Banks Capital B1, B1.b, B2: the two rules and the bank's own caution above them, in one place.
 * The weight is the one an ordinary exposure carries; what a particular asset weighs is asked of
 * what that asset IS (`riskWeightOf`).
 */
function rulesFor(rows: readonly BankDecl[], ctx: MechanismContext, bank: PartyId): CapitalRules {
  const view = ctx.participant(bank);
  const decl = declOf(rows, bank);
  return {
    minWeighted: ctx.params.ratio(LENDING_PARAMS.capitalRatio),
    minLeverage: ctx.params.ratio(LENDING_PARAMS.leverageRatio),
    buffer: ctx.params.ratio(bankParam(bank, 'capitalBuffer')),
    weight: ctx.params.ratio(LENDING_PARAMS.riskWeight),
    limitPerName: ctx.params.ratio(bankParam(bank, 'limitPerBorrower')),
    sovereignWeight: ctx.params.ratio(LENDING_PARAMS.sovereignWeight),
    tradingWeight: ctx.params.ratio(TRADING_BOOK_RISK_WEIGHT),
    // Dealer Desks F2: the same target the dealing line quotes around, read once and used for both
    // — what a holding weighs and what the book may be worth are one line drawn in one place.
    targets:
      decl === undefined
        ? new Map<InstrumentId, number>()
        : liquidityTargets(
            view,
            decl,
            liquidityPlan(view, ctx.params.ratio(bankParam(bank, 'liquidityCushion'))),
          ),
  };
}

/** B1, B3, B3.a: every bank's position, taken and published before anybody decides anything. */
function publishCapital(rows: readonly BankDecl[], ctx: MechanismContext): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (!b.status.alive || decl === undefined) continue;
    const ccy = ctx.registry.currencyOf(b.region);
    const p = capitalOf(ctx, b.id, ccy, rulesFor(rows, ctx, b.id));
    publish(ctx, p);
    // XI-4, B3: and its treasury allots the room it has left between the lines that spend it, in
    // the order of what each of them earned on the capital it used. Published with the position
    // because it is derived from it, and read back by both lines (Law 4).
    publishLines(ctx, b.id, ccy, decl, p.byLine, p.headroom, p.capital);
  }
}

/**
 * Banks Capital C2, C2.a, C2.b, A3: RECAPITALISATION FIRST, IF SOMEBODY WILL PROVIDE IT.
 *
 * A bank that closed the last period below its own line published what it must raise (B3). It comes
 * here and asks for it: a size and no level, into a venue named after it, against bids from the
 * other banks — each pricing THIS name the way it prices any unsecured claim on it, and each
 * bounded by what it will have out to that name (F3) and by the money it actually holds.
 *
 * C2.b is the outcome that must be reachable: nobody has to buy. A raise that finds no bid is
 * recorded as a failure and the bank is exactly where it was, one rung further down the ladder.
 */
function runRaises(rows: readonly BankDecl[], ctx: MechanismContext): void {
  const b = book(ctx);
  for (const p of ctx.parties.ofKind(BANK)) {
    if (!p.status.alive || declOf(rows, p.id) === undefined) continue;
    const short = mustRaise(rows, ctx, p.id);
    if (short <= 0) continue;
    const ccy = ctx.registry.currencyOf(p.region);
    const bids: Order[] = [];
    for (const other of ctx.parties.ofKind(BANK)) {
      const decl = declOf(rows, other.id);
      if (decl === undefined || !other.status.alive || other.id === p.id) continue;
      const view = ctx.participant(other.id);
      const q = quote(
        view,
        decl,
        p.id,
        regulationOf(view),
        costOfFunds(ctx, other.id, ccy).perAnnum,
        seenDefaults(ctx),
      );
      bids.push(...bidsFor(view, p.id, ccy, q.rate, room(view, decl, p.id).most));
    }
    const taken = runRaise(ctx, p.id, ccy, short, bids, b.next);
    b.next += taken.length;
  }
}

/** B3: what it published that it must raise to be back above both lines, or nothing (Law 19). */
function mustRaise(rows: readonly BankDecl[], ctx: MechanismContext, bank: PartyId): number {
  const said = ctx.journal.ofKind('bank.capitalPlan').filter((e) => e.subjects.includes(bank));
  const last = said[said.length - 1];
  // Only the plan it published at the LAST close: a plan from a month ago is a fact about a month
  // that is over, and a bank that has since raised or earned its way back is not raising again.
  if (last === undefined || last.period + 1 !== ctx.period) return 0;
  const short = last.data['short'];
  return typeof short === 'number' && short > 0 ? short : 0;
}

/** C1.b: every default anybody published — public, so every bank saw them (Expectations A2). */
function seenDefaults(ctx: MechanismContext): readonly Event[] {
  return ctx.journal.ofKind('credit.default');
}

function declOf(rows: readonly BankDecl[], bank: PartyId): BankDecl | undefined {
  return rows.find((b) => b.bank === bank);
}

/**
 * C1.a, XI-4 joint one: this bank's BLENDED cost of funds, per annum — what it actually paid on
 * what it owes, blended with what its own capital costs it, over the whole of what funds its book.
 *
 * XI-4 names the mix: "deposits, wholesale borrowing AND CAPITAL", and all three of them are in
 * this number. THE INTEREST HALF IS READ OFF THE WIRE — every coupon this bank actually paid last
 * period, whoever it was paid to: its depositors by class (Banks Funding B1), the banks and funds
 * that lent it money overnight or for a month (Money Market B), and the central bank's window where
 * it went there (C4). Nothing here is a rate anybody stated: it is what left the account, divided
 * by what it owes. The capital half is what its owners require on the part of the book they fund,
 * because a bank that ignored that would price every asset as though equity were free — which is
 * the same deletion of the joint that pricing at the policy rate is, from the other side.
 *
 * B2, B2.b: ONE RATE PER LIABILITY, and it is the paid one. The deposit decision and this read do
 * not compute the same number twice — the decision pays an instruction and this reads the payment,
 * so a bank that paid up for money is dearer here the period after it did, by exactly what it paid.
 *
 * It is a DIFFERENT number from C1.c's capital charge and both belong (XI-4 lists both): the blend
 * is what funding the position costs, and the charge is what the regulatory capital that particular
 * asset consumes has to earn on top.
 */
export interface FundingCost {
  /** B2: the blend — what one unit of what funds this bank's book costs it, per annum. */
  readonly perAnnum: number;
  /** B2.b: what it ACTUALLY PAID on what it owes last period, annualised. Read off the wire. */
  readonly interest: number;
  /** XI-4: what its owners require on the part of the book they fund. Nothing where they fund none. */
  readonly onCapital: number;
  readonly owed: number;
  /** A1: the RESIDUAL, as it stands — negative for a bank that is insolvent, and said so. */
  readonly capital: number;
}

function costOfFunds(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): FundingCost {
  const owed = owedBy(ctx, bank, ccy);
  const capital = ctx.participant(bank).equity();
  /**
   * Banks Capital A1, XI-4: A HOLE IS NOT A SOURCE OF FUNDS.
   *
   * Capital is the residual (A1), and a bank whose residual is negative is insolvent — a real state
   * that stays real: it is published under the bank's own name below, the audit sees it and the
   * resolution trigger reads it. What a negative residual is NOT is money funding the book with a
   * return its owners require on it: there is nothing there for them to require one on. So what
   * funds the book is what it owes plus the capital there IS, and a bank with none funds itself
   * entirely with debt.
   *
   * Blending the hole in made `perAnnum` NEGATIVE — measured at −0.0894 for a bank 31bn short —
   * and a negative cost of funds reaches the dealing quote as a negative EDGE, which is a bid above
   * the desk's own offer. Twenty-three periods into a thirty-period run the market refused it at
   * the site: `[Clearing A2] bank.a is on both sides of mkt.ust.bill.2026-09-15 at crossing prices`
   * (`13b-12`). The crossing was arithmetic that had lost its meaning, not a decision anybody took.
   */
  const funded = atLeast(capital, 0, 'a hole funds nothing: there is no less capital than none');
  const funding = add(owed, funded, 'what funds its book');
  const required = ctx.params.perAnnum(bankParam(bank, 'returnOnCapital'));
  const onCapital = mul(funded, required, 'what its own capital costs it');
  const blend = (interest: number): FundingCost => ({
    perAnnum:
      funding <= 0 ? 0 : div(add(interest, onCapital, 'what its funding costs it'), funding, 'per annum'),
    interest,
    onCapital,
    owed,
    capital,
  });
  if (funding <= 0 || ctx.period === 0) return blend(0);
  const previous = period(ctx.period - 1);
  // Law 8: a rate is per annum, so what it paid over this period is divided by the fraction of a
  // year the period actually was — read off the calendar's own dates, never a periods-per-year.
  const year = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(previous),
    ctx.calendar.startOf(ctx.period),
  );
  if (year <= 0) return blend(0);
  return blend(div(couponsPaid(ctx, bank, ccy), year, 'what it paid on what it owes, per annum'));
}

/**
 * B2.b, Law 18, Law 19: WHAT EVERY BANK PAID IN COUPONS LAST PERIOD, walked ONCE.
 *
 * A settled period is finished, so what left a bank in it cannot change — and every borrower that
 * shops asks every bank what its money costs, which asked this same question three thousand times a
 * bank. Ninety thousand walks of a ledger that had already stopped moving was four fifths of the
 * cost of a period. One walk, held under the period it is about, and every reader gets the number
 * that walk found.
 *
 * It is layout and not a mechanism: the walk is the same walk, and what it answers is what it
 * answered. The two LIVE parts of a bank's cost of funds — what it owes and what its capital is —
 * are read where they are asked for, because both move inside a period as loans settle.
 */
interface CouponsPaid {
  /**
   * Which period this walk is about, or NONE because nothing has been walked yet. It was a
   * `readonly number` initialised to -1 and written through a cast — a sentinel standing for
   * "missing" in a field whose type said it could not be missing, and a lie about the field being
   * readonly in the one function whose job is to move it (item 13b.1).
   */
  walked: Option<number>;
  readonly byBank: Map<string, number>;
}

function couponsPaid(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const held = ctx.state<CouponsPaid>('banks.couponsPaid', () => ({
    walked: none<number>(),
    byBank: new Map(),
  }));
  if (!held.walked.some || held.walked.value !== ctx.period) {
    held.byBank.clear();
    for (const r of ctx.ledger.inPeriod(period(ctx.period - 1))) {
      if (r.outcome !== 'settled' || r.instruction.cause !== 'coupon') continue;
      for (const leg of r.instruction.legs) {
        if (!isMoneyLeg(leg)) continue;
        const key = `${leg.from.holder}\u0000${leg.ccy}`;
        const before = held.byBank.get(key);
        held.byBank.set(
          key,
          before === undefined ? leg.amount : add(before, leg.amount, 'coupons it paid'),
        );
      }
    }
    held.walked = some(ctx.period);
  }
  // A bank that paid no coupon in that period paid nothing, and nothing is a number: the walk
  // above visited every settled instruction of it, so an absence here is an answer and not a gap.
  // `zeroIfNone` is the one place that says so, and it says it for quantities only (Appendix A).
  return zeroIfNone(held.byBank.get(`${bank}\u0000${ccy}`));
}

/**
 * What this bank owes: every liability of its own that anybody holds.
 *
 * Law 18: the instruments an ISSUER has out, by name. Walking every instrument in the world to find
 * one bank's own was the second of the two scans a shopping borrower paid for, and a world with a
 * share line per listed firm has hundreds of them.
 */
function owedBy(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const i of ctx.instruments.issuedBy(bank)) {
    if (i.ccy !== ccy) continue;
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
function shop(rows: readonly BankDecl[], ctx: MechanismContext, borrower: PartyId, want: number, ccy: CurrencyCode): {
  readonly best: Quote | undefined;
  readonly lend: number;
} {
  let best: Quote | undefined;
  let lend = 0;
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (decl === undefined || !b.status.alive) continue;
    const view = ctx.participant(b.id);
    const reg = regulationOf(view);
    const r = room(view, decl, borrower);
    if (r.most <= 0) {
      ctx.record(
        'credit.declined',
        [b.id, borrower],
        {
          bank: b.id,
          borrower,
          asked: want,
          binds: r.binds,
          capitalRoom: r.capital.some ? r.capital.value : null,
          appetiteRoom: r.appetite,
          fundingRoom: r.funding.some ? r.funding.value : null,
        },
        false,
      );
      continue;
    }
    const q = quote(view, decl, borrower, reg, costOfFunds(ctx, b.id, ccy).perAnnum, seenDefaults(ctx));
    const takeable = atMost(r.most, want, 'nobody lends more than the borrower asked for');
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
  wanted: number,
  rate: number,
  ccy: CurrencyCode,
  /**
   * Corporate Credit C9, A3: whether this is a drawing on the borrower's LINE at this bank. A line
   * is drawn and repaid at the borrower's option, so it is ONE row that its outstanding moves on —
   * never a new loan every period, which would turn a facility into a pile of term loans and make
   * the borrower's exposure a thing you have to add up rather than a thing you can look at.
   */
  onTheLine = false,
  /**
   * A4 (13d): WHAT IT IS SECURED ON, in the words of whoever asked. A bank does not know what a
   * dwelling is and must not: what it knows is that the request named an instrument and a quantity
   * it could take and realise, which is what security IS. Empty is unsecured and is stated either
   * way, and `lossGivenDefault` is the one place the difference is priced.
   */
  security: readonly { readonly instrument: InstrumentId; readonly qty: number }[] = [],
): InstrumentId | undefined {
  const b = book(ctx);
  // Law 8, B1: money is created in whole pieces of itself, so a loan is drawn in whole pieces. What
  // the arithmetic asked for below one piece is not lent, because it is not money.
  const principal = ctx.registry.payable(ccy, wanted);
  if (principal <= 0) return undefined;
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
    // A4: what the request named, and nothing is inferred. It was always empty until 13d gave this
    // world a thing a bank could take and realise; a request that names none is still unsecured,
    // and that is a statement rather than an absence.
    security,
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
      // B1, B1.b: INTO THE BORROWER'S OWN ACCOUNT, which is at the borrower's own bank and not at
      // whichever bank won the business. When they are the same bank the money leg has one issuer
      // on both ends and no reserve leaves it, which is B1.a and the whole of endogenous money.
      // When they are not, the lender has paid somebody who banks elsewhere: the borrower's bank
      // owes the deposit, the lending bank settles the reserves, and B1.b's "reserves move when
      // the borrower spends it" arrives at the moment of the drawing rather than after it. A
      // borrower left holding the LENDER's money would be a borrower with an account it never
      // opened, invisible to every read that asks what it has (Money A1, B3).
      to: ctx.accountOf(borrower, ccy),
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
  amount: Qty,
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
      // B1.b, as above: the drawing lands in the borrower's own account, wherever that is.
      to: ctx.accountOf(borrower, ccy),
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
function overdraft(rows: readonly BankDecl[], ctx: MechanismContext, o: OverdraftContext): OverdraftDecision {
  const decl = declOf(rows, o.issuer);
  if (decl === undefined) return { allow: false };
  const view = ctx.participant(o.issuer);
  // A1, XI-8: a loan is a contract with somebody, and some parties are nobody to contract with —
  // an estate is being wound up, and a household has no lender in this world at all (Households
  // C1.d). The kind says so and the bank reads it (Law 15); the refusal is the answer, recorded.
  const borrows = ctx.registry.partyKind(ctx.parties.get(o.holder).kind).borrows;
  const r = room(view, decl, o.holder);
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
  return { allow: true };
}

/**
 * Money B3.c: every drawing the kernel allowed becomes a row before the audit sees the period. The
 * loan creates a deposit, which is what brings the account back from below zero — so what looked
 * like an overdraft during the period is a loan by the end of it, with a rate and a lender.
 */
function bookDraws(rows: readonly BankDecl[], ctx: MechanismContext): void {
  const b = book(ctx);
  const draws = [...b.draws];
  b.draws = [];
  for (const d of draws) {
    const bank = d.issuer as PartyId;
    const decl = declOf(rows, bank);
    if (decl === undefined) continue;
    const view = ctx.participant(bank);
    const q = quote(
      view,
      decl,
      d.holder as PartyId,
      regulationOf(view),
      costOfFunds(ctx, bank, d.ccy as CurrencyCode).perAnnum,
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
    contributor: 'banks',
    spec: 'Banks Lending F1 Banks Lending F1.a Banks Lending F2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!isLoan(i.terms)) continue;
        // Register F2, Banks Capital D6: the lender of record can have CEASED since the row was
        // written — an acquirer takes a resolved bank's loans, an estate takes a dead firm's — and
        // a reference to it resolves to whoever succeeded it. Nothing about the row changed.
        const lender = view.parties.resolve(i.terms.lender).id;
        const held = view.register.quantity(lender, i.id);
        const holding = view.register.holding(lender, i.id);
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

/**
 * Dealer Desks F2, Banks Capital B1.a: THE TRADING BOOK IS CAPITALISED, and this measures it.
 *
 * F2 says no desk is exempt from its own bank's capital, and 11.2 made that structurally true by
 * deleting the desk. What is left to check is arithmetic: whatever a bank is holding ABOVE where
 * its own treasury wants a line, at the marks in force, at the weight a trading position carries,
 * is inside the risk-weighted assets it published. A bank whose published requirement did not move
 * when it ran a position up would be a bank whose dealing was free, which is the thing F2 forbids.
 *
 * It is a MEASUREMENT and never a rule: what it finds is reported with an owner and a size, and
 * nothing here adjusts a weight or a position (the audit never repairs).
 */
function tradingBookIsCapitalised(): Family {
  return {
    name: 'accounts',
    contributor: 'banks',
    spec: 'Dealer Desks F2 Banks Capital B1 Banks Capital B1.a',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const weight = view.params.ratio(TRADING_BOOK_RISK_WEIGHT);
      for (const e of view.journal.ofKind('bank.capital')) {
        if (e.period !== view.period) continue;
        const bank = e.subjects[0];
        const rwa = e.data['weighted'];
        if (bank === undefined || typeof rwa !== 'number') continue;
        const self = partyId(bank);
        if (!view.parties.has(self) || !view.parties.get(self).status.alive) continue;
        // Law 19: the targets are the ones the BANK published with its book this period, not a
        // second copy of the treasury's arithmetic. A check that rebuilt them would be checking
        // one derivation against another and would pass whatever either of them did.
        const said = view.journal
          .ofKind('bank.dealing')
          .filter((x) => x.period === view.period && x.subjects.includes(self));
        const lines = said[said.length - 1]?.data['lines'];
        if (typeof lines !== 'object' || lines === null) continue;
        const terms: number[] = [];
        for (const [id, row] of Object.entries(lines as Record<string, unknown>)) {
          if (typeof row !== 'object' || row === null) continue;
          const want = (row as Record<string, unknown>)['target'];
          const value = (row as Record<string, unknown>)['worth'];
          if (typeof want !== 'number' || typeof value !== 'number' || value <= want) continue;
          terms.push(mul(sub(value, want, `${id} above its target`), weight, 'weighted'));
        }
        const asked = sum(terms);
        // Law 7: both sides are walks over the same holdings at the same marks, so what separates
        // them is the dust of the two walks. THE OTHER SIDE'S IS THE OTHER SIDE'S: `rwa` is the
        // bank's weighting of its WHOLE book, a sum over far more terms than this check can see, so
        // it publishes the count and the magnitude its own sum had and the dust is derived from
        // both (worklist 13a, finding 12b-5: a gap of 2^-12 on 5.0e10 was the missing half).
        const theirTerms = e.data['weightedTerms'];
        const theirMagnitude = e.data['weightedMagnitude'];
        // A publication without its count cannot be compared against: the reader would be deriving
        // a dust of zero for a walk it knows is not free, which is the defect, not the fix. Skip
        // the bank, as this check already skips one whose weighting it cannot read at all.
        if (typeof theirTerms !== 'number' || typeof theirMagnitude !== 'number') continue;
        const dust =
          asked.dust +
          dustOf(theirTerms, theirMagnitude) +
          dustOf(2, Math.abs(rwa) + Math.abs(asked.value));
        if (rwa + dust >= asked.value) continue;
        out.push({
          family: 'accounts',
          spec: 'Dealer Desks F2',
          owner: bank,
          size: sub(asked.value, rwa, 'weighted assets its dealing book asked for and did not get'),
          unit: currencyUnit(view.registry.currencyOf(view.parties.get(self).region)),
          period: view.period,
          message: `${bank}: its dealing book weighs ${asked.value} and it published ${rwa} of risk-weighted assets in total`,
        });
      }
      return out;
    },
  };
}

/**
 * The module, built from the banks this world has (Law 4: `BANKS` is that list and there is no
 * second one). A world with two of them or with four is this world with a different table and no
 * number in it restated, which is what lets the count be measured (XI-15).
 */
/**
 * Dealer Desks A3: WHICH BANKS MAKE A MARKET IN ONE NAMED LINE.
 *
 * A market in one name has a few makers, not all of them and not one — and which few is DRAWN,
 * because a bank that has never taken a view of a firm does not quote it. The draw lives with the
 * listing that made it (`ListedDecl.makers`), so the world that assembles both hands it in here.
 * A line nobody drew makers for answers `undefined`, and then the bank's own `makes` list decides,
 * which is every line that has no per-name makers: sovereign paper, a fund's shares, a bill.
 */
export type MakersOf = (instrument: InstrumentId) => readonly string[] | undefined;

export function banks(rows: readonly BankDecl[], makersOf?: MakersOf): SystemModule {
  return {
  id: 'banks',
  spec: 'Banks Lending',
  // It needs nobody. What a borrower is short of and what a borrower has failed to pay both reach
  // it as journal events, which are the kernel's — so a world with banks in it can lend whether or
  // not it has firms, and a bank's answer to Money B3.a exists as soon as there is a bank.
  requires: [],
  instrumentKinds: [loanKind, subordinatedKind],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: LENDING_PARAMS.capitalRatio,
      value: 0.08,
      unit: 'ratio of risk-weighted assets',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Lending B2.a: the capital a bank must hold against what it lends. It is a rule somebody wrote, not a fact about the world, and it is the number a downturn makes bind.',
    },
    {
      id: LENDING_PARAMS.riskWeight,
      value: 1,
      unit: 'ratio',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Lending B2.a: how much of the requirement a unit of unsecured lending consumes. One, because an unsecured loan to a firm is the thing the requirement was written about; a weight per security arrives when there is security to weigh (worklist 13d).',
    },
    {
      id: LENDING_PARAMS.sovereignWeight,
      value: 0,
      unit: 'ratio',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Corporate Credit E5.c, Sovereign E5: how much of the capital requirement a unit of the sovereign own paper consumes. Zero under the standard for a claim on the issuer of the money it is promised in, and that is a RULE somebody wrote rather than a fact about the world — it is most of why a bank holds sovereign paper as its liquidity buffer instead of lending the money out, and it is exactly the kind of number a polity can change (worklist 14).',
    },
    {
      id: TRADING_BOOK_RISK_WEIGHT,
      value: 1,
      unit: 'ratio of the position',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Dealer Desks D2: how much of a bank capital requirement a unit of a trading position consumes. It is a rule somebody wrote, not a fact about the world, and it is the number that makes carrying inventory cost capital as well as cash. One, because a position taken with a view is the thing the requirement was written about; a weight per kind of position arrives with the derivative layer (worklist 13a).',
    },
    {
      id: P_COVERAGE,
      value: 1,
      unit: 'ratio of what could leave',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Funding C2: the liquid assets a bank must hold against the money that could leave it. ONE, because that is what the rule says in the world this one imports it from — cover the outflow, not a part of it — and Law 2 allows a real-world primitive to be imported where a real-world equilibrium may not. It is a rule somebody wrote and not a fact about the world, which is why it is the kind of number a polity can change (worklist 14) and why a bank holds sovereign paper instead of lending the money out (Sovereign E2.a, E5).',
    },
    ...rows.flatMap((r): ParamDecl[] => [
      // Law 15: ONE DECLARATION PER LINE THIS BANK RUNS, walked rather than named. A world with a
      // third line of business registers a third row here without this loop being touched.
      ...Object.entries(r.appetite).map((entry): ParamDecl => {
        const [line, share] = entry;
        return {
          id: lineParam(r.bank, line, 'capitalAtRisk'),
          value: share,
          unit: 'ratio of its own capital',
          dimension: 'ratio',
          kind: 'preference',
          owner: 'model',
          why: `Dealer Desks D1, F1, XI-4: the most of its own capital ${r.bank} will have standing behind its ${line} line. Every capacity is finite and enumerable, and a book full of one thing stops bidding for everything, which is how one line's trouble reaches another. It is what this line ASKS ITS OWN TREASURY FOR each period, which is why every line needs one: a line whose ask is whatever is left is not competing for anything, and the treasury that serves it first hands it the lot (\`13b-7\`). It is a share of CAPITAL and not an amount of money: an amount would have to be restated every time this world changed size, and a number restated to keep a result is a result wearing a preference's name.`,
        };
      }),
      {
        id: lineParam(r.bank, DEALING, 'concentration'),
        value: r.concentration,
        unit: 'ratio of its own dealing book',
        dimension: 'ratio',
        kind: 'preference',
        owner: 'model',
        why: `Dealer Desks D1: the most of its book ${r.bank} will have in ONE line. ${r.why} A dealer without a limit is a synthetic counterparty wearing a dealer's name (Clearing B3.a), and this is the number that makes it one. It is a share rather than a count of pieces because a count would mean something different in a line quoted in shares and a line quoted in par, and would have to be restated every time a price moved.`,
      },
    ]),
    {
      id: SUB_PARAMS.periods,
      value: 52,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Banks Capital A2.b, A3: how long a bank borrows the layer between its owners and its creditors for. A year, because capital that runs off next week is not capital — it is funding — and the whole point of the layer is that it is still there when the loss arrives.',
    },
    {
      id: LENDING_PARAMS.leverageRatio,
      value: 0.03,
      unit: 'ratio of assets, unweighted',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Banks Capital B1.b: the BACKSTOP — capital against everything it holds, with no weights in it at all. It exists because B1 weights, and a rule that weights can be gamed by holding what the rule calls safe: a bank stuffed with zero-weighted paper passes the weighted test at any size. Which of the two binds is an outcome and differs by bank (B1.c), which is the whole reason to have both.',
    },
    {
      id: LENDING_PARAMS.operatingCost,
      value: 0.005,
      unit: 'per annum on the principal',
      dimension: 'perAnnum',
      /**
       * XI-14, Law 2, Appendix B: IT IS A PLACEHOLDER AND IT WAS DECLARED A TECHNOLOGY.
       *
       * Its own reason says what it stands in for — "the people, the assessment, the collecting" —
       * and this world has no such people: the number is added into the rate a bank quotes
       * (`quote.ts`) and PAID TO NOBODY. A wage bill charged and never paid is margin wearing the
       * clothes of a cost (Law 5: every flow has two sides), and stating it as a share of the
       * principal is a cost expressed as a share of money, which is what `phoenix/no-value-recipe`
       * refuses on the production side.
       *
       * 13d is where a bank employs people, and the day it does, this dies and the cost is hours
       * somebody was paid for. Until then it is a claim about the answer with a scheduled death,
       * which is precisely what Law 2 calls a placeholder (item 13b.1).
       */
      kind: 'placeholder',
      standsInFor: {
        mechanism: "Banks Lending C1.d — the credit officer's hours, paid to a named person",
        worklistItem: '13d',
      },
      owner: 'model',
      why: 'Banks Lending C1.d: what it costs a bank to make and keep a loan — the people, the assessment, the collecting. Nobody is paid it: it is added into the rate the bank quotes and lands nowhere, so it is a shape standing in for an employment relationship this world does not have yet (worklist 13d).',
    },
    ...rows.flatMap((b) => [
      {
        id: bankParam(b.bank, 'credit.memory'),
        value: b.memoryPeriods,
        unit: 'periods',
        dimension: 'periods' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending C1.b, C4: how far back ${b.bank} looks when it judges a borrower. ${b.why}`,
      },
      {
        id: bankParam(b.bank, 'returnOnCapital'),
        value: b.returnOnCapital,
        unit: 'per annum',
        dimension: 'perAnnum' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending C1.c: what ${b.bank} needs to earn on the capital a loan consumes.`,
      },
      {
        id: bankParam(b.bank, 'capitalBuffer'),
        value: b.capitalBuffer,
        unit: 'ratio of risk-weighted assets',
        dimension: 'ratio' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending B2.a: how far above the requirement ${b.bank} insists on running. Its own caution, which is why two banks stop lending at different moments.`,
      },
      {
        id: bankParam(b.bank, 'depositMargin'),
        value: b.depositMargin,
        unit: 'per annum',
        dimension: 'perAnnum' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Funding B1.a, B3: what ${b.bank} keeps for itself out of what the money it takes in is worth to it. Two banks that keep the same margin are one bank, and the one that keeps less wins the deposit and earns less on it — which is what a net interest margin IS.`,
      },
      {
        id: bankParam(b.bank, 'bufferMemory'),
        value: b.bufferMemory,
        unit: 'periods',
        dimension: 'periods' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Money Market A2.a, Banks Funding C2.a: how far back ${b.bank} looks at its own account when it decides what to hold against what could leave, and how far back it looks at what its funding has been costing it. The buffer is derived from what it has actually seen, never from a ratio of its deposits.`,
      },
      {
        id: bankParam(b.bank, 'liquidityCushion'),
        value: b.liquidityCushion,
        unit: 'ratio of what could leave, above the rule',
        dimension: 'ratio' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Funding C2, Money Market A2.a: what ${b.bank} holds liquid ABOVE what the rule asks of it. Its own caution, and the reason two banks facing the same depositors carry different portfolios — a bank that runs on the floor is one bad week from the window.`,
      },
      {
        id: bankParam(b.bank, 'limitPerBorrower'),
        value: b.limitPerBorrower,
        unit: 'ratio of its own capital',
        dimension: 'ratio' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Lending F3, B2.c: the most ${b.bank} will have out to one name. A limit that binds is what makes concentration a thing it manages rather than a thing it reports.`,
      },
    ]),
  ],
  phases: [
    {
      name: 'banks.capital',
      spec: 'Banks Capital B1 Banks Capital B1.a Banks Capital B1.b Banks Capital B1.c Banks Capital B2 Banks Capital B3 Banks Capital B3.a',
      cycle: 'anchor',
      // AFTER THE MARKS ARE TAKEN, which is the only moment its book has a value: capital is the
      // residual (A1), and a residual computed against prints that have not happened yet is not one
      // (Clearing F1.a). So a bank lends this period against the position it closed the last one
      // with — a lag, and a real one: a bank finds out what its capital allowed after the quarter
      // it allowed it in, which is exactly why B3's consequences arrive late enough to matter.
      anchor: { after: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        publishCapital(rows, ctx);
      },
    },
    {
      name: 'banks.raise',
      spec: 'Banks Capital A3 Banks Capital C2 Banks Capital C2.a Banks Capital C2.b',
      cycle: 0,
      // Before it decides anything about lending: a bank short of capital raises what it can first
      // and then lends what is left of its room (C2: recapitalisation FIRST).
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        runRaises(rows, ctx);
      },
    },
    {
      name: 'lending.write',
      spec: 'Banks Lending B1 Banks Lending C1 Banks Lending C2 Banks Lending C3',
      cycle: 0,
      // Clearing F1: it acts on what it has already been told. A borrower says what it is short of
      // in the period it finds out, and the credit is arranged in the next one — which is a lag
      // and is stated as one, because arranging a loan takes longer than noticing you need it.
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        runRequests(rows, ctx);
        // Clearing F1: everything that prices off a bank's own economics this period reads it here
        // — its own dealing line pricing what an inventory costs to carry, a firm deciding whether
        // a project clears its cost of capital, a schedule in a bond market. One number, published
        // once, read by all of them (Law 4).
        publishCostOfFunds(rows, ctx);
        publishQuotes(rows, ctx);
        publishReservations(rows, ctx);
      },
    },
    {
      name: 'banks.treasury',
      spec: 'Banks Funding B1 Banks Funding B1.a Banks Funding B2 Banks Funding B3 Money Market D2',
      cycle: 0,
      // After it has published what money costs it: a board is priced off its own funding and its
      // rivals' boards, and both are reads of what was published (Law 4, Law 19).
      anchor: { after: 'lending.write' },
      run: (ctx: MechanismContext): void => {
        const classes = classesSeen(ctx);
        if (classes.length === 0) return;
        for (const b of ctx.parties.ofKind(BANK)) {
          if (!b.status.alive || declOf(rows, b.id) === undefined) continue;
          const ccy = ctx.registry.currencyOf(b.region);
          setBoard(ctx, b.id, ccy, classes, ownDeposits(ctx, b.id, ccy));
        }
      },
    },
    {
      name: 'banks.arbitrage',
      spec: 'Fund Shares E3 Fund Shares E3.a Dealer Desks D1',
      cycle: 0,
      // After it has published what money costs it: what carrying a position costs is the number
      // that decides whether closing a gap is worth doing at all (D3), and it is that publication.
      anchor: { after: 'lending.write' },
      run: (ctx: MechanismContext): void => {
        for (const b of ctx.parties.ofKind(BANK)) {
          if (b.status.alive) arbitrage(ctx, b.id, rows);
        }
      },
    },
    {
      name: 'banks.dealing',
      spec: 'Dealer Desks D5 Dealer Desks E4',
      cycle: 'anchor',
      // After the marks are in the books, so what it says the book is worth is what it is worth.
      anchor: { after: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        for (const b of ctx.parties.ofKind(BANK)) {
          if (b.status.alive) publishDealing(ctx, b.id, rows);
        }
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
        bookDraws(rows, ctx);
        publishStandard(ctx);
      },
    },
    {
      name: 'banks.buffer',
      spec: 'Banks Funding C2 Banks Funding C2.a Money Market A2.a',
      cycle: 'anchor',
      // AFTER THE FLOWS AND BEFORE THE SESSION. What its account did to it this week is only known
      // once the week's payments have happened, and what it holds against a bad one is what every
      // schedule it posts in the session is measured against — so it is taken here, once, and
      // published, and the session reads it rather than deriving a second one (Law 4).
      anchor: { before: 'lending.book' },
      run: (ctx: MechanismContext): void => {
        const memory = ctx.state<ReserveMemory>('reserves', () => ({ moves: {} }));
        for (const b of ctx.parties.ofKind(BANK)) {
          if (!b.status.alive || declOf(rows, b.id) === undefined) continue;
          publishBuffer(ctx, b.id, ctx.registry.currencyOf(b.region), memory);
        }
      },
    },
  ],
  // Money Market A3, B1: and the same one face in a VENUE. Its schedule for a session reaches the
  // book through the kernel's door (Clearing B2), so the market that clears it decides nothing.
  venueParticipants: [{ partyKind: BANK, orders: sessionOrders }],
  // Law 4, Dealer Desks A1: ONE face. Every order a bank posts into any market comes from here.
  participants: [
    {
      partyKind: BANK,
      // XI-13, Dealer Desks A1: a dealer puts its own capital behind what it thinks a line is worth
      // and carries the loss when it is wrong. Every order this face posts is that.
      speculative: true,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
        dealingOrders(view, m, rows, makersOf),
    },
  ],
  marks: [
    { instrumentKind: LOAN, value: (ctx, i) => worthToItsLender(rows, ctx, i) },
  ],
  creditDecisions: [{ partyKind: BANK, decide: (ctx, o) => overdraft(rows, ctx, o) }],
  families: [bookMoves(), tradingBookIsCapitalised()],
  };
}

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
function worthToItsLender(rows: readonly BankDecl[], ctx: MechanismContext, i: Instrument): Option<number> {
  if (!isLoan(i.terms)) return none<number>();
  const decl = declOf(rows, i.terms.lender);
  if (decl === undefined) return none<number>();
  const view = ctx.participant(i.terms.lender);
  const pd = probabilityOfDefault(view, decl, i.terms.borrower, seenDefaults(ctx));
  const loss = mul(pd, lossGivenDefault(i.terms.security), 'what it expects to lose per unit');
  return some(sub(1, loss, 'what a unit is worth to it'));
}

/**
 * A4 (13d): what a funding request said it was secured on, read out of the event that asked. It is
 * data crossing the 4.9b door and it is checked here rather than trusted: an instrument this world
 * does not have, or a quantity that is not one, is not security.
 */
function securityIn(said: unknown): readonly { readonly instrument: InstrumentId; readonly qty: number }[] {
  if (!Array.isArray(said)) return [];
  const out: { instrument: InstrumentId; qty: number }[] = [];
  for (const row of said as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const instrument = (row as Record<string, unknown>)['instrument'];
    const qty = (row as Record<string, unknown>)['qty'];
    if (typeof instrument !== 'string' || typeof qty !== 'number' || !(qty > 0)) continue;
    out.push({ instrument: instrument as InstrumentId, qty });
  }
  return out;
}

/** C2: a borrower that said what it is short of gets quotes, and takes the keenest that will have it. */
function runRequests(rows: readonly BankDecl[], ctx: MechanismContext): void {
  if (ctx.period === 0) return;
  const said = period(ctx.period - 1);
  for (const e of [...ctx.journal.ofKind('firms.funding'), ...ctx.journal.ofKind('housing.funding')]) {
    if (e.period !== said) continue;
    const borrower = e.subjects[0];
    const want = e.data['short'];
    if (borrower === undefined || typeof want !== 'number' || want <= 0) continue;
    // A4 (13d): the request may name what it is secured on. A bank reading this does not learn
    // what the thing IS — it learns that there is an instrument it could take and realise, which
    // is the whole of what security means to a lender (Law 15).
    const security = securityIn(e.data['security']);
    const party = ctx.parties.get(borrower as PartyId);
    // XI-8, Firm Birth D5: what it asked for last period it asked for as a going concern. It has
    // since ceased, and an estate is winding it up rather than borrowing: there is nobody left to
    // sign, so the request dies with the borrower.
    if (!party.status.alive) continue;
    const ccy = ctx.registry.currencyOf(party.region);
    const { best, lend } = shop(rows, ctx, borrower as PartyId, want, ccy);
    if (best === undefined || lend <= 0) continue;
    // C9, F1.a: one row per (lender, borrower). A borrower that comes back to the same bank is
    // drawing on what it already has there, not taking a new loan every week — and the margin it
    // draws at is the one that was struck when the line was agreed (A2, A3).
    write(ctx, best.bank, borrower as PartyId, lend, best.rate, ccy, security.length === 0, security);
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
 * C2, C9, XI-4 joint two: what the keenest bank would lend this borrower NOW, and how much of it.
 *
 * A firm deciding whether to invest needs what its debt costs AT THE MARGIN — not the average
 * coupon on what it already owes, which XI-4 names as the way joint two is deleted. So every bank
 * prices every name it could lend to, the borrower is told the keenest of them and how much room
 * that bank has for it, and the decision it takes with that is its own. It is PRIVATE between the
 * two of them: what a bank would lend one firm is nobody else's business, and a borrower reading
 * its own quote is reading what it was told and not what anybody else was.
 *
 * A bank with no room quotes nothing, and a borrower nobody will lend to gets no quote at all —
 * which is B2.a doing its work one step earlier than the loan: a firm with a good project and no
 * lender does not invest.
 */
function publishQuotes(rows: readonly BankDecl[], ctx: MechanismContext): void {
  for (const p of ctx.parties.all()) {
    if (!p.status.alive || !ctx.registry.partyKind(p.kind).borrows) continue;
    const ccy = ctx.registry.currencyOf(p.region);
    let best: Quote | undefined;
    let most = 0;
    for (const b of ctx.parties.ofKind(BANK)) {
      const decl = declOf(rows, b.id);
      if (decl === undefined || !b.status.alive || b.id === p.id) continue;
      const view = ctx.participant(b.id);
      const reg = regulationOf(view);
      const r = room(view, decl, p.id);
      if (r.most <= 0) continue;
      const q = quote(view, decl, p.id, reg, costOfFunds(ctx, b.id, ccy).perAnnum, seenDefaults(ctx));
      if (best === undefined || q.rate < best.rate) {
        best = q;
        most = r.most;
      }
    }
    if (best === undefined) continue;
    ctx.record(
      'credit.quoted',
      [p.id],
      {
        borrower: p.id,
        bank: best.bank,
        rate: best.rate,
        most,
        costOfFunds: best.costOfFunds,
        expectedLoss: best.expectedLoss,
        capitalCharge: best.capitalCharge,
        operatingCost: best.operatingCost,
        ccy,
      },
      false,
    );
  }
}

/**
 * Corporate Credit E5, E5.a-c: what each bank requires, per annum, to hold a named issuer's paper.
 *
 * It is published because something else in this world prices off it and there is ONE of it (Law 4):
 * a bank's schedule in a bond market is built from what it requires, and a schedule that computed
 * its own version of the same three terms would be a second answer to one question. E5.d is what
 * the market then does with it: the level a book clears at is where the marginal holder's
 * reservation sits, and a spread below every reservation means demand is genuinely zero.
 */
function publishReservations(rows: readonly BankDecl[], ctx: MechanismContext): void {
  const obligors = new Set<PartyId>();
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    if (!profile.liabilityOfIssuer || profile.pricing === 'money') continue;
    obligors.add(i.issuer.value);
  }
  // A bank is a name anybody may lend to whether or not it has paper outstanding right now, and
  // somebody deciding overnight whether to place cash with it needs the answer before the first
  // row exists (Money Market B2). So every live bank is an obligor here, always.
  for (const b of ctx.parties.ofKind(BANK)) if (b.status.alive) obligors.add(b.id);
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (decl === undefined || !b.status.alive) continue;
    const view = ctx.participant(b.id);
    const ccy = ctx.registry.currencyOf(b.region);
    const funds = costOfFunds(ctx, b.id, ccy).perAnnum;
    const reg = { ...regulationOf(view), riskWeight: view.params.ratio(LENDING_PARAMS.sovereignWeight) };
    const required: Record<string, number> = {};
    const expectedLoss: Record<string, number> = {};
    const capitalCost: Record<string, number> = {};
    const terms: Record<string, unknown> = {};
    for (const obligor of obligors) {
      const r = holderReservation(view, decl, reg, funds);
      required[obligor] = r.rate;
      // C1.b, C4: THE TWO BELIEFS, published separately from any one price built out of them.
      // What this bank expects to lose on an unsecured claim on that name is its own model — the
      // one its loan book is priced and provisioned with, used once (Law 4) — and what the capital
      // such a claim consumes costs it is its own required return on that capital. Somebody
      // pricing a different claim on the same name (a week of money, say: Money Market B2) needs
      // these two and not a rate assembled for a year-long loan, so both are said plainly here and
      // the composing is done by whoever is asking the question.
      const unsecured = quote(view, decl, obligor, regulationOf(view), funds, seenDefaults(ctx));
      expectedLoss[obligor] = unsecured.expectedLoss;
      capitalCost[obligor] = unsecured.capitalCharge;
      terms[obligor] = {
        costOfFunds: r.costOfFunds,
        expectedLoss: r.expectedLoss,
        capitalCharge: r.capitalCharge,
      };
    }
    ctx.record(
      'bank.reservation',
      [b.id],
      { bank: b.id, ccy, required, expectedLoss, capitalCost, terms },
      false,
    );
  }
}

/**
 * C1.a, XI-4 joint one: what each bank actually paid for what it owed, published under its own name.
 *
 * A bank's cost of funds is a FACT ABOUT THE BANK with one writer (Law 4), and it is public because
 * something else in this world prices off it: the bank's own dealing line carries inventory funded
 * by the liabilities this number is the cost of (Dealer Desks D3), a firm weighs a project against
 * what borrowing costs, and a schedule in a bond market is built on it. It is a read of what
 * already left the bank (Observer A5) and it causes nothing by itself.
 */
function publishCostOfFunds(rows: readonly BankDecl[], ctx: MechanismContext): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    if (declOf(rows, b.id) === undefined || !b.status.alive) continue;
    const home = ctx.registry.currencyOf(b.region);
    // Law 8, Currency A3: ONE PER CURRENCY IT OWES IN. What funding costs a bank is a number in a
    // money — it is what it paid on what it owes, and it owes in every money it has taken a
    // liability in. This published its HOME currency only, while `publishQuotes` prices a loan off
    // `costOfFunds(bank, the BORROWER's currency)`: a bank lending to a European firm quoted off a
    // euro funding cost that nothing in this world had published, so the one number every reader is
    // supposed to share existed for a dollar and nowhere else (Law 4, Law 19).
    // Currency A3, Law 8: AND IN EVERY OTHER MONEY IT MIGHT LEND IN. `publishQuotes` asks every
    // bank what it would lend a European firm and prices that quote off
    // `costOfFunds(bank, the BORROWER's currency)` — so a dollar bank quoting a euro loan priced it
    // off a euro funding cost that nothing in this world had published, and the one number every
    // reader is supposed to share existed for a dollar and nowhere else (Law 4, Law 19).
    //
    // It stays ONE event per bank, because a reader asking a bank what money costs it means its own
    // money and `lastOwn` must not depend on which currency a loop reached last. The others are
    // beside it, named, and the home one is not repeated among them.
    const alsoIn: Record<string, FundingCost> = {};
    for (const ccy of ctx.registry.currencies.keys()) {
      if (ccy !== home) alsoIn[ccy] = costOfFunds(ctx, b.id, ccy);
    }
    ctx.record(
      'bank.costOfFunds',
      [b.id],
      // B2.b, Law 4: the blend AND ITS PARTS, so what it paid and what its capital costs it are
      // readable separately by whoever needs one of them — and so that nobody has to re-derive
      // either from the other (Law 19).
      { bank: b.id, ccy: home, ...costOfFunds(ctx, b.id, home), alsoIn },
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
