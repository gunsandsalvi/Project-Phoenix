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
import { atMostCash, negated, noCash, sumCash } from '../../core/measure.js';
import {
  asCash,
  asPerPiece,
  amountOf,
  asRatio,
  heldAsMoney,
  type Cash,
  over,
  type PerPiece,
  plus,
  type Ratio,
  ratioOf,
  minus,
  scale,
} from '../../core/measure.js';
import { Missing } from '../../core/errors.js';
import type { Family, Violation } from '../../audit/audit.js';
import { asQty, downTick, NO_QTY, type Qty } from '../../core/tick.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { Event } from '../../journal/journal.js';
import { period, type Period } from '../../calendar/calendar.js';
import { rate as perAnnum } from '../../core/rate.js';
import { addMonths } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { currencyUnit, moneyInstrumentId, paramId, partyId } from '../../core/ids.js';
import { weightOf, gridPerMember } from '../../parties/party.js';
import { atMost, dustOf, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { Instrument } from '../../register/instruments.js';
import type { OverdraftContext, OverdraftDecision } from '../../registry/kinds.js';
import { BANK, FUND } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { ParamDecl } from '../../registry/params.js';
import type { SystemModule } from '../../world/module.js';
import { bankParam, lineParam, DEALING, TRADING_BOOK_RISK_WEIGHT, type BankDecl } from './data.js';
import { capitalOf, publish, type CapitalRules } from './capital.js';
import { arbitrage, dealingOrders, deskBorrows, publishDealing } from './dealing.js';
import {
  classesSeen,
  liquidityPlan,
  liquidityTargets,
  publishBuffer,
  P_COVERAGE,
  ownDeposits,
  sessionOrders,
  setBoard,
} from './treasury.js';
import { isSub, runRaise, subordinatedKind, subordinatedOf, SUB_PARAMS } from './subordinated.js';
import {
  advisoryOrders,
  costOfAProcess,
  operatingCostOf,
  processesRun,
  staffOrders,
  STAFF_PARAMS,
} from './staff.js';
import { financedFor, type PrimeDeps, PRIME, runPrime } from './prime.js';
import { LENDING, publishLines, roomFor } from './lines.js';
import { LOAN, creditorOf, loanId, loanKind, isLoan, type LoanTerms } from './loan.js';
import { TERM_MONTHS } from '../../registry/credit.js';
import {
  benchmarkNow,
  type Covenants,
  FACILITY,
  loanRate,
  rateOn,
  facilityLoanId,
  type FacilityTerms,
  isFacility,
} from '../../registry/credit.js';
import type { Holding } from '../../register/register.js';
import {
  CREDIT_DAY_COUNT,
  creditInputs,
  creditView,
  type CreditView,
  exposureTo,
  lossGivenDefault,
  room,
  type Quote,
  type Regulation,
} from './credit-view.js';
import { pathsOf, rolls, takes } from './workout.js';
import { weightOfName } from './capital.js';
import { type Statement } from '../../registry/statements.js';
import { creditDefaults } from '../../registry/banking.js';
import { paperOfferedIn } from '../../registry/notices.js';
import { ALLOTTED } from './lines.js';
import { KEPT_BACK } from './treasury.js';

export * from './data.js';
export { DEALING, LENDING, roomFor } from './lines.js';
export * from './loan.js';
export * from './capital.js';
export * from './subordinated.js';
export * from './treasury.js';
export * from './dealing.js';
export * from './dealing-quote.js';
export * from './credit-view.js';

export const LENDING_PARAMS = {
  capitalRatio: paramId('regulation.capitalRatio'),
  riskWeight: paramId('regulation.riskWeight.loan'),
  sovereignWeight: paramId('regulation.riskWeight.sovereign'),
  undrawnWeight: paramId('regulation.creditConversion.undrawn'),
  leverageRatio: paramId('regulation.leverageRatio'),
  hoursPerLoanPeriod: STAFF_PARAMS.hoursPerLoanPeriod,
  /** A2: how long a loan runs for, in MONTHS, because that is what the calendar places (Law 8). */
  loanMonths: paramId('lending.loanMonths'),
} as const;

/** What a bank was asked for, by whom, and what it said (C3.a: a decline is an answer). */
interface Book {
  /** Money B3.a: what the kernel allowed as a drawing this period, waiting to become a row. */
  draws: { holder: string; issuer: string; ccy: string; amount: Cash }[];
}

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('book', () => ({ draws: [] }));
}

/**
 * Law 4, Law 19 (item 9.1): THE NEXT FREE ROW NAME, asked of the register rather than of a counter
 * this module keeps beside its own working state.
 *
 * A loan and a subordinated line are INSTRUMENTS — the register is their home and it is the one
 * writer of what exists — so a sequence kept here was a second thing to keep true, and it counted
 * ATTEMPTS rather than rows: `b.next` moved on a raise that took nothing and on a loan whose
 * settlement failed, so the names it produced had gaps that meant nothing.
 */
function freeLine(ctx: MechanismContext, name: (n: number) => InstrumentId): InstrumentId {
  for (let n = 1; ; n += 1) {
    const id = name(n);
    if (!ctx.instruments.has(id)) return id;
  }
}

function regulationOf(view: ParticipantView): Regulation {
  return {
    capitalRatio: view.params.ratio(LENDING_PARAMS.capitalRatio),
    riskWeight: view.params.ratio(LENDING_PARAMS.riskWeight),
    // C1.d (13d): what servicing costs is what this bank's own staff cost it over the book they
    // service — a read, and it replaces `loan.operatingCost`, which was a wage bill charged to
    // every borrower and paid to nobody (XI-14, Law 5).
    operatingCost: operatingCostOf(view),
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
    undrawnWeight: ctx.params.ratio(LENDING_PARAMS.undrawnWeight),
    // Dealer Desks F2: the same target the dealing line quotes around, read once and used for both
    // — what a holding weighs and what the book may be worth are one line drawn in one place.
    targets:
      decl === undefined
        ? new Map<InstrumentId, Cash>()
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
  for (const p of ctx.parties.ofKind(BANK)) {
    if (!p.status.alive || declOf(rows, p.id) === undefined) continue;
    const short = mustRaise(rows, ctx, p.id);
    if (short <= 0) continue;
    runRaise(ctx, p.id, ctx.registry.currencyOf(p.region), short);
  }
}

/**
 * C2.b (item 10d): THE LENDERS' SIDE, as a schedule the kernel asks for like every other.
 *
 * This loop used to live inside `runRaises`, which gathered every other bank's bid by hand and
 * handed them to a venue this module cleared itself. A schedule is a participant's and a book is
 * the kernel's, so what was a private gather is now a declaration (Clearing B2) — and the same
 * face answers for a subordinated line that answers for every other market a bank is in.
 *
 * Each lender prices THE NAME the way it prices any unsecured claim on it, subscribes out of the
 * money it actually holds, and will not go past its own limit for that name (F3). A lender that
 * cannot cost its own funding does not quote a rate, so it does not bid (A-45) — no view, no money.
 */
function subscribes(
  view: ParticipantView,
  m: MarketDecl,
  rows: readonly BankDecl[],
): readonly Order[] {
  if (!('instrument' in m)) return [];
  const line = view.instruments.get(m.instrument);
  if (!line.status.live || !isSub(line.terms) || !line.issuer.some) return [];
  const issuer = line.issuer.value;
  // C2.b: it does not subscribe to its own paper. A bank buying its own capital has raised nothing.
  if (issuer === view.self.id) return [];
  const decl = declOf(rows, view.self.id);
  if (decl === undefined) return [];
  // N9, §46: what a unit is worth TO IT at the rate it requires — its own view, so two lenders
  // requiring different things bid different levels and the book has a shape (Expectations A3).
  const required = view.params.perAnnum(bankParam(decl.bank, 'returnOnCapital'));
  const worth = view.worth(m.instrument, required);
  if (!worth.some || worth.value <= 0) return [];
  const appetite = roomFor(view, LENDING);
  if (!appetite.some) return [];
  const spare = atMostCash(
    appetite.value,
    heldAsMoney(view.cash(m.ccy), m.ccy, 'the money it holds'),
    'it subscribes out of the money it has',
  );
  if (spare.pieces <= 0) return [];
  const qty = downTick(amountOf(spare, worth.value, 'units it bids for'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: worth.value, qty }];
}

/**
 * M&A B4, §29 D1 (10f.4): WHAT THIS BANK WILL RUN A SALE FOR, AND HOW MANY IT CAN RUN AT ONCE.
 *
 * Published like everything else a bank's counterparties need (Observer A3, Law 19), so a seller
 * choosing who runs its process reads what the banks said rather than being told by this module.
 * A bank that cannot price an hour of the trade or has nobody in it publishes nothing, which is the
 * refusal and not a zero: it is not in the business this period.
 *
 * WHAT IT CHARGES IS WHAT THE WORK COSTS IT, and there is no percentage of a deal anywhere. The
 * seller appoints the cheapest of them with room, so the fee falls to what the keenest can do it
 * for, and what stops the fall is that a bank which cannot cover its people stops publishing
 * (Law 6: the refusal is the mechanism, and 10e.4 set the same one for a manager's fee).
 */
function publishAdvisory(ctx: MechanismContext): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    if (!b.status.alive) continue;
    const view = ctx.participant(b.id);
    const capacity = processesRun(view);
    if (capacity <= 0) continue;
    const fee = costOfAProcess(view);
    if (fee === undefined) continue;
    ctx.record(
      'advisory.quoted',
      [b.id],
      { bank: String(b.id), fee, capacity, ccy: ctx.registry.currencyOf(b.region) },
      true,
    );
  }
}

/** B3: what it published that it must raise to be back above both lines, or nothing (Law 19). */
function mustRaise(rows: readonly BankDecl[], ctx: MechanismContext, bank: PartyId): number {
  const last = ctx.journal.lastOf('bank.capitalPlan', bank);
  // Only the plan it published at the LAST close: a plan from a month ago is a fact about a month
  // that is over, and a bank that has since raised or earned its way back is not raising again.
  if (last === undefined || last.period + 1 !== ctx.period) return 0;
  const short = last.data['short'];
  return typeof short === 'number' && short > 0 ? short : 0;
}

/** C1.b: every default anybody published — public, so every bank saw them (Expectations A2). */
function seenDefaults(ctx: MechanismContext): readonly Event[] {
  return creditDefaults(ctx.journal);
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
  /**
   * B2: the blend — what one unit of what funds this bank's book costs it, per annum.
   *
   * A-45, Missing is Missing: NONE where the bank cannot cost its funding, which is three real
   * states — a bank funded by nothing at all, the opening period before anything has been paid, and
   * a period of no length. Zero is not "unknown" here, it is "MONEY IS FREE", and it used to flow
   * straight into `quote()` and into the desk's edge: in period 0 every bank in the world quoted as
   * if its funding cost it nothing. A bank that cannot cost its funding does not quote a rate,
   * which is what the surrounding code does everywhere else.
   */
  readonly perAnnum: Option<Ratio>;
  /** B2.b: what it ACTUALLY PAID on what it owes last period, annualised. Read off the wire. */
  readonly interest: Cash;
  /** XI-4: what its owners require on the part of the book they fund. Nothing where they fund none. */
  readonly onCapital: Cash;
  readonly owed: Cash;
  /** A1: the RESIDUAL, as it stands — negative for a bank that is insolvent, and said so. */
  readonly capital: Cash;
}

function costOfFunds(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): FundingCost {
  const owed = owedBy(ctx, bank, ccy);
  // Currency B1, C4 (16.0): its residual is kept in its home money; what funds a book in another
  // money is that residual TRANSLATED at the rate in force — a report, never a conversion (B3).
  const capital = ctx.valuation.inMoney(ctx.participant(bank).equity(), ccy, ctx.period);
  /**
   * Banks Capital A1, A2, XI-4 (17.0): WHAT FUNDS THE BOOK is what it owes plus the capital layer —
   * the equity that absorbs first and the subordinated claims that absorb next (A2.a, A2.b). The
   * subordinated layer costs it the coupon it pays, which is in `interest` below; the equity costs
   * it what its owners require; and neither is counted twice, because `owedBy` leaves the layer
   * out of what it owes.
   *
   * A BANK WHOSE RESIDUAL IS A HOLE IS INSOLVENT, and an insolvent bank cannot cost its funding:
   * there is nothing for its owners to require a return on and no book its liabilities fund. The
   * floor that used to stand here (`atLeastCash(capital, 0)`) hid that state as free capital; Law 6
   * says the state is the answer. It is said under the bank's own name (`bank.insolvent`, at
   * `publishCostOfFunds`), the resolution trigger reads its published capital, and until then it
   * quotes nothing — which is what a bank in the hands of its resolver does.
   */
  if (capital.pieces < 0) {
    return {
      perAnnum: none<Ratio>(),
      interest: asCash(0, ccy, 'an insolvent bank has no cost of funds to say'),
      onCapital: noCash(ccy),
      owed,
      capital,
    };
  }
  const layer = plus(
    capital,
    ctx.valuation.inMoney(subordinatedOf(ctx, bank), ccy, ctx.period),
    'the capital layer',
  );
  const funding = plus(owed, layer, 'what funds its book');
  const required = ctx.params.perAnnum(bankParam(bank, 'returnOnCapital'));
  const onCapital = scale(capital, required, 'what its own capital costs it');
  const blend = (interest: Cash): FundingCost => ({
    perAnnum:
      funding.pieces <= 0
        ? none<Ratio>()
        : some(
            ratioOf(plus(interest, onCapital, 'what its funding costs it'), funding, 'per annum'),
          ),
    interest,
    onCapital,
    owed,
    capital,
  });
  const unknown = (why: string): FundingCost => ({
    perAnnum: none<Ratio>(),
    interest: asCash(0, ccy, why),
    onCapital,
    owed,
    capital,
  });
  // A-45: three states, and none of them is "money is free". A bank funded by nothing has no blend
  // to strike; the opening period has nothing paid to strike it from; a period of no length has no
  // year to annualise over. Each says so, and a reader that cannot go on without one stops.
  if (funding.pieces <= 0) return unknown('a bank funded by nothing has no cost of funds');
  if (ctx.period === 0)
    return unknown('nothing has been paid yet, so nothing says what funds cost');
  const previous = period(ctx.period - 1);
  // Law 8: a rate is per annum, so what it paid over this period is divided by the fraction of a
  // year the period actually was — read off the calendar's own dates, never a periods-per-year.
  const year = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(previous),
    ctx.calendar.startOf(ctx.period),
  );
  if (year <= 0) return unknown('a period of no length annualises to nothing');
  return blend(
    over(
      couponsPaid(ctx, bank, ccy),
      asRatio(year, 'the fraction of a year this period was'),
      'what it paid on what it owes, per annum',
    ),
  );
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
  readonly byBank: Map<string, Cash>;
}

function couponsPaid(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): Cash {
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
          before === undefined
            ? asCash(leg.amount, leg.ccy, 'a coupon it paid')
            : plus(before, asCash(leg.amount, leg.ccy, 'a coupon it paid'), 'coupons it paid'),
        );
      }
    }
    held.walked = some(ctx.period);
  }
  // A bank that paid no coupon in that period paid nothing, and nothing is a number: the walk
  // above visited every settled instruction of it, so an absence here is an answer and not a gap.
  // `zeroIfNone` is the one place that says so, and it says it for quantities only (Appendix A).
  const paid = held.byBank.get(`${bank}\u0000${ccy}`);
  return paid ?? noCash(ccy);
}

/**
 * What this bank owes: every liability of its own that anybody holds.
 *
 * Law 18: the instruments an ISSUER has out, by name. Walking every instrument in the world to find
 * one bank's own was the second of the two scans a shopping borrower paid for, and a world with a
 * share line per listed firm has hundreds of them.
 */
function owedBy(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): Cash {
  const terms: Cash[] = [];
  for (const i of ctx.instruments.issuedBy(bank)) {
    if (i.ccy !== ccy || !i.status.live) continue;
    if (!ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    // Banks Capital A2.b (17.0): the subordinated layer is capital, counted there and not here.
    if (isSub(i.terms)) continue;
    // Money A1: what a bank owes is owed AT ITS FACE, so what it has issued of a liability is what
    // it owes — one of itself for each unit (item 16: the door, said once).
    terms.push(asCash(i.issued, i.ccy, `what ${bank} owes on ${i.id}`));
  }
  return sumCash(ccy, terms, `what ${bank} owes`).value;
}

/**
 * C1, C2, C3.a, Law 4, Law 19 (13f, finding `12d-3`): THE BORROWER TAKES THE QUOTE IT WAS GIVEN.
 *
 * Every bank works out what it requires of a name and the keenest is PUBLISHED under that name
 * (`publishQuotes`). This used to work the whole thing out a second time when the borrower came
 * back, and the two answers disagreed: same borrower, same period, same bank, quoted
 * 0.013676115348016367 and written 0.013676161104839884. Three parts in a million, which is nowhere
 * near the dust of either derivation — the inputs move between the two phases, because a bank reads
 * its cost of funds afresh and what it has seen default afresh. One fact, two writers, and a
 * borrower took a loan at a rate it was not quoted with both numbers published under its name.
 *
 * So there is one derivation and this is the READ of it. What is written is what was quoted. The
 * second derivation is gone; `quote()` is called in exactly one place now.
 *
 * C3.a: and where the quoting bank cannot lend after all — its room moved between the quote and the
 * request, which is a real thing that happens — that is a REFUSAL to record, not a silently
 * different price. A bank that never says no has no credit standard.
 */
function shop(
  rows: readonly BankDecl[],
  ctx: MechanismContext,
  borrower: PartyId,
  want: Cash,
): {
  readonly best: Quote | undefined;
  readonly lend: Cash;
} {
  const quoted = quotedFor(ctx, borrower);
  if (quoted === undefined) {
    // C3, C3.a: NOBODY WOULD QUOTE THIS NAME. Declining IS the credit decision, so each bank says
    // which of its OWN constraints stopped it (B2.d) — and it is asked here, where there is a
    // request with an amount on it, because what C3.a makes visible is declined VOLUME.
    refuse(rows, ctx, borrower, want);
    return { best: undefined, lend: noCash(want.ccy) };
  }
  const decl = declOf(rows, quoted.bank);
  const bank = ctx.parties.get(quoted.bank);
  if (decl === undefined || !bank.status.alive) {
    ctx.record(
      'credit.declined',
      [quoted.bank, borrower],
      {
        bank: quoted.bank,
        borrower,
        asked: want.pieces,
        ccy: want.ccy,
        binds: 'the bank that quoted it has gone',
      },
      false,
    );
    return { best: undefined, lend: noCash(want.ccy) };
  }
  const r = room(ctx.participant(quoted.bank), decl, exposureTo(ctx.participant(quoted.bank), borrower));
  if (r.most.pieces <= 0) {
    refuse(rows, ctx, borrower, want);
    return { best: undefined, lend: noCash(want.ccy) };
  }
  return {
    best: quoted,
    lend: atMostCash(r.most, want, 'nobody lends more than the borrower asked for'),
  };
}

/**
 * C3, C3.a, B2.d: what each bank says when it will not have this name, and WHY — its own binding
 * constraint, which is the whole of what a credit standard is. A bank that never says no has none.
 */
function refuse(
  rows: readonly BankDecl[],
  ctx: MechanismContext,
  borrower: PartyId,
  want: Cash,
): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (decl === undefined || !b.status.alive || b.id === borrower) continue;
    const cv = creditViewFor(rows, ctx, b.id, want.ccy);
    const declines = cv === undefined ? none<string>() : cv.of(borrower).declines;
    const r = room(ctx.participant(b.id), decl, cv === undefined ? exposureTo(ctx.participant(b.id), borrower) : cv.exposureTo(borrower));
    if (r.most.pieces > 0 && !declines.some) continue;
    ctx.record(
      'credit.declined',
      [b.id, borrower],
      {
        bank: b.id,
        borrower,
        asked: want.pieces,
        // C3.a: WHY — the constraint that bound, or the view's own reason (17.0): a name that would
        // not open its books, one whose earnings do not cover its debt, one the market prices worse.
        binds: declines.some ? declines.value : r.binds,
        capitalRoom: r.capital.some ? r.capital.value.pieces : null,
        appetiteRoom: r.appetite.pieces,
        fundingRoom: r.funding.some ? r.funding.value.pieces : null,
        ccy: want.ccy,
      },
      false,
    );
  }
}

/**
 * Prime Brokerage B1, B3, C3 (item 13.3): the three things the broker's period is allowed to do with
 * this module's own lending machinery — and nothing else. It is passed in rather than imported the
 * other way so that `prime.ts` decides and this file acts, which is the same split `staff.ts` has.
 */
function primeDeps(rows: readonly BankDecl[]): PrimeDeps {
  return {
    financed: (ctx, broker, client, ccy) => financedFor(ctx, broker, client, isLoan, ccy),
    /**
     * B3: at the rate THIS broker charges THIS client — its own cost of funds, its own view of the
     * borrower, its own capital charge — computed by the one function that prices a bank's credit
     * (Law 4). A broker that could not cost its own funding does not lend, which is A-45's answer
     * everywhere else in this module.
     */
    lend: (ctx, broker, client, amount, ccy): boolean => {
      const decl = declOf(rows, broker);
      if (decl === undefined) return false;
      const cv = creditViewFor(rows, ctx, broker, ccy);
      if (cv === undefined) return false;
      const q = cv.of(client);
      if (q.declines.some) return false;
      // C9, F1.a: one row per (lender, borrower) — a client that comes back is drawing on what it
      // already has here, never taking a new loan every week.
      return (
        // C9 (17b.8): a prime broker's financing is a LINE, over the term a line is written for.
        write(ctx, broker, client, amount, q.rate, ccy, ctx.params.months(TERM_MONTHS.working), true) !==
        undefined
      );
    },
    /**
     * C3: the money comes back and what is owed falls by it. It is the reverse of the drawing and
     * the same two legs: the units the broker holds go back to the party that issued them, and the
     * money goes the other way, in ONE instruction (XI-5).
     *
     * It pays what the client HAS. What it has not got is not borrowed from somewhere to cover the
     * call and is not forgiven: it stays owed, and the caller records it (Law 6, C3.b).
     */
    repay: (ctx, broker, client, amount, ccy): Cash => {
      const line = lineOf(ctx, broker, client);
      if (line === undefined) return noCash(ccy);
      const owed = ctx.register.quantity(broker, line.id);
      const account = ctx.accountOf(client, ccy);
      const cash = ctx.register.quantity(client, moneyInstrumentId(account.issuer, ccy));
      const paying = ctx.registry.payable(
        atMostCash(
          atMostCash(
            amount,
            heldAsMoney(owed, ccy, 'what it owes on the row'),
            'it cannot repay more than it owes',
          ),
          heldAsMoney(cash, ccy, 'what is in its account'),
          'it cannot pay money it has not got',
        ),
      );
      if (paying <= 0) return noCash(ccy);
      const r = ctx.settle({
        legs: [
          {
            kind: 'asset',
            from: broker,
            to: client,
            instrument: line.id,
            qty: paying,
            pricePerUnit: some(asPerPiece(1, 'at what it promised')),
            accruedPerUnit: none(),
          },
          {
            kind: 'money',
            from: ctx.accountOf(client, ccy),
            to: ctx.accountOf(broker, ccy),
            ccy,
            amount: paying,
          },
        ],
        cause: 'maturity',
        reason: `${client} meets a margin call from ${broker}`,
      });
      return r.outcome === 'settled' ? heldAsMoney(paying, ccy, 'what it paid') : noCash(ccy);
    },
  };
}

/** Law 19: the keenest quote published under this name, most recent first. Read, never rebuilt. */
function quotedFor(ctx: MechanismContext, borrower: PartyId): Quote | undefined {
  const e = ctx.journal.lastOf('credit.quoted', borrower);
  if (e !== undefined) {
    const bank = e.data['bank'];
    const rate = e.data['rate'];
    if (typeof bank === 'string' && typeof rate === 'number') {
      return {
        bank: bank as PartyId,
        rate: asRatio(rate, 'the rate it quoted'),
        costOfFunds: numberIn(e.data['costOfFunds']),
        expectedLoss: numberIn(e.data['expectedLoss']),
        capitalCharge: numberIn(e.data['capitalCharge']),
        operatingCost: numberIn(e.data['operatingCost']),
      };
    }
  }
  return undefined;
}

/** A published component, read back as it was written. Missing is Missing, and nothing defaults. */
function numberIn(v: unknown): Ratio {
  if (typeof v !== 'number') {
    throw new Missing('Law 4', 'a quote published without one of the terms it was built from');
  }
  // Item 16: a published quote's terms are RATES, and they re-enter the type system here.
  return asRatio(v, 'a term of a quote this bank published');
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
  wanted: Cash,
  rate: Ratio,
  ccy: CurrencyCode,
  /**
   * Corporate Credit C9, A3: whether this is a drawing on the borrower's LINE at this bank. A line
   * is drawn and repaid at the borrower's option, so it is ONE row that its outstanding moves on —
   * never a new loan every period, which would turn a facility into a pile of term loans and make
   * the borrower's exposure a thing you have to add up rather than a thing you can look at.
   */
  /**
   * Corporate Credit A2 (17b.8): HOW LONG THE BORROWER SAID IT NEEDS IT FOR. A term is a decision
   * about a need and the need is the borrower's — a roof over decades, a stock of grain over weeks
   * — and it used to be one number in this module for every loan in the world (21.60(a)).
   */
  months: number,
  onTheLine = false,
  /**
   * A4 (13d): WHAT IT IS SECURED ON, in the words of whoever asked. A bank does not know what a
   * dwelling is and must not: what it knows is that the request named an instrument and a quantity
   * it could take and realise, which is what security IS. Empty is unsecured and is stated either
   * way, and `lossGivenDefault` is the one place the difference is priced.
   */
  security: readonly { readonly instrument: InstrumentId; readonly qty: Qty }[] = [],
): InstrumentId | undefined {
  // Law 8, B1: money is created in whole pieces of itself, so a loan is drawn in whole pieces. What
  // the arithmetic asked for below one piece is not lent, because it is not money.
  /**
   * XI-15, Law 8 (13d): A BORROWER CAN BE A CELL. Everything a cell does is denominated per member —
   * a household that borrows is a million households each borrowing its own share — so the drawing
   * is struck per member and the total is that times how many of them there are. A named borrower
   * stands for one of itself and this is the amount it asked for, to the piece.
   */
  const who = ctx.parties.get(borrower);
  const members = weightOf(who);
  const share = gridPerMember(
    ctx.registry,
    who,
    over(wanted, asRatio(members, 'the members it has'), 'per member').pieces,
  );
  const principal = share.total;
  if (principal <= 0) return undefined;
  const existing = onTheLine ? lineOf(ctx, bank, borrower) : undefined;
  if (existing !== undefined) return draw(ctx, existing, principal, ccy);
  const id = freeLine(ctx, (n) => loanId(bank, borrower, n));
  const drawn = ctx.calendar.startOf(ctx.period);
  /**
   * B4 (17d.2): WHAT THE QUOTE BECOMES ON THE ROW. A bank quotes a NAME a rate; that rate is what
   * money costs plus what this borrower costs on top, so the margin is what is left when the
   * fixing is taken out and it is the margin that is struck. Where the benchmark has never fixed
   * there is nothing to float over and the row is a fixed one, which is the honest answer (17d.3).
   */
  const fixing = benchmarkNow(ctx.journal, ctx.calendar, ccy, ctx.period);
  const terms: LoanTerms = {
    kind: LOAN,
    originator: bank,
    borrower,
    ...loanRate(rate, fixing),
    drawn,
    // A2: a year, placed by date like every other maturity in this world (Money G3.a). It is the
    // calendar's own month arithmetic and not `civil(y + 1, m, d)`, which is not a date when the
    // day is a leap day and stopped the world the first time a loan was drawn on one (item 0).
    maturity: addMonths(drawn, months),
    dayCount: 'ACT/365F',
    // Bond F3, Small-Business Pools B1, Housing C2 (11.2): a TERM LOAN — one the borrower said it
    // repays on a schedule — repays its principal every period it has left; a LINE is drawn and
    // repaid at the borrower's option (C9) and falls due once. It is the same fact as `onTheLine`
    // seen from the borrower's side, and it is struck here, at origination, like the rate.
    amortising: !onTheLine,
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
      pricePerUnit: some(asPerPiece(1, 'at what it promised')),
      accruedPerUnit: none(),
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
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${bank} lends ${principal} to ${borrower}`,
  });
  if (r.outcome !== 'settled') return undefined;
  ctx.record(
    'credit.written',
    [bank, borrower, id],
    { bank, borrower, loan: id, principal, rate },
    false,
  );
  return id;
}

/**
 * Corporate Credit B4, Bond N5.b, XI-7 (17d.2): THE ROWS RESET, and that is the whole of floating.
 *
 * A borrower's payment moves when money does, and this is where. Every live row that floats has its
 * coupon restruck to the margin it was written at plus what the overnight book compounded to over
 * the period that just ended — the kernel's one writer of what a fixing does to a line, which
 * records it publicly (`coupon.fixed`) because a holder of the claim learns what it pays next.
 *
 * It runs BEFORE the period's dated actions, because a coupon that fixed after the interest it
 * applies to fell due would be a rate nobody could have known they were paying (Clearing F1.a).
 *
 * A row whose benchmark did not fix this period KEEPS WHAT IT HAD, and that is not a posted
 * benchmark: nothing was published, so nothing reset, and the row goes on paying what it last
 * agreed to pay until the book trades again. What is forbidden is inventing a fixing, not a
 * contract that has not reset.
 */
function fixFloatingRows(ctx: MechanismContext): void {
  const byCcy = new Map<string, Option<{ readonly named: string; readonly perAnnum: Ratio }>>();
  for (const i of ctx.instruments.ofKind(LOAN)) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    const t = i.terms;
    if (!t.floatsOver.some) continue;
    // Law 18: one fixing per money, not one per row — every loan in a currency floats over the
    // same book, and the read walks the journal.
    let fixing = byCcy.get(i.ccy);
    if (fixing === undefined) {
      fixing = benchmarkNow(ctx.journal, ctx.calendar, i.ccy, ctx.period);
      byCcy.set(i.ccy, fixing);
    }
    if (!fixing.some || fixing.value.named !== t.floatsOver.value) continue;
    const now = plus(t.margin, fixing.value.perAnnum, 'the margin and what money cost');
    if (now === rateOn(t)) continue;
    ctx.fixCoupon(i.id, perAnnum(now, { kind: 'annual' }), fixing.value.named);
  }
}

/**
 * §29 B2, B2.b, E1, Banks Lending A3.a, Corporate Credit C9 (17b.1): THE LENDER AGREES TO LEND AND DOES NOT LEND.
 *
 * Nothing moves and nothing is created. What exists after this is a named bank's promise to a named
 * borrower, at a size and a rate that bank decided, and the bank's OWN CAPITAL STANDS BEHIND IT from
 * now (`headroom`) — which is the whole difference between a commitment and a kind word, and the
 * reason a bank cannot commit to every deal in the world at once. The money is made when it is
 * drawn, inside the instruction that draws it, so a deal that does not close never made any.
 *
 * And it LAPSES. A commitment with no end is a free option the lender did not sell (§18 B4's
 * sentence, as true here), and what this one is — an underwritten commitment for a deal that has to
 * close — stands behind the borrower for the period the deal has to happen in and no longer.
 */
function commit(
  ctx: MechanismContext,
  bank: PartyId,
  borrower: PartyId,
  limit: Cash,
  rate: Ratio,
  ccy: CurrencyCode,
  /** A2 (17b.8): the term the borrower asked for, carried on the promise so the drawing has it. */
  months: number,
): void {
  if (bank === borrower || !ctx.parties.get(bank).status.alive) return;
  /**
   * Corporate Credit C9 (17b′.2): ONE LINE PER LENDER PER BORROWER — so a borrower that comes back
   * while one stands is not written a second one. It is INCREASED.
   *
   * *"A draw taps the existing line at the margin it was struck at; a new line opens only when none
   * is live."* A committed line that has been drawn does not lapse (17b.8a), so after a buyout the
   * company has a live line with its lender and nothing left on it; without this, the
   * recapitalisation C3 describes could never be funded by the bank that funded the purchase, which
   * is the bank that knows the name. What the lender does instead is what a lender does: it looks
   * at the borrower again, and it raises the limit or it does not.
   *
   * The new limit is what is OUTSTANDING plus what its room allows now — `room` already has this
   * borrower's exposure in it, so the two do not double-count — and the covenant is struck again on
   * the accounts as they stand, which is the point of asking for them.
   */
  const already = ctx.agreements
    .ofKind(FACILITY)
    .find((a) => a.state === 'performing' && a.debtor === borrower && a.creditor === bank);
  /**
   * B2, B2.a, Reporting A2 (17b.8a): THE COVENANT THE LENDER ASKS FOR, off the borrower's own
   * published accounts with THIS COMMITMENT on them. *"No worse than this leaves you"* — and what
   * makes it the lender's ask rather than the borrower's arithmetic is that the SIZE is the
   * lender's: it committed what its own room allowed, so a bank that would commit less draws a
   * tighter line.
   *
   * A BORROWER THAT HAS PUBLISHED NOTHING GETS NO COMMITMENT. Terms nobody can test are not terms
   * (Reporting A2.a), and a promise about accounts that do not exist is worse than no promise.
   */
  /**
   * 17b′.2: THE FRESHEST BOOKS IT HAS, whichever act produced them — a closed quarter, or the
   * management accounts it prepared because this ask was for a commitment (`reporting.interim`).
   * *"What this company's books say"* is one question and there is one read of it (Law 4).
   */
  const said = ctx.published.latestAccounts(borrower);
  const annual = scale(limit, rate, 'what the line would cost it a year if it drew all of it');
  // NO ACCOUNTS, NO COMMITMENT. A lender that cannot test a promise has not taken a credit
  // decision, and a commitment with nothing to test is not one (Reporting A2.a, §29 E1).
  if (said === undefined || said.balance.assets.pieces <= 0) return;
  if (said.earned.pieces <= 0 || annual.pieces <= 0) return;
  const covenant: Covenants = {
    leverage: ratioOf(
      plus(said.balance.liabilities, limit, 'what it would owe with this drawn'),
      said.balance.assets,
      'the most it may owe against what it holds',
    ),
    coverage: ratioOf(said.earned, annual, 'the least it must earn against what this costs it'),
  };
  const row = facilityLoanId(bank, borrower);
  const outstanding = asCash(
    ctx.instruments.has(row) ? Number(ctx.register.heldTotal(row).value) : 0,
    ccy,
    'what it has already drawn on the line it has',
  );
  const terms: FacilityTerms = {
    kind: FACILITY,
    limit: already === undefined ? limit : plus(outstanding, limit, 'the line, increased'),
    rate,
    covenant,
    // B2.b: the period the deal has to close in, which is the one after the ask was answered.
    until: period(ctx.period + 1),
    // A2: the term the drawing will run for, struck here because this is where it was agreed —
    // and it is the term the BORROWER asked for (17b.8), not this lender's line convention.
    maturity: addMonths(ctx.calendar.startOf(ctx.period), months),
  };
  if (already === undefined) {
    ctx.owes({
      debtor: borrower,
      creditor: bank,
      ccy,
      // It owes nothing NOW. What it owes is what it draws, when it draws it — which is exactly the
      // difference between a commitment and a debt (Law 2).
      owed: 0,
      terms,
      why: `${String(bank)} commits ${limit.pieces} to ${String(borrower)} for a deal that has not closed`,
    });
  } else {
    // Law 15: the same two parties, the same row, different terms — never a second kind and never a
    // second line (C9). What changed is the size, the date it stands until, and the promise.
    ctx.restate(already.id, terms);
  }
  ctx.record(
    'credit.committed',
    [String(bank), String(borrower)],
    {
      bank: String(bank),
      borrower: String(borrower),
      limit: terms.limit.pieces,
      // C9: whether this opened a line or raised one the borrower already had.
      increased: already !== undefined,
      rate,
      ccy,
    },
    true,
  );
}

/**
 * Corporate Credit B2, B2.a (17b.8a): THE LENDER TESTS WHAT IT ASKED FOR, on the accounts the
 * borrower published.
 *
 * *"Covenants are how credit risk is observed BEFORE a default; without them the only credit
 * dynamic the model has is the binary one, and an assessment has nothing to update on between
 * paying and gone."* A levered company is where that bites hardest: it trips a covenant long before
 * it misses a payment, and the trip is what its lenders, its assessors and its owner react to.
 *
 * It is the same arithmetic the bond's own test does, on the same two published numbers, and it
 * writes the same event — so a breach is one kind of fact however the money was lent (Law 4). One
 * per line per set of accounts: the test is a pure read and costs nothing to repeat, and what must
 * not happen twice is the ANNOUNCEMENT, which the journal is the one writer of (Law 19).
 */
function testFacilityCovenants(ctx: MechanismContext): void {
  for (const a of ctx.agreements.ofKind(FACILITY)) {
    if (a.state !== 'performing') continue;
    const t = a.terms;
    if (!isFacility(t)) continue;
    const promised = t.covenant;
    // 17b′.2: tested on the freshest books it has, which is the same read the promise was struck on.
    const said = ctx.published.latestAccounts(a.debtor);
    if (said === undefined) continue;
    const row = facilityLoanId(a.creditor, a.debtor);
    if (saidAlready(ctx, row, said.quarter)) continue;
    const broke: string[] = [];
    // B2: what it owes against what it holds. A firm with no assets has no ratio that means
    // anything and has breached, which is what the worst case IS rather than a number pushed back.
    const levered =
      said.balance.assets.pieces <= 0
        ? undefined
        : ratioOf(said.balance.liabilities, said.balance.assets, 'what it owes against what it holds');
    if (levered === undefined || levered > promised.leverage) broke.push('leverage');
    // B2, A3.a: what it earns against what the line costs it a year at the rate it was committed at.
    const annual = scale(t.limit, t.rate, 'what the line costs it a year');
    if (
      annual.pieces > 0 &&
      ratioOf(said.earned, annual, 'what it earns against what it costs') < promised.coverage
    ) {
      broke.push('coverage');
    }
    if (broke.length === 0) continue;
    ctx.record(
      'covenant.breached',
      [String(a.debtor), String(row)],
      {
        issuer: String(a.debtor),
        bond: String(row),
        lender: String(a.creditor),
        quarter: said.quarter,
        broke: broke.join(' and '),
        leverage: levered ?? null,
        promised: promised.leverage,
        earned: said.earned.pieces,
        owedPerYear: annual.pieces,
        ccy: a.ccy,
      },
      true,
    );
  }
}

/** Law 19: whether this line's breach on these accounts has already been announced. */
function saidAlready(ctx: MechanismContext, row: InstrumentId, quarter: string): boolean {
  return ctx.journal
    .ofKind('covenant.breached')
    .some((e) => e.data['bond'] === String(row) && e.data['quarter'] === quarter);
}

/**
 * B2.b (17b.1): A COMMITMENT NOBODY DREW LAPSES, and the lender's capital is its own again.
 *
 * It is TERMINATED rather than discharged: nothing was ever owed on it, and what ended it is its own
 * terms rather than a payment (Register: `terminated` says so instead of quietly becoming a
 * discharge). A deal that did not happen leaves the bank exactly where it was, which is what makes
 * committing to the next one a decision it can still take.
 */
function lapseFacilities(ctx: MechanismContext): void {
  for (const a of ctx.agreements.ofKind(FACILITY)) {
    if (a.state !== 'performing') continue;
    const t = a.terms;
    if (!isFacility(t) || ctx.period <= t.until) continue;
    /**
     * B2.a (17b.8a): A LINE THAT WAS DRAWN IS NOT OVER. What lapses is a promise nobody used; a
     * credit agreement the borrower drew on stands while anything is outstanding on it, because the
     * COVENANT is a term of it and a covenant that expired the week after the drawing is not one.
     */
    const row = facilityLoanId(a.creditor, a.debtor);
    if (ctx.instruments.has(row) && ctx.register.heldTotal(row).value > 0) continue;
    ctx.endAgreement(a.id, 'the deal it was committed for did not close');
    ctx.record(
      'credit.lapsed',
      [String(a.creditor), String(a.debtor)],
      { bank: String(a.creditor), borrower: String(a.debtor), limit: t.limit.pieces, ccy: a.ccy },
      true,
    );
  }
}

/**
 * C9: the borrower's live line at this bank, if it has one. One row, whatever it has drawn.
 *
 * D4, XI-11: AT THIS BANK means this bank is owed it NOW. A row this bank wrote and has since sold
 * is somebody else's asset, and a further drawing on it would put the bank's money behind a loan
 * that is not its own — so the register answers, not the terms (Law 19).
 */
function lineOf(ctx: MechanismContext, bank: PartyId, borrower: PartyId): Instrument | undefined {
  // Law 18, A1: a loan is ISSUED BY ITS BORROWER — the borrower owes the money — so the rows that
  // could be this borrower's line are the rows it issued, which the register indexes. Scanning every
  // instrument in the world to find them was a walk per request.
  return ctx.instruments
    .issuedBy(borrower)
    .find((i) => i.status.live && isLoan(i.terms) && ctx.register.quantity(bank, i.id) > 0);
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
  // Register F2, XI-15 (11.1): WHOEVER OWES THE LINE NOW. The terms name who signed it; a cell
  // that merged onto another (its bank moved) or a firm whose estate took its book is succeeded,
  // and the row was reseated onto the successor. The first cell to draw on a line it inherited
  // addressed a party that had ceased, and settlement refused it (Money E4).
  const borrower = ctx.parties.resolve(line.terms.borrower).id;
  // D4: the money comes from whoever is owed the line now, which is whoever holds it (Law 19). A
  // line nobody is owed is a line nobody can be drawn on.
  const owed = creditorOf((id) => ctx.register.holdersOf(id), line);
  if (!owed.some) return undefined;
  const lender = owed.value;
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: borrower,
      to: lender,
      instrument: line.id,
      qty: amount,
      pricePerUnit: some(asPerPiece(1, 'at what it promised')),
      accruedPerUnit: none(),
    },
    {
      kind: 'money',
      from: { holder: lender, issuer: lender },
      // B1.b, as above: the drawing lands in the borrower's own account, wherever that is.
      to: ctx.accountOf(borrower, ccy),
      ccy,
      amount,
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${borrower} draws ${amount} on its line at ${lender}`,
  });
  if (r.outcome !== 'settled') return undefined;
  ctx.record(
    'credit.draw',
    [lender, borrower, line.id],
    { bank: lender, borrower, loan: line.id, amount },
    false,
  );
  return line.id;
}

/**
 * Money B3.a: the credit decision behind an overdraft. It is the same decision as any other loan —
 * the room this bank's own capital supports and its own limit for that name — and what it allows is
 * a DRAWING, which becomes a row before the period closes. A bank with no room refuses, and the
 * payment fails: that is the refusal B3.c requires and it is recorded by settlement.
 */
function overdraft(
  rows: readonly BankDecl[],
  ctx: MechanismContext,
  o: OverdraftContext,
): OverdraftDecision {
  const decl = declOf(rows, o.issuer);
  if (decl === undefined) return { allow: false };
  const view = ctx.participant(o.issuer);
  // A1, XI-8: a loan is a contract with somebody, and some parties are nobody to contract with —
  // an estate is being wound up, and a household has no lender in this world at all (Households
  // C1.d). The kind says so and the bank reads it (Law 15); the refusal is the answer, recorded.
  const borrows = ctx.registry.partyKind(ctx.parties.get(o.holder).kind).borrows;
  const r = room(view, decl, exposureTo(view, o.holder));
  /**
   * B3.a, item 0 (stop 12): AND A BANK THAT CANNOT COST ITS OWN FUNDING DOES NOT ALLOW ONE.
   *
   * What prices the row is this bank's own cost of funds, and in the opening period it has none —
   * it has paid for nothing yet. `bookDraws` used to discover that at the CLOSE, after the drawing
   * had been allowed and the payment made, and threw; so any overdraft in period 0 stopped the
   * world. The decision belongs where the decision is: a lender that cannot price a drawing
   * refuses it, which is an answer (C3.a) and leaves the payment to fail as B3.c says it should.
   */
  const priced = costOfFunds(ctx, o.issuer, o.ccy).perAnnum.some;
  if (!borrows || !priced || r.most.pieces < o.shortfall) {
    ctx.record(
      'credit.declined',
      [o.issuer, o.holder],
      {
        bank: o.issuer,
        borrower: o.holder,
        asked: o.shortfall,
        binds: !borrows
          ? 'nobody lends to a party of this kind'
          : !priced
            ? 'it cannot cost its own funding'
            : r.binds,
        overdraft: true,
      },
      false,
    );
    return { allow: false };
  }
  book(ctx).draws.push({
    holder: o.holder,
    issuer: o.issuer,
    ccy: o.ccy,
    amount: asCash(o.shortfall, o.ccy, 'what it is overdrawn by'),
  });
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
    /**
     * A-45, Money B3.a: WHOEVER ALLOWED THE DRAWING WRITES THE ROW THAT PRICES IT — and a bank that
     * cannot cost its own funding cannot price it. This used to reach `quote` with a cost of zero in
     * the opening period, so the first overdraft of every run was written at a rate struck off free
     * money. An unpriced drawing is a defect in the money issuer that allowed it, and it says so
     * rather than inventing a rate.
     */
    // A-45, B3.a: whoever allowed the drawing writes the row that prices it, and `overdraft` above
    // does not allow one it cannot price — so the cost is there. The throw that used to stand here
    // was the same fact asserted twice, one phase too late (item 0, stop 12).
    const cv = creditViewFor(rows, ctx, bank, d.ccy as CurrencyCode);
    if (cv === undefined) continue;
    // Register F2, Money E4 (14.1): THE BORROWER AS IT IS NOW. A party can draw in one phase and
    // cease in a later one of the same period — a fund wound up into its manager after its last
    // fee overdrew — and the row for what it drew is its successor's to owe; a drawing whose line
    // ceased into nobody is booked to nobody.
    const borrower = ctx.parties.resolve(d.holder as PartyId);
    if (!borrower.status.alive) continue;
    // The bank already allowed the drawing (Money B3.a); what is decided here is its price, and
    // the row is priced off the one view (Law 4) whether or not that view would open a NEW line.
    const q = cv.of(borrower.id);
    // C9: an overdraft is a drawing on the borrower's line, not a new loan every week.
    // C9 (17b.8): a drawing on a committed line, over the term a line is written for.
    write(
      ctx,
      bank,
      borrower.id,
      d.amount,
      q.rate,
      d.ccy as CurrencyCode,
      ctx.params.months(TERM_MONTHS.working),
      true,
    );
  }
}

/**
 * F2: new lending, amortisation, prepayment and write-off account for the change in the book. The
 * book is the sum of the rows a bank holds (F1), so what it was plus what the wire did to it is
 * what it is — and a book that moved with no instruction behind it is the scalar F1.a forbids,
 * arrived at by another route.
 */
/** A row nobody is owed has no holding to walk; naming the absence once keeps the check one read. */
const NO_HOLDING = none<Holding>();

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
        // D4, XI-11: WHOEVER IS OWED IT holds every unit of it — and who that is can have changed
        // since the row was written, because a loan can be sold (D4) and a resolved bank's loans go
        // to its acquirer (Banks Capital D6). The register says who; nothing about the row changed.
        const owed = creditorOf((id) => view.register.holdersOf(id), i);
        // A row repaid to the last unit is owed to nobody, and so is one not yet drawn; what the
        // clause says of it is that nothing is outstanding, which is the same comparison.
        const held = owed.some ? view.register.quantity(owed.value, i.id) : NO_QTY;
        const holding = owed.some ? view.register.holding(owed.value, i.id) : NO_HOLDING;
        // Law 7: `issued` is a running total that carries the dust of every drawing it has taken,
        // and what the lender holds is a sum over the lots those drawings made. The comparison is
        // entitled to both walks and to nothing else.
        const lots = holding.some ? holding.value.lots.length : 0;
        const dust = i.issuedDust + dustOf(lots + 2, Math.abs(i.issued) + Math.abs(held));
        // F1.a: WHOEVER IS OWED IT holds every unit of it. The check used to name the originator
        // and say a sold loan was 13f's problem; a sold loan is now an ordinary thing (XI-11), and
        // what the clause actually says — the outstanding and the holding are one number — is true
        // of the party that owns the row today whoever wrote it.
        if (withinDust(held, i.issued, dust)) continue;
        out.push({
          family: 'flows',
          spec: 'Banks Lending F1.a',
          owner: i.id,
          size: minus(i.issued, held, 'units not with the lender of record'),
          unit: i.unit,
          period: view.period,
          message: `${i.id}: ${i.issued} outstanding and ${owed.some ? String(owed.value) : 'nobody'}, who is owed it, holds ${held}`,
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
/**
 * Banks Lending A1.a, D4, D4.a, XI-11 (17.8): NO BANK LOAN IS HELD OUTSIDE THE BANKING SYSTEM.
 *
 * A loan is not a security. It has no market and no price anybody but its holder can see (A1.a,
 * D1), so what stands behind it is a lender that wrote it, watches the borrower and can enforce —
 * and a row sitting in a pension fund has none of that. D4.a is the same fact from the other end:
 * a loan too large for one bank is written by SEVERAL BANKS, never sold to the public, and the way
 * credit risk actually reaches an investor is a NOTE issued against a pool held by a named vehicle
 * (XI-11, §42 C1) rather than the row itself changing hands into a household's portfolio.
 *
 * IT IS A FORBID, AND A FORBID THAT HOLDS BREAKS SILENTLY (Part II) — which is exactly why it is a
 * family and not a refusal at a door. Nothing in this world sells a loan anywhere it should not
 * today; the day something does, this says so with the holder, the row and how much, and nobody
 * has to have remembered the rule.
 *
 * WHO MAY BE OWED ONE IS THE KIND'S TO SAY (`banking`, Law 15), so adding a kind means answering
 * the question rather than editing a list here. An ESTATE is not one of them and does not need to
 * be: it holds what a dead bank held, and only until it has sold it (XI-8, Register F2) — winding a
 * loan book up is not running one, and a check that fired on it would be reporting a succession.
 */
function loansStayInTheBankingSystem(): Family {
  return {
    name: 'ownership',
    contributor: 'banks',
    spec: 'Banks Lending A1.a Banks Lending D4 Banks Lending D4.a XI-11',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.ofKind(LOAN)) {
        if (!i.status.live) continue;
        for (const holder of view.register.holdersOf(i.id)) {
          if (!view.parties.has(holder)) continue;
          const who = view.parties.get(holder);
          const kind = view.registry.partyKind(who.kind);
          if (kind.banking || kind.terminal === true) continue;
          const units = view.register.quantity(holder, i.id);
          if (units <= 0) continue;
          out.push({
            family: 'ownership',
            spec: 'Banks Lending D4.a',
            owner: String(holder),
            size: units,
            unit: String(i.ccy),
            period: view.period,
            message:
              `${String(holder)} is a ${String(who.kind)} and is owed ${String(units)} of ` +
              `${String(i.id)}: a bank loan is not distributed outside the banking system`,
          });
        }
      }
      return out;
    },
  };
}

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
        const bookCcy = view.registry.currencyOf(view.parties.get(self).region);
        const terms: Cash[] = [];
        for (const [id, row] of Object.entries(lines as Record<string, unknown>)) {
          if (typeof row !== 'object' || row === null) continue;
          const want = (row as Record<string, unknown>)['target'];
          const value = (row as Record<string, unknown>)['worth'];
          if (typeof want !== 'number' || typeof value !== 'number' || value <= want) continue;
          terms.push(
            scale(
              minus(
                asCash(value, bookCcy, `what ${id} is worth`),
                asCash(want, bookCcy, `what ${id} targets`),
                `${id} above its target`,
              ),
              asRatio(weight, 'what this kind weighs'),
              'weighted',
            ),
          );
        }
        const asked = sumCash(bookCcy, terms, 'what its dealing book weighs');
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
          dustOf(2, Math.abs(rwa) + Math.abs(asked.value.pieces));
        if (rwa + dust >= asked.value.pieces) continue;
        out.push({
          family: 'accounts',
          spec: 'Dealer Desks F2',
          owner: bank,
          size: asked.value.pieces - rwa,
          unit: currencyUnit(view.registry.currencyOf(view.parties.get(self).region)),
          period: view.period,
          message: `${bank}: its dealing book weighs ${asked.value.pieces} and it published ${rwa} of risk-weighted assets in total`,
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
    nouns: [
      {
        name: ALLOTTED,
        kind: 'working',
        holds:
          'what each of a bank\u2019s lending lines was allotted this period, and the period it was allotted in',
        why: 'it is how this module gets from its allotment phase to the line that spends the room, and nothing outside it has an opinion about a room nobody has lent out of yet (0e\u2032.4). It was the bank\u2019s own `bank.lines` event read back by its writer in the same period, with every line walking the whole row list out of `unknown[]` to find itself. The event stays as the record of the allotment.',
      },
      {
        name: KEPT_BACK,
        kind: 'working',
        holds:
          'what each bank reckoned it keeps back against a bad week, and the period it reckoned it in',
        why: 'it is how this module gets from its buffer phase to its own treasury\u2019s reading of where it stands, within one period (0e\u2032.4). What the WORLD prices against is the published `bank.buffer`, which is still written and still read across modules through `registry/banking.ts`; this is the same number the same bank reads about itself in the period it reckoned it.',
      },
      {
        name: 'book',
        kind: 'working',
        holds:
          'what the kernel allowed as a customer drawing this period, waiting to become a loan row',
        why: 'the same interval as the money market’s: the kernel has said yes, the row does not exist yet, and by the end of the period it does (Money B3.a).',
      },
      {
        name: 'banks.creditView',
        kind: 'working',
        holds: 'each bank’s credit view this period, per money, and the statements borrowers opened with their asks',
        why: 'a within-period memo of one derivation read by the quote, the row, the provision and the reservation (Law 4, Law 18); it is rebuilt every period from the record and holds nothing the record does not.',
      },
      {
        name: 'banks.recoveries',
        kind: 'working',
        holds: 'the periods walked so far and what each party’s claims on estates were paid and written off',
        why: 'a memo of a walk over the settled ledger that a settled period cannot change (Law 18); the ledger is the source and this is not a second copy of it.',
      },
      {
        name: 'banks.couponsPaid',
        kind: 'working',
        holds:
          'which period this walk covers and what each bank paid in it, so the walk is taken once',
        why: 'a within-period memo of a walk over the settled ledger, so a read made once per bank does not re-walk the period per bank (Law 18). The ledger is the source and this is not a second copy of it.',
      },
    ],
    spec: 'Banks Lending',
    // It needs nobody. What a borrower is short of and what a borrower has failed to pay both reach
    // it as journal events, which are the kernel's — so a world with banks in it can lend whether or
    // not it has firms, and a bank's answer to Money B3.a exists as soon as there is a bank.
    /**
     * Reporting A2 (17b′.2): §48 IS NOT A `requires` AND CANNOT BE. A bank prices credit off the
     * accounts a company prepared, and `reporting` reads what banks published about their own
     * regulation — the two need each other, and `requires` is a DAG (Part XIII refuses the cycle).
     * What orders them is the phase graph, which is where a mutual need belongs: the statement is
     * struck after revaluation and read by the lender in the next period (Clearing F1.a). A world
     * assembled without §48 has banks that lend and never COMMIT, which is *no accounts, no
     * commitment* working exactly as it should.
     */
    requires: [],
    instrumentKinds: [loanKind, subordinatedKind],
    // Prime Brokerage A1 (item 13.3): a named bank and a named client, with a contract that can be
    // ended. It is the ninth kind of commitment in this world and the first between a bank and a pool.
    agreementKinds: [
      {
        /**
         * §29 B2, B2.b, E1, Banks Lending A3.a, Corporate Credit C9 (17b.1): A LENDER HAS AGREED TO LEND AND HAS NOT
         * LENT — the one thing a deal can be made conditional on, because the money it promises is
         * only made when it is drawn.
         */
        id: FACILITY,
        what: 'a bank has committed to lend a named borrower a size at a rate, until it lapses',
        // B2, XI-8: a promise to lend on demand, and an estate lends nothing (Banks Lending A1).
        // An acquirer that bought the committing bank's book stands behind what it promised.
        binds: 'aGoingConcern',
        /**
         * Banks Lending A3.a, A3.b: WHAT IS PROMISED AND NOT DRAWN. The lender's capital stands
         * behind it BEFORE the borrower draws, because the lender cannot refuse when it does — and
         * what has been drawn is read off the drawing's own row in the register (Law 19).
         */
        headroom: (row, _at, reads) => {
          const t = row.terms;
          if (!isFacility(t)) return noCash(row.ccy);
          const drawn = reads.drawnOn(facilityLoanId(row.creditor, row.debtor));
          return minus(t.limit, heldAsMoney(drawn, row.ccy, 'what it has drawn'), 'its headroom');
        },
      },
      {
        id: PRIME,
        // A1, XI-8: an acquirer that bought the book took the clients with it, which is what buying
        // a book is. An estate finances nobody (Banks Lending A1) and the relationship ends.
        binds: 'aGoingConcern',
        what: 'a bank holds a client\u2019s book, decides what it requires against it, and finances the rest',
      },
    ],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: LENDING_PARAMS.loanMonths,
        value: 12,
        unit: 'months',
        dimension: 'months',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Corporate Credit C9, Banks Lending A2 (17b.8): how long a WORKING-CAPITAL LINE runs for — a year is what a commercial facility is written for. It is a convention of the market and it is stated in MONTHS because that is the grain the calendar places a maturity on (Law 8, Money G3.a): a term in years would be converted somewhere, and the conversion is where a duration stops being the number it was declared as. It used to be the term of EVERY loan in this world, a mortgage and a buyout included (finding 21.60(a)); a term is a decision about a need and the need is the borrower\u2019s, so what a loan is written for is now what the ask said (`CreditAsk.months`) and this is the number the borrowers of a LINE ask for. The workout still reads it: a lender re-agreeing a row lends for its own term.',
      },
      {
        id: LENDING_PARAMS.undrawnWeight,
        value: 0.5,
        unit: 'weight on a unit of undrawn commitment',
        dimension: 'ratio',
        kind: 'policy',
        owner: 'standardSetter',
        why: 'Banks Lending A3.a, A3.b: what a PROMISE to lend consumes of a bank\u2019s capital, per unit it has committed and not yet lent. A committed line cannot be refused when it is drawn, so the capital has to stand behind it before it is — and it consumes less than the loan it would become, because not every line is drawn. A half is the supervisor\u2019s judgement of that, and it is a POLICY about a promise rather than a measurement of anything: what it is FOR is that a facility costs a bank something before it is drawn, which is the whole of A3.b \u2014 a facility that costs nothing until drawn is a free option the bank did not sell.',
      },
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
        id: STAFF_PARAMS.hoursPerLinePeriod,
        value: 6,
        unit: 'hours of a dealer per line per period',
        dimension: 'count',
        kind: 'technology',
        owner: 'model',
        why: 'Dealer Desks D1, D4 (13d): what it takes to QUOTE one line — somebody prices it, somebody carries the position, somebody answers the phone. How many lines a desk can cover is therefore the hours it employs over this, so a desk that sheds staff drops lines and their books journal `market.noView` because nobody is standing in them. A coverage stated directly would be a count of people wearing a policy’s clothes.',
      },
      {
        id: STAFF_PARAMS.hoursPerProcess,
        value: 120,
        unit: 'hours of a corporate-finance banker per sale',
        dimension: 'count',
        kind: 'technology',
        owner: 'model',
        why: 'M&A B4, §29 D1 (10f.4): what it takes to RUN A SALE — preparing the company, finding the buyers, running the auction, closing it. Three weeks of somebody, which is what a small process is, and it is what makes an IBD a CAPACITY: how many a bank can run at once is the hours it employs over this, so a bank that sheds its bankers runs fewer sales and a bank that never met a wage runs none. It is also what the bank CHARGES, because what the work costs is the only thing in this world a fee could honestly be — a percentage of the deal is a fee with no work in it (Law 2), and it would make a large sale dearer to run than a small one for no reason anybody could name.',
      },
      {
        id: LENDING_PARAMS.hoursPerLoanPeriod,
        value: 0.6,
        unit: 'hours of a lending officer per loan per period',
        dimension: 'count',
        kind: 'technology',
        owner: 'model',
        why: 'Banks Lending C1.d (13d): what it takes to keep ONE loan — the assessment, the monitoring, the collecting — in hours of somebody who is paid for them. It replaces `loan.operatingCost`, which was half a per cent a year on every principal added into every quote and paid to nobody: a wage bill charged and never paid is margin wearing the clothes of a cost (Law 5), and stating it as a share of the principal made a small loan and a large one cost the same to service, which is the opposite of true. It is per LOAN because a loan costs about the same to make whatever its size, and that is why the cost per unit of principal now falls as the loan gets bigger — out of the arithmetic rather than out of a table.',
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
          id: bankParam(b.bank, 'arrangerFee'),
          value: b.arrangerFee,
          unit: 'share of what an issue raises',
          dimension: 'ratio' as const,
          kind: 'preference' as const,
          owner: 'model' as const,
          why: `Corporate Credit C1, C6, C7.b: what ${b.bank} charges to BRING an issue — its own price for the work of building a book. The RISK half of an underwriting fee is not here: on a backstopped deal it is what this bank requires of the issuer's name over the placement, which it publishes already (E5), so the backstop costs more than best effort as a consequence rather than by a rule (C11.e).`,
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
        // AFTER THE MARKS ARE TAKEN, which is the only moment its book has a value: capital is the
        // residual (A1), and a residual computed against prints that have not happened yet is not one
        // (Clearing F1.a). So a bank lends this period against the position it closed the last one
        // with — a lag, and a real one: a bank finds out what its capital allowed after the quarter
        // it allowed it in, which is exactly why B3's consequences arrive late enough to matter.
        anchor: { after: 'revaluation' },
        reads: [
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'bank.capital' },
          { kind: 'event', name: 'bank.capitalPlan' },
          { kind: 'event', name: 'bank.lines' },
        ],
        run: (ctx: MechanismContext): void => {
          publishCapital(rows, ctx);
        },
      },
      {
        name: 'banks.raise',
        spec: 'Banks Capital A3 Banks Capital C2 Banks Capital C2.a Banks Capital C2.b',
        // Before it decides anything about lending: a bank short of capital raises what it can first
        // and then lends what is left of its room (C2: recapitalisation FIRST).
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'event', name: 'bank.capitalPlan', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'bank.raise.offered' }],
        run: (ctx: MechanismContext): void => {
          runRaises(rows, ctx);
        },
      },
      {
        /**
         * Corporate Credit B4, Bond N5.b (17d.2): THE ROWS RESET, before anything falls due on them.
         * A coupon that fixed after the interest it applies to had fallen due would be a rate
         * nobody could have known they were paying (Clearing F1.a).
         */
        name: 'lending.fix',
        spec: 'Corporate Credit B4 Bond N5.b',
        anchor: { before: 'corporateActions' },
        reads: [{ kind: 'event', name: 'index.benchmark', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'coupon.fixed' }],
        run: (ctx: MechanismContext): void => {
          fixFloatingRows(ctx);
        },
      },
      {
        name: 'lending.write',
        spec: 'Banks Lending B1 Banks Lending C1 Banks Lending C2 Banks Lending C3',
        // Clearing F1: it acts on what it has already been told. A borrower says what it is short of
        // in the period it finds out, and the credit is arranged in the next one — which is a lag
        // and is stated as one, because arranging a loan takes longer than noticing you need it.
        anchor: { after: 'corporateActions' },
        reads: [
          { kind: 'event', name: 'bond.offered', of: 'anyPeriod' },
          // 17d.2: a row is written as a margin over what money cost, so the writer reads the fixing.
          { kind: 'event', name: 'index.benchmark', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.quoted', of: 'anyPeriod' },
          { kind: 'event', name: 'covenant.breached', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.request', of: 'anyPeriod' },
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
          { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
          { kind: 'event', name: 'disclosed', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'bank.costOfFunds' },
          { kind: 'event', name: 'bank.insolvent' },
          { kind: 'event', name: 'bank.reservation' },
          { kind: 'event', name: 'bank.underwriting' },
          { kind: 'event', name: 'covenant.breached' },
          { kind: 'event', name: 'credit.committed' },
          { kind: 'event', name: 'credit.declined' },
          { kind: 'event', name: 'credit.draw' },
          { kind: 'event', name: 'credit.lapsed' },
          { kind: 'event', name: 'credit.quoted' },
          { kind: 'event', name: 'credit.repaid' },
          { kind: 'event', name: 'credit.written' },
        ],
        run: (ctx: MechanismContext): void => {
          // C9: the borrowers that said they have money spare pay their lines down before the ones
          // that said they are short are lent to — the same money, and a bank that lent out what a
          // repayment was about to bring back would be sizing its book against a number that had
          // already moved (Clearing F1).
          runRepayments(rows, ctx);
          // B2.b (17b.1): a commitment nobody drew is over before this period's are made, so the
          // room it was holding is the bank's own again when it decides on this period's asks.
          lapseFacilities(ctx);
          // B2.a (17b.8a): and what stands is tested against the accounts the borrower published,
          // before this period's decisions are taken on it.
          testFacilityCovenants(ctx);
          runRequests(rows, ctx);
          // Clearing F1: everything that prices off a bank's own economics this period reads it here
          // — its own dealing line pricing what an inventory costs to carry, a firm deciding whether
          // a project clears its cost of capital, a schedule in a bond market. One number, published
          // once, read by all of them (Law 4).
          publishCostOfFunds(rows, ctx);
          publishQuotes(rows, ctx);
          publishReservations(rows, ctx);
          publishAdvisory(ctx);
          publishUnderwriting(rows, ctx);
        },
      },
      {
        name: 'lending.workout',
        spec: 'Banks Lending E3 Banks Lending C3 Banks Lending D1',
        // Clearing F1: after the period's payments, so a miss this period is a miss this phase can
        // see — and a maturity NEXT period is the one it can still do something about. A row that
        // falls due this morning is already paid or already failed; agreeing an extension the week
        // before is what a lender and a borrower actually do.
        anchor: { after: 'corporateActions' },
        reads: [
          // 17d.2: a row is written as a margin over what money cost, so it reads the fixing.
          { kind: 'event', name: 'index.benchmark', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.request', of: 'anyPeriod' },
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
          { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
          { kind: 'event', name: 'disclosed', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'credit.reagreed' },
          { kind: 'event', name: 'credit.restructured' },
          { kind: 'event', name: 'credit.rolled' },
        ],
        run: (ctx: MechanismContext): void => {
          runWorkouts(rows, ctx);
        },
      },
      {
        name: 'banks.treasury',
        spec: 'Banks Funding B1 Banks Funding B1.a Banks Funding B2 Banks Funding B3 Money Market D2',
        // After it has published what money costs it: a board is priced off its own funding and its
        // rivals' boards, and both are reads of what was published (Law 4, Law 19).
        anchor: { after: 'lending.write' },
        reads: [
          { kind: 'event', name: 'bank.depositRate', of: 'anyPeriod' },
          { kind: 'event', name: 'deposit.classes', of: 'anyPeriod' },
          { kind: 'event', name: 'moneyMarket.print', of: 'anyPeriod' },
          { kind: 'event', name: 'moneyMarket.refused', of: 'anyPeriod' },
        ],
        writes: [{ kind: 'event', name: 'bank.depositRate' }],
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
        // After it has published what money costs it: what carrying a position costs is the number
        // that decides whether closing a gap is worth doing at all (D3), and it is that publication.
        anchor: { after: 'lending.write' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [
          { kind: 'event', name: 'advisory.ran', of: 'anyPeriod' },
          { kind: 'event', name: 'auction.announced', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.buffer', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.capital', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.capitalPlan', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.costOfFunds', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.depositRate', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.lines', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.liquidity', of: 'anyPeriod' },
          { kind: 'event', name: 'bank.reservation', of: 'anyPeriod' },
          { kind: 'event', name: 'bond.offered', of: 'anyPeriod' },
          { kind: 'event', name: 'centralBank.corridor', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.quoted', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.written', of: 'anyPeriod' },
          { kind: 'event', name: 'deposit.classes', of: 'anyPeriod' },
          { kind: 'event', name: 'firms.funding', of: 'anyPeriod' },
          { kind: 'event', name: 'fund.listedStruck', of: 'anyPeriod' },
          { kind: 'event', name: 'housing.funding', of: 'anyPeriod' },
          { kind: 'event', name: 'moneyMarket.print', of: 'anyPeriod' },
          { kind: 'event', name: 'moneyMarket.refused', of: 'anyPeriod' },
          { kind: 'event', name: 'prime.wanted', of: 'anyPeriod' },
        ],
        writes: [],
        run: (ctx: MechanismContext): void => {
          for (const b of ctx.parties.ofKind(BANK)) {
            if (b.status.alive) arbitrage(ctx, b.id, rows);
          }
        },
      },
      {
        name: 'banks.dealing',
        spec: 'Dealer Desks D5 Dealer Desks E4',
        // After the marks are in the books, so what it says the book is worth is what it is worth.
        anchor: { after: 'revaluation' },
        reads: [
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
        ],
        writes: [{ kind: 'event', name: 'bank.dealing' }],
        run: (ctx: MechanismContext): void => {
          for (const b of ctx.parties.ofKind(BANK)) {
            if (b.status.alive) publishDealing(ctx, b.id, rows);
          }
        },
      },
      {
        name: 'lending.book',
        spec: 'Money B3.a Money B3.c Banks Lending B1',
        // Before the audit sees the period, and before anything can DIE of it: an overdraft the bank
        // allowed is a drawing, and a drawing is a row. A party that ceased still carrying a raw
        // negative balance would leave its bank holding a claim with no instrument behind it, and the
        // estate nothing to assume — so this runs first and what is left is always a loan.
        anchor: { before: 'revaluation' },
        reads: [
          // 17d.2: a row is written as a margin over what money cost, so it reads the fixing.
          { kind: 'event', name: 'index.benchmark', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.declined', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.written', of: 'thisPeriod' },
          { kind: 'event', name: 'credit.request', of: 'anyPeriod' },
          { kind: 'event', name: 'disclosed', of: 'anyPeriod' },
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
          { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'credit.draw' },
          { kind: 'event', name: 'credit.standard' },
          { kind: 'event', name: 'credit.written' },
        ],
        run: (ctx: MechanismContext): void => {
          bookDraws(rows, ctx);
          publishStandard(ctx);
        },
      },
      {
        /**
         * Prime Brokerage A1, B1, C1-C3, E1 (item 13.3): THE BROKER'S PERIOD — value the client's
         * book, decide what it requires against it, publish the line, call what is over it and lend
         * what is under it.
         *
         * With the other lending, and for the same reason: a prime loan is a loan, it consumes the
         * same capital and the same room, and it is priced by the same quote. What is different is
         * only the DECISION about how much (C1), which is the one thing §15 calls the core.
         */
        name: 'banks.prime',
        spec: 'Prime Brokerage A1 Prime Brokerage B1 Prime Brokerage B3 Prime Brokerage C1 Prime Brokerage C3 Prime Brokerage C3.b Prime Brokerage E1',
        // After the period's lending decisions and before the session, so a client that is lent to
        // this morning can put the money to work this afternoon, and one that is CALLED this morning
        // knows what it must sell before the books open (C3, XI-2).
        anchor: { after: 'lending.write' },
        // C1.b: the broker's quote is priced off every default anybody published, like every other
        // line this module writes — a read this phase makes for itself, so it is declared here.
        reads: [
          { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
          { kind: 'event', name: 'prime.wanted', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.request', of: 'anyPeriod' },
          { kind: 'event', name: 'disclosed', of: 'anyPeriod' },
          { kind: 'event', name: 'rating.action', of: 'anyPeriod' },
          { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
        ],
        writes: [{ kind: 'event', name: 'prime.line' }],
        run: (ctx: MechanismContext): void => {
          runPrime(ctx, primeDeps(rows), FUND);
        },
      },
      {
        name: 'banks.buffer',
        spec: 'Banks Funding C2 Banks Funding C2.a Money Market A2.a',
        // AFTER THE FLOWS AND BEFORE THE SESSION. What its account did to it this week is only known
        // once the week's payments have happened, and what it holds against a bad one is what every
        // schedule it posts in the session is measured against — so it is taken here, once, and
        // published, and the session reads it rather than deriving a second one (Law 4).
        anchor: { before: 'lending.book' },
        reads: [{ kind: 'event', name: 'bank.buffer', of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'bank.buffer' }],
        run: (ctx: MechanismContext): void => {
          for (const b of ctx.parties.ofKind(BANK)) {
            if (!b.status.alive || declOf(rows, b.id) === undefined) continue;
            publishBuffer(ctx, b.id, ctx.registry.currencyOf(b.region));
          }
        },
      },
    ],
    // Money Market A3, B1: and the same one face in a VENUE. Its schedule for a session reaches the
    // book through the kernel's door (Clearing B2), so the market that clears it decides nothing.
    venueParticipants: [
      { partyKind: BANK, orders: sessionOrders },
      // Labour A1, A3, Banks Lending C1.d (13d): a bank wants the hours its book takes, in a trade of
      // its own, at what an hour is worth to it — and it is matched by the same rule as every other
      // employer. A bank whose book earns nothing bids nothing and hires nobody, which is how a
      // shrinking bank sheds staff without anybody writing a rule for it.
      { partyKind: BANK, orders: staffOrders },
      // Labour A1, A3, M&A B4 (10f.4): and the corporate-finance department, in a trade of its own —
      // a banker who runs a sale is not a lending officer. What it wants is the hours the sales it
      // ran last period took, so a department nobody appointed shrinks.
      { partyKind: BANK, orders: advisoryOrders },
    ],
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
      {
        partyKind: BANK,
        // Banks Capital C2.b (item 10d): the other bank's side of a subordinated raise. It is a
        // separate face from the dealing desk because it is a separate reason: a desk makes a market
        // in what it chooses to make one in, and this is a lender deciding to hold a name's capital.
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
          subscribes(view, m, rows),
      },
    ],
    marks: [{ instrumentKind: LOAN, value: (ctx, i) => worthToItsLender(rows, ctx, i) }],
    creditDecisions: [{ partyKind: BANK, decide: (ctx, o) => overdraft(rows, ctx, o) }],
    // Securities Lending B1, E1: WHY A BANK IS SHORT, answered where the desk's own view of a line
    // lives. The lending module clears the fee and writes the loan; what it may not do is decide for
    // a party it does not own that the party wants to be short (Observer A4).
    borrowNeeds: [
      { partyKind: BANK, needs: (view: ParticipantView) => deskBorrows(view, rows, makersOf) },
    ],
    families: [bookMoves(), tradingBookIsCapitalised(), loansStayInTheBankingSystem()],
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
function worthToItsLender(
  rows: readonly BankDecl[],
  ctx: MechanismContext,
  i: Instrument,
): Option<PerPiece> {
  if (!isLoan(i.terms)) return none<PerPiece>();
  // D1: what a loan is worth is ITS OWN CREDITOR's judgement of it, and after a sale that is the
  // buyer of the row rather than the bank that wrote it (XI-11).
  const creditor = creditorOf((id) => ctx.register.holdersOf(id), i);
  if (!creditor.some) return none<PerPiece>();
  const decl = declOf(rows, creditor.value);
  if (decl === undefined) return none<PerPiece>();
  const cv = creditViewFor(rows, ctx, creditor.value, i.ccy);
  if (cv === undefined) return none<PerPiece>();
  const pd = cv.of(i.terms.borrower).probabilityOfDefault;
  /**
   * C5.a (13d): what stands behind it, at the MARKET's own price, read when the question is asked.
   * So a bank holding claims secured on a thing whose price is falling carries them lower, without
   * anybody tightening anything — which is what a lending standard actually is.
   */
  const owed = ctx.register.heldTotal(i.id).value;
  const loss = scale(
    lossGivenDefault(
      cv.lossGivenDefault,
      // N13, Law 15 (17.7c): WHAT IS PLEDGED IS THE KIND'S TO SAY. It read the loan's own terms,
      // which is the one read that had to be rewritten for every claim that can be secured; the
      // ranking is the same answer an estate takes and every kind already has one.
      ctx.registry.instrumentKind(i.kind).ranking(i).secured,
      (pledged) => {
        const print = ctx.prices.latest(pledged, ctx.period);
        return print.some ? print.value.price : undefined;
      },
      asCash(owed, i.ccy, 'what is owed on this row'),
    ),
    pd,
    'what it expects to lose per unit',
  );
  // Item 16: par less what it expects to lose, per unit — a PRICE, which is what a mark is.
  return some(
    asPerPiece(
      minus(asRatio(1, 'par'), loss, 'what a unit is worth to it'),
      'what a unit is worth to it',
    ),
  );
}

/**
 * Banks Lending E3, 21.59 (17.7): WHAT EACH LENDER DOES WITH THE ROWS IT ALREADY HOLDS.
 *
 * Two decisions, on two kinds of row, and they are the same decision seen at two moments.
 *
 * A row that is STILL PERFORMING and reaches its maturity is rolled when this lender would write it
 * again today — same row, another term, at what its view of the name now requires (21.59). Nothing
 * in this world funds a maturity: a borrower publishes what its wages and its orders cost it, never
 * what falls due (`firms publishFunding`), so a healthy borrower with a loan maturing had no channel
 * to refinance it and failed on the date. That is not a credit event, it is a missing mechanism, and
 * this is it. A name its own standard now turns away is not rolled and has to find the money.
 *
 * A row that has STOPPED performing is restructured when new terms bring this creditor more than
 * enforcement would (E3, `workout.ts`). It acts on a miss from an earlier period, because a workout
 * is negotiated after the payment fails rather than in the instant it does — the same lag the credit
 * decision itself carries, and stated as one.
 *
 * Both agree TIME AND PRICE only: what is forgiven is a redemption at what it fetched (E5) and is
 * not this, and what is pledged is an act with two sides and is not this either.
 */
function runWorkouts(rows: readonly BankDecl[], ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  // A2, Law 4: how long this lender lends for is one number and it is the one a new loan is written
  // for. A term agreed today runs from today, whatever the row's original one was.
  const until = addMonths(today, ctx.params.months(LENDING_PARAMS.loanMonths));
  const years = asRatio(
    yearFraction(CREDIT_DAY_COUNT, today, until),
    'how long the new terms carry it',
  );
  for (const i of ctx.instruments.ofKind(LOAN)) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    // D4, XI-11: the creditor is whoever is owed it NOW, which after a sale is not the bank that
    // wrote it (Law 19). A row nobody is owed has nobody to agree with.
    const creditor = creditorOf((id) => ctx.register.holdersOf(id), i);
    if (!creditor.some) continue;
    const decl = declOf(rows, creditor.value);
    if (decl === undefined) continue;
    const cv = creditViewFor(rows, ctx, creditor.value, i.ccy);
    if (cv === undefined) continue;
    const borrower = ctx.parties.resolve(i.terms.borrower).id;
    // XI-8, Firm Birth D5: an estate is winding the borrower up and there is nobody left to sign.
    // A row whose borrower has finished existing altogether is not a workout either: what became of
    // it is the estate's record, and the line itself ends when it is spent (Register E2, 17.9b).
    if (!ctx.parties.get(borrower).status.alive) continue;
    const name = cv.of(borrower);
    const agreed: LoanTerms = {
      ...i.terms,
      // E3 (17d.2): a re-agreement moves the MARGIN, which is what the two of them agree; the
      // fixing is nobody's to agree and resets on its own.
      ...loanRate(name.rate, benchmarkNow(ctx.journal, ctx.calendar, i.ccy, ctx.period)),
      maturity: until,
    };
    if (i.status.performing) {
      // 21.59: the row that is about to fall due, and not one that has a year to run. NEXT period,
      // because this period's payments have already been made or already failed by the time this
      // runs — an extension agreed after the money was due is not what saved anybody.
      if (ctx.calendar.periodOf(i.terms.maturity) !== ctx.period + 1) continue;
      if (!rolls(name)) continue;
      ctx.reagree(i.id, agreed, 'rolled');
      ctx.record(
        'credit.rolled',
        [String(creditor.value), String(borrower), String(i.id)],
        { bank: String(creditor.value), borrower: String(borrower), loan: String(i.id), rate: name.rate },
        false,
      );
      continue;
    }
    // E3, Housing C5.a: what it expects to lose of a unit of THIS row if the name fails — its own recovery
    // record, netted against the market's own price of whatever stands behind it.
    const owed = ctx.register.heldTotal(i.id).value;
    const loss = lossGivenDefault(
      cv.lossGivenDefault,
      // N13, Law 15: what is pledged behind it, as the claim's own kind ranks it (17.7c).
      ctx.registry.instrumentKind(i.kind).ranking(i).secured,
      (pledged) => {
        const print = ctx.prices.latest(pledged, ctx.period);
        return print.some ? print.value.price : undefined;
      },
      asCash(owed, i.ccy, 'what is owed on this row'),
    );
    const paths = pathsOf(loss, name.probabilityOfDefault, name.capitalCharge, years);
    if (takes(paths) !== 'agree') continue;
    ctx.reagree(i.id, agreed, 'restructured');
    ctx.record(
      'credit.restructured',
      [String(creditor.value), String(borrower), String(i.id)],
      {
        bank: String(creditor.value),
        borrower: String(borrower),
        loan: String(i.id),
        rate: name.rate,
        agreeing: paths.agreeing,
        enforcing: paths.enforcing,
      },
      false,
    );
  }
}

/**
 * Banks Lending C9, F2, Corporate Credit A1 (17.9a): A BORROWER WITH MORE MONEY THAN IT NEEDS PAYS
 * ITS LINE DOWN.
 *
 * C9 says a line is drawn and repaid AT THE BORROWER'S OPTION, and only half of that was built: a
 * borrower could draw, and nothing it ever did brought the row back down again. So a firm that had
 * a good quarter carried the debt of its worst one for ever, went on paying interest on it, and
 * went on consuming its lender's capital and its own large-exposure limit for money it was not
 * using. F2's *"new lending, amortisation, prepayment and write-off account for the change in the
 * book"* named this and the comment had been standing over three of the four.
 *
 * WHAT IT REPAYS IS ITS OWN PUBLISHED NUMBER, read the way its ask is read. A borrower publishes
 * what it is short of through the one door every borrower uses (`ctx.request`), and that number is
 * SIGNED: short of money it asks, and over it publishes the surplus as a negative. Nothing here
 * peeks at a borrower's account to decide it has too much (Observer A4) — the borrower said so,
 * last period, in public to its lenders.
 *
 * DEAREST FIRST, because that is what paying down debt means: of two lines it owes, the one that
 * costs it more is the one it retires. And it repays only what it holds — settlement would refuse
 * the rest anyway, and a borrower that promised more than it has is a failed instruction rather
 * than a repayment (Money E1).
 */
function runRepayments(rows: readonly BankDecl[], ctx: MechanismContext): void {
  if (ctx.period === 0) return;
  const said = period(ctx.period - 1);
  for (const req of ctx.requests(said)) {
    if (req.short.pieces >= 0) continue;
    const borrower = ctx.parties.resolve(req.borrower).id;
    const who = ctx.parties.get(borrower);
    if (!who.status.alive) continue;
    let spare = negated(req.short, 'the money it published it does not need');
    // C9, F1.a: its own rows, dearest first. The register says which lines it owes and the terms
    // say what each costs it; nothing here keeps a second list of a borrower's debts (Law 19).
    const mine = ctx.instruments
      .issuedBy(borrower)
      .filter((i) => i.status.live && isLoan(i.terms) && i.ccy === req.short.ccy)
      .sort((a, b) => (isLoan(b.terms) ? rateOn(b.terms) : 0) - (isLoan(a.terms) ? rateOn(a.terms) : 0));
    for (const line of mine) {
      if (spare.pieces <= 0) break;
      const owed = creditorOf((id) => ctx.register.holdersOf(id), line);
      if (!owed.some) continue;
      if (declOf(rows, owed.value) === undefined) continue;
      const held = ctx.register.quantity(owed.value, line.id);
      if (held <= 0) continue;
      const cash = ctx.register.quantity(borrower, moneyInstrumentId(who.bank, req.short.ccy));
      // Law 8: it pays whole pieces of the money, and no more than any of the three things that
      // limit it — what it has spare, what is outstanding, and what is in its account. None of the
      // three is a bound on an outcome: each is what paying IS (Law 6).
      const paying = downTick(
        atMost(
          atMost(spare.pieces, held, 'it cannot repay more than is outstanding'),
          cash,
          'it cannot pay money it has not got',
        ),
      );
      if (paying <= 0) continue;
      const r = ctx.settle({
        legs: [
          {
            kind: 'asset',
            from: owed.value,
            to: borrower,
            instrument: line.id,
            qty: asQty(paying),
            pricePerUnit: some(asPerPiece(1, 'at what it promised')),
            accruedPerUnit: none(),
          },
          {
            kind: 'money',
            from: ctx.accountOf(borrower, req.short.ccy),
            to: ctx.accountOf(owed.value, req.short.ccy),
            receipt: { of: 'returnOfCapital' },
            ccy: req.short.ccy,
            amount: paying,
          },
        ],
        cause: 'maturity',
        reason: `${String(borrower)} pays down ${String(line.id)} out of money it does not need`,
      });
      if (r.outcome !== 'settled') continue;
      spare = minus(spare, heldAsMoney(paying, req.short.ccy, 'what it paid down'), 'what is left spare');
      ctx.record(
        'credit.repaid',
        [String(owed.value), String(borrower), String(line.id)],
        {
          bank: String(owed.value),
          borrower: String(borrower),
          loan: String(line.id),
          paid: paying,
          rate: isLoan(line.terms) ? rateOn(line.terms) : 0,
        },
        false,
      );
    }
  }
}

/** C2: a borrower that said what it is short of gets quotes, and takes the keenest that will have it. */
function runRequests(rows: readonly BankDecl[], ctx: MechanismContext): void {
  if (ctx.period === 0) return;
  const said = period(ctx.period - 1);
  /**
   * Corporate Credit A1, Law 15 (item 0e): EVERY BORROWER, through the kernel's one read.
   *
   * It was `[...ofKind('firms.funding'), ...ofKind('housing.funding')]` — two other modules' event
   * names, spelled out here, so a third borrower had to be added to this list by hand and none was:
   * the small-business sector published nothing a bank would look at and got no credit at all
   * (BK4). What a borrower is short of is one kind now, written through `ctx.request` and read
   * through `ctx.requests`, so a sector that borrows is a sector this sees.
   */
  for (const req of ctx.requests(said)) {
    if (req.short.pieces <= 0) continue;
    const borrower = String(req.borrower);
    const want = req.short;
    // A4 (13d): the request may name what it is secured on. A bank reading this does not learn
    // what the thing IS — it learns that there is an instrument it could take and realise, which
    // is the whole of what security means to a lender (Law 15).
    const security = req.security;
    const party = ctx.parties.get(req.borrower);
    // XI-8, Firm Birth D5: what it asked for last period it asked for as a going concern. It has
    // since ceased, and an estate is winding it up rather than borrowing: there is nobody left to
    // sign, so the request dies with the borrower.
    if (!party.status.alive) continue;
    const ccy = ctx.registry.currencyOf(party.region);
    // Corporate Credit A1, C7, Law 4: ONE SHORTFALL, ONE CHANNEL. A borrower that brought paper to
    // the market against the number it published is raising it there, and a bank writing a loan
    // against the same published number would fund the same hole twice — which is the residual with
    // no holder Appendix B forbids, arriving as money nobody needed. A book that did not clear
    // raised nothing, and the firm is short again in its next accounts: that is the cost of a
    // failed auction (C7) and it is a lag, not a loss.
    if (broughtPaper(ctx, borrower as PartyId, said)) continue;
    const { best, lend } = shop(rows, ctx, borrower as PartyId, want);
    if (best === undefined || lend.pieces <= 0) continue;
    /**
     * §29 B2, B2.b, E1 (17b.1): IT ASKED FOR A PROMISE AND IT GETS A PROMISE.
     *
     * The same decision, a different thing produced. The borrower said which of the two it wanted,
     * the bank priced the name and found the room exactly as it does for a row, and what it writes
     * is a commitment its capital stands behind and its borrower may draw — which is *"the credit
     * market decides which buyouts occur, and that is a real constraint, not a rate applied to a
     * plan"* arriving as the ordinary answer to an ordinary ask.
     */
    if (req.wants === 'commitment') {
      commit(ctx, best.bank, borrower as PartyId, lend, best.rate, ccy, req.months);
      continue;
    }
    // C9, F1.a: one row per (lender, borrower). A borrower that comes back to the same bank is
    // drawing on what it already has there, not taking a new loan every week — and the margin it
    // draws at is the one that was struck when the line was agreed (A2, A3).
    // C9, Bond F3 (11.2a.2): a line if it repays at its option, a term loan if on a schedule —
    // what the borrower SAID, never inferred from whether it pledged something.
    write(
      ctx,
      best.bank,
      borrower as PartyId,
      lend,
      best.rate,
      ccy,
      req.months,
      req.repays === 'atOption',
      security,
    );
  }
}

/**
 * Corporate Credit A1: whether this borrower took the OTHER channel with the shortfall it published
 * in that period. A read of a public announcement (Law 19, Observer A3) — the issuer said what it
 * brought and what it was short of, and a bank reads it off the market like anybody else.
 */
function broughtPaper(ctx: MechanismContext, borrower: PartyId, said: Period): boolean {
  return paperOfferedIn(ctx.journal, String(borrower), said);
}

/**
 * C3.a: declined volume is visible. A bank that never says no has no credit standard, so what the
 * banks between them turned away this period is published as a count and a volume — an aggregate
 * read of events that already happened, causing nothing (Observer A5). Who was refused stays
 * between the two of them; that it happened, and how much of it, does not.
 */
function publishStandard(ctx: MechanismContext): void {
  const declined = ctx.journal.ofKind('credit.declined').filter((e) => e.period === ctx.period);
  const written = ctx.journal.ofKindIn('credit.written', ctx.period);
  const volume = (rows: readonly Event[], key: string): number =>
    sum(rows.map((e) => (typeof e.data[key] === 'number' ? e.data[key] : 0))).value;
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
    // Item 13.3: the KIND's capability narrowed by what the party's own module says about THIS one
    // (`mayBorrow`). It read the kind alone, which was right while a kind that could borrow meant
    // every party of it could — and stopped being right the moment a pool's permission became its
    // MANDATE's. A quote published for a money fund that may never borrow is a price for a trade
    // that cannot happen.
    if (!p.status.alive || !ctx.participant(p.id).mayBorrow()) continue;
    const ccy = ctx.registry.currencyOf(p.region);
    let best: Quote | undefined;
    let most = noCash(ccy);
    for (const b of ctx.parties.ofKind(BANK)) {
      const decl = declOf(rows, b.id);
      if (decl === undefined || !b.status.alive || b.id === p.id) continue;
      /**
       * Small-Business Pools A5, A6.a, XI-15 (11.2): A CELL IS BANK-DEPENDENT, and its bank is a
       * dimension of its key — the lattice said so when it keyed the population on it. The bank
       * that quotes such a borrower is that bank; the rest of the world does not know its name
       * (Law 1: the firm with one account gets one quote, and a tightening at its bank reaches it
       * with nowhere else to go, A5.a). A population keyed on no bank is quoted by every bank
       * that issues its money, like a named borrower.
       */
      if (p.representation === 'cell' && 'bank' in p.key && p.key['bank'] !== String(b.id))
        continue;
      /**
       * Money A1, B1, B1.a, XI-12: A BANK LENDS ITS OWN MONEY INTO EXISTENCE, so a bank that issues
       * no pounds cannot write a pound loan — the drawing leg has its own account on the paying
       * side and there is no such account (item 0, stop 21).
       *
       * Lending across a currency is a real business and a different one: the lender funds itself
       * in the borrower's money first, which is the currency layer (XI-12) and is not built. Until
       * it is, a bank quotes where it issues, and a borrower in a money none of this world's banks
       * issue gets no quote — which is the honest answer and the one C3 wants recorded.
       */
      if (!ctx.instruments.has(moneyInstrumentId(b.id, ccy))) continue;
      const view = ctx.participant(b.id);
      // A-45: a bank that cannot cost its funding does not quote. It is not the cheapest lender in
      // the world, which is what a zero made it in every opening period.
      const cv = creditViewFor(rows, ctx, b.id, ccy);
      if (cv === undefined) continue;
      const r = room(view, decl, cv.exposureTo(p.id));
      if (r.most.pieces <= 0) continue;
      // C3 (17.0): and a bank that has a REASON not to lend to this name does not quote it — its
      // books were not opened, its earnings do not cover its debt, or the market already prices
      // its paper above what this bank would lend at. `refuse` says which, when the name asks.
      const q = cv.of(p.id);
      if (q.declines.some) continue;
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
        /**
         * B4 (17d.2): WHAT IT WOULD COST THIS NAME TODAY, all in — what money costs plus what this
         * borrower costs on top. Every reader of *what borrowing costs* wants this one and is
         * unchanged by the row underneath it becoming a floater (Law 4, Law 19).
         */
        rate: best.rate,
        /**
         * B4: and the MARGIN, which is the half the two of them actually agree and the half that is
         * struck on the row. A reader that wants to know what this bank thinks of this NAME wants
         * this one, because it is the only half that is about the borrower.
         */
        margin: marginOf(ctx, best.rate, ccy),
        most: most.pieces,
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
 * B4 (17d.2): THE HALF OF A QUOTE THAT IS ABOUT THE BORROWER. A quote is what money costs plus what
 * this name costs on top; the margin is what is left when the fixing is taken out, and where the
 * book has never traded there is no fixing to take out and the quote is all margin — which is what
 * a fixed row is (17d.3).
 */
function marginOf(ctx: MechanismContext, quoted: Ratio, ccy: CurrencyCode): Ratio {
  return loanRate(quoted, benchmarkNow(ctx.journal, ctx.calendar, ccy, ctx.period)).margin;
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
  // 17.0: THE NAMES A BANK HAS A REASON TO PRICE — every issuer of live paper anybody could bring
  // it, and every live bank: a name anybody may lend to whether or not it has paper outstanding,
  // because somebody deciding overnight whether to place cash with it needs the answer before the
  // first row exists (Money Market B2).
  const obligors = new Set<PartyId>();
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    if (!profile.liabilityOfIssuer || profile.pricing === 'money') continue;
    obligors.add(i.issuer.value);
  }
  for (const b of ctx.parties.ofKind(BANK)) if (b.status.alive) obligors.add(b.id);
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (decl === undefined || !b.status.alive) continue;
    const ccy = ctx.registry.currencyOf(b.region);
    // A-45: it publishes what it would require only where it can say what money costs it.
    const cv = creditViewFor(rows, ctx, b.id, ccy);
    if (cv === undefined) continue;
    const required: Record<string, number> = {};
    const expectedLoss: Record<string, number> = {};
    const capitalCost: Record<string, number> = {};
    const terms: Record<string, unknown> = {};
    for (const obligor of obligors) {
      // C1.b, C4, E5: ONE VIEW OF THE NAME, and its parts said plainly — what it requires to HOLD
      // the name's paper, what it expects to lose on it and what the capital costs it — so somebody
      // pricing a different claim on the same name (a week of money, say: Money Market B2) composes
      // from the same beliefs rather than from a rate assembled for a year-long loan (Law 4).
      const v = cv.of(obligor);
      required[obligor] = v.required;
      expectedLoss[obligor] = v.expectedLoss;
      capitalCost[obligor] = v.capitalCharge;
      terms[obligor] = {
        costOfFunds: v.costOfFunds,
        expectedLoss: v.expectedLoss,
        capitalCharge: v.capitalCharge,
        probabilityOfDefault: v.probabilityOfDefault,
        lossGivenDefault: v.lossGivenDefault,
        riskWeight: v.riskWeight,
        grade: v.grade.some ? v.grade.value : null,
        coverage: v.coverage.some ? v.coverage.value : null,
        leverage: v.leverage.some ? v.leverage.value : null,
        marketYield: v.marketYield.some ? v.marketYield.value : null,
        declines: v.declines.some ? v.declines.value : null,
      };
    }
    ctx.record(
      'bank.reservation',
      [b.id],
      {
        bank: b.id,
        ccy,
        required,
        expectedLoss,
        capitalCost,
        lossGivenDefault: cv.lossGivenDefault,
        terms,
      },
      false,
    );
  }
}

/** The per-period memo of every bank's view: one derivation per bank per money per period (Law 4). */
interface Views {
  period: number;
  byKey: Map<string, CreditView | undefined>;
  asked: Map<string, Statement> | undefined;
}

/**
 * 17.0: THIS BANK'S CREDIT VIEW THIS PERIOD, in one money — formed once and read by the quote, the
 * overdraft's row, the provision, the refusal and the published reservation alike. Nothing where
 * the bank cannot cost its funding (A-45) or is not one of this module's.
 */
function creditViewFor(
  rows: readonly BankDecl[],
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
): CreditView | undefined {
  const memo = ctx.state<Views>('banks.creditView', () => ({
    period: ctx.period,
    byKey: new Map(),
    asked: undefined,
  }));
  if (memo.period !== ctx.period) {
    memo.period = ctx.period;
    memo.byKey.clear();
    memo.asked = undefined;
  }
  const key = `${String(bank)}\u0000${ccy}`;
  if (memo.byKey.has(key)) return memo.byKey.get(key);
  const decl = declOf(rows, bank);
  const funds = costOfFunds(ctx, bank, ccy).perAnnum;
  if (decl === undefined || !funds.some) {
    memo.byKey.set(key, undefined);
    return undefined;
  }
  if (memo.asked === undefined) {
    // Corporate Credit A4: the books borrowers opened with their asks — last period's, which is
    // what this period arranges credit against (Clearing F1), and this period's.
    memo.asked = new Map<string, Statement>();
    for (const at of [period(ctx.period - 1), period(ctx.period)]) {
      if (at < 0) continue;
      for (const r of ctx.requests(at)) {
        if (r.statement.some) memo.asked.set(String(r.borrower), r.statement.value);
      }
    }
  }
  const view = ctx.participant(bank);
  const reg = regulationOf(view);
  const rules = rulesFor(rows, ctx, bank);
  const made = creditView(
    view,
    decl,
    ccy,
    creditInputs(
      ctx,
      view,
      funds.value,
      reg,
      seenDefaults(ctx),
      (name) => weightOfName(ctx, name, ccy, rules),
      memo.asked,
    ),
  );
  memo.byKey.set(key, made);
  return made;
}

/**
 * Corporate Credit C1, C6 (17.2): WHAT THIS BANK CHARGES TO BRING AN ISSUE, under its own name.
 *
 * An issuer shopping for an arranger reads what each bank published, exactly as a borrower reads
 * what each bank quoted (Observer A3) — a module cannot see a bank's own price any other way, and
 * should not. What it will COMMIT is its dealing line's allotted room, which it publishes already
 * (`bank.lines`), and what the risk costs it is its published credit view of the name (E5): three
 * public facts, and the fee for either basis is composed from them by whoever is asking.
 */
function publishUnderwriting(rows: readonly BankDecl[], ctx: MechanismContext): void {
  for (const b of ctx.parties.ofKind(BANK)) {
    const decl = declOf(rows, b.id);
    if (decl === undefined || !b.status.alive) continue;
    ctx.record(
      'bank.underwriting',
      [b.id],
      { bank: b.id, fee: ctx.params.ratio(bankParam(b.id, 'arrangerFee')) },
      true,
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
/**
 * A-45, Observer A3: what a bank PUBLISHES about what money costs it, as a record a reader can take
 * a number out of. `perAnnum` is an `Option` inside the engine and a published event is data, so
 * the field is PRESENT with the rate or ABSENT altogether — never an option object a reader would
 * have to know the shape of, and never a zero standing in for "it cannot say".
 */
function published(cost: FundingCost): Record<string, unknown> {
  // Law 8: money on the record is its pieces beside the money it is in.
  const parts = {
    interest: cost.interest.pieces,
    onCapital: cost.onCapital.pieces,
    owed: cost.owed.pieces,
    capital: cost.capital.pieces,
    ccy: cost.owed.ccy,
  };
  return cost.perAnnum.some ? { ...parts, perAnnum: cost.perAnnum.value } : parts;
}

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
    const alsoIn: Record<string, Record<string, unknown>> = {};
    for (const ccy of ctx.registry.currencies.keys()) {
      if (ccy !== home) alsoIn[ccy] = published(costOfFunds(ctx, b.id, ccy));
    }
    // Banks Capital A1, C1 (17.0): A HOLE IS SAID, under the bank's own name, once a period. The
    // floor that hid it as free capital is gone; what stands here is the state.
    const own = costOfFunds(ctx, b.id, home);
    if (own.capital.pieces < 0) {
      ctx.record(
        'bank.insolvent',
        [b.id],
        { bank: b.id, ccy: home, capital: own.capital.pieces, owed: own.owed.pieces },
        true,
      );
    }
    ctx.record(
      'bank.costOfFunds',
      [b.id],
      // B2.b, Law 4: the blend AND ITS PARTS, so what it paid and what its capital costs it are
      // readable separately by whoever needs one of them — and so that nobody has to re-derive
      // either from the other (Law 19).
      { bank: b.id, ccy: home, ...published(costOfFunds(ctx, b.id, home)), alsoIn },
      true,
    );
  }
}

/**
 * A bank's own book: the sum of the loan rows it holds.
 *
 * D4, XI-11: WHAT IT HOLDS, which is the whole of the question. A row it sold is not its exposure
 * any more and a row it bought is — and because a party sees its own holdings and nobody else's
 * (§45: no observer sees private state), asking what it holds is both the right read and the only
 * one it is entitled to. The filter that named a lender on the terms is gone with the fact.
 */
export function loanBook(view: ParticipantView): number {
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!isLoan(i.terms)) continue;
    terms.push(view.quantity(h.instrument));
  }
  return sum(terms).value;
}
