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
 * falling is the depositor leaving, and that happens in the world (E1, `moveDeposits`) rather than
 * in this function — which is why the two banks' margins matter: the gap between them is what a
 * depositor is deciding about, and only the class whose switching cost is smaller than that gap
 * moves. That is A1.c and E4.a arriving as an outcome rather than as a stated stickiness.
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
import { currencyUnit, moneyInstrumentId } from '../../core/ids.js';
import { add, div, mul, sub, sum, zeroIfNone } from '../../core/num.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide, shareFor } from '../../ledger/settlement.js';
import { weightOf } from '../../parties/party.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { none, some, type Option } from '../../core/option.js';
import {
  classOf,
  MM_PARAMS,
  switchingCost,
  type DepositClassDecl,
} from './data.js';

/** One day count for what a bank pays on money, stated once (Law 8: a rate has a period). */
const DEPOSIT_DAY_COUNT = 'ACT/365F';



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
    const cls = classOf(ctx.registry, ctx.parties.get(holder).kind);
    if (cls === undefined) continue;
    const held = ctx.register.totalQuantity(holder, money);
    if (held === 0) continue;
    out.set(cls.id, add(zeroIfNone(out.get(cls.id)), held, `${cls.id} at ${bank}`));
  }
  return out;
}

/**
 * A1.a: THE SPLIT, per member, and the one place it is taken. The limit applies PER MEMBER and the
 * cell is homogeneous, so what is covered is exact rather than an average of a distribution — which
 * is what makes E4's break in the loop real rather than notional: a large cell of small depositors
 * is covered and a cell of large ones is not.
 */
function coveredPerMember(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): number {
  const cls = classOf(ctx.registry, ctx.parties.get(holder).kind);
  const perMember = ctx.register.quantity(holder, moneyInstrumentId(bank, ccy));
  return covered(cls, perMember, limit);
}

/** A1.a: the split itself, on one balance — the one place it is drawn, for whoever is asking. */
function covered(cls: DepositClassDecl | undefined, perMember: number, limit: number): number {
  if (cls?.insured !== true) return 0;
  return perMember < limit ? perMember : limit;
}

/** A1.a, D4: what this world insures, per member, of what a depositor holds at a bank (XI-15). */
export function insuredAt(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): number {
  const covered = coveredPerMember(ctx, bank, holder, ccy, limit);
  return covered <= 0 ? 0 : mul(covered, weightOf(ctx.parties.get(holder)), 'insured');
}

/** A1.a, E4: the other side of the same split — what nobody insures, per member. */
export function uninsuredAt(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  limit: number,
): number {
  const perMember = ctx.register.quantity(holder, moneyInstrumentId(bank, ccy));
  if (perMember <= 0) return 0;
  return sub(perMember, coveredPerMember(ctx, bank, holder, ccy, limit), 'uninsured');
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
): number {
  const money = moneyInstrumentId(bank, ccy);
  const terms: number[] = [];
  for (const holder of ctx.register.holdersOf(money)) {
    if (holder === bank) continue;
    const p = ctx.parties.get(holder);
    if (classOf(ctx.registry, p.kind) === undefined) continue;
    terms.push(mul(uninsuredAt(ctx, bank, holder, ccy, limit), weightOf(p), 'what can run'));
  }
  return sum(terms).value;
}

/**
 * B1: the payment itself, holder by holder, for the days the period was. A negative rate is a real
 * rate and the payment simply goes the other way — the depositor pays the bank for holding it —
 * because a money leg has a payer and a payee and the sign says which is which (Money C1).
 */
export function payDepositInterest(
  ctx: MechanismContext,
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
    const cls = classOf(ctx.registry, party.kind);
    if (cls === undefined) continue;
    const rate = announced(ctx, bank, cls.id);
    if (!rate.some || rate.value === 0) continue;
    const balance = ctx.register.quantity(holder, money);
    // Law 8: interest is paid in whole pieces of the money, per member of a cell — each member is
    // a real holder with a real account. A rate that comes to less than one piece pays nothing,
    // which is what a rate that small IS.
    const wanted = mul(balance, mul(rate.value, year, 'for the days'), 'interest');
    const paying = wanted > 0;
    const share = shareFor(ctx.registry, party, currencyUnit(ccy), paying ? wanted : -wanted);
    const perMember = share.perMember;
    if (perMember <= 0) continue;
    const amount = perMember;
    const side = cellSide(party, amount);
    const leg: Leg = {
      kind: 'money',
      from: paying ? { holder: bank, issuer: bank } : { holder, issuer: bank },
      to: paying ? { holder, issuer: bank } : { holder: bank, issuer: bank },
      ccy,
      amount: share.total,
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

/**
 * B1.a, D5.a: what a bank is offering a class, read off the last thing it announced. It is the
 * rate on the board — the same fact a rival prices against and a depositor moves for, which is why
 * it is one public number and not two private ones (Law 4).
 */
export function announced(ctx: MechanismContext, bank: PartyId, cls: string): Option<number> {
  const said = ctx.journal.ofKind('bank.depositRate').filter((e) => e.subjects.includes(bank));
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
 * EACH ONE DECIDES IN ITS OWN VIEW (Observer A4). The taxonomy is this market's — it is a fact
 * about a regulation and a cost, published once and read by everybody (Law 4) — but the decision is
 * the depositor's, so it is taken through `ctx.participant`, off what that party may see: the
 * boards, which are public, and its own balance, which is nobody else's business. The market never
 * reads a depositor's account to decide for it.
 *
 * Two reasons to move, and they are different reasons. It moves for the RATE when what staying has
 * already cost it is more than moving costs — and that is an AMOUNT against an AMOUNT, which is the
 * whole of why a class drains instead of crossing. As a rate the switching cost was one comparison
 * every member of a class answered identically, so the moment a bank moved its board past the
 * number, all of them went at once: a representative agent with a threshold (App B). As an amount
 * it is weighed against the money the gap has actually cost this depositor — its own balance times
 * the gap, over as long as it has been where it is — so the big account goes first and the small one
 * may never go at all. Nothing is forecast: it is what has already happened to it (Law 17).
 *
 * And it moves for SAFETY when its own bank looks in trouble and it has money there that nobody
 * insures: that is E4 exactly, and it is why a run is a wholesale phenomenon first (E4.a), without
 * anything anywhere saying that wholesale money is flighty.
 *
 * The move is the kernel's door and it can FAIL: a bank that cannot pay the withdrawal does not,
 * and the depositor is still there when the next period opens (Money E1.b).
 */
export function moveDeposits(ctx: MechanismContext, banks: readonly PartyId[]): void {
  const since = ctx.period > 0 ? asPeriod(ctx.period - 1) : asPeriod(0);
  const shaky = new Set(banks.filter((b) => looksInTrouble(ctx, b, since)));
  for (const p of ctx.parties.all()) {
    // E1: only a depositor that chooses where it banks answers a rate. A bank settles at the
    // central bank and a desk is its own bank's arm (Law 15: the profile says, nothing branches).
    if (!p.status.alive || !ctx.registry.partyKind(p.kind).choosesBank) continue;
    const cls = classOf(ctx.registry, p.kind);
    if (cls === undefined || !banks.includes(p.bank)) continue;
    const view = ctx.participant(p.id);
    const ccy = ctx.registry.region(p.region).ccy;
    const going = wouldMove(view, cls, banks, shaky, ccy);
    if (!going.some) continue;
    ctx.moveBank(p.id, going.value.to, going.value.reason);
  }
}

/** Where a depositor would rather bank, decided in its own view and out of what it can see. */
function wouldMove(
  view: ParticipantView,
  cls: DepositClassDecl,
  banks: readonly PartyId[],
  shaky: ReadonlySet<PartyId>,
  ccy: CurrencyCode,
): Option<{ to: PartyId; reason: string }> {
  const self = view.self;
  const own = board(view, self.bank, cls.id);
  let best: { bank: PartyId; rate: number } | undefined;
  for (const b of banks) {
    if (b === self.bank || shaky.has(b)) continue;
    const rate = board(view, b, cls.id);
    if (!rate.some) continue;
    if (best === undefined || rate.value > best.rate) best = { bank: b, rate: rate.value };
  }
  if (best === undefined) return none();
  const balance = view.quantity(moneyInstrumentId(self.bank, ccy));
  if (balance <= 0) return none();
  // A2, XI-15: what is insured is a stated amount of the money the deposit is IN, per member.
  const limit = view.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
  const uninsured = sub(balance, covered(cls, balance, limit), 'what nobody insures');
  if (shaky.has(self.bank) && uninsured > 0) {
    return some({ to: best.bank, reason: `${self.id} moves what nobody insures away from ${self.bank}` });
  }
  if (!own.some) return none();
  const gap = sub(best.rate, own.value, 'what it would gain');
  if (gap <= 0) return none();
  const cost = view.params.amount(switchingCost(cls.id), currencyUnit(ccy));
  const foregone = mul(balance, mul(gap, stayed(view), 'over the time it has stayed'), 'what staying cost it');
  return foregone > cost
    ? some({ to: best.bank, reason: `${self.id} moves to ${best.bank}, which pays more for ${cls.id} money` })
    : none();
}

/**
 * E1: how long this depositor has banked where it banks, in years — since it last moved, or since
 * the world opened if it never has. Its own move is its own event (`deposit.moved`, A4), so the
 * clock is a read and not a stored counter, and it resets when the depositor moves: one that has
 * just gone somewhere does not go again the next week on the same gap.
 */
function stayed(view: ParticipantView): number {
  const last = view.lastOwn('deposit.moved');
  const from = last.some ? last.value.period : asPeriod(0);
  return yearFraction(
    DEPOSIT_DAY_COUNT,
    view.calendar.startOf(from),
    view.calendar.startOf(view.period),
  );
}

/**
 * B1.a, D5.a: what a bank is offering a class, as this depositor sees it — the rate on the board,
 * which is public because that is the whole of why it works (E2.a). It is the same fact
 * `announced` reads for the bank's own side, through the view a participant has (Law 4).
 */
function board(view: ParticipantView, bank: PartyId, cls: string): Option<number> {
  const said = view.lastPublicAbout('bank.depositRate', String(bank));
  if (!said.some) return none<number>();
  const rates = said.value.data['rates'];
  if (typeof rates !== 'object' || rates === null) return none<number>();
  const rate = (rates as Record<string, unknown>)[cls];
  return typeof rate === 'number' ? some(rate) : none<number>();
}
