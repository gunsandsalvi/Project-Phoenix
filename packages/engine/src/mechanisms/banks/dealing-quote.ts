/**
 * How a desk prices: the two sides come out of its own economics, and the spread is what is left.
 *
 * @spec Dealer Desks B1 Dealer Desks B2 Dealer Desks B3 Dealer Desks B4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C3 Dealer Desks C4 Dealer Desks C5 Dealer Desks C5.a Dealer Desks C5.b Dealer Desks D1 Dealer Desks D3 Dealer Desks D4 Dealer Desks D4.a Clearing A2 Clearing A3 Clearing A4 Clearing B3 Clearing B3.a Clearing E3 Expectations B3 XI-4 XI-13 Law 2 Law 8
 *
 * THERE IS NO WIDTH IN THIS FILE. C5.a and C5.b forbid a spread applied to a mid and a stated width
 * per book, per instrument or per client, and the way to make that impossible is to have no number
 * to state: the only declared numbers a desk reads are its own limit, the return it needs on its
 * capital and the capital charge. Everything else in the two prices below is a READ of the desk's
 * own position and its own history, and the lint that refuses an undeclared literal is what keeps
 * it that way.
 *
 * THE TWO SIDES ARE DERIVED SEPARATELY and neither is a mid. What a desk wants to give up a unit is
 * its own view of the unit PLUS what holding one has to earn it; what it will pay to take one on is
 * its own view MINUS what holding one will cost it. Those are two different questions with two
 * different answers, and the bid-offer is their difference, which is C5's "therefore the output".
 *
 * WHAT ONE MORE UNIT COSTS IT (D3, XI-4 joint three) is the whole mechanism. A position consumes
 * cash and capital and is charged for both EVERY PERIOD IT IS HELD, so what a desk needs on one
 * more unit is what one period of that comes to — and it needs it again next period, and the period
 * after, which is what makes a position it has not shed cost it more than one it has. A desk that
 * carries inventory for free has no reason to shed it, its quotes never skew, and order flow stops
 * moving prices.
 *
 * IT DOES NOT GUESS HOW LONG IT WILL HOLD. The first build of this charged for the carry over the
 * desk's own expected holding period, read off its own turnover — and a desk that had sold a line
 * once, in small size, expected to hold the next unit for fifty thousand periods and offered a bill
 * worth one at nearly three. A market order took it, the print stood, and the curve derived a yield
 * that does not exist. What a desk knows it will pay is this period's charge; the rest is the skew.
 *
 * WHY IT SKEWS (C2, C2.a). The further into its limit a desk already is, the more the next unit
 * costs it, so both sides come down together: long, it bids lower AND offers lower, because it
 * wants to sell. That is how a book mean-reverts with nobody telling it to, and it is why order
 * flow moves prices.
 *
 * WHY IT WIDENS (C3, C4). Risk is the width of the desk's OWN recent surprises about this line
 * (Expectations B3) — a read, in the instrument's own money. Adverse selection is how one-sided the
 * flow it faced was: a desk that only ever got hit on one side was facing somebody who knew more,
 * and what that is worth is its own uncertainty again, on the share of the flow that went one way.
 *
 * B4: none of this reads the book. The desk's schedule is a function of its own state and its own
 * history, so it posts the same two prices whether the market has a hundred orders in it or none —
 * which is what stops it being the buyer of last resort with a different name.
 */
import { yearFraction } from '../../calendar/daycount.js';
import { nextPeriod, type Calendar, type Period } from '../../calendar/calendar.js';
import type { InstrumentId } from '../../core/ids.js';
import { add, div, material, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import { priceAtYield, requiredYieldOf } from './treasury.js';
import { downTick } from '../../core/tick.js';
import { NO_QTY, type Qty } from '../../core/tick.js';

/** Law 8: a rate is per annum or it is not a rate; this is the convention these reads use. */
const DAY_COUNT = 'ACT/365F' as const;

/** What a desk will do in one line this period, and what each half of it is made of (D5). */
export interface DeskQuote {
  readonly instrument: InstrumentId;
  /** C1: its own view of what a unit is worth, before its own position is taken into account. */
  readonly view: number;
  /** What one more unit costs it to carry, be wrong about, and face informed flow in. */
  readonly edge: number;
  /** C2: how far both sides are pushed down by what it is already holding. */
  readonly skew: number;
  readonly bid: number;
  readonly offer: number;
  /** D1: what room it has left, and what it holds. Either can be nothing (D4: it stops). */
  /** Law 8: WHOLE PIECES of the line, on both sides. A quote for four tenths of a share is a
   * quote nobody can hit, and it used to be possible because a size is money over a price. */
  readonly bidSize: Qty;
  readonly offerSize: Qty;
  /** D4, D5: which of its limits shrank the bid, so a desk stepping back says which one did it. */
  readonly binds: QuoteBinding;
}

/** The desk's own numbers, read from its own view before it prices anything. */
export interface DeskState {
  /**
   * C2.a: where its own treasury wants this line held, in the bank's own money. Zero for a line
   * nobody holds for liquidity, which is the ordinary case and is what "flat" used to mean.
   */
  targetIn(instrument: InstrumentId): number;
  /** D1: the most its whole book may be worth. */
  readonly limitAggregate: number;
  /**
   * D1: the most of that book it will have in ONE line, as a share of the whole. Law 8: the units
   * a limit binds in are the line's own, and they fall out of this share and its own view of the
   * line rather than being restated per line in pieces that mean something different in each.
   */
  readonly concentration: number;
  /** D3, D2: what a unit of value costs it to carry for one period — funding plus capital charge. */
  readonly ratePerPeriod: number;
  /** What its whole book is worth at the last marks, so the aggregate limit is a real constraint. */
  readonly bookValue: number;
  /** F1: the money it actually has. A desk cannot pay for what it bid with money it has not got. */
  readonly cash: number;
  /** How many lines it is quoting this period, so its money is spread over them and no further. */
  readonly linesQuoted: number;
}

/** Which of a desk's three real constraints bound its bid (D4: it shrinks, and this says why). */
export type QuoteBinding = 'position' | 'book' | 'money' | 'none';

/** The smaller of two quantities: arithmetic, and the smaller one is the constraint that binds. */
const least = (a: number, b: number): number => (a < b ? a : b);

/**
 * C1, XI-13: the desk's own view of what a unit is worth.
 *
 * Its OWN outlook of the price where it has one — formed from what it actually traded at, corrected
 * at its own speed (Expectations B1) — and what its own book CARRIES a unit at where it has none,
 * because a desk that has never traded a line has exactly one thing to go on and that is it. For
 * almost everything the carrying value is the last print, so this is the last print; for a claim on
 * a book it is what the book comes to (Fund Shares B1), which is what a market maker in one quotes
 * around and is not derived from the price this session is about to discover (XI-13, Clearing A4).
 *
 * AND FOR A LINE NOBODY HAS EVER PRICED there is neither. For a share that is the end of it — a
 * desk that does not know what a thing is worth does not make a market in it (D4). But a DATED
 * CLAIM ON A NAME is worth what its own payments are worth to whoever is asked, and this bank has
 * already published what it requires of that name (Corporate Credit E5): its own cash flows at its
 * own reservation yield. That is not a price it read anywhere — it is the answer it gives when a
 * new line is brought and somebody asks what it will pay for a thing that has never traded
 * (Sovereign C3.b). It is the LAST resort and not the first, because a reservation is what a holder
 * would need to buy and keep, and a market maker quotes around where the market is.
 */
function viewOf(view: ParticipantView, instrument: InstrumentId): Option<number> {
  // XI-13, AND IT IS THE WHOLE OF WHY THIS ORDER IS WHAT IT IS. A DATED CLAIM is worth what its own
  // payments are worth at what this bank requires of the name (Corporate Credit E5), and that is
  // asked FIRST — because it is the one input to this quote that does not read the print.
  //
  // It used to be the last resort, reached only when the desk had no outlook and no mark, and what
  // stood in front of it was `outlook('price.<instrument>')`: the desk's own adaptive expectation
  // OF THE PRICE, formed from the prints. That is XI-13's fixed point written out — the print moves
  // the outlook, the outlook moves the view, the view moves the quote, the quote moves the print —
  // and in the one market this model funds itself through it walked a bill that pays 1.00 in seven
  // days to a print of 3.3337, at which no representable yield discounts its own payments to what
  // somebody paid, so the curve threw and the world stopped. The comment at the top of this file
  // records the same symptom from an earlier build and a different cause; this is the cause that
  // survived it.
  //
  // A reservation is still not a price and this is still not a mid: it is one of the two inputs the
  // quote is built from, and the position skew below is what moves the two sides with flow. What is
  // gone is a desk whose only anchor was where the market last was.
  const issuer = view.instruments.get(instrument).issuer;
  if (issuer.some) {
    const required = requiredYieldOf(view, issuer.value);
    if (required.some) {
      const worth = priceAtYield(view, instrument, required.value);
      if (worth.some && worth.value > 0) return worth;
    }
  }
  // A CLAIM THAT PROMISES NOTHING has no such anchor — there are no payments to discount (Equity
  // A4) — so for a share the desk's own outlook, and then what it carries one at, is all there is.
  // `priceAtYield` answers none for it, which is what brings the flow here.
  const own = view.outlook(`price.${instrument}`);
  if (own.some && own.value.expected > 0) return some(own.value.expected);
  const carried = view.mark(instrument);
  return carried.some && carried.value > 0 ? carried : none();
}

/**
 * C3: how wrong this desk's own view of this line has recently been, in the line's own money. A
 * read of its own surprises (Expectations B3) and never a stated volatility. A desk that has never
 * been surprised about a line is not thereby certain of it — it has no history — so what it charges
 * for risk is nothing extra and what it charges for CARRY is still there.
 */
function riskOf(view: ParticipantView, instrument: InstrumentId): number {
  const own = view.outlook(`price.${instrument}`);
  return own.some ? own.value.confidence : 0;
}

/**
 * C4: how one-sided the flow it faced was, as a share of the flow. A desk sees its OWN fills and
 * nothing else (Expectations A2, Observer A4): what it bought and what it sold. All one way means
 * whoever it faced knew something, and what that is worth to it is its own uncertainty on that
 * share of the flow. Two-way flow costs it nothing — which is B1's reason for quoting at all.
 */
function adverseOf(view: ParticipantView, instrument: InstrumentId, risk: number): number {
  const bought = view.outlook(`bought.${instrument}`);
  const sold = view.outlook(`sold.${instrument}`);
  const b = bought.some && bought.value.expected > 0 ? bought.value.expected : 0;
  const s = sold.some && sold.value.expected > 0 ? sold.value.expected : 0;
  const both = add(b, s, 'the flow it faced');
  if (both <= 0) return 0;
  return mul(div(Math.abs(sub(b, s, 'how one-sided it was')), both, 'the share of it'), risk, 'adverse selection');
}

/**
 * C1-C5: the quote. None when the desk has no view of the line at all — a desk that does not know
 * what a thing is worth does not make a market in it, which is a legitimate state (D4) and not a
 * gap to fill with somebody else's number.
 */
export function quoteFor(
  view: ParticipantView,
  instrument: InstrumentId,
  state: DeskState,
): Option<DeskQuote> {
  const value = viewOf(view, instrument);
  if (!value.some) return none();
  const mine = value.value;
  const limitPerInstrument = div(
    mul(state.limitAggregate, state.concentration, 'the most of the book in one line'),
    mine,
    'units of this line that comes to',
  );
  const inventory = view.free(instrument);
  // C2.a: where its own treasury wants the line, in the same pieces the inventory is counted in.
  const target = div(state.targetIn(instrument), mine, 'units its treasury wants held');
  const risk = riskOf(view, instrument);
  const adverse = adverseOf(view, instrument, risk);
  // D3, D2: what carrying one more unit costs it for the period it is quoting in — the money it
  // ties up and the capital it consumes, at the rate its own bank charged it this morning. How
  // long it ends up holding it is not a number it has to guess: it pays this again next period,
  // and the skew below is what a position it has not shed does to what it will pay for the next.
  const carry = mul(mine, state.ratePerPeriod, 'what a unit costs it for a period');
  const edge = add(add(carry, risk, 'what it must earn on a unit'), adverse, 'and for who it faces');
  // C2, C2.a: how far the book is from where it should be, as a share of the room it has. ABOVE
  // the target it bids lower AND offers lower, because it wants to sell; BELOW it, both sides go
  // up, because it wants to buy. That is how a book mean-reverts with nobody telling it to, and
  // with a treasury target on a liquidity line it is also how the treasury sells its portfolio:
  // it moves the target, and its own desk's quote is what the market sees.
  const away = sub(inventory, target, 'how far the book is from where it should be');
  const used = limitPerInstrument > 0
    ? div(away, limitPerInstrument, 'how much of its room that uses')
    : 1;
  const skew = mul(used, edge, 'what its own position does to both sides');
  const bid = sub(sub(mine, edge, 'what it will pay'), skew, 'less what it is already carrying');
  const offer = sub(add(mine, edge, 'what it wants for one'), skew, 'less what it wants to shed');
  const room = sub(limitPerInstrument, away, 'units of room left');
  // D1, D4, F1: three real constraints and the binding one decides, which is what "it shrinks its
  // size" means. A position limit it set itself; the room left in its whole book, so a desk full of
  // one thing stops bidding for everything; and the money it actually has, spread over the lines it
  // is quoting, because a desk that bid its whole account in every book at once would be promising
  // the same money several times over. Law 6: none of these is a bound on a price or on anybody
  // else's behaviour — each is this party deciding how much it will take on, which is a decision.
  const inBook = div(sub(state.limitAggregate, state.bookValue, 'room in the whole book'), mine, 'units');
  const inMoney = state.linesQuoted > 0
    ? div(div(state.cash, state.linesQuoted, 'its money over the lines it quotes'), mine, 'units')
    : 0;
  // Law 8: AND IT IS A WHOLE NUMBER OF PIECES. Every one of the three above is money divided by a
  // price, so every one of them is a fraction of a piece of the line — and a quote for four tenths
  // of a share is a quote nobody can hit. It rounds DOWN because each of the three is what this
  // desk CAN take on, and rounding what somebody can do upwards invents a piece it has not got.
  const size = downTick(least(least(room, inBook), inMoney));
  // D4, D5: the WHOLE BOOK is asked first, because a book with no room in it is why the desk has
  // stopped in every line at once and a per-line limit only ever binds inside a book that has room.
  const binds: QuoteBinding =
    size <= 0 ? (inBook <= 0 ? 'book' : room <= 0 ? 'position' : 'money')
      : size === room ? 'position'
        : size === inBook ? 'book'
          : 'money';
  const canBid = bid > 0 && size > 0 && material(size, 2, limitPerInstrument);
  return some({
    instrument,
    view: mine,
    edge,
    skew,
    bid,
    offer,
    bidSize: canBid ? size : NO_QTY,
    // What it will let go is what it HOLDS, which the register already counts in whole pieces.
    offerSize: material(inventory, 2, inventory) ? inventory : NO_QTY,
    binds: canBid ? 'none' : binds,
  });
}

/**
 * D2, D3: what a unit of value costs a desk to carry for ONE period — what its bank paid for the
 * money, plus the return it needs on the capital the position consumes. Both are real charges and
 * both are paid (rent.ts); this is the rate they are paid at.
 *
 * Law 8: the two inputs are per annum and the answer is per period, so the period is turned into
 * the fraction of a year the calendar says it is — never a periods-per-year anybody stated.
 */
export function rateOf(
  costOfFunds: number,
  capitalRatio: number,
  riskWeight: number,
  requiredReturn: number,
  yearFractionOfPeriod: number,
): number {
  const capital = mul(
    mul(capitalRatio, riskWeight, 'the capital a unit of the book consumes'),
    requiredReturn,
    'what that capital must earn',
  );
  return mul(add(costOfFunds, capital, 'what a unit of the book costs it per annum'), yearFractionOfPeriod, 'this period of it');
}

/** The fraction of a year this period is, off the calendar (Law 8, Money G3.a). */
export function periodOfYear(calendar: Calendar, at: Period): number {
  return yearFraction(DAY_COUNT, calendar.startOf(at), calendar.startOf(nextPeriod(at)));
}
