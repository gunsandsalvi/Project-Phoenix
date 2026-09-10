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
 * keeps is a row it does not have to write — and a bank with more money than it wants is worth what
 * it can get for the spare, which is the floor (Money Market B5.a). Out of that it keeps its own
 * margin, and it pays a class less the harder that class is to move, because a class that will not
 * move for half a point does not have to be paid half a point.
 *
 * Nothing here is capped. What the rate cannot exceed is what the money is worth to the bank, and
 * that is not a bound but the alternative it would take instead (D1 against D3); what stops it
 * falling is that the depositor leaves, which is E1 and happens in the world rather than here.
 *
 * C2.a: THE BUFFER IS A PREFERENCE DERIVED FROM ITS OWN LIABILITIES, and here that is literal: what
 * this bank holds against is the worst week its own account has actually had, over the memory it
 * keeps. A bank funded by money that runs has seen bigger weeks than one funded by money that does
 * not, so the buffer differs between two banks with the same balance sheet size, which is what A2.a
 * asks for and what a stated ratio of deposits could never give.
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { moneyInstrumentId } from '../../core/ids.js';
import { add, div, material, mul, sub, sum, zeroIfNone } from '../../core/num.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import { weightOf } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import { none, some, type Option } from '../../core/option.js';
import {
  classOf,
  DEPOSIT_CLASSES,
  funderOf,
  mmParam,
  switchingCost,
  type DepositClassDecl,
} from './data.js';

/** One day count for what a bank pays on money, stated once (Law 8: a rate has a period). */
const DEPOSIT_DAY_COUNT = 'ACT/365F';

/** What this module keeps: what each bank pays each class, and what each class left there. */
export interface DepositBook {
  /** B1.a: bank -> class -> per annum, as that bank decided it this period. */
  readonly rates: Record<string, Record<string, number>>;
  /** F1: bank -> class -> what that class held at the last close (a read, kept for the report). */
  readonly balances: Record<string, Record<string, number>>;
  /** C2.a: bank -> the net moves its reserve account has taken, most recent last. */
  readonly reserveMoves: Record<string, number[]>;
}

export function emptyDeposits(): DepositBook {
  return { rates: {}, balances: {}, reserveMoves: {} };
}

const at = (book: Record<string, Record<string, number>>, bank: string): Record<string, number> => {
  const row = book[bank];
  if (row !== undefined) return row;
  const made: Record<string, number> = {};
  book[bank] = made;
  return made;
};

/** F1: the bank's deposit lines by class — a read of who actually banks there (never a stored total). */
export function depositsByClass(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
): Map<string, number> {
  const money = moneyInstrumentId(bank, ccy);
  const out = new Map<string, number>();
  for (const holder of ctx.register.holdersOf(money)) {
    if (holder === bank) continue;
    const cls = classOf(ctx.parties.get(holder).kind);
    if (cls === undefined) continue;
    const held = ctx.register.totalQuantity(holder, money);
    if (held === 0) continue;
    out.set(cls.id, add(zeroIfNone(out.get(cls.id)), held, `${cls.id} at ${bank}`));
  }
  return out;
}

/** A1.a, D4: what this world insures, per member, of what a depositor holds at a bank (XI-15). */
export function insuredAt(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): number {
  const cls = classOf(ctx.parties.get(holder).kind);
  if (cls === undefined || !cls.insured) return 0;
  const perMember = ctx.register.quantity(holder, moneyInstrumentId(bank, ccy));
  // A1.a: the limit applies PER MEMBER, and the cell is homogeneous, so `weight x min(balance,
  // limit)` is exact. That is what makes E4's break in the loop real rather than notional: a large
  // cell of small depositors is covered and a cell of large ones is not.
  const covered = perMember < limit ? perMember : limit;
  return covered <= 0 ? 0 : mul(covered, weightOf(ctx.parties.get(holder)), 'insured');
}

/**
 * B1.a, A5, D3: what each bank decides to pay each class, from what the money is worth to it.
 *
 * `worth` is the alternative it is actually holding: the dearest rate it is paying in the market
 * right now if it is borrowing there, and the floor if it is not (B5.a). Two banks facing the same
 * corridor pay different rates because they keep different margins and because one of them is
 * short; the same bank pays its classes differently because they are differently hard to keep.
 */
export function setRates(
  ctx: MechanismContext,
  book: DepositBook,
  bank: PartyId,
  ccy: CurrencyCode,
  worth: number,
): void {
  if (funderOf(bank) === undefined) return;
  const margin = ctx.params.get(mmParam(bank, 'depositMargin'));
  const rates = at(book.rates, bank);
  const was = at(book.balances, bank);
  const now = depositsByClass(ctx, bank, ccy);
  for (const cls of DEPOSIT_CLASSES) {
    const sticky = ctx.params.get(switchingCost(cls.id));
    rates[cls.id] = sub(
      sub(worth, margin, 'what it keeps'),
      sticky,
      `what it pays ${cls.id}`,
    );
  }
  for (const [cls, balance] of now) was[cls] = balance;
}

/**
 * What this bank pays a class, per annum. A bank that has never decided has not decided (Appendix
 * A): the payment below asks, gets none, and pays nothing rather than paying a default.
 */
export function rateFor(book: DepositBook, bank: PartyId, cls: DepositClassDecl): Option<number> {
  const rate = at(book.rates, bank)[cls.id];
  return rate === undefined ? none<number>() : some(rate);
}

/**
 * B1: the payment itself, holder by holder, for the days the period was. A negative rate is a real
 * rate and the payment simply goes the other way — the depositor pays the bank for holding it —
 * because a money leg has a payer and a payee and the sign says which is which (Money C1).
 */
export function payDepositInterest(
  ctx: MechanismContext,
  book: DepositBook,
  bank: PartyId,
  ccy: CurrencyCode,
): void {
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
    const cls = classOf(party.kind);
    if (cls === undefined) continue;
    const rate = rateFor(book, bank, cls);
    if (!rate.some || rate.value === 0) continue;
    const balance = ctx.register.quantity(holder, money);
    const perMember = mul(balance, mul(rate.value, year, 'for the days'), 'interest');
    // Law 7: interest of a hundredth of a rounding is not a payment. Sending it would be an
    // instruction whose amount is the dust of the multiplication that produced it.
    if (!material(perMember, 2, Math.abs(balance))) continue;
    const paying = perMember > 0;
    const amount = paying ? perMember : -perMember;
    const side = cellSide(party, amount);
    const leg: Leg = {
      kind: 'money',
      from: paying ? { holder: bank, issuer: bank } : { holder, issuer: bank },
      to: paying ? { holder, issuer: bank } : { holder: bank, issuer: bank },
      ccy,
      amount: totalFor(party, amount),
      fromCell: paying || side === undefined ? none() : some(side),
      toCell: paying && side !== undefined ? some(side) : none(),
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

/** C2.a: what this bank's account has actually done to it, over the memory it keeps. */
export function rememberReserves(
  book: DepositBook,
  bank: PartyId,
  move: number,
  memory: number,
): void {
  const moves = book.reserveMoves[bank] ?? [];
  moves.push(move);
  while (moves.length > memory) moves.shift();
  book.reserveMoves[bank] = moves;
}

/**
 * A2.a, C2, C2.a: the buffer this bank holds against what could leave — the worst week its own
 * account has had, over its own memory. A bank that has never had a bad week holds nothing against
 * one, which is a real (and dangerous) position and not a missing number.
 */
export function bufferOf(book: DepositBook, bank: PartyId): number {
  const moves = book.reserveMoves[bank] ?? [];
  let worst = 0;
  for (const m of moves) if (m < worst) worst = m;
  return -worst;
}

/** What the bank's whole deposit base comes to, for the reads that need one number (F1). */
export function depositBase(byClass: ReadonlyMap<string, number>): number {
  return sum([...byClass.values()]).value;
}

/**
 * F4, Observer A5: the metric somebody outside can see — what it holds liquid against what could
 * leave. A bank with nothing that could leave has no ratio, and says so rather than showing one.
 */
export function liquidityMetric(liquid: number, couldLeave: number): Option<number> {
  return couldLeave <= 0 ? none<number>() : some(div(liquid, couldLeave, 'liquidity metric'));
}

/**
 * E2, E2.a, Money Market D5.a: whether this bank LOOKS in trouble, to somebody outside it.
 *
 * Only from what is published: it was refused in the market, it drew the window, or it closed the
 * period below zero at the central bank. All three are public events about a real thing that
 * happened to it, which is E2.a's whole requirement — a depositor cannot see a capital ratio nobody
 * published, and it does not need to.
 */
export function looksInTrouble(ctx: MechanismContext, bank: PartyId, since: Period): boolean {
  for (const kind of ['moneyMarket.refused', 'moneyMarket.window', 'bank.short'] as const) {
    for (const e of ctx.journal.ofKind(kind)) {
      if (e.period >= since && e.subjects.includes(bank)) return true;
    }
  }
  return false;
}

/** B1.a: what a bank has said it pays a class this period, read off its own announcement. */
export function announced(ctx: MechanismContext, bank: PartyId, cls: string): Option<number> {
  const said = ctx.journal
    .ofKind('bank.depositRate')
    .filter((e) => e.subjects.includes(bank) && e.period === ctx.period);
  const last = said[said.length - 1];
  if (last === undefined) return none<number>();
  const rates = last.data['rates'];
  if (typeof rates !== 'object' || rates === null) return none<number>();
  const rate = (rates as Record<string, unknown>)[cls];
  return typeof rate === 'number' ? some(rate) : none<number>();
}

/**
 * E1, E2, E3.a, E4: every depositor's own answer to what its bank is paying and what it looks like.
 *
 * Two reasons to move, and they are different reasons. It moves for the RATE when another bank pays
 * it more than its own does by more than moving costs it — which is why the class with the lowest
 * switching cost is the one that moves first and the insured one hardly moves at all. And it moves
 * for SAFETY when its own bank looks in trouble and it has money there that nobody insures: that is
 * E4 exactly, and it is why a run is a wholesale phenomenon first (E4.a), without anything anywhere
 * saying that wholesale money is flighty.
 *
 * The move is the kernel's door and it can FAIL: a bank that cannot pay the withdrawal does not,
 * and the depositor is still there when the next period opens (Money E1.b).
 */
export function moveDeposits(ctx: MechanismContext, banks: readonly PartyId[], limit: number): void {
  const since = ctx.period > 0 ? asPeriod(ctx.period - 1) : asPeriod(0);
  const shaky = new Set(banks.filter((b) => looksInTrouble(ctx, b, since)));
  for (const p of ctx.parties.all()) {
    if (!p.status.alive || banks.includes(p.id)) continue;
    const cls = classOf(p.kind);
    if (cls === undefined || !banks.includes(p.bank)) continue;
    const ccy = ctx.registry.region(p.region).ccy;
    const own = announced(ctx, p.bank, cls.id);
    const sticky = ctx.params.get(switchingCost(cls.id));
    let best: { bank: PartyId; rate: number } | undefined;
    for (const b of banks) {
      if (b === p.bank || shaky.has(b)) continue;
      const rate = announced(ctx, b, cls.id);
      if (!rate.some) continue;
      if (best === undefined || rate.value > best.rate) best = { bank: b, rate: rate.value };
    }
    if (best === undefined) continue;
    const perMember = ctx.register.quantity(p.id, moneyInstrumentId(p.bank, ccy));
    if (perMember <= 0) continue;
    const uninsured = cls.insured ? sub(perMember, perMember < limit ? perMember : limit, 'uninsured') : perMember;
    const forSafety = shaky.has(p.bank) && uninsured > 0;
    const forRate = own.some && sub(best.rate, own.value, 'what it would gain') > sticky;
    if (!forSafety && !forRate) continue;
    ctx.moveBank(
      p.id,
      best.bank,
      forSafety
        ? `${p.id} moves what nobody insures away from ${p.bank}`
        : `${p.id} moves to ${best.bank}, which pays more for ${cls.id} money`,
    );
  }
}
