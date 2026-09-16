/**
 * What a deal costs, what stands behind it, and the borrowing a buyer that cannot pay for one asks
 * for.
 *
 * @spec Private Equity B1 Private Equity B2 Private Equity B2.a Private Equity B2.b Private Equity B3 Private Equity E1 Corporate Credit A1 Corporate Credit C9 Law 2 Law 6 Law 19
 *
 * §29 B2: *"most of the price is debt raised against the target itself"*, and B2.a says whose it is:
 * *"the debt is the TARGET's liability, not the fund's — which is why a failed buyout kills the firm
 * and not the fund."* So the borrowing this file publishes is published IN THE TARGET'S NAME. A
 * request says who will owe the money, and the answer is the company, not the buyer; a fund that
 * borrowed for its deals would be a fund a failed deal can kill, which is the clause read backwards.
 *
 * B2.b is then not a rule anywhere: *"the deal only happens if lenders will lend, at a price — the
 * credit market decides which buyouts occur."* The bank prices the TARGET, out of the target's own
 * published accounts and its own room for that name, and what comes back is a commitment or nothing.
 * A company nobody will commit to is a company nobody can buy with borrowed money, and no line here
 * says so.
 */
import { asCash, minus, plus, type Cash, noCash, type Ratio } from '../../core/measure.js';
import { strikeOf } from '../../registry/funding.js';
import type { Civil } from '../../calendar/civil.js';
import { none, some, type Option } from '../../core/option.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import {
  FACILITY,
  facilityLoanId,
  isFacility,
  LOAN,
  type LoanTerms,
} from '../../registry/credit.js';
import { paramId, type ParamId } from '../../core/ids.js';
import type { InstrumentId } from '../../core/ids.js';
import type { MechanismContext } from '../../world/context.js';

/**
 * §29 C1, B2 (17b.8): HOW LONG A BUYOUT'S DEBT RUNS FOR. It is the deal's number and not the
 * market's line convention: what the company carries afterwards is what C1 is about.
 */
export const DEAL_MONTHS: ParamId = paramId('control.dealMonths');

/** E1: one lender's standing promise to a named company, as the party that will draw it reads it. */
export interface Facility {
  readonly bank: PartyId;
  readonly limit: Cash;
  readonly rate: Ratio;
  readonly maturity: Civil;
}

/**
 * E1, Corporate Credit C9 (17b.1): WHAT A LENDER HAS PROMISED THIS NAME FOR A DEAL AND NOT LENT.
 *
 * Read off the agreement store rather than an event, because what matters is that the promise still
 * STANDS: a lapsed one is a promise that was made and is over (Law 19). A commitment past the
 * period it was made for is not drawn here even though the lender has not yet struck it off its own
 * book — the borrower knows when its commitment papers expired.
 *
 * ONE LENDER, because a drawing is one row between two parties (Banks Lending A1, F1.a) and the
 * biggest promise is the one a buyer would use. A deal funded by several banks at once is a
 * syndicate and it is §7 C7's mechanism, not this one.
 */
export function facilityFor(
  ctx: MechanismContext,
  target: PartyId,
  ccy: CurrencyCode,
): Option<Facility> {
  let best: Facility | undefined;
  for (const a of ctx.agreements.ofKind(FACILITY)) {
    if (a.state !== 'performing' || a.debtor !== target || a.ccy !== ccy) continue;
    const t = a.terms;
    if (!isFacility(t) || ctx.period > t.until || t.limit.pieces <= 0) continue;
    if (best !== undefined && t.limit.pieces <= best.limit.pieces) continue;
    best = { bank: a.creditor, limit: t.limit, rate: t.rate, maturity: t.maturity };
  }
  return best === undefined ? none<Facility>() : some(best);
}

/** The same promise as a number, for the buyer working out what it still has to find. */
export function committedTo(ctx: MechanismContext, target: PartyId, ccy: CurrencyCode): Cash {
  const f = facilityFor(ctx, target, ccy);
  return f.some ? f.value.limit : noCash(ccy);
}

/**
 * B2, B2.a, Corporate Credit A1 (17b.2): THE COMPANY ASKS FOR THE MONEY THAT WILL BUY IT.
 *
 * One ask per target and not one per bidder: a request is a fact about who will owe the money, and
 * a company cannot owe two deals' debt at once. Where two buyers want the same firm the ask is the
 * biggest of the holes they would have to fill, because that is what the company would have to
 * carry if the deal that happens is the dearest of them — and the two of them meet in the book
 * afterwards (B4), not here.
 *
 * It asks for a COMMITMENT and not for money (`wants`), which is the whole of why a tender can be
 * conditional: what a bank promises here is drawn inside the instruction that completes the
 * purchase, or it is never drawn at all.
 */
export function askToFund(
  ctx: MechanismContext,
  target: PartyId,
  /** B2: what a lender is asked to commit, which is nothing where the buyer can find it all. */
  wanted: Cash,
  /** B3, A2: what the buyer itself must bring, and therefore what a pool has to have called. */
  cheque: Cash,
  buyer: PartyId,
  /**
   * §29 C1, Corporate Credit A2 (17b.8): OVER THE YEARS THE COMPANY WILL SERVICE IT. A buyout's
   * debt is what the firm carries afterwards — *"it operates and services its debt out of cash
   * flow, and the higher leverage means less room"* — so its term is a term of the deal and not the
   * length of a working-capital line. It is handed in because the caller owns the number.
   */
  months: number,
): void {
  if (cheque.pieces <= 0 && wanted.pieces <= 0) return;
  const who = ctx.parties.get(target);
  if (!who.status.alive) return;
  // Corporate Credit A1: its own money, and a deal in another one is 17b.7's (Cross-Border C4).
  if (ctx.registry.currencyOf(who.region) !== wanted.ccy) return;
  /**
   * C9: IT DOES NOT SAY IT TWICE WHILE THE LAST ONE IS UNANSWERED. A borrower publishes in one
   * period and hears in the next, so an ask repeated every period is the same money asked for three
   * times over and three lenders' capital held against one deal. What is already committed was
   * subtracted by the caller.
   */
  const asked = ctx.journal.lastOf('control.financing', String(target));
  if (asked !== undefined && asked.period >= ctx.period - 1) return;
  if (wanted.pieces > 0) {
    ctx.request(target, {
      ccy: wanted.ccy,
      short: wanted,
      // B2: a buyout's debt is a term loan the company pays down, never a line it draws at will.
      repays: 'onSchedule',
      // E1: a lender that has agreed to lend and has not lent, so a deal can be conditional on it.
      wants: 'commitment',
      months,
    });
  }
  /**
   * A2, B3 (17b.4): AND THE BUYER SAYS WHAT IT HAS TO BRING, because a pool's capital is *"called
   * when a deal needs it"* and the pool has to be able to find out that it is buying. The module
   * that owns the pool reads this through the registry (`financingBy`), never by this name.
   */
  ctx.record(
    'control.financing',
    [String(target), String(buyer)],
    {
      target: String(target),
      buyer: String(buyer),
      wanted: wanted.pieces,
      cheque: cheque.pieces,
      ccy: wanted.ccy,
    },
    true,
  );
}

/** What the buyer still has to find: the price, less what it holds and what a lender has promised. */
export const holeIn = (cost: Cash, has: Cash, committed: Cash): Cash =>
  minus(
    minus(cost, has, 'what it cannot pay for out of what it holds'),
    committed,
    'and what a lender has already promised the company',
  );

/**
 * §29 A2, B3 (17b.4): WHAT A BUYER COULD BRING ITSELF — its own money, and for a pool the capital
 * it could still call. *"Capital is committed, not paid"*, so what it could call is not money it
 * has; it is money it can require, and a buyer weighing whether it can pay for a company counts it
 * beside its balance and then has to actually call it before the tender.
 *
 * It is the pool's own published number (`fund.struck`), read through the registry like every other
 * public fact about a party (Law 19, 0e′.3). A buyer that is not a pool has published none and its
 * equity is its balance, which is the ordinary case.
 */
export function equityOf(
  ctx: MechanismContext,
  buyer: PartyId,
  cash: Cash,
  /**
   * Cross-Border A2, Law 8 (17b.7): the money the buyer's own commitments are in. What its
   * investors promised it is promised in ITS money, so it counts toward a deal struck in that money
   * and toward no other — a pool looking at a foreign company has what it holds there and nothing
   * else, and buying the money first is its own decision in the pair.
   */
  home: CurrencyCode,
): Cash {
  if (cash.ccy !== home) return cash;
  const struck = strikeOf(ctx.journal, String(buyer));
  if (!struck.some || struck.value.couldCall <= 0) return cash;
  return plus(
    cash,
    asCash(struck.value.couldCall, cash.ccy, 'what it could still call from its investors'),
    'what it could bring itself',
  );
}

/** A buyer's own money, as money rather than a count — one crossing, named (Law 8). */
export const cashOf = (pieces: number, ccy: CurrencyCode): Cash =>
  asCash(pieces, ccy, 'what the buyer holds toward it');

/**
 * §29 B2, B2.a, Banks Lending A1, A2, F1.a (17b.3): THE ROW A COMMITMENT IS DRAWN INTO, made once.
 *
 * One row per (lender, borrower), named so the lender's own headroom read and the borrower's
 * drawing are asking about the same number (Law 4, Law 19), and a second drawing adds to it rather
 * than writing a new one. The rate and the term came from the promise, not from the day it was
 * drawn: commitment papers say both, and the party that draws is not the party that decided them.
 *
 * It is here rather than beside either caller because a buyout draws it to pay sellers and a
 * recapitalisation draws it to pay the owner, and a row with two sets of terms would be two rows.
 */
export function facilityRow(
  ctx: MechanismContext,
  target: PartyId,
  facility: Facility,
  ccy: CurrencyCode,
): InstrumentId {
  const row = facilityLoanId(facility.bank, target);
  if (ctx.instruments.has(row)) return row;
  const terms: LoanTerms = {
    kind: LOAN,
    originator: facility.bank,
    borrower: target,
    rate: facility.rate,
    drawn: ctx.calendar.startOf(ctx.period),
    maturity: facility.maturity,
    dayCount: 'ACT/365F',
    // Bond F3: a term loan the company pays down, never a line it draws at will — which is what
    // makes C1's *"less room out of cash flow"* a payment and not a mood.
    amortising: true,
    // A4: unsecured. What a lender takes security over here is a covenant it bids for, and no
    // holder bids a covenant yet (finding 21.60(b), positioned at 17b.8).
    security: [],
  };
  ctx.issue({ id: row, kind: LOAN, issuer: some(target), ccy, terms, market: none() });
  return row;
}
