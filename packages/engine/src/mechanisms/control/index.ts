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
import { atMost, sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide } from '../../ledger/settlement.js';
import type { Instrument } from '../../register/instruments.js';
import { asQty } from '../../core/tick.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { FIRM } from '../../registry/profiles.js';
import { weightOf } from '../../parties/party.js';
import { yearFraction } from '../../calendar/daycount.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { about } from '../../world/context.js';

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
  return worthAt(buyer, ctx, target, line, quotedTo(ctx, buyer.self.id));
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
  if (said === undefined || said.earned <= 0 || said.periods <= 0) return none<PerPiece>();
  // Law 8: the periodicity is part of the number. What it published covers a span of periods; what
  // a required return is quoted in is a year, so the two are put in the same unit by the calendar
  // rather than by a factor typed here.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + said.periods)),
  );
  if (ofAYear <= 0) return none<PerPiece>();
  const annual = over(said.earned, asRatio(ofAYear, 'the fraction of a year that was'), 'what it earns a year, as it published it');
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
 * Law 19: what a bank quoted THIS name, per annum. Its own cost of money, read off the wire.
 *
 * The journal already keeps the last event of a kind a party is a subject of, so this asks it
 * rather than walking every quote the world has ever published back to the beginning.
 */
function quotedTo(ctx: MechanismContext, who: PartyId): Option<Ratio> {
  const e = ctx.journal.lastOf('credit.quoted', who);
  if (e === undefined) return none<Ratio>();
  const rate = e.data['rate'];
  // Item 16: what a bank quoted it re-enters here — a rate per annum on what it would borrow.
  return typeof rate === 'number' ? some(asRatio(rate, 'what a bank quoted it')) : none<Ratio>();
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
export function tenders(
  holder: ParticipantView,
  line: InstrumentId,
): readonly Order[] {
  const units = downTick(holder.free(line));
  if (units <= 0) return [];
  const own = holder.outlook(about({ on: 'price', instrument: line }));
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
  fills: readonly { readonly party: PartyId; readonly side: string; readonly qty: Qty }[],
  price: PerPiece,
): void {
  const paid: Qty[] = [];
  let bought = NO_QTY;
  for (const f of fills) {
    if (f.side !== 'sell') continue;
    const units = downTick(f.qty);
    if (units <= 0) continue;
    const cash = downTick(valueAt(price, units, 'what it pays for them'));
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
    bought = addQty(bought, units, 'bought');
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
   * A5, D4: AND WHEN NOTHING IS LEFT OUTSIDE, the two balance sheets combine. A majority makes a
   * subsidiary — which is what the world above records — and it is holding ALL of it that makes the
   * residual claim entirely the acquirer's, so that combining is a description of what is true
   * rather than a decision somebody has to take. That is why `combine` needed no new caller and no
   * new rule: it needed the one condition under which it is not a lie.
   */
  if (held >= inIssue) combine(ctx, bid.buyer, bid.target);
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
      fromCell: none(),
      toCell: none(),
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
      fromCell: none(),
      toCell: none(),
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

export function controlBidsFor(
  view: ParticipantView,
  ctx: MechanismContext,
  lines: readonly Instrument[],
): readonly Bid[] {
  const ccy = view.registry.currencyOf(view.self.region);
  const cash = view.cash(ccy);
  if (cash <= 0) return [];
  // B1: what this buyer's own money costs it, which is the same number for every company on the
  // list, and a firm nobody will lend to does not look at the list at all.
  const required = quotedTo(ctx, view.self.id);
  if (!required.some || required.value <= 0) return [];
  const out: Bid[] = [];
  for (const i of lines) {
    const target = i.issuer.some ? i.issuer.value : view.self.id;
    if (target === view.self.id) continue;
    const worth = worthAt(view, ctx, target, i.id, required);
    if (!worth.some) continue;
    const outstanding = ctx.register.heldTotal(i.id).value;
    if (outstanding <= 0) continue;
    // A1: control is more than half of what exists, read off the register rather than declared.
    const needs = downTick(scale(outstanding, asRatio(1 / 2, 'more than half of what exists'), 'more than half of what exists'));
    if (needs <= 0) continue;
    // B2: and it does not bid for what it cannot pay for. A bid it could not honour is not a bid.
    const would = valueAt(worth.value, needs, 'what control would cost it at its own number');
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
          const lines = equityLines(ctx);
          if (lines.length === 0) return;
          for (const p of ctx.parties.ofKind(FIRM)) {
            if (!p.status.alive) continue;
            for (const bid of controlBidsFor(ctx.participant(p.id), ctx, lines)) runTender(ctx, bid);
          }
        },
      },
    ],
    participants: [],
    families: [deals()],
  };
}

/** A1: what control of a line costs at a price — a read for the observer and for a test. */
export const costOfControl = (outstanding: Qty, price: PerPiece): Cash =>
  valueAt(price, atMost(outstanding, outstanding, 'all of it'), 'what all of it would cost');

/** B2.a: the premium, as the distance between two numbers rather than a percentage anybody set. */
export const premiumOver = (paid: PerPiece, printed: PerPiece): PerPiece =>
  minus(paid, printed, 'what control was worth above what a share was trading at');
