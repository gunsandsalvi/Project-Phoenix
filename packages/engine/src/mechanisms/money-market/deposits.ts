/**
 * Who funds a bank, what it pays them, and the money that actually leaves when it does.
 *
 * @spec Banks Funding A1 Banks Funding A1.a Banks Funding A1.b Banks Funding A1.c Banks Funding A1.d Banks Funding A5 Banks Funding B1 Banks Funding B1.a Banks Funding B2.b Banks Funding C2 Banks Funding C2.a Banks Funding D3 Banks Funding F1 Money Market D2 Money Market A2.a Law 8 XI-15
 *
 * B1: A DEPOSIT RATE IS A REAL PAYMENT TO A REAL HOLDER, so it is an instruction, per class, every
 * period, for the days the balance was there. Nothing accrues inside the money instrument (Money
 * A1): an account is a holding of a bank's liability, and what the bank pays on it is a decision it
 * takes and a payment it makes.
 *
 * B1.a: THE RATE IS THE BANK'S OWN DECISION, and it is made of what the money is worth TO IT. A
 * bank that is borrowing in the market is worth the rate the market is charging it — a deposit it
 * keeps is a row it does not have to write — and one with more money than it wants is worth what it
 * can get for the spare, which is the floor (Money Market B5.a). Out of that comes its own margin,
 * and then what it does not have to pay: a class that will not move for half a point does not have
 * to be paid half a point, which is where A1.d's stickiness does its work and why the same bank
 * pays its three classes three different rates.
 *
 * Nothing here is capped. What the rate cannot exceed is what the money is worth to the bank, and
 * that is not a bound but the alternative it would take instead (D1 against D3). What stops it
 * falling is the depositor leaving, and that decision is not in this module at all: it belongs to
 * the module that owns the depositor (`SystemModule.bankChoices`), because a household's reason to
 * move is not a fund's. What the bank sees of it is the gap between the boards — which is why the
 * banks' margins matter, and why A1.c and E4.a arrive as an outcome rather than as a stated
 * stickiness.
 *
 * C2.a: THE BUFFER IS A PREFERENCE DERIVED FROM ITS OWN LIABILITIES, and here that is literal: what
 * this bank holds against is the worst week its own account has actually had, over the memory it
 * keeps. A bank funded by money that runs has seen bigger weeks than one funded by money that does
 * not, so the buffer differs between two banks with the same balance sheet size, which is what A2.a
 * asks for and what a stated ratio of deposits could never give.
 */
import { noCash, sumCash } from '../../core/measure.js';
import type { Qty } from '../../core/tick.js';
import {
  type Cash,
  type Ratio,
  absolute,
  acrossMembers,
  asCash,
  asPerMember,
  asRatio,
  heldAsMoney,
  minus,
  plus,
  ratioOf,
  scale,
} from '../../core/measure.js';
import { period as asPeriod } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { moneyInstrumentId } from '../../core/ids.js';
import { atMost, sum, zeroIfNone } from '../../core/num.js';
import type { Leg } from '../../ledger/instruction.js';
import { weightOf } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import { none, some, type Option } from '../../core/option.js';
import { classOf, type DepositClassDecl } from './data.js';
import { depositRateFor } from '../../registry/banking.js';

/** One day count for what a bank pays on money, stated once (Law 8: a rate has a period). */
const DEPOSIT_DAY_COUNT = 'ACT/365F';

/** F1: the bank's deposit lines by class — a read of who actually banks there (never a stored total). */
export function depositsByClass(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
): Map<string, number> {
  const money = moneyInstrumentId(bank, ccy);
  const out = new Map<string, Qty>();
  for (const holder of ctx.register.holdersOf(money)) {
    if (holder === bank) continue;
    const cls = classOf(ctx.registry, ctx.parties.get(holder).kind);
    if (cls === undefined) continue;
    const held = ctx.register.quantity(holder, money);
    if (held === 0) continue;
    out.set(cls.id, plus(zeroIfNone(out.get(cls.id)), held, `${cls.id} at ${bank}`));
  }
  return out;
}

/**
 * A1.a: THE SPLIT, per member, and the one place it is taken. The limit applies PER MEMBER and the
 * cell is homogeneous, so what is covered is exact rather than an average of a distribution — which
 * is what makes E4's break in the loop real rather than notional: a large cell of small depositors
 * is covered and a cell of large ones is not.
 */
/** 0f.1: the register holds the cell's TOTAL; the guarantee covers each member up to the limit. */
function coveredPerMember(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): number {
  const cls = classOf(ctx.registry, ctx.parties.get(holder).kind);
  const perMember = ctx.register.perMember(holder, moneyInstrumentId(bank, ccy));
  return covered(cls, perMember, limit);
}

/** A1.a: the split itself, on one balance — the one place it is drawn, for whoever is asking. */
function covered(cls: DepositClassDecl | undefined, perMember: number, limit: number): number {
  if (cls?.insured !== true) return 0;
  return atMost(perMember, limit, 'the guarantee covers no more than it says it covers');
}

/** A1.a, D4: what this world insures, per member, of what a depositor holds at a bank (XI-15). */
export function insuredAt(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): Cash {
  const covered = coveredPerMember(ctx, bank, holder, ccy, limit);
  return covered <= 0
    ? noCash(ccy)
    : asCash(
        acrossMembers(
          asPerMember<'money:piece'>(covered, 'what one member has covered'),
          weightOf(ctx.parties.get(holder)),
          'insured',
        ),
        ccy,
        'insured',
      );
}

/** A1.a, E4: the other side of the same split — what nobody insures, per member. */
export function uninsuredAt(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): Cash {
  const total = ctx.register.quantity(holder, moneyInstrumentId(bank, ccy));
  if (total <= 0) return noCash(ccy);
  // 0f.1: the register holds the TOTAL; what is uninsured is the total less what each member has
  // covered, over the members.
  return minus(
    heldAsMoney(total, ccy, 'what it holds'),
    asCash(
      acrossMembers(
        asPerMember<'money:piece'>(
          coveredPerMember(ctx, bank, holder, ccy, limit),
          'what one member has covered',
        ),
        weightOf(ctx.parties.get(holder)),
        'what its members have covered',
      ),
      ccy,
      'what its members have covered',
    ),
    'uninsured',
  );
}

/**
 * C2, C2.a, E4, E4.a: WHAT COULD LEAVE — and it is not a ratio of the deposit base. It is the part
 * of that base nobody insures, holder by holder, which is A1.a read the other way round and is
 * derived from its own liabilities exactly as C2.a asks. A bank funded by a few large wholesale
 * accounts has nearly all of it exposed; one funded by a cell of small insured households has
 * nearly none, which is why a run is a wholesale phenomenon first.
 */
export function couldLeave(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  limit: number,
): Cash {
  const money = moneyInstrumentId(bank, ccy);
  const terms: Cash[] = [];
  for (const holder of ctx.register.holdersOf(money)) {
    if (holder === bank) continue;
    const p = ctx.parties.get(holder);
    if (classOf(ctx.registry, p.kind) === undefined) continue;
    // 0f.1: `uninsuredAt` is the holder's TOTAL uninsured money; nothing scales it.
    terms.push(uninsuredAt(ctx, bank, holder, ccy, limit));
  }
  return sumCash(ccy, terms, 'what could leave').value;
}

/**
 * B1: the payment itself, holder by holder, for the days the period was. A negative rate is a real
 * rate and the payment simply goes the other way — the depositor pays the bank for holding it —
 * because a money leg has a payer and a payee and the sign says which is which (Money C1).
 */
export function payDepositInterest(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): void {
  if (ctx.period === 0) return;
  const previous = asPeriod(ctx.period - 1);
  const year = yearFraction(
    DEPOSIT_DAY_COUNT,
    ctx.calendar.startOf(previous),
    ctx.calendar.startOf(ctx.period),
  );
  if (year <= 0) return;
  const money = moneyInstrumentId(bank, ccy);
  for (const holder of [...ctx.register.holdersOf(money)].sort()) {
    if (holder === bank) continue;
    const party = ctx.parties.get(holder);
    // Money E4, XI-8: a party that has ceased is paid nothing, because there is nobody to pay. Its
    // balance is its estate's to claim under the estate's own name, and a payment addressed to the
    // dead party would be a payment to somebody who is not there.
    if (!party.status.alive) continue;
    const cls = classOf(ctx.registry, party.kind);
    if (cls === undefined) continue;
    const rate = announced(ctx, bank, cls.id);
    if (!rate.some || rate.value === 0) continue;
    const balance = ctx.register.quantity(holder, money);
    // Law 8: interest is paid in whole pieces of the money, per member of a cell — each member is
    // a real holder with a real account. A rate that comes to less than one piece pays nothing,
    // which is what a rate that small IS.
    const wanted = scale(
      balance,
      scale(rate.value, asRatio(year, 'the part of a year it covers'), 'for the days'),
      'interest',
    );
    const paying = wanted > 0;
    // 0f.1: `balance` is the cell's TOTAL, so `wanted` is the total interest: it goes on the grid
    // as money and the leg carries it whole. `shareFor` took a PER-MEMBER number and multiplied it
    // by the weight, which with a total in was interest on the whole cell, once per member.
    const amount = ctx.registry.payable(
      heldAsMoney(absolute(wanted, 'what moves, either way'), ccy, 'the interest'),
    );
    if (amount <= 0) continue;
    const share = { total: amount };
    const leg: Leg = {
      kind: 'money',
      from: paying ? { holder: bank, issuer: bank } : { holder, issuer: bank },
      to: paying ? { holder, issuer: bank } : { holder: bank, issuer: bank },
      // Treasury C1: interest either way — the bank paying a depositor, or a customer paying the
      // bank on an overdrawn account. What it IS does not change with which way it goes.
      receipt: { of: 'interest' },
      ccy,
      amount: share.total,
    };
    ctx.settle({
      legs: [leg],
      // B2.b: ONE RATE PER LIABILITY. This payment is what leaves, and the blended cost of funds is
      // a read of what left (Banks Lending C1.a) — the same number, read once, in one place.
      cause: 'coupon',
      reason: `${bank} pays ${cls.id} interest to ${holder}`,
    });
  }
}

/** What the bank's whole deposit base comes to, for the reads that need one number (F1). */
export function depositBase(byClass: ReadonlyMap<string, number>): number {
  return sum([...byClass.values()]).value;
}

/**
 * F4, Observer A5: the metric somebody outside can see — what it holds liquid against what could
 * leave. A bank with nothing that could leave has no ratio, and says so rather than showing one.
 */
export function liquidityMetric(liquid: Cash, couldLeave: Cash): Option<Ratio> {
  return couldLeave.pieces <= 0
    ? none<Ratio>()
    : some(ratioOf(liquid, couldLeave, 'liquidity metric'));
}

/**
 * B1.a, D5.a: what a bank is offering a class, read off the last thing it announced. It is the
 * rate on the board — the same fact a rival prices against and a depositor moves for, which is why
 * it is one public number and not two private ones (Law 4).
 */
export function announced(ctx: MechanismContext, bank: PartyId, cls: string): Option<Ratio> {
  return depositRateFor(ctx.journal, String(bank), cls);
}
