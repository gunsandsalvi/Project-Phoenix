/**
 * Where a household puts what it does not spend.
 *
 * @spec Households C2 Households D1 Households D1.a Households D5 Households D5.a Households D6 Equity B1 Equity B3 Equity B6 Equity C2 Equity C2.a Expectations B3 Sovereign E2.f Money A1 XI-13 XI-15 Law 8
 *
 * D5.a: the choice between a deposit and paper bought directly is a real substitution, and it is
 * how a rate reaches a saver. A deposit is a holding of a bank's money — it returns nothing at all
 * here, because paying for deposits is a decision a bank has not been given yet (Banks Funding B1,
 * worklist 11) — so what a household gives up by holding paper instead is access to its money, and
 * what it wants for giving that up is its own liquidity preference. Nothing else enters: it is not
 * offered fund shares pro rata, and it is nobody's residual holder (D6).
 *
 * It bids at the price its OWN required yield gives, never at the print: the price it will pay is
 * the one at which the paper returns what it wants, so a line that is dear to it does not fill and
 * a line that is cheap does. What it does not put into paper stays in its account, which is what
 * saving into a deposit is (C2).
 *
 * It only considers paper that COMES BACK inside its own horizon. Anything longer it would have to
 * sell before maturity at a price nobody can tell it, and what that is worth is the other two
 * reasons D5 names — yield against risk — which a household cannot weigh until something in this
 * world prices risk (worklist 9). Liquidity is the reason it has now, and this is the whole of it.
 */
import { nextPeriod, period } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { instrumentId, type InstrumentId, type MarketId, type PartyId, type VenueId } from '../../core/ids.js';
import type { Event } from '../../journal/journal.js';
import { add, atMost, div, material, mul, sub, sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import type { Instrument } from '../../register/instruments.js';
import type { ParticipantView } from '../../world/context.js';
import { levelsBelow, rungsOver } from './demand.js';
import type { Qty } from '../../core/tick.js';

/** A bid for paper: a size at the level this cell's own requirement puts on it. */
export interface PaperBid {
  readonly market: MarketId;
  readonly instrument: InstrumentId;
  readonly price: number;
  readonly qty: number;
}

/** A line this cell would hold, and what it thinks one unit of it is worth. */
export interface SavingLine {
  readonly instrument: Instrument;
  readonly price: number;
  /**
   * Indices C2, Fund Shares A4, D5 (13d): whether this line is a CLAIM ON A BOOK rather than on an
   * issuer — a vehicle that holds the market instead of a company that is part of it. It is the
   * kind's own declared pricing answering (`pricing: 'derived'`), never a list of fund ids, and it
   * is what tells retail money that would rather own the market from a name it picked.
   */
  readonly tracks: boolean;
}

/**
 * D5, D5.a, §46 B1, B3, B6 (13d): everywhere this cell's savings could go, in one pass over the
 * lines that exist, with what IT thinks each is worth.
 *
 * WHAT A HOUSEHOLD KNOWS HOW TO DO IS WATCH A PRICE. This function used to run three different
 * epistemologies in one loop: it discounted a bond's own cash flows on the curve family's declared
 * day count, it capitalised a company's published earnings per share over its book value, and it
 * marked a fund at its book. In the same period, for the same cell, that is a securities analyst —
 * and the engine differentiates parties by what they may SEE, what they HOLD, what they OWE and
 * what they PREFER, and in no way at all by what they are able to WORK OUT. One analytical
 * technology distributed to everybody is the representative agent one level up.
 *
 * Three things followed from it, and the third is load-bearing:
 *
 *  - XI-16's disagreement was doing half its work. Parties disagreed only because they had observed
 *    different things at speeds drawn apart, and adaptive learning from a common set of prints
 *    converges. A household that EXTRAPOLATES and a desk that discounts cash flows disagree
 *    structurally and permanently, and hardest right after a large move (§46 A3).
 *  - There was no unsophisticated money. A dealing desk charges for one-sided flow because it was
 *    "facing somebody who knew more"; in a world where every counterparty knows as much as the
 *    desk, that term measures noise and market-making has no customers.
 *  - And it was two representations of one thing (Law 4): what a claim is worth to a saver was
 *    computed here AND printed by the market, and the two never had to agree.
 *
 * SO IT IS THE SAME LADDER A LOAF IS BOUGHT ON (`consume.ts`): its own outlook of that line's price
 * where it has one, the last print where it has not, and NOTHING where the line has never printed
 * — a cell that has never seen a price for a thing cannot name one, and that is the absence of a
 * reason rather than a floor (B6). Two cells with different memories therefore bid different levels
 * for the same line, which is what gives the book two sides.
 *
 * WHAT IS STILL TWO KINDS IS THE HORIZON, and that is a fact about the instrument rather than an
 * opinion about it: paper promises dated payments and a cell will not tie its money up past its own
 * horizon (D5), so a line that does not come back inside it is not somewhere its money can go.
 * Nothing is discounted to find that out — the question is WHEN, and the calendar answers it.
 */
export function savingLines(
  view: ParticipantView,
  horizonPeriods: number,
): { readonly paper: SavingLine[]; readonly shares: SavingLine[] } {
  const region = view.self.region;
  const ccy = view.registry.currencyOf(region);
  const on = view.calendar.startOf(view.period);
  // Money G3.a: a horizon is a DATE the calendar places, never a count of periods turned into years.
  const by = view.calendar.startOf(period(view.period + horizonPeriods));
  const paper: SavingLine[] = [];
  const shares: SavingLine[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || !i.market.some || i.ccy !== ccy || !i.issuer.some) continue;
    const profile = view.registry.instrumentKind(i.kind);
    if (profile.physical === true) continue;
    /**
     * §46 B1, B3: WHAT THIS CELL THINKS THE LINE IS WORTH, and it is the only valuation in here.
     * Its own outlook where it has formed one — which it forms from what it has actually paid, at
     * its own speed — and the last print where it has not, because a print is public (Clearing E1)
     * and a saver reading the tape is not a saver reading accounts. A line neither of those answers
     * for is one this cell posts nothing in.
     */
    const outlook = view.outlook(`price.${String(i.id)}`);
    const print = view.print(i.id);
    const expected = outlook.some
      ? outlook.value.expected
      : print.some
        ? print.value.price
        : undefined;
    if (expected === undefined || expected <= 0) continue;
    /**
     * §46 B3, Equity B3: AND WHAT IT WILL PAY IS THE BOTTOM OF THE RANGE IT THINKS THE PRICE COULD
     * BE IN — its own expectation less how wrong it has recently been. It is the same widening
     * `consume.ts` posts a loaf's demand curve across, used here in one direction rather than two,
     * because a saver buying a claim that promises it nothing wants the margin on its side.
     *
     * It is also where the disagreement lives (§46 A3, XI-13). Two cells looking at one print want
     * different prices for it because one of them has been surprised and the other has not, and a
     * book whose two sides agreed on a number would not be a market.
     */
    const price = sub(expected, outlook.some ? outlook.value.confidence : 0, 'what it will pay');
    if (price <= 0) continue;
    const flows = profile.cashFlows(i, on, view.calendar);
    const last = flows[flows.length - 1];
    if (last !== undefined) {
      // D5: it promises dated payments, so the question is whether the money comes back in time.
      if (compareCivil(last.date, by) > 0) continue;
      paper.push({ instrument: i, price, tracks: profile.pricing === 'derived' });
      continue;
    }
    shares.push({ instrument: i, price, tracks: profile.pricing === 'derived' });
  }
  return { paper, shares };
}


/**
 * D5, D5.a: the bids for paper, one per line, for the money the cell decided this line gets.
 * `perLine` is its own budget divided by every place its money could go, so the same money is never
 * committed twice and no class of thing is preferred by a rule nobody stated.
 */
export function paperBids(
  view: ParticipantView,
  eligible: readonly SavingLine[],
  /**
   * XI-15: what ONE MEMBER puts into one line. The cell's order is that, times how many there are.
   * It is asked PER LINE (13d) because a saver that would rather own the market than pick names
   * puts a different amount behind the two, and which of them a line is, is the line's own answer.
   */
  budgetFor: (line: SavingLine) => number,
  lines: number,
  weight: number,
): PaperBid[] {
  const out: PaperBid[] = [];
  for (const e of eligible) {
    const perLine = budgetFor(e);
    if (perLine <= 0) continue;
    // Bond N9.b: what it must find is the clean price plus what has accrued and travels with it.
    const dirty = add(e.price, view.accrued(e.instrument.id), 'what a unit costs it');
    // Law 8, XI-15: EVERY MEMBER holds whole units, so what one of them bids for is a whole number
    // of them and the cell posts that many for each of the members it stands for. A cell bidding
    // for 108,739,763,087.57 units is one bidding for a fraction of a unit apiece, which is not a
    // unit and not a bid.
    const perMember = downTick(div(perLine, dirty, 'units one member bids for'));
    const qty = mul(perMember, weight, 'what the cell bids for');
    // Law 7: this line's share against what the whole budget would have bought — a share that
    // small is the rounding of the split, not a bid.
    const whole = div(mul(perLine, lines, 'the whole of it'), dirty, 'what it would buy');
    if (perMember <= 0 || !material(qty, lines + 1, mul(whole, weight, 'the cell')) || !e.instrument.market.some) continue;
    out.push({ market: e.instrument.market.value, instrument: e.instrument.id, price: e.price, qty });
  }
  return out;
}

/**
 * §46 B3: the extra this cell wants for holding a claim that promises nothing — how wide its own
 * income surprises have been against what it expects to be paid, as a rate. A read of its own
 * history and its own outlook, in its own units, with nothing stated anywhere: a cell that has
 * never been surprised wants nothing extra, and one whose income has been all over the place wants
 * a great deal, which is why the same firm is worth different amounts to two of them.
 */
export function ownUncertainty(view: ParticipantView): number {
  const income = view.outlook('income');
  if (!income.some || income.value.expected <= 0) return 0;
  const year = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(nextPeriod(view.period)),
  );
  if (year <= 0) return 0;
  return div(
    div(income.value.confidence, income.value.expected, 'how wrong its income has been'),
    year,
    'per annum',
  );
}

/** A bid or an offer in a share line, at the level this cell's own opinion puts on it. */
export interface ShareOrder {
  readonly market: MarketId;
  readonly instrument: InstrumentId;
  readonly side: 'buy' | 'sell';
  /** 'market' only when it is selling because it needs the money (XI-2). */
  readonly price: number | 'market';
  readonly qty: number;
}

/**
 * Equity B1, B6, Clearing A2, A2.a: the opinion as a schedule — ONE LINE, ONE SIDE.
 *
 * A cell is never on both sides of one book. A bid and an ask from the same party in the same
 * session is a party trading with itself, and the price that came out of it would be a wash: the
 * book would print a trade that moved nothing between two balance sheets (Law 5, Observer B4).
 *
 * Which side it is on is its own circumstance and not a rule about shares:
 *
 * - It has money to put into this line, so it is a BUYER of it. Its curve tops out at its own
 *   opinion — the most the claim is worth to it — and runs down from there, because a claim that
 *   gets cheaper is one it wants more of (demand.ts). What it already holds it is holding; what it
 *   would take for that is not a question it is asking in a period when it is buying.
 * - It has nothing to put in, so the only thing it can do in this line is let go of what it has:
 *   it OFFERS its holding at what it thinks the holding is worth, and a seller with no buyer above
 *   that keeps its shares (B6).
 * - It needs its money back, so it offers at whatever the market gives, which is what a forced
 *   seller posts and never a price it named (XI-2).
 */
export function shareOrders(
  view: ParticipantView,
  lines: readonly SavingLine[],
  budgetFor: (line: SavingLine) => number,
  short: number,
  steps: number,
): ShareOrder[] {
  const out: ShareOrder[] = [];
  const weight = view.self.representation === 'cell' ? view.self.weight : 1;
  for (const line of lines) {
    const perLine = budgetFor(line);
    if (!line.instrument.market.some) continue;
    const market = line.instrument.market.value;
    const id = line.instrument.id;
    const units = mul(view.free(id), weight, 'shares it could sell');
    if (short > 0 || perLine <= 0 || steps < 1) {
      if (!material(units, 2, units)) continue;
      const price = short > 0 ? ('market' as const) : line.price;
      out.push({ market, instrument: id, side: 'sell', price, qty: units });
      continue;
    }
    // XI-15: the rungs are ONE MEMBER's, in whole shares, and the cell posts that many apiece.
    //
    // Law 8: AND NEVER MORE OF A LINE THAN THERE IS. What a buyer's money reaches at a price is
    // `budget / price`, which is the curve and is right; how many of the thing exist is a different
    // fact, and it is the one that makes a bid a bid. It is the mirror of the seller's "never more
    // than it holds" — the arithmetic of what there is to deliver, not a limit on what anybody
    // wants — and without it a line whose price has collapsed has every saver bidding for many
    // times the whole company, and the demand at a level stops being an exact count at all
    // (`asQty` threw at 1.5e16 in a year-long run; worklist 12c is why the price collapsed).
    const exist = line.instrument.issued;
    for (const rung of rungsOver(levelsBelow(line.price, steps), perLine)) {
      const wanted = mul(rung.qty, weight, 'what the cell puts in');
      const qty = downTick(atMost(wanted, exist, 'there are no more units of it than were issued'));
      if (qty <= 0) continue;
      out.push({ market, instrument: id, side: 'buy', price: rung.price, qty });
    }
  }
  return out;
}

/**
 * D2, D5, D5.a: the third thing a saver can do with its money. A deposit returns nothing; a bill
 * returns what a bill returns and ties the money up; a MONEY FUND is a claim on short paper that
 * can be asked for back at any time, and what it returns is what the fund published it returned.
 *
 * The substitution is a consequence, not an allocation (D5): the cell puts new savings into the
 * fund only when what the fund actually returned clears what it requires of anything it holds
 * instead of money, and it asks for its money back when its own cushion is short — which is the
 * real reason anybody redeems, and the reason a shock to incomes becomes a redemption wave.
 */
export interface FundOrder {
  readonly venue: VenueId;
  readonly side: 'buy' | 'sell';
  /** Per member of the cell (XI-15); the posting carries the cell's weight. */
  /** XI-15, Law 8: whole shares for each member of the cell. A member cannot hold part of one. */
  readonly sharesPerMember: Qty;
}

/** What a cell holds in one fund, and what the fund last said a share of it is worth. */
export interface FundPosition {
  readonly venue: VenueId;
  readonly fund: string;
  readonly line: InstrumentId;
  readonly perShare: number;
  /** D2.a: what it offers a saver, after its manager. This is what competes with a deposit. */
  readonly offered: number;
  /** XI-15, Law 8: whole shares for each member. A member cannot redeem part of one. */
  readonly sharesPerMember: Qty;
  readonly worthPerMember: number;
}

/**
 * D2, D5: what this cell has in the funds it can reach, at what each of them last published. The
 * venue says which line its shares are and the fund publishes what a share is worth (B1), so a
 * saver reads two public facts and nothing private (Observer A3).
 */
export function fundPositions(
  venues: readonly VenueDecl[],
  struck: readonly Event[],
  view: ParticipantView,
): FundPosition[] {
  const out: FundPosition[] = [];
  for (const v of venues) {
    if (v.key['kind'] !== 'fund') continue;
    const fund = v.key['fund'];
    // The venue says which line its shares are, because the venue is public data about itself and
    // a household has no business knowing how another module names things (Law 15).
    const line = v.key['share'];
    if (fund === undefined || line === undefined) continue;
    const last = struck.filter((e) => e.data['fund'] === fund).pop();
    if (last === undefined) continue;
    const perShare = last.data['perShare'];
    const offered = last.data['offered'];
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    const held = view.quantity(instrumentId(line));
    out.push({
      venue: v.id,
      fund,
      line: instrumentId(line),
      perShare,
      offered: typeof offered === 'number' ? offered : 0,
      sharesPerMember: held,
      worthPerMember: mul(held, perShare, 'what its shares are worth'),
    });
  }
  return out;
}

/**
 * D2, D5, D5.a: what a saver does with a money fund. It is where its CUSHION lives, because a fund
 * of short paper is a substitute for a deposit that pays it something (D2) — so what it holds over
 * what it is about to spend goes in, and what it is short of what it is about to spend comes back
 * out. Nothing here is an allocation: the flow is a consequence of the cell's own budget, and it
 * stops entirely the moment what the fund offers stops clearing what the cell requires (D5).
 */
export function fundOrders(
  positions: readonly FundPosition[],
  required: number,
  toFund: number,
  short: number,
): FundOrder[] {
  const out: FundOrder[] = [];
  for (const p of positions) {
    // C2: it asks for its money back because it needs it. Nothing else in a household's life is a
    // reason to redeem, and a rule that redeemed on a bad return would be a run written into it.
    if (short > 0) {
      if (p.sharesPerMember <= 0) continue;
      // Law 8: A SHARE IS INDIVISIBLE, so what a member hands back is a whole number of them, and
      // it is the number DOWN — a member short of less than one share's worth asks for nothing and
      // stays short, which is a real state and is what an indivisible claim does to somebody who
      // needs a little of it. Asking for the share above would be redeeming a hundred times the
      // need on every small shortfall in the population, which is the cell grain deciding the
      // aggregate (XI-15) rather than anybody's decision.
      const want = downTick(div(short, p.perShare, 'shares it must give back'));
      out.push({
        venue: p.venue,
        side: 'sell',
        sharesPerMember: atMost(want, p.sharesPerMember, 'it cannot hand back more than the position it holds'),
      });
      continue;
    }
    // D2.a: and it puts money in when what the fund offers clears what it wants for giving up its
    // money — the competition D2 names, against a deposit that pays it nothing. Whole shares again,
    // and DOWN this time: what its money buys, never a share it cannot pay for.
    if (toFund <= 0 || p.offered < required) continue;
    const buying = downTick(div(toFund, p.perShare, 'shares it asks for'));
    if (buying <= 0) continue;
    out.push({ venue: p.venue, side: 'buy', sharesPerMember: buying });
  }
  return out;
}

/** C2: how much of what it is about to spend its account cannot cover, per member (Law 7 as above). */
export function shortForSpending(cash: number, spend: number): number {
  const gap = spend - cash;
  const scale = sum([cash, spend]);
  return gap > 0 && material(gap, scale.terms, scale.value) ? gap : 0;
}

/**
 * D2: the part of its money that belongs in a fund rather than in its account — what it holds over
 * what it is about to spend, up to the cushion it wants. Above the cushion it would rather have
 * paper (D5), and below what it is about to spend there is nothing to put anywhere.
 */
export function cushionForFund(cash: number, spend: number, spare: number): number {
  const over = cash - spend - spare;
  const scale = sum([cash, spend, spare]);
  return over > 0 && material(over, scale.terms, scale.value) ? over : 0;
}

/**
 * C2, D1: what a cell has left over once it has spent and kept its cushion, per member.
 *
 * Law 7: a residue that is the rounding of the subtraction is not money it has over. A cell that
 * ends up with 1e-310 "spare" would post an order for 1e-313 units, and a quantity that small
 * cannot be a cell's per-member share of anything: multiplied back by the weight it does not give
 * the total again, and the wire refuses it (XI-15). The dust is the magnitudes it came out of.
 */
export function sparePerMember(cash: number, spend: number, buffer: number): number {
  const left = cash - spend - buffer;
  const scale = sum([cash, spend, buffer]);
  return left > 0 && material(left, scale.terms, scale.value) ? left : 0;
}

/** The total a cell of this weight commits, from a per-member decision (XI-15). */
export function totalOf(perMember: number, weight: number): number {
  return mul(perMember, weight, 'what the cell commits');
}

export type { PartyId };
