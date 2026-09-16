/**
 * The market for control: what somebody will pay for a whole firm, and what its owners will take.
 *
 * @spec M&A A1 M&A A2 M&A A3 M&A A4 M&A A5 M&A B1 M&A B2 M&A B2.a M&A B3 M&A B4 M&A C1 M&A C2 M&A D4 M&A D5 M&A E1 M&A E2 Equity B1 Equity E3 Equity F3 Labour A3 Labour D1 Private Equity A5 Private Equity C5 Private Equity D1 XI-2 XI-8 Law 2 Law 3 Law 4 Law 5 Law 6 Law 15 Law 19
 *
 * A PREMIUM IS A PRICE AND IT HAS TO CLEAR. Every share in this world already has a market, and
 * every holder already has its own number for what a share is worth to it (Equity B1, §46 A3). A
 * takeover is what happens when somebody else's number for the WHOLE firm is higher than theirs for
 * their piece of it — so the tender is an ordinary book with an unusual buyer, and the premium is
 * the distance between two valuations rather than a percentage anybody chose.
 *
 * NOBODY HAS TO TENDER (C2). Each holder answers from its own valuation, and a holder that thinks
 * the firm is worth more than the bid keeps its shares. So a bid can FAIL, and a failed bid is an
 * event with consequences rather than a silence — the acquirer is exactly where it was, minus what
 * it spent finding out.
 *
 * WHAT MAKES IT A TAKEOVER RATHER THAN A LARGE PURCHASE is the acceptance condition (A1): the bid is
 * for control, so it is conditional on getting it. Below the condition nothing settles at all —
 * there is no half-acquisition where the acquirer has paid for shares it did not want alone.
 *
 * AND ON COMPLETION THE TWO BALANCE SHEETS COMBINE (A4, A5, D4). The target's rows are reseated to
 * the acquirer, its own paper is assumed, its shares cease to exist because the residual claim they
 * were has been bought, and the party is terminated with the acquirer as its successor — which is
 * the same door an estate uses, because "this party's obligations are now that one's" is one fact
 * and there is one way to write it (XI-8, Register F2).
 */
import {
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  over,
  type PerPiece,
  pricedAt,
  type Ratio,
  scale,
  valueAt,
} from '../../core/measure.js';
import { addQty, NO_QTY, type Qty } from '../../core/tick.js';
import {
  partyId,
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { currencyUnit } from '../../core/ids.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { atMost, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { sumCash } from '../../core/measure.js';
import { downTick } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import { Missing } from '../../core/errors.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Instrument } from '../../register/instruments.js';
import { asQty } from '../../core/tick.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { weightOf, gridPerMember, totalOverMembers } from '../../parties/party.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { about } from '../../world/context.js';
import { hasEverMetAPayroll } from '../../registry/wages.js';
import { costOfMoneyQuotedTo, namesQuotedIn } from '../../registry/banking.js';
import {
  ownFundingSince,
  ownStrikeSince,
  strikeOf,
  strikesPublished,
} from '../../registry/funding.js';
import { advisoryQuotesIn } from '../../registry/notices.js';
import { askToFund, cashOf, committedTo, type Facility, facilityFor, holeIn } from './deal.js';
import { facilityLoanId, LOAN, type LoanTerms } from '../../registry/credit.js';

/** Law 9: one book per target, because what is being priced is control of THAT firm. */
export const tenderVenue = (target: PartyId): VenueId => venueId(`control:${target}`);

/**
 * B1, A3: WHAT A WHOLE FIRM IS WORTH TO THIS ACQUIRER, and it is the same number it values anything
 * else by: what it expects to get out of it, against what its own money costs it.
 *
 * There is no synergy coefficient and no control premium here. An acquirer's number differs from a
 * holder's because the two have different costs of capital and different views of the same firm,
 * which is what §46 A3 says disagreement IS — and it is the whole reason a market for control
 * exists at all. A world where everyone valued a firm identically would never see a takeover.
 */
export function worthToBuyer(
  buyer: ParticipantView,
  ctx: MechanismContext,
  target: PartyId,
  line: InstrumentId,
): Option<PerPiece> {
  return worthAt(buyer, ctx, target, line, costOfMoneyOf(ctx, buyer.self.id));
}

/**
 * The same number, with WHAT THIS BUYER REQUIRES already in hand.
 *
 * Law 18: a firm's own cost of money is a fact about the firm and not about the company it is
 * looking at, so it is read once and carried down the list of them. Read per company it was the
 * same answer fetched three hundred times over.
 */
function worthAt(
  buyer: ParticipantView,
  ctx: MechanismContext,
  target: PartyId,
  line: InstrumentId,
  required: Option<Ratio>,
): Option<PerPiece> {
  /**
   * A3, B1: WHAT IT WOULD GET, AGAINST WHAT IT REQUIRES — the same comparison it makes about a
   * machine, because "what is this stream worth to me" is one question and a world with two answers
   * to it has two valuation technologies (Law 4, Capital Programme B1).
   *
   * What it would get is what the target PUBLISHED (Reporting A2), annualised by the span of its own
   * report; there is no second set of accounts and no forecast. What it requires is what a bank
   * quoted THIS acquirer, per annum — its own cost of money, published under its own name, and a
   * firm nobody will lend to does not buy companies.
   *
   * That is why two acquirers want the same firm at different prices, and it is the whole reason a
   * market for control exists: the disagreement is load-bearing (§46 A3). There is no synergy term
   * and no control premium anywhere — the premium is what the BOOK produces.
   */
  if (!required.some || required.value <= 0) return none<PerPiece>();
  // Reporting A2, A2.a: the last accounts it published. Read through the kernel's one typed read,
  // never rebuilt and never re-parsed here (item 3).
  const said = ctx.published.lastStatement(target);
  if (said === undefined || said.earned.pieces <= 0 || said.periods <= 0) return none<PerPiece>();
  // Law 8: the periodicity is part of the number. What it published covers a span of periods; what
  // a required return is quoted in is a year, so the two are put in the same unit by the calendar
  // rather than by a factor typed here.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + said.periods)),
  );
  if (ofAYear <= 0) return none<PerPiece>();
  const annual = over(
    said.earned,
    asRatio(ofAYear, 'the fraction of a year that was'),
    'what it earns a year, as it published it',
  );
  const whole = over(annual, required.value, 'what that stream is worth at what it requires');
  const shares = ctx.register.heldTotal(line).value;
  if (shares <= 0) return none<PerPiece>();
  const perShare = pricedAt(whole, shares, 'what one share of it is worth to this buyer');
  const printed = buyer.print(line);
  // B1: it bids only where its own number is above what a share is already trading at. Below that
  // it can buy shares in the market like anybody else and does not need a tender.
  if (printed.some && perShare <= printed.value.price) return none<PerPiece>();
  return some(perShare);
}

/**
 * M&A B1, B3, §29 B1 (item 10f.6): WHAT THIS BUYER'S OWN MONEY COSTS IT, per annum, read off the
 * wire — and there are two kinds of buyer in this world, with one source each.
 *
 * A COMPANY reads what a bank quoted it: its own cost of money, published under its own name, and
 * a firm nobody will lend to does not buy companies (B3). A POOL reads what its investors require
 * of it, which its own NAV pass publishes beside its duration band for exactly this kind of reader
 * — a prospectus states a target return, and a closed-end fund raised to buy companies has one
 * whether or not a bank has ever quoted it.
 *
 * It is ONE question with two sources and not two questions (Law 4): the same `worthAt` divides by
 * whatever comes back, and nothing here knows what a pool is — it knows that one published a number
 * under this name and the other did not. It is the same shape `wageFacing` has for what an hour
 * costs an employer: its own experience where it has one, what was published where it has not.
 *
 * The journal already keeps the last event of a kind a party is a subject of, so this asks it
 * rather than walking every quote the world has ever published back to the beginning.
 */
function costOfMoneyOf(ctx: MechanismContext, who: PartyId): Option<Ratio> {
  const quoted = costOfMoneyQuotedTo(ctx.journal, String(who));
  if (quoted.some) return quoted;
  const struck = strikeOf(ctx.journal, String(who));
  return struck.some && struck.value.requires.some
    ? some(asRatio(struck.value.requires.value, 'what its own investors require of it'))
    : none<Ratio>();
}

/** A1: the bid. A price, and how much of the firm it has to get for the bid to mean anything. */
export interface Bid {
  readonly buyer: PartyId;
  readonly target: PartyId;
  readonly line: InstrumentId;
  readonly price: PerPiece;
  /** A1: the acceptance condition, as a count of shares — control, read off what exists. */
  readonly needs: Qty;
  readonly ccy: CurrencyCode;
}

/**
 * C1, C2, Equity B1: EVERY HOLDER ANSWERS FROM ITS OWN NUMBER. It tenders at what a share is worth
 * to it; a holder that thinks the firm is worth more than the bid keeps its shares, which is C2 and
 * is the reason the premium is not a parameter.
 *
 * A holder with no view posts nothing. It is not a refusal and not an acceptance — it is a holder
 * with nothing to say, and inventing an answer for it would be inventing a seller (Law 2).
 */
export function tenders(holder: ParticipantView, line: InstrumentId): readonly Order[] {
  const units = downTick(holder.free(line));
  if (units <= 0) return [];
  /**
   * §35 A1, XI-2 (item 10f.3): THE DISPOSAL — an owner sells a business it holds, and the reason is
   * the reason anybody sells anything: it needs the money. A holder that has published it is short
   * of what it wants to build takes what the book gives it rather than naming a level, which is
   * what makes this a SALE and not a valuation: a seller that would only part with it at its own
   * number is not disposing of anything.
   *
   * It is one read (`firms.funding`) and it is the same one the bond channel and the flotation use,
   * because a firm has one hole and three ways to fill it — borrow, issue, or sell something — and
   * a second number here would be a second hole (Law 4).
   */
  if (mustSell(holder)) {
    return [{ party: holder.self.id, side: 'sell', price: 'market', qty: units }];
  }
  const at = wouldTakeFor(holder, line);
  if (!at.some) return [];
  // It sells at its own number or better. What it gets is the clearing price, which is at or above
  // it — the ordinary meaning of an offer, and the reason a tender clears rather than being taken.
  return [{ party: holder.self.id, side: 'sell', price: at.value, qty: units }];
}

/**
 * Firm E4, §29 D1, D3, D4 (items 10f.3, 10f.6): WHETHER THIS HOLDER IS SELLING BECAUSE IT NEEDS THE
 * MONEY, which is the reason a disposal and a private-equity exit both have.
 *
 * Two kinds of holder publish it and the question is the same for both (Law 4). A COMPANY publishes
 * what it is short of for the thing it wants to build (`firms.funding`) — it sells a business to
 * fund one. A POOL publishes what it must find (`fund.struck`'s shortfall) — its redeemers are owed
 * money, or its broker called and it could not pay, or it is winding up and everything it holds is
 * on the queue (A4). **That is §29 D**: the exit happens because the fund has to produce cash, the
 * proceeds reach the investors through the redemption queue that was already there (D3), and
 * *"in a bad market it does not happen"* (D4) because the book strikes what it strikes and a seller
 * with no buyer keeps what it holds (Equity B6).
 *
 * Neither of them names a price when it is in this position (XI-2): a forced seller that named one
 * would not be one.
 */
function mustSell(holder: ParticipantView): boolean {
  const own = ownFundingSince(holder, holder.period);
  if (own.some && own.value.shortTerm > 0) return true;
  const pool = ownStrikeSince(holder, holder.period);
  return pool.some && pool.value.shortfall > 0;
}

/**
 * C1, C2, §29 C5 (item 10f.3): WHAT A HOLDER WOULD TAKE FOR ONE, and it is its own number.
 *
 * Its OUTLOOK of the price where it has formed one, which is what a holder of something that trades
 * has: it watches the tape and it knows what a share of this goes for. **And what its own accounts
 * carry it at where it has not** — which is the holder of a company that has never traded, and
 * there are eight thousand of those in this world (10f.1). Without the second half nobody could
 * ever answer a tender for a private company: an outlook is formed from prints, a private line
 * makes none, and every bid for one failed with *"nobody tendered"* — the acquisition the owner
 * asked for, refused by the one read that cannot answer for it.
 *
 * A carrying value is not a price and is not used as one (Law 3, B3): it is what this holder would
 * take, and a book of them meeting a buyer's own number is what strikes the level. A holder
 * carrying it above the bid keeps its shares, which is C2 exactly and is where the refusal lives.
 */
function wouldTakeFor(holder: ParticipantView, line: InstrumentId): Option<PerPiece> {
  const own = holder.outlook(about({ on: 'price', instrument: line }));
  if (own.some) return some(asPerPiece(own.value.expected, `what it expects ${line} to be worth`));
  // Register A1.c: its own LOTS — what it paid, which is what its accounts carry it at and is the
  // one number a holder of something with no price has. It reads its own books and nobody else's.
  const held = holder.holdings().find((h) => h.instrument === line);
  if (held === undefined) return none<PerPiece>();
  const units = sum(held.lots.map((l) => l.qty)).value;
  const ccy = holder.instruments.get(line).ccy;
  const cost = sumCash(
    ccy,
    held.lots.map((l) => valueAt(l.basisPerUnit, l.qty, ccy, 'what it paid for them')),
    'what it paid for them',
  ).value;
  if (units <= 0 || cost.pieces <= 0) return none<PerPiece>();
  return some(pricedAt(cost, units, 'what its own books carry one at'));
}

/**
 * M&A B4, §29 D1 (item 10f.4): THE PROCESS — a bank runs it, every bidder is in the same book, and
 * the winner is the one the book gave the most of.
 *
 * *"Formal exit processes and m&a processes lead by IBD departments."* A sale is not a bilateral
 * tender that appears from nowhere: the bank is APPOINTED (the cheapest with people free, read off
 * what the banks published), it puts every bidder into one venue against every holder's ask, and
 * the solver strikes the level. **That is B4 biting numerically**: a second bidder in the book
 * raises the level the holders are met at, whether or not it wins.
 *
 * The winner is the bidder the book filled most — ties on the higher bid, then on the name — and it
 * buys the whole cleared volume at the struck level. A bidder below the level did not fill at all
 * and lost on price; one above it that was out-filled walks away, which is what losing a controlled
 * auction is. Only the winner's acceptance condition is tested, because it is the only one that
 * ends up holding anything: short of control, nothing settles at all (A1).
 *
 * A sale with no bank to run it does not happen. That is a real constraint and not a gap — an IBD
 * is people, and a world whose banks employ none of them is one where companies change hands by
 * private treaty, which this world has no mechanism for and does not pretend to.
 */
export function runProcess(ctx: MechanismContext, target: PartyId, bids: readonly Bid[]): void {
  const first = bids[0];
  if (first === undefined) return;
  const bank = appoint(ctx, first.ccy);
  if (bank === undefined) {
    ctx.record(
      'control.failed',
      [first.buyer, target],
      {
        buyer: String(first.buyer),
        target: String(target),
        why: 'no bank had people free to run it',
      },
      true,
    );
    return;
  }
  runTender(ctx, pickWinner(ctx, target, bids), bids, bank);
}

/**
 * B4: THE WINNER — the highest bidder, which is the one the book fills first and fills most.
 *
 * It is not the whole of a contest and the record says so: what the losing bids do here is raise
 * the level the holders are met at, because they are in the same book (`runTender` posts all of
 * them). Who ends up with the company is the highest of them, which is what a controlled auction
 * concludes with.
 */
function pickWinner(ctx: MechanismContext, target: PartyId, bids: readonly Bid[]): Bid {
  // Audit D3: the highest bid, and a tie broken on the name so two worlds from one seed agree.
  const sorted = [...bids].sort((a, b) =>
    a.price === b.price ? String(a.buyer).localeCompare(String(b.buyer)) : b.price - a.price,
  );
  const best = sorted[0];
  if (best === undefined) throw new Missing('M&A B4', `no bidder for ${String(target)}`);
  if (sorted.length > 1) {
    ctx.record(
      'control.contested',
      [target],
      {
        target: String(target),
        bidders: sorted.length,
        // B4: what the others were willing to pay, which is what is in the book beside the winner.
        bids: sorted.map((b) => ({ buyer: String(b.buyer), price: b.price })),
      },
      true,
    );
  }
  return best;
}

/**
 * How many sales each bank has been appointed to run THIS period, emptied when the period turns.
 *
 * A capacity is a fact about a period — the hours a bank paid for this week — so a counter that
 * carried over would have every bank at capacity for ever after a busy fortnight (Law 8: the
 * periodicity is part of the number).
 */
function appointments(ctx: MechanismContext): { ran: Record<string, number> } {
  const held = ctx.state<{ at: number; ran: Record<string, number> }>('control.advisory', () => ({
    at: -1,
    ran: {},
  }));
  if (held.at !== Number(ctx.period)) {
    held.at = Number(ctx.period);
    held.ran = {};
  }
  return held;
}

/**
 * M&A B4 (10f.4): WHO RUNS IT — the cheapest bank with people free, read off what the banks
 * published (`advisory.quoted`) and never chosen by this module.
 *
 * Capacity is counted as the period goes, so a bank with two bankers runs two sales and the third
 * seller goes to somebody else or does not sell. That is the constraint the owner asked for: what a
 * bank can run at once is the people it employs, and nothing states a number.
 */
function appoint(
  ctx: MechanismContext,
  ccy: CurrencyCode,
): { readonly bank: PartyId; readonly fee: Cash } | undefined {
  const ran = appointments(ctx);
  let best: { bank: PartyId; fee: Cash } | undefined;
  for (const q of advisoryQuotesIn(ctx.journal, ctx.period)) {
    if (q.ccy !== String(ccy)) continue;
    if (zeroIfNone(ran.ran[q.bank]) >= q.capacity) continue;
    if (best === undefined || q.fee < best.fee) best = { bank: partyId(q.bank), fee: q.fee };
  }
  if (best === undefined) return undefined;
  const count = zeroIfNone(ran.ran[String(best.bank)]) + 1;
  ran.ran[String(best.bank)] = count;
  // Labour D1 (10f.4): what its people did this period, published under its own name, so its own
  // hiring next period asks for the hours these took (`advisoryOrders`) and no more (Law 19).
  ctx.record('advisory.ran', [best.bank], { bank: String(best.bank), processes: count }, false);
  return best;
}

/**
 * A1, B2, C1, C2: THE TENDER. Everyone who holds the line is asked; the book clears; and either
 * enough came in to meet the condition or nothing settles at all.
 */
export function runTender(
  ctx: MechanismContext,
  bid: Bid,
  /** B4: every bidder for this target, all of them in one book. The winner is `bid`. */
  bids: readonly Bid[] = [bid],
  /** 10f.4: the bank that ran it and what it is owed for the work, where a process was run. */
  ran?: { readonly bank: PartyId; readonly fee: Cash },
): void {
  const venue = tenderVenue(bid.target);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      name: `control of ${String(bid.target)}`,
      clearedBy: 'control',
      unit: currencyUnit(bid.ccy),
      ccy: bid.ccy,
      key: { target: String(bid.target) },
    });
  }
  let offered = NO_QTY;
  for (const holder of ctx.register.holdersOf(bid.line)) {
    if (holder === bid.buyer) continue;
    for (const o of tenders(ctx.participant(holder), bid.line)) {
      ctx.post(venue, o);
      offered = addQty(offered, o.qty, 'what was offered');
    }
  }
  if (offered <= 0) {
    ctx.record(
      'control.failed',
      [bid.buyer, bid.target],
      { buyer: String(bid.buyer), target: String(bid.target), why: 'nobody tendered' },
      true,
    );
    return;
  }
  // B4: EVERY BIDDER, in the one book. A second bidder raises the level the holders are met at
  // whether or not it wins, which is what "the price is contested" means arithmetically.
  for (const b of bids)
    ctx.post(venue, { party: b.buyer, side: 'buy', price: b.price, qty: b.needs });
  const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
  if (!isCleared(outcome)) {
    ctx.record(
      'control.failed',
      [bid.buyer, bid.target],
      { buyer: String(bid.buyer), target: String(bid.target), why: outcome.kind },
      true,
    );
    return;
  }
  const taken = downTick(
    sum(outcome.fills.filter((f) => f.side === 'sell').map((f) => f.qty)).value,
  );
  if (taken < bid.needs) {
    /**
     * A1: THE ACCEPTANCE CONDITION. The bid was for control and it did not get control, so nothing
     * settles — there is no half-acquisition in which the acquirer has paid for a minority it never
     * wanted. The holders keep their shares and the acquirer keeps its money.
     */
    ctx.record(
      'control.failed',
      [bid.buyer, bid.target],
      {
        buyer: String(bid.buyer),
        target: String(bid.target),
        why: 'short of the acceptance condition',
        tendered: taken,
        needed: bid.needs,
      },
      true,
    );
    return;
  }
  settleTender(ctx, bid, outcome.fills, outcome.price);
  if (ran !== undefined) payTheBank(ctx, bid, ran);
}

/**
 * M&A B4, §35 A1 (10f.4): THE FEE — real income for real work, out of the proceeds.
 *
 * The seller's bank is paid by the BUYER's counterparty in the deal it ran, which is who has just
 * received the money: a sell-side mandate is paid out of what the sale brought in. It is one
 * instruction with both legs (Law 5) and it is settled or it is not — a bank whose client cannot
 * pay it has done the work and not been paid, which is a recorded state and not an adjustment.
 *
 * Nothing about the fee is a share of the deal (Law 2): it is what the work cost the bank that
 * published it, and the seller chose the cheapest one with people free.
 */
function payTheBank(
  ctx: MechanismContext,
  bid: Bid,
  ran: { readonly bank: PartyId; readonly fee: Cash },
): void {
  // Law 8: a money leg moves a count of the money's own smallest piece, and the fee was struck in
  // those pieces (`costOfAProcess`: hours at what an hour costs). Down, because what it charges is
  // what it can actually be paid in whole pieces.
  const fee = downTick(ran.fee.pieces);
  if (fee <= 0) return;
  const r = ctx.settle({
    legs: [
      {
        kind: 'money',
        from: ctx.accountOf(bid.buyer, bid.ccy),
        to: ctx.accountOf(ran.bank, bid.ccy),
        ccy: bid.ccy,
        amount: asQty(fee, 'what the bank is paid for running it'),
      },
    ],
    cause: 'transfer',
    reason: `${String(ran.bank)} ran the sale of ${String(bid.target)}`,
  });
  ctx.record(
    'advisory.fee',
    [ran.bank, bid.target],
    {
      bank: String(ran.bank),
      target: String(bid.target),
      payer: String(bid.buyer),
      fee,
      paid: r.outcome === 'settled',
    },
    true,
  );
}

/**
 * A2, §29 B2, B2.a, B3, B4, B5, Law 5 (17b.3): THE SHARES ONE WAY AND THE MONEY THE OTHER, IN ONE
 * NUMBERED INSTRUCTION PER HOLDER — AND THE MONEY COMES FROM TWO PLACES.
 *
 * *"Most of the price is debt raised against the target itself"* (B2), and *"the debt is the
 * TARGET's liability, not the fund's"* (B2.a). The only two-sided way for a company's own borrowing
 * to reach its own shareholders is for it to get its shares back for the money — so a fill the
 * facility pays for is a REDEMPTION: the units go back to their issuer and cease, which settlement
 * already does for any asset leg whose payee is the issuer (Register B3), and the drawing that pays
 * for them is a leg of the same instruction. A fill the buyer pays for is the ordinary purchase it
 * always was, and its shares move.
 *
 * The buyer ends up holding a majority of what is LEFT, because the denominator shrank. That is
 * what leverage is, and it is arithmetic rather than a rule — and B4 is not a step anywhere:
 * *"leverage up, interest cost up, ownership changed in the register"* is what this instruction did.
 *
 * WHOLE FILLS, NOT A SHARE OF EACH. Every seller is met at the same price for the same shares and a
 * share is fungible, so which of them was redeemed and which was bought is immaterial to all of
 * them — where splitting each fill between two payers would put a cell's grain through the mill
 * twice (XI-15). The facility is offered the biggest fills first so that the least of it goes
 * unused; ties break on the name, so two worlds from one seed agree (Audit D3).
 *
 * B5 then balances by construction rather than by a check: what the sellers were paid IS what was
 * drawn plus what the buyer put up, leg by leg, and *"the debt proceeds stop at the target"* is the
 * one thing that cannot happen here — they were never the target's to stop.
 */
function settleTender(
  ctx: MechanismContext,
  bid: Bid,
  fills: readonly { readonly party: PartyId; readonly side: string; readonly qty: Qty }[],
  price: PerPiece,
): void {
  const paid: Qty[] = [];
  const unfilled: { readonly holder: string; readonly wanted: Qty; readonly filled: Qty }[] = [];
  let bought = NO_QTY;
  // E1: what a lender promised the company for this deal, and nothing if none did.
  const facility = facilityFor(ctx, bid.target, bid.ccy);
  let room = facility.some ? facility.value.limit.pieces : 0;
  let drawn = 0;
  const sells = [...fills]
    .filter((f) => f.side === 'sell')
    .sort((a, b) => (b.qty === a.qty ? String(a.party).localeCompare(String(b.party)) : b.qty - a.qty));
  for (const f of sells) {
    const seller = ctx.parties.get(f.party);
    /**
     * XI-15, Law 8: A CELL IS A POPULATION AND WHAT IT HANDS OVER IS STRUCK PER MEMBER, because a
     * member is a real holder with a real account and cannot part with a fraction of a share or be
     * paid a fraction of the smallest piece of its money.
     *
     * So the fill is cut to the grain of the cell it came from FIRST, and both legs are that
     * per-member figure times the weight. It used to divide the fill by the weight and hand the
     * quotient to `asQty`, which refuses a fraction — so a tender that struck against a cell whose
     * weight did not divide the fill exactly STOPPED THE RUN (item 0, stop 13) rather than buying
     * the shares that were there.
     *
     * What the cut drops is not demand left unmet somewhere: those shares simply stay where they
     * were, and the tender bought fewer of them and says so.
     */
    const share = gridPerMember(ctx.registry, seller, downTick(f.qty) / weightOf(seller));
    const units = share.total;
    const perMember = ctx.registry.payable(
      valueAt(price, share.perMember, bid.ccy, 'what a member is paid'),
    );
    const cash = totalOverMembers(seller, perMember);
    if (units <= 0 || cash <= 0) {
      unfilled.push({ holder: String(f.party), wanted: f.qty, filled: NO_QTY });
      continue;
    }
    if (units < downTick(f.qty)) {
      unfilled.push({ holder: String(f.party), wanted: f.qty, filled: units });
    }
    // B2, B3: WHO PAYS FOR THIS ONE. The company, out of what its lender promised, while the
    // promise still covers the whole of it; otherwise the buyer, out of its own money (B3's equity
    // cheque). Nothing is split and nothing is part-drawn.
    const onTheFacility = facility.some && cash <= room;
    const legs: Leg[] = onTheFacility
      ? drawnLegs(ctx, bid, facility.value, f.party, units, cash, price)
      : [
          {
            kind: 'asset',
            from: f.party,
            to: bid.buyer,
            instrument: bid.line,
            qty: units,
            pricePerUnit: some(price),
            accruedPerUnit: none(),
          },
          {
            kind: 'money',
            from: ctx.accountOf(bid.buyer, bid.ccy),
            to: ctx.accountOf(f.party, bid.ccy),
            ccy: bid.ccy,
            amount: cash,
          },
        ];
    const r = ctx.settle({
      legs,
      cause: 'trade',
      reason: onTheFacility
        ? `${String(bid.target)} buys back ${units} of ${String(bid.line)} with what it drew for the tender`
        : `${String(bid.buyer)} buys ${units} of ${String(bid.line)} in its tender for ${String(bid.target)}`,
    });
    if (r.outcome !== 'settled') continue;
    paid.push(cash);
    if (onTheFacility) {
      room -= cash;
      drawn += cash;
    } else bought = addQty(bought, units, 'bought');
  }
  // A2, Part II: WHAT THE BOOK STRUCK AND THE HOLDERS COULD NOT DELIVER. A tender that bought less
  // than it cleared is the ordinary consequence of a cell's grain, and it is the difference between
  // a majority won and a majority missed — so it is measured rather than left in the gap between
  // `control.acquired` and the session's own volume.
  if (unfilled.length > 0) {
    ctx.record('tender.unfilled', [bid.buyer, bid.target, bid.line], { by: unfilled }, true);
  }
  if (bought <= 0 && drawn <= 0) return;
  ctx.record(
    'control.acquired',
    [bid.buyer, bid.target, bid.line],
    {
      buyer: String(bid.buyer),
      target: String(bid.target),
      shares: bought,
      // D5: what was PAID, which is what the acquirer actually put up and can be checked against
      // the accounts of everybody who was paid (Law 19). It is not the bid and not the offer.
      paid: sum(paid).value,
      // B5 (17b.3): THE SOURCES, beside the use they paid for. What the company borrowed against
      // itself and what the buyer put up out of its own money add to what the sellers were paid,
      // and the family that checks it reads these three off this one event (17b.5).
      drawn,
      cheque: sum(paid).value - drawn,
      price,
    },
    true,
  );
  /**
   * A4, item 9: AND NOW SOMETHING HAPPENS. A takeover used to buy the shares and stop: the target
   * stayed a separate party with its own balance sheet, its own line and its own board for ever,
   * and `combine` — which is what A4 and A5 are about — was exported and called by nobody, because
   * there was nothing for a completed tender to write to.
   *
   * What a completed tender establishes is CONTROL, and control is an OUTCOME read off the register:
   * a holder with more than half of what is in issue has the votes. Nothing here is a threshold
   * somebody declared — "more than half" is arithmetic, and the two numbers are both reads.
   */
  const held = Number(ctx.register.quantity(bid.buyer, bid.line));
  const inIssue = Number(ctx.instruments.get(bid.line).issued);
  if (inIssue <= 0) return;
  const already = ctx.control.controllerOf(bid.target);
  if (already === undefined && held * 2 > inIssue) {
    ctx.takeControl(bid.buyer, bid.target, 'shares', `holds ${held} of ${inIssue} shares in issue`);
  }
  /**
   * A5, D4, §29 A5, B2.a (item 10f.3): AND WHEN NOTHING IS LEFT OUTSIDE, ONE OF TWO THINGS HAPPENS.
   *
   * A majority makes a SUBSIDIARY — which is what the world above records — and it is holding ALL
   * of it that settles what the acquirer may do with the residual, because it is entirely its own.
   * Then there are two deals and they are different deals:
   *
   *  - **a MERGER**: the two balance sheets combine and the combined firm is one party (§35 A4);
   *  - **an ACQUISITION**: the target goes on being a company with its own balance sheet, its own
   *    name and its own debts (§29 A5), and if it had a market that market CLOSES — which is the
   *    take-private, and is why *"a failed buyout kills the firm and not the fund"* (§29 B2.a).
   *
   * WHICH ONE IS NOT A KIND BRANCH AND NOT A FLAG (Law 15): it is whether the acquirer could RUN
   * what it bought, and running a business is having people do the work. A buyer that has never
   * had a wage bill cannot absorb a factory into itself — it has nobody to operate it — so it owns
   * it instead, which is what a fund is and what §29 A5 describes without this module knowing that
   * a fund exists. A buyer that employs can take the plant, the stock and the workers inside.
   */
  if (held < inIssue) return;
  if (couldRunIt(ctx, bid.buyer)) combine(ctx, bid.buyer, bid.target);
  else own(ctx, bid.buyer, bid.target, bid.line);
}

/**
 * §29 B2, B2.a, B4, Banks Lending B1, Register B3 (17b.3): THE COMPANY BORROWS AND BUYS ITS OWN
 * SHARES BACK, in one instruction with the seller's.
 *
 * Three legs and every one of them two-sided. The shares go back to their ISSUER, which settlement
 * turns into a redemption — the units cease and what is in issue falls (Register B3). The row is
 * issued by the target to the bank that committed it, which is B2.a: *"the debt is the target's
 * liability, not the fund's."* And the bank's own money is created against it and lands in the
 * seller's account, which is Banks Lending B1 — nothing left the bank to make this loan, and the
 * proceeds never sat anywhere: they paid the shareholder they were borrowed to pay (B5).
 *
 * The row is `facilityLoanId`, which is the same name the lender's own headroom read uses, so what
 * has been drawn on the commitment is one number read off the register by both of them (Law 4,
 * Law 19). A second drawing on the same deal adds to the same row rather than writing a new one.
 */
function drawnLegs(
  ctx: MechanismContext,
  bid: Bid,
  facility: Facility,
  seller: PartyId,
  units: Qty,
  cash: Qty,
  price: PerPiece,
): Leg[] {
  const row = facilityLoanId(facility.bank, bid.target);
  if (!ctx.instruments.has(row)) {
    const terms: LoanTerms = {
      kind: LOAN,
      originator: facility.bank,
      borrower: bid.target,
      // A2: the rate and the term were struck when the line was committed, not when it is drawn.
      rate: facility.rate,
      drawn: ctx.calendar.startOf(ctx.period),
      maturity: facility.maturity,
      dayCount: 'ACT/365F',
      // Bond F3: a buyout's debt is a term loan the company pays down, never a line it draws at
      // will — which is what makes C1's *"less room out of cash flow"* a payment and not a mood.
      amortising: true,
      // A4: unsecured. What a lender takes security over in a buyout is a covenant it bids for,
      // and no holder bids a covenant yet (finding 21.60(b), positioned at 17b.8).
      security: [],
    };
    ctx.issue({ id: row, kind: LOAN, issuer: some(bid.target), ccy: bid.ccy, terms, market: none() });
  }
  return [
    {
      kind: 'asset',
      from: seller,
      to: bid.target,
      instrument: bid.line,
      qty: units,
      pricePerUnit: some(price),
      accruedPerUnit: none(),
    },
    {
      kind: 'asset',
      from: bid.target,
      to: facility.bank,
      instrument: row,
      qty: cash,
      pricePerUnit: some(asPerPiece(1, 'at what it promised')),
      accruedPerUnit: none(),
    },
    {
      kind: 'money',
      from: { holder: facility.bank, issuer: facility.bank },
      to: ctx.accountOf(seller, bid.ccy),
      ccy: bid.ccy,
      amount: cash,
    },
  ];
}

/**
 * §35 A4, §29 A5, Law 15: WHETHER THE ACQUIRER COULD RUN WHAT IT BOUGHT.
 *
 * A wage bill, read off the wire (Law 19). It is the whole test and it is a fact rather than a
 * label: a party that has never paid anybody has nobody to operate a factory, so what it has bought
 * is a company and not a business, and it owns one rather than becoming one. A pool of money is in
 * that position permanently and this module never has to know what a pool of money is.
 */
function couldRunIt(ctx: MechanismContext, buyer: PartyId): boolean {
  return hasEverMetAPayroll(ctx, buyer);
}

/**
 * §29 A5, B2.a, Equity E3 (item 10f.3): IT OWNS IT, AND THE MARKET CLOSES.
 *
 * The target survives: same party, same balance sheet, same name, same debts — which is the whole
 * of why a buyout that fails kills the company and not the buyer. What ends is the market, because
 * every share is in one pair of hands and there is nobody left to trade with; the prints it made
 * stay where they are and go visibly stale (Clearing E4), which is §29 C5's *"a value that is not
 * a market price"* arrived at by the book closing rather than by a rule about unlisted things.
 *
 * A company that had no market is bought the same way and nothing is delisted, because there was
 * nothing to delist — one path, two starting states, and no branch on which (Law 15).
 */
function own(ctx: MechanismContext, buyer: PartyId, target: PartyId, line: InstrumentId): void {
  if (ctx.control.controllerOf(target) === undefined) {
    ctx.takeControl(buyer, target, 'shares', 'wholly owned, and it goes on being a company');
  }
  const wasListed = ctx.instruments.get(line).market.some;
  if (wasListed) ctx.delist(line);
  ctx.record(
    'control.owned',
    [buyer, target, line],
    {
      buyer: String(buyer),
      target: String(target),
      line: String(line),
      // E3: whether this was a TAKE-PRIVATE — a line that traded this morning and does not now.
      tookPrivate: wasListed,
      why: 'the buyer employs nobody, so it owns the company rather than absorbing the business',
    },
    true,
  );
}

/**
 * A4, A5, D4, XI-8: THE TWO BALANCE SHEETS COMBINE. The target's own paper is assumed by the
 * acquirer, ITS HOLDINGS MOVE TO THE ACQUIRER, its shares cease to be a claim on anything because
 * the residual they were has been bought, and the party is terminated with the acquirer as
 * successor.
 *
 * It is the estate's door and not a second one: "this party's obligations are now that one's" is one
 * fact, and a world with two ways to write it would have two ways for it to be wrong (Law 4).
 *
 * A-70: THE DOCSTRING SAID "its holdings are reseated" AND THE FUNCTION DID NOT DO IT. `assume` maps
 * to `reseat`, which changes an instrument's ISSUER — that moves the target's LIABILITIES, and there
 * was nothing here that moved its cash, its plant, its inventory or its paper. Had it ever been
 * called, `cease` would have marked the target dead with holdings still under its id and the `names`
 * family would have reported "`target` has ceased but still holds `instrument`" for every line it
 * held, every period, for the rest of the run — Register F2, the clause the docstring cites.
 */
export function combine(ctx: MechanismContext, buyer: PartyId, target: PartyId): void {
  // XI-3, §25 C1: it is ENDING and it has not ended. Item 7's state, and the interval where its
  // book is being moved used to be indistinguishable from a firm trading normally.
  ctx.standing(target, 'winding', `being combined into ${String(buyer)}`);
  handOver(ctx, buyer, target);
  for (const i of ctx.instruments.issuedBy(target)) {
    if (!i.status.live) continue;
    // A5: including the shares themselves — a residual claim on a firm that is now part of another
    // firm is a claim on that other firm, and the register says so rather than a rule.
    ctx.settle({
      legs: [{ kind: 'assume', from: target, to: buyer, instrument: i.id }],
      cause: 'corporateAction',
      reason: `${String(buyer)} assumes ${String(i.id)} from ${String(target)}`,
    });
  }
  /**
   * Register F2, Appendix B: NO DEATH WITHOUT A DESTINATION. What it still holds after the two
   * passes above is encumbered — a lien is not free and the register refuses to move bound units —
   * so the acquirer has bought a company whose assets are pledged to somebody else, and it stays a
   * named party under the acquirer's control until those liens are released. Combining it would
   * write a residual with no holder, which is the one thing the estate's door exists to prevent.
   */
  if (ctx.register.holdingsOf(target).length > 0) {
    ctx.takeControl(buyer, target, 'shares', 'wholly owned; its remaining holdings are encumbered');
    ctx.record(
      'control.combined',
      [buyer, target],
      { buyer: String(buyer), target: String(target), combined: false, why: 'encumbered' },
      true,
    );
    return;
  }
  // It is inside the acquirer now, so there is no longer a party for anybody to control (Law 4).
  ctx.releaseControl(target, `combined into ${String(buyer)}`);
  ctx.cease(target, buyer);
  ctx.record(
    'control.combined',
    [buyer, target],
    { buyer: String(buyer), target: String(target), combined: true },
    true,
  );
}

/**
 * A4: EVERYTHING THE TARGET HOLDS, to the acquirer, by name and to the piece — which is what the
 * docstring above always claimed and what the register needs before the party can end.
 *
 * It is one instruction, so it is atomic: either the whole balance sheet moves or none of it does
 * and the target is still there holding it (Money E1, Register C3.b). Money moves as money and
 * everything else as an asset, because a deposit is a holding and is not moved as one (Money D2).
 */
function handOver(ctx: MechanismContext, buyer: PartyId, target: PartyId): void {
  const view = ctx.participant(target);
  const legs: Leg[] = [];
  const monies = new Set<CurrencyCode>();
  for (const h of view.holdings()) {
    const inst = ctx.instruments.get(h.instrument);
    if (ctx.registry.instrumentKind(inst.kind).pricing === 'money') {
      monies.add(inst.ccy);
      continue;
    }
    const free = Number(view.free(h.instrument));
    if (free <= 0) continue;
    const print = view.print(h.instrument);
    legs.push({
      kind: 'asset',
      from: target,
      to: buyer,
      instrument: h.instrument,
      qty: asQty(free),
      pricePerUnit: print.some ? some(print.value.price) : none(),
      accruedPerUnit: none(),
    });
  }
  for (const ccy of monies) {
    const cash = Number(view.cash(ccy));
    if (cash <= 0) continue;
    legs.push({
      kind: 'money',
      from: ctx.accountOf(target, ccy),
      to: ctx.accountOf(buyer, ccy),
      ccy,
      amount: asQty(cash),
    });
  }
  if (legs.length === 0) return;
  ctx.settle({
    legs,
    cause: 'corporateAction',
    reason: `${String(target)} is combined into ${String(buyer)} and hands over what it holds`,
  });
}

/**
 * E1, E2: what must be true of a deal that happened. A firm that was acquired has a successor, and
 * the shares somebody bought are held by the party that bought them — measured, never enforced.
 */
/**
 * §29 B5, Law 19 (17b.5): THE SOURCES AND USES OF EVERY DEAL, MEASURED.
 *
 * *"The sources and uses of a deal must balance exactly, and the money must come out of named
 * accounts. Sellers paid the equity cheque while the debt proceeds stop at the target is a deal
 * that did not balance."*
 *
 * They balance by construction: every fill is one numbered instruction whose money leg pays a named
 * seller out of either the drawing or the buyer's own account, so what the sellers were paid IS what
 * was drawn plus what the buyer put up. **A VERIFY that cannot fail still has to be measured**, and
 * that is the whole reason this family exists rather than a comment saying it holds: it is what
 * would say so the day somebody adds a third source, a fee taken out of the middle, or a leg that
 * moves money without a counterparty. A FORBID that holds is as valuable as a mechanism that works,
 * and it breaks silently (Part II).
 *
 * The tolerance is arithmetic dust derived from the three magnitudes, never a band (Law 7).
 */
function sourcesAndUses(): Family {
  return {
    // Audit B7: it is a FLOWS question — money that reached a named seller against the two places
    // it came from — so it joins the family that owns that question rather than opening a tenth.
    name: 'flows',
    contributor: 'control',
    spec: 'Private Equity B5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const e of view.journal.ofKindIn('control.acquired', view.period)) {
        const paid = e.data['paid'];
        const drawn = e.data['drawn'];
        const cheque = e.data['cheque'];
        if (typeof paid !== 'number' || typeof drawn !== 'number' || typeof cheque !== 'number') {
          continue;
        }
        const sources = sum([drawn, cheque]);
        if (withinDust(sources.value, paid, sum([sources.value, paid]).dust)) continue;
        const named = e.data['target'];
        out.push({
          family: 'flows',
          spec: 'Private Equity B5',
          owner: partyId(typeof named === 'string' ? named : String(e.subjects[1])),
          size: sources.value - paid,
          unit: 'money',
          period: view.period,
          message: `the deal for ${String(named)} drew ${drawn} and paid a cheque of ${cheque} against ${paid} to the sellers`,
        });
      }
      return out;
    },
  };
}

function deals(): Family {
  return {
    name: 'names',
    contributor: 'control',
    spec: 'M&A E1 M&A E2 XI-8',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const e of view.journal.ofKind('control.combined')) {
        const named = e.data['target'];
        if (typeof named !== 'string') continue;
        const target = partyId(named);
        const p = view.parties.get(target);
        if (!p.status.alive) continue;
        // E2: a firm that was combined into another is not still trading on its own account.
        out.push({
          family: 'names',
          spec: 'M&A E2',
          owner: target,
          size: 1,
          // 16.1: a count of parties, not money — the unit says so instead of naming a currency.
          unit: 'parties',
          period: view.period,
          message: `${named} was acquired and is still alive`,
        });
      }
      return out;
    },
  };
}

/**
 * B1, B3, A3: WHO BIDS FOR WHOM. A firm looks at the listed lines it can see, and bids where its own
 * number for a share is above what the market is asking AND it can pay for control out of what it
 * holds. Both halves are its own reads; neither is a rule about when takeovers happen.
 *
 * A3: it pays in CASH here, out of money it has. Paying in its own shares is dilution and paying
 * with borrowed money is a credit decision somebody else takes — both are real and both are the
 * next step, and neither is invented by assuming it.
 */
/**
 * Equity A1: THE LINES A BIDDER COULD BID FOR — the residual claims. A share is the residual claim
 * and not a liability, which is the whole of what makes a firm buyable: buying every liability of a
 * firm buys you nothing, and buying the residual buys you the firm. So what a bidder looks for is a
 * claim ON somebody that its issuer does not owe, which the register and the kind profile already
 * say, with nothing here branching on a kind id (Law 15).
 *
 * Law 18: it is the same list for every bidder, so it is found ONCE for the session rather than
 * once per firm. Asked per firm it was a walk over every instrument in the world for each of three
 * thousand of them, to arrive at the same list every time.
 *
 * 10f.1: it is EVERY firm's residual and not only the ones with a market, which is what it always
 * said and could not deliver — a private company is bought by buying its shares, and until this
 * world had private companies in it there were none to buy. What a tender does not need is a
 * market: it opens a venue of its own (`runTender`), which is what a controlled process IS.
 */
export function equityLines(ctx: MechanismContext): readonly Instrument[] {
  const out: Instrument[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some) continue;
    if (ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    out.push(i);
  }
  return out;
}

/**
 * M&A B3, §29 B1 (item 10f.6): WHO COULD BUY A COMPANY — everybody who published a cost of money
 * this period, and nobody else.
 *
 * It is a walk of the JOURNAL and not of the party list, which is two things at once. It is the
 * right ANSWER — *"it must be able to fund it, so the credit market decides which deals happen"*
 * (B3), so a party nobody has quoted and no investors have required anything of cannot value a
 * company at all — and it is the right TRAVERSAL (Law 18): the buyers are the handful who published
 * rather than every party in the world asked one by one.
 *
 * It is also what lets a POOL buy a company without this module knowing what a pool is: it
 * published a number under its own name and that is the whole of the qualification.
 */
function couldBuy(ctx: MechanismContext): readonly PartyId[] {
  const out = new Set<PartyId>();
  const published = [
    ...namesQuotedIn(ctx.journal, ctx.period),
    ...strikesPublished(ctx.journal).filter((e) => e.period === ctx.period),
  ];
  for (const e of published) {
    for (const named of e.subjects) {
      const who = partyId(named);
      if (ctx.parties.has(who)) out.add(who);
    }
  }
  return [...out];
}

/**
 * §29 B2, B3 (17b.2): A DEAL THIS BUYER WANTS AND CANNOT PAY FOR OUT OF WHAT IT HOLDS.
 *
 * It is not a refusal and not a bid: it is the hole, which is the thing a lender is asked to fill.
 */
export interface Wanted {
  readonly bid: Bid;
  /** What control would cost at this buyer's own number for the company. */
  readonly cost: Cash;
  /** B3: the equity cheque — what the buyer itself has to bring, and has to have called (A2). */
  readonly has: Cash;
  /** B2: the rest, and what a lender is asked to commit. Nothing where the buyer can find it all. */
  readonly hole: Cash;
}

/**
 * M&A B1, B3, §29 B2, B3 (17b.2): WHAT THIS BUYER WOULD DO, SPLIT BY WHETHER IT CAN PAY FOR IT.
 *
 * One walk of the lines for both answers, because it is one question asked once: what is each of
 * these companies worth to me, and can I find the money. A deal it can fund becomes a bid in this
 * period's book; a deal it cannot becomes a borrowing the company is asked to arrange, and comes
 * back as a bid two periods later or not at all (§29 B2.b).
 *
 * **What counts as money it can pay with** is its own balance PLUS what a lender has committed to
 * the target for this deal (`committedTo`), because the commitment is drawn inside the instruction
 * that completes the purchase (17b.3). A deal a bank has promised to fund is therefore bid for and
 * not asked about again.
 */
export function controlDealsFor(
  view: ParticipantView,
  ctx: MechanismContext,
  lines: readonly Instrument[],
): { readonly bids: readonly Bid[]; readonly wanted: readonly Wanted[] } {
  const ccy = view.registry.currencyOf(view.self.region);
  const held = view.cash(ccy);
  // B3: a buyout with no equity cheque at all is not a buyout. A buyer with nothing of its own is
  // not levering anything — it is asking a bank to buy a company and hold the shares for it.
  if (held <= 0) return { bids: [], wanted: [] };
  const cash = cashOf(held, ccy);
  // B1: what this buyer's own money costs it, which is the same number for every company on the
  // list, and a firm nobody will lend to does not look at the list at all.
  const required = costOfMoneyOf(ctx, view.self.id);
  if (!required.some || required.value <= 0) return { bids: [], wanted: [] };
  const bids: Bid[] = [];
  const wanted: Wanted[] = [];
  for (const i of lines) {
    const target = i.issuer.some ? i.issuer.value : view.self.id;
    if (target === view.self.id) continue;
    const worth = worthAt(view, ctx, target, i.id, required);
    if (!worth.some) continue;
    const outstanding = ctx.register.heldTotal(i.id).value;
    if (outstanding <= 0) continue;
    // A1: control is more than half of what exists, read off the register rather than declared.
    const needs = downTick(
      scale(
        outstanding,
        asRatio(1 / 2, 'more than half of what exists'),
        'more than half of what exists',
      ),
    );
    if (needs <= 0) continue;
    const cost = valueAt(worth.value, needs, ccy, 'what control would cost it at its own number');
    const bid = { buyer: view.self.id, target, line: i.id, price: worth.value, needs, ccy };
    /**
     * E1, B2 (17b.3): IT DOES NOT BID FOR WHAT IT CANNOT PAY FOR — and what it can pay with is its
     * own money AND what a lender has committed to the company, because the commitment is drawn
     * inside the instruction that completes the purchase. This is the line that makes a LEVERED bid
     * possible: without it every buyer bids only what it holds, and §29 B is a section about a
     * mechanism this world does not have.
     */
    const committed = committedTo(ctx, target, ccy);
    const hole = holeIn(cost, cash, committed);
    if (hole.pieces <= 0) bids.push(bid);
    else wanted.push({ bid, cost, has: cash, hole });
  }
  return { bids, wanted };
}

export function control(): SystemModule {
  return {
    id: 'control',
    spec: 'M&A',
    nouns: [
      {
        name: 'control.advisory',
        kind: 'working',
        holds: 'how many sales each bank has been appointed to run this period',
        why: 'a counter within this module’s own phase, so a bank is not appointed to more processes than it has people for (10f.4). It is not a fact about the world — what the world keeps is the `advisory.ran` event each appointment writes — and it does not survive the phase in any sense a reader could use.',
      },
    ],
    requires: ['equity', 'firms'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'control.tender',
        spec: 'M&A A1 M&A B1 M&A B2 M&A C1 M&A C2',
        /**
         * After the markets, because what a share is worth to anybody is what this period's book
         * said, and a bid struck on last period's print is a bid at a stale number (Law 19). Before
         * the banks turn what is left into rows, because a buyer that pays for control out of a
         * balance it turns out not to have is overdrawn, and an overdraft here is a LOAN.
         */
        anchor: { before: 'lending.book' },
        // Clearing F1.a (17.0a): THE PHASE RUNS NOW. A bid is struck on what the target published
        // (B1: the acquirer's own valuation of the target's earnings), and until every company
        // prepared a statement only listed companies had one — so no bid was ever formed and this
        // phase's read set had never been tested against a run. These are the reads it makes.
        reads: [
          { kind: 'event', name: 'advisory.quoted', of: 'anyPeriod' },
          { kind: 'event', name: 'control.combined', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.quoted', of: 'thisPeriod' },
          { kind: 'event', name: 'fund.struck', of: 'thisPeriod' },
          { kind: 'event', name: 'reporting.report', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'control.acquired' },
          { kind: 'event', name: 'control.advisory' },
          { kind: 'event', name: 'control.combined' },
          { kind: 'event', name: 'control.contested' },
          { kind: 'event', name: 'control.failed' },
          { kind: 'event', name: 'control.financing' },
          { kind: 'event', name: 'control.owned' },
          { kind: 'event', name: 'control.tender' },
          { kind: 'event', name: 'credit.request' },
          { kind: 'event', name: 'tender.unfilled' },
        ],
        run: (ctx: MechanismContext): void => {
          const lines = equityLines(ctx);
          if (lines.length === 0) return;
          /**
           * B4 (10f.4): EVERY BID FOR ONE COMPANY IS ONE PROCESS. Gathered first and run once, so
           * two buyers that want the same target meet in the same book instead of taking turns at
           * it in whatever order the party list happens to be in — which is not an auction, and
           * which let the first bidder past the post buy it before the second was asked.
           */
          const byTarget = new Map<PartyId, Bid[]>();
          const holes = new Map<PartyId, Wanted>();
          for (const p of couldBuy(ctx)) {
            if (!ctx.parties.get(p).status.alive) continue;
            const { bids, wanted } = controlDealsFor(ctx.participant(p), ctx, lines);
            for (const bid of bids) {
              const held = byTarget.get(bid.target);
              if (held === undefined) byTarget.set(bid.target, [bid]);
              else held.push(bid);
            }
            // §29 B2 (17b.2): ONE ASK PER TARGET, and where two buyers want the same firm it is the
            // biggest hole — what the company would have to carry if the dearest of them wins. The
            // two of them meet in the book (B4) and not here.
            for (const w of wanted) {
              const worst = holes.get(w.bid.target);
              if (worst === undefined || w.hole.pieces > worst.hole.pieces) holes.set(w.bid.target, w);
            }
          }
          for (const [target, bids] of byTarget) runProcess(ctx, target, bids);
          // B2.b: and the companies nobody could pay for outright ask the credit market whether
          // they can be bought at all. A deal that comes back funded is a bid two periods from now.
          for (const [target, w] of holes) {
            if (byTarget.has(target)) continue;
            askToFund(ctx, target, w.hole, w.has, w.bid.buyer);
          }
        },
      },
    ],
    participants: [],
    families: [deals(), sourcesAndUses()],
  };
}

/** A1: what control of a line costs at a price — a read for the observer and for a test. */
export const costOfControl = (outstanding: Qty, price: PerPiece, ccy: CurrencyCode): Cash =>
  valueAt(price, atMost(outstanding, outstanding, 'all of it'), ccy, 'what all of it would cost');

/** B2.a: the premium, as the distance between two numbers rather than a percentage anybody set. */
export const premiumOver = (paid: PerPiece, printed: PerPiece): PerPiece =>
  minus(paid, printed, 'what control was worth above what a share was trading at');
