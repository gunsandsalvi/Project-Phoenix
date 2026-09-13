/**
 * The market for control: what somebody will pay for a whole firm, and what its owners will take.
 *
 * @spec M&A A1 M&A A2 M&A A3 M&A A4 M&A A5 M&A B1 M&A B2 M&A B2.a M&A B3 M&A C1 M&A C2 M&A D4 M&A D5 M&A E1 M&A E2 Equity B1 Equity F3 XI-8 Law 2 Law 3 Law 4 Law 6 Law 19
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
  partyId,
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { currencyUnit } from '../../core/ids.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { add, atMost, div, mul, sub, sum } from '../../core/num.js';
import { downTick, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide } from '../../ledger/settlement.js';
import { asQty } from '../../core/tick.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { FIRM } from '../../registry/profiles.js';
import { weightOf } from '../../parties/party.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period as periodOf } from '../../calendar/calendar.js';

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
): Option<number> {
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
  const required = quotedTo(ctx, buyer.self.id);
  if (!required.some || required.value <= 0) return none<number>();
  const said = reportOf(ctx, target);
  if (said === undefined || said.earned <= 0 || said.periods <= 0) return none<number>();
  // Law 8: the periodicity is part of the number. What it published covers a span of periods; what
  // a required return is quoted in is a year, so the two are put in the same unit by the calendar
  // rather than by a factor typed here.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + said.periods)),
  );
  if (ofAYear <= 0) return none<number>();
  const annual = div(said.earned, ofAYear, 'what it earns a year, as it published it');
  const whole = div(annual, required.value, 'what that stream is worth at what it requires');
  const shares = ctx.register.heldTotal(line).value;
  if (shares <= 0) return none<number>();
  const perShare = div(whole, shares, 'what one share of it is worth to this buyer');
  const printed = buyer.print(line);
  // B1: it bids only where its own number is above what a share is already trading at. Below that
  // it can buy shares in the market like anybody else and does not need a tender.
  if (printed.some && perShare <= printed.value.price) return none<number>();
  return some(perShare);
}

/** Law 19: what a bank quoted THIS name, per annum. Its own cost of money, read off the wire. */
function quotedTo(ctx: MechanismContext, who: PartyId): Option<number> {
  const said = ctx.journal.ofKind('credit.quoted');
  for (let i = said.length - 1; i >= 0; i -= 1) {
    const e = said[i];
    if (e?.data['borrower'] !== who) continue;
    const rate = e.data['rate'];
    return typeof rate === 'number' ? some(rate) : none<number>();
  }
  return none<number>();
}

interface Published {
  readonly earned: number;
  readonly periods: number;
}

/** Reporting A2: the last accounts this firm published. Read, never rebuilt (A2.a). */
function reportOf(ctx: MechanismContext, who: PartyId): Published | undefined {
  const said = ctx.journal.ofKind('reporting.report');
  for (let i = said.length - 1; i >= 0; i -= 1) {
    const e = said[i];
    if (e?.data['company'] !== who) continue;
    const { earned, from, to } = e.data;
    if (typeof earned !== 'number' || typeof from !== 'number' || typeof to !== 'number') {
      return undefined;
    }
    return { earned, periods: add(sub(to, from, 'the span it covered'), 1, 'inclusive') };
  }
  return undefined;
}

/** A1: the bid. A price, and how much of the firm it has to get for the bid to mean anything. */
export interface Bid {
  readonly buyer: PartyId;
  readonly target: PartyId;
  readonly line: InstrumentId;
  readonly price: number;
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
export function tenders(
  holder: ParticipantView,
  line: InstrumentId,
): readonly Order[] {
  const units = downTick(holder.free(line));
  if (units <= 0) return [];
  const own = holder.outlook(`price.${String(line)}` as never);
  if (!own.some) return [];
  // It sells at its own number or better. What it gets is the clearing price, which is at or above
  // it — the ordinary meaning of an offer, and the reason a tender clears rather than being taken.
  return [{ party: holder.self.id, side: 'sell', price: own.value.expected, qty: units }];
}

/**
 * A1, B2, C1, C2: THE TENDER. Everyone who holds the line is asked; the book clears; and either
 * enough came in to meet the condition or nothing settles at all.
 */
export function runTender(ctx: MechanismContext, bid: Bid): void {
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
  let offered = 0;
  for (const holder of ctx.register.holdersOf(bid.line)) {
    if (holder === bid.buyer) continue;
    for (const o of tenders(ctx.participant(holder), bid.line)) {
      ctx.post(venue, o);
      offered = add(offered, o.qty, 'what was offered');
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
  ctx.post(venue, { party: bid.buyer, side: 'buy', price: bid.price, qty: bid.needs });
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
}

/** A2, Law 5: the shares one way and the money the other, in one numbered instruction per holder. */
function settleTender(
  ctx: MechanismContext,
  bid: Bid,
  fills: readonly { readonly party: PartyId; readonly side: string; readonly qty: number }[],
  price: number,
): void {
  const paid: number[] = [];
  let bought = 0;
  for (const f of fills) {
    if (f.side !== 'sell') continue;
    const units = downTick(f.qty);
    if (units <= 0) continue;
    const cash = downTick(mul(units, price, 'what it pays for them'));
    if (cash <= 0) continue;
    const seller = ctx.parties.get(f.party);
    // XI-15: a cell is a population and what it hands over is struck PER MEMBER, because a member is
    // a real holder with a real account and cannot part with a fraction of a share.
    const shareSide = cellSide(seller, asQty(units / weightOf(seller), 'its shares per member'));
    const cashSide = cellSide(seller, asQty(cash / weightOf(seller), 'its cash per member'));
    const legs: Leg[] = [
      {
        kind: 'asset',
        from: f.party,
        to: bid.buyer,
        instrument: bid.line,
        qty: units,
        pricePerUnit: some(price),
        accruedPerUnit: none(),
        fromCell: shareSide === undefined ? none() : some(shareSide),
        toCell: none(),
      },
      {
        kind: 'money',
        from: ctx.accountOf(bid.buyer, bid.ccy),
        to: ctx.accountOf(f.party, bid.ccy),
        ccy: bid.ccy,
        amount: cash,
        fromCell: none(),
        toCell: cashSide === undefined ? none() : some(cashSide),
      },
    ];
    const r = ctx.settle({
      legs,
      cause: 'trade',
      reason: `${String(bid.buyer)} buys ${units} of ${String(bid.line)} in its tender for ${String(bid.target)}`,
    });
    if (r.outcome !== 'settled') continue;
    paid.push(cash);
    bought = add(bought, units, 'bought');
  }
  if (bought <= 0) return;
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
      price,
    },
    true,
  );
}

/**
 * A4, A5, D4, XI-8: THE TWO BALANCE SHEETS COMBINE. The target's own paper is assumed by the
 * acquirer, its holdings are reseated, its shares cease to be a claim on anything because the
 * residual they were has been bought, and the party is terminated with the acquirer as successor.
 *
 * It is the estate's door and not a second one: "this party's obligations are now that one's" is one
 * fact, and a world with two ways to write it would have two ways for it to be wrong (Law 4).
 */
export function combine(ctx: MechanismContext, buyer: PartyId, target: PartyId): void {
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== target) continue;
    // A5: including the shares themselves — a residual claim on a firm that is now part of another
    // firm is a claim on that other firm, and the register says so rather than a rule.
    ctx.settle({
      legs: [{ kind: 'assume', from: target, to: buyer, instrument: i.id }],
      cause: 'corporateAction',
      reason: `${String(buyer)} assumes ${String(i.id)} from ${String(target)}`,
    });
  }
  ctx.cease(target, buyer);
  ctx.record(
    'control.combined',
    [buyer, target],
    { buyer: String(buyer), target: String(target) },
    true,
  );
}

/**
 * E1, E2: what must be true of a deal that happened. A firm that was acquired has a successor, and
 * the shares somebody bought are held by the party that bought them — measured, never enforced.
 */
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
          unit: currencyUnit('USD' as CurrencyCode),
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
export function controlBidsFor(view: ParticipantView, ctx: MechanismContext): readonly Bid[] {
  const ccy = view.registry.currencyOf(view.self.region);
  const cash = view.cash(ccy);
  if (cash <= 0) return [];
  const out: Bid[] = [];
  for (const i of ctx.instruments.all()) {
    /**
     * Equity A1: A SHARE IS THE RESIDUAL CLAIM AND NOT A LIABILITY, which is the whole of what
     * makes a firm buyable: buying every liability of a firm buys you nothing, and buying the
     * residual buys you the firm. So what a bidder looks for is a claim ON somebody that its issuer
     * does not owe — which the register and the kind profile already say, with nothing here
     * branching on a kind id (Law 15).
     */
    if (!i.status.live || !i.issuer.some) continue;
    if (ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    const target = i.issuer.value;
    if (target === view.self.id) continue;
    const worth = worthToBuyer(view, ctx, target, i.id);
    if (!worth.some) continue;
    const outstanding = ctx.register.heldTotal(i.id).value;
    if (outstanding <= 0) continue;
    // A1: control is more than half of what exists, read off the register rather than declared.
    const needs = downTick(div(outstanding, 2, 'more than half of what exists'));
    if (needs <= 0) continue;
    // B2: and it does not bid for what it cannot pay for. A bid it could not honour is not a bid.
    const would = mul(needs, worth.value, 'what control would cost it at its own number');
    if (would > cash) continue;
    out.push({ buyer: view.self.id, target, line: i.id, price: worth.value, needs, ccy });
  }
  return out;
}

export function control(): SystemModule {
  return {
    id: 'control',
    spec: 'M&A',
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
        cycle: 'anchor',
        run: (ctx: MechanismContext): void => {
          for (const p of ctx.parties.ofKind(FIRM)) {
            if (!p.status.alive) continue;
            for (const bid of controlBidsFor(ctx.participant(p.id), ctx)) runTender(ctx, bid);
          }
        },
      },
    ],
    participants: [],
    families: [deals()],
  };
}

/** A1: what control of a line costs at a price — a read for the observer and for a test. */
export const costOfControl = (outstanding: number, price: number): number =>
  mul(atMost(outstanding, outstanding, 'all of it'), price, 'what all of it would cost');

/** B2.a: the premium, as the distance between two numbers rather than a percentage anybody set. */
export const premiumOver = (paid: number, printed: number): number =>
  sub(paid, printed, 'what control was worth above what a share was trading at');
