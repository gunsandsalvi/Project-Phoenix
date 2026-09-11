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
import { add, div, material, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { downTick } from '../../core/tick.js';
import { curveFamilyOf, priceAt } from '../../prices/curve.js';
import type { Instrument } from '../../register/instruments.js';
import { TREASURY } from '../../registry/profiles.js';
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
}

/**
 * D5, D5.a, Equity B3: everywhere this cell's savings could go, in one pass over the lines that
 * exist, with what it thinks each is worth.
 *
 * TWO KINDS OF CLAIM and one traversal, because a line is one or the other by what it promises and
 * the cell has to look at it once to find out (Law 4: one decision, one pass, one budget):
 *
 * - **Paper.** It promises dated payments (Bond N5), so the cell prices it at the level that makes
 *   those payments return what it requires: its liquidity premium, what it wants for giving up
 *   access to its money. It will only tie its money up for its own horizon, so anything that comes
 *   back later is not somewhere its money can go.
 * - **A share.** It promises NOTHING (Equity A4), so nothing about it can be discounted. What it IS
 *   is a claim on the residual (A1) — on what the issuer EARNS, which is what B3 says a saver holds
 *   it for — and since §48 every public company PUBLISHES what it earned, on its own fiscal
 *   calendar, to everybody at once (`reporting.report`). So that is what the cell goes on: a public,
 *   dated fact about the thing it is buying a piece of, capitalised at what it requires of a claim
 *   that promises nothing — its liquidity premium PLUS how wrong its own income has recently been
 *   (§46 B3). Two cells with different histories therefore want different prices for the same firm,
 *   and that disagreement is what gives the book two sides (§46 A3, XI-13).
 *
 *   IT USED TO BE WHAT THE ISSUER HAD PAID (`payout.declared`), and that was the best a saver could
 *   do before there were reports — but it left the book without an anchor, which is worklist 12c's
 *   whole finding: two firms in forty declared a payout, so thirty-eight listed lines had no party
 *   in them with a reason that was not the last print, and `market.noView` said so every period. A
 *   distribution is management's choice about what to do with the residual; the residual is what the
 *   claim IS. One fact, read once, for every listed line (Law 4, Law 19).
 *
 *   AND THE RESIDUAL HAS TWO PARTS, because the balance sheet and the income statement are both in
 *   that report and they answer different halves of one question. What a share IS, is a piece of
 *   what the company owns net of what it owes (A1) — its published `assets - liabilities`, over the
 *   register's count of shares. What holding it EARNS is the residual the company adds to that each
 *   period, which is worth what this cell requires of a claim that promises nothing. So:
 *
 *       worth = the book it is a piece of  +  what it earns a year / what this cell requires
 *
 *   Nothing is discounted, nothing is forecast and no multiple is imposed: both terms are figures
 *   the company itself published, and the only thing that is this cell's own is what it requires,
 *   which differs between cells and is why they disagree (§46 A3). A company LOSING money is worth
 *   less than its book to a saver and the second term is negative — and if it is negative enough the
 *   whole is, and this cell simply names no price. That is the absence of a reason, never a floor
 *   (Law 6): nobody is stopped from bidding, there is just nothing to bid for.
 *
 * This is the "yield against risk" D5 names, and it is the reason the horizon comment above pointed
 * at worklist 9: until something in this world priced risk, a cell had only liquidity to weigh.
 */
export function savingLines(
  view: ParticipantView,
  required: number,
  uncertainty: number,
  horizonPeriods: number,
): { readonly paper: SavingLine[]; readonly shares: SavingLine[] } {
  const region = view.self.region;
  const ccy = view.registry.region(region).ccy;
  const on = view.calendar.startOf(view.period);
  // Money G3.a: a horizon is a DATE the calendar places, never a count of periods turned into years.
  const by = view.calendar.startOf(period(view.period + horizonPeriods));
  const year = yearFraction('ACT/365F', on, view.calendar.startOf(nextPeriod(view.period)));
  const forShares = add(required, uncertainty, 'what it requires of a claim that promises nothing');
  const sovereigns = new Set(
    view.parties.ofKind(TREASURY).filter((t) => t.region === region && t.status.alive).map((t) => t.id),
  );
  const paper: SavingLine[] = [];
  const shares: SavingLine[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || !i.market.some || i.ccy !== ccy || !i.issuer.some) continue;
    const profile = view.registry.instrumentKind(i.kind);
    if (profile.physical === true) continue;
    const flows = profile.cashFlows(i, on, view.calendar);
    const last = flows[flows.length - 1];
    if (last !== undefined) {
      // Paper. It has to be somebody's whose paper this cell can price off a public curve, and it
      // has to come back inside its own horizon (D5).
      if (!sovereigns.has(i.issuer.value)) continue;
      if (compareCivil(last.date, by) > 0) continue;
      const family = view.registry.curveFamily(curveFamilyOf(i.issuer.value, ccy));
      const price = priceAt(flows, required, on, family.dayCount, `what ${i.id} is worth to a saver`);
      if (price > 0) paper.push({ instrument: i, price });
      continue;
    }
    // A share: the book it is a piece of, plus what it earns on that book (A1, B3, §48).
    if (forShares <= 0 || year <= 0 || i.issued <= 0) continue;
    const said = publishedBy(view, i);
    if (!said.some) continue;
    // Law 8: what it earns is per period and what the cell requires is per annum, so the period is
    // turned into the fraction of a year the calendar says it is.
    const onTheBook = div(
      div(said.value.earnedPerShare, year, 'what it earns a share, per annum'),
      forShares,
      'what earning that is worth',
    );
    const value = add(said.value.bookPerShare, onTheBook, 'what the claim is worth to it');
    // A company far enough under water is one this cell will not name a price for. That is not a
    // floor: it is the absence of a reason to bid, and a saver with no reason posts nothing (B6).
    if (value > 0) shares.push({ instrument: i, price: value });
  }
  return { paper, shares };
}

/**
 * Equity B3, Reporting A1, A2, Law 19: WHAT THIS COMPANY LAST TOLD EVERYBODY IT EARNED, per share
 * and per period.
 *
 * It is a READ of a public event and nothing else: the company published a figure for a span of
 * periods it named (§48), and how many shares that residual is divided between is the register's own
 * count of them (Equity A2). Nothing is estimated here and nothing is stored — a saver that had its
 * own idea of what a company earned would be holding a second set of that company's books (Law 4),
 * and what a BANK thinks it will earn next is a different object with a different owner, published
 * under its own name (`research.estimate`) and deliberately not consulted here: a saver reading the
 * analysts would be one more party with no reason of its own (Reporting E2).
 */
function publishedBy(
  view: ParticipantView,
  i: Instrument,
): Option<{ readonly bookPerShare: number; readonly earnedPerShare: number }> {
  if (!i.issuer.some) return none();
  const said = view.lastPublicAbout('reporting.report', i.issuer.value);
  if (!said.some) return none();
  const earned = said.value.data['earned'];
  const assets = said.value.data['assets'];
  const liabilities = said.value.data['liabilities'];
  const from = said.value.data['from'];
  const to = said.value.data['to'];
  if (
    typeof earned !== 'number' ||
    typeof assets !== 'number' ||
    typeof liabilities !== 'number' ||
    typeof from !== 'number' ||
    typeof to !== 'number'
  ) {
    return none();
  }
  const periods = to - from + 1;
  if (periods <= 0) return none();
  return some({
    bookPerShare: div(
      sub(assets, liabilities, 'the residual its shares are a claim on'),
      i.issued,
      'what that is a share',
    ),
    earnedPerShare: div(
      div(earned, periods, 'what it earned a period'),
      i.issued,
      'what that is a share',
    ),
  });
}

/**
 * D5, D5.a: the bids for paper, one per line, for the money the cell decided this line gets.
 * `perLine` is its own budget divided by every place its money could go, so the same money is never
 * committed twice and no class of thing is preferred by a rule nobody stated.
 */
export function paperBids(
  view: ParticipantView,
  eligible: readonly SavingLine[],
  /** XI-15: what ONE MEMBER puts into one line. The cell's order is that, times how many there are. */
  perLine: number,
  lines: number,
  weight: number,
): PaperBid[] {
  if (perLine <= 0) return [];
  const out: PaperBid[] = [];
  for (const e of eligible) {
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
  perLine: number,
  short: number,
  steps: number,
): ShareOrder[] {
  const out: ShareOrder[] = [];
  const weight = view.self.representation === 'cell' ? view.self.weight : 1;
  for (const line of lines) {
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
      const qty = downTick(wanted < exist ? wanted : exist);
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
        sharesPerMember: want > p.sharesPerMember ? p.sharesPerMember : want,
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
