/**
 * A bank's DEALING LINE: the only face it shows any market.
 *
 * @spec Dealer Desks A1 Dealer Desks A2 Dealer Desks A3 Dealer Desks A4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C5 Dealer Desks D1 Dealer Desks D2 Dealer Desks D3 Dealer Desks D4 Dealer Desks D5 Dealer Desks E3 Dealer Desks E4 Dealer Desks F1 Dealer Desks F2 Dealer Desks F3 Fund Shares E3 Fund Shares E3.a Securities Lending B1 Securities Lending E1 Clearing A3 Clearing B2 XI-4 Law 4 Law 15
 *
 * A1 says a dealer is "a named party, usually a bank's trading arm, WITH ITS OWN BALANCE SHEET
 * INSIDE A BANK'S", and F2 says no desk is exempt from its own bank's capital and funding. Both are
 * one statement: the dealing book is a SUB-LEDGER of one balance sheet. So there is no desk here —
 * the bank quotes, out of the bank's own inventory, funded at the bank's own cost of funds, against
 * the bank's own capital, and what a line of business is, is data about the bank (`lines.ts`).
 *
 * D3's "funding cost on the inventory, paid every period it holds it" is met more truly than a rent
 * between two parties that were economically one: the bank pays deposit interest and session rates
 * on the liabilities that fund the inventory, every period, in real instructions to real holders —
 * and the quote's carry reads that blended cost (`bank.costOfFunds`, one writer, Law 4) plus what
 * the capital the position consumes has to earn. A bank that carried inventory for free would have
 * to be a bank that paid nothing for its money.
 */
import { linesCovered } from './staff.js';
import {
  instrumentId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { Missing } from '../../core/errors.js';
import {
  type PerPiece,
  amountOf,
  asAmount,
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  over,
  plus,
  type Ratio,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import { atLeast, atMost, material, sum } from '../../core/num.js';
import { upTick } from '../../core/tick.js';
import { delivers, type MarketDecl } from '../../clearing/market.js';
import type { VenueDecl } from '../../clearing/venue.js';
import type { Order } from '../../clearing/solver.js';
import { wasTraded } from '../../prices/price-store.js';
import type { BorrowNeed, MechanismContext, ParticipantView } from '../../world/context.js';
import {
  bankOf,
  bankParam,
  lineParam,
  TRADING_BOOK_CAPITAL_RATIO,
  TRADING_BOOK_RISK_WEIGHT,
  type BankDecl,
} from './data.js';
import { DEALING, roomFor } from './lines.js';
import { ccyOf, liquidityPlan, liquidityTargets } from './treasury.js';
import { periodOfYear, quoteFor, rateOf, type DeskQuote, type DeskState } from './dealing-quote.js';
import { downTick, subQty } from '../../core/tick.js';
import type { Qty } from '../../core/tick.js';
import { NO_QTY } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';

/**
 * D1, XI-4: its own appetite, and what its treasury allotted it to GROW BY. A line's limit is what
 * it is already carrying plus the room it was given — a bank with no room left keeps the book it
 * has and stops adding to it, which is what scarce capital does to a dealer and is arithmetic
 * rather than a bound (Law 6: it cannot spend room that does not exist). Before its treasury has
 * allotted anything it has its own appetite and nothing else to go on.
 */
function allotted(view: ParticipantView, appetite: Cash, carried: Cash): Cash {
  const room = roomFor(view, DEALING);
  if (!room.some) return appetite;
  const may = plus(carried, room.value, 'what it carries plus the room it was given');
  return atMost(may, appetite, 'a line cannot spend room that does not exist');
}

/**
 * A3, D5, F2: what this bank's DEALING book is worth at the last marks.
 *
 * THE BOOK IS WHAT IS NOT WHERE THE TREASURY WANTS IT. A bank holding exactly the liquidity
 * portfolio its own treasury asked for is running no dealing position at all: the paper is there
 * because the treasury decided it should be, and the desk's limit has nothing to say about it. What
 * the desk is carrying is the DISTANCE from that target, either way — long of it, or short of where
 * it is supposed to be with an order to place. That is the same line drawn twice: it is the trading
 * book for the limit here, and it is the trading book for the capital requirement (Dealer Desks F2).
 */
export function bookValue(
  view: ParticipantView,
  targets: ReadonlyMap<InstrumentId, Cash>,
): Cash {
  const terms: Cash[] = [];
  for (const [id, want] of targets) {
    const mark = view.mark(id);
    if (!mark.some) continue;
    const held = valueAt(mark.value, view.quantity(id), 'what this line is worth');
    /**
     * D1, D4: ONE HOLDING, TWO OWNERS, AND THE TREASURY'S CLAIM ON IT COMES FIRST.
     *
     * The register cannot say which lots the dealing line bought, so what separates them is the
     * treasury's stated want: of what the bank holds of this line, the treasury has asked for
     * `want`, and what is left over is the position the DESK took. It is not the distance either
     * way — a bank holding LESS of a line than its treasury asked for has a treasury with a
     * purchase to make, not a desk carrying anything, and reading that gap as dealing risk put
     * every desk in every seed past its own aggregate limit before it had quoted (`12d-12`,
     * `13b-5`).
     *
     * Law 6: the treasury cannot claim more of a line than there is of it, so its claim is the
     * smaller of what it wants and what is there — arithmetic, not a floor under a decision. What
     * that leaves is the desk's, and it cannot be negative because the bank cannot be short a line
     * it has not borrowed (Register C4).
     */
    const treasurys = atMost(want, held, 'the treasury cannot claim more of a line than there is of it');
    terms.push(minus(held, treasurys, 'what the desk is carrying of it'));
  }
  return sum(terms).value;
}

/** Banks Funding C1: what this bank could pay with, as the bank itself last published it (F4). */
function liquidOf(view: ParticipantView, ccy: CurrencyCode): Cash {
  const said = view.lastOwn('bank.liquidity');
  const liquid = said.some ? said.value.data['liquid'] : undefined;
  return typeof liquid === 'number' && liquid > 0
    ? asCash(liquid, 'what it published it could pay with')
    : asCash(view.cash(ccy), 'what is in the account');
}

/**
 * D1, F2: the capital this bank has, as the bank itself last published it (Banks Capital B3). Its
 * dealing book stands behind that capital and nothing else — a limit read off anything but its own
 * published position would be a second answer to what this bank is worth (Law 4, Law 19).
 */
function capitalOf(view: ParticipantView): Cash {
  const said = view.lastOwn('bank.capital');
  const capital = said.some ? said.value.data['capital'] : undefined;
  // Item 16: money re-entering from what this bank published, at the read that knows what it is.
  return typeof capital === 'number' && capital > 0
    ? asCash(capital, 'what it published as its capital')
    : asCash(0, 'a bank that has published nothing');
}

/** C5: how many books it is making a market in this period — its money is spread over them. */
function linesQuoted(view: ParticipantView, d: BankDecl): number {
  let n = 0;
  for (const m of view.markets) {
    const subject = delivers(m);
    if (!subject.some) continue;
    const i = view.instruments.get(subject.value);
    if (i.status.live && d.makes.includes(String(i.kind))) n += 1;
  }
  return n;
}

/**
 * D2, D3, XI-4 joint three: what one unit of the book costs the bank for one period — what it pays
 * for the money that funds it, plus the return it needs on the capital the position consumes.
 *
 * Both are the BANK's own numbers, read from what the bank itself published (Law 4, Law 19): a
 * dealing line with its own cost of funds would be a second answer to what money costs this bank,
 * and one with its own required return would be a second answer to what its capital costs.
 */
export function carryRate(view: ParticipantView, ccy: CurrencyCode): Ratio | undefined {
  /**
   * Law 8, Currency A3, XI-12: IN THE MONEY THE LINE IS IN, which is not always this bank's own.
   *
   * A desk quoting a line denominated in another money funds that position in that money — it buys
   * it spot and fixes what it costs forward (the FX swap: 13b's `fx-derivatives`), or it borrows
   * it. What that comes to is what the bank itself published for that money, beside its own, under
   * `alsoIn`. Charging its home cost of funds against a foreign line was a bank quoting a euro
   * position at the price of dollars — and adding a euro interest cost to a dollar capital charge
   * was two currencies in one sum (Appendix B). Measured at 0.0887 against 0.0044 for the same
   * bank's own money (worklist 13b, finding `12d-14`).
   *
   * A money it has NOT funded and cannot say what costs it has no rate here, and its desk does not
   * quote that line — which is the honest answer and the refusal Money B3.c is about.
   */
  const said = view.lastOwn('bank.costOfFunds');
  if (!said.some) return undefined;
  const home = said.value.data['ccy'];
  const perAnnum =
    home === String(ccy)
      ? said.value.data['perAnnum']
      : ((said.value.data['alsoIn'] as Record<string, { perAnnum?: unknown }> | undefined)?.[
          String(ccy)
        ]?.perAnnum ?? undefined);
  if (typeof perAnnum !== 'number') return undefined;
  return rateOf(
    asRatio(perAnnum, 'what this bank published as its cost of funds'),
    view.params.ratio(TRADING_BOOK_CAPITAL_RATIO),
    view.params.ratio(TRADING_BOOK_RISK_WEIGHT),
    view.params.perAnnum(bankParam(view.self.id, 'returnOnCapital')),
    periodOfYear(view.calendar, view.period),
  );
}

/** The state the quote is priced from: all of it the bank's own, read at the moment it quotes. */
export function stateOf(view: ParticipantView, d: BankDecl): DeskState | undefined {
  const rate = carryRate(view, ccyOf(view));
  if (rate === undefined) return undefined;
  const ccy = ccyOf(view);
  const targets = liquidityTargets(
    view,
    d,
    liquidityPlan(view, view.params.ratio(bankParam(view.self.id, 'liquidityCushion'))),
  );
  return {
    // C2.a: where its own treasury wants each line held. The treasury posts nothing; this is how
    // what it decided reaches the market (Law 4: one decider, one face).
    targetIn: (instrument: InstrumentId): Cash => {
      const want = targets.get(instrument);
      if (want === undefined) {
        throw new Missing(
          'Dealer Desks C5',
          `${view.self.id} was asked what it wants held of ${instrument}, which is not a line it makes`,
          { bank: view.self.id, instrument },
        );
      }
      return want;
    },
    // D1, F1, XI-4: what it will put behind its dealing book is its own appetite AND what its
    // treasury allotted this line out of the room the bank has left — whichever is the smaller,
    // because a line cannot spend room that does not exist (arithmetic, not a bound). The treasury
    // allots to what earned (`bank.lines`), so a dealing line that made less than the lending line
    // in a period when the room ran out stops bidding, and that is what a thin market looks like
    // when capital is scarce rather than when a number said so.
    limitAggregate: allotted(
      view,
      scale(
        capitalOf(view),
        view.params.ratio(lineParam(view.self.id, DEALING, 'capitalAtRisk')),
        'what it will put behind its dealing book',
      ),
      bookValue(view, targets),
    ),
    concentration: view.params.ratio(lineParam(view.self.id, DEALING, 'concentration')),
    ratePerPeriod: rate,
    rateIn: (money) => carryRate(view, money),
    bookValue: bookValue(view, targets),
    // F1: what it could actually pay for one more unit. For a DESK that was the cash in its own
    // account; for a bank it is its LIQUID ASSETS — the account, what comes back tomorrow, and what
    // its unencumbered paper would raise (Banks Funding C1) — because a bank buying inventory
    // settles out of the same pool it settles everything else from, and a settlement balance on a
    // Friday afternoon is not a statement about what a bank can fund. It is the number the bank
    // published about itself (Law 4, Law 19), and before it has published one, the account is all
    // it can be sure of.
    cash: liquidOf(view, ccy),
    linesQuoted: linesQuoted(view, d),
  };
}

/**
 * A2, C5, E3: the quote, as two orders in the same book. The same posted quote belongs in every
 * book the bank makes a market in (C5), and every bank that makes a line is in that line's session
 * — so they face each other through it, which is the interdealer market (E3) without a second
 * venue for it.
 */
/**
 * D1, A3, A-54: whether this line is inside what THIS desk's people can cover.
 *
 * The lines it can make in — its own `BankDecl.makes`, and where a line names its makers, the ones
 * that name it — in the instruments store's own order, taking the first `linesCovered` of them. The
 * order is stable, so a desk that loses an hour drops the same line every time rather than a
 * different one each period.
 *
 * It used to walk EVERY live line in the world and take the first `room` of those, which is one
 * global cutoff applied identically to every bank: two desks with different businesses covered the
 * same lines, and a line past position `room` in the store was quoted by NOBODY however many banks
 * made its kind. The candidate set is the desk's own now, so the cutoff is too — which is what A3
 * means by dealing in a name being a second decision from dealing in a kind.
 */
export function coveredLines(
  view: ParticipantView,
  d: BankDecl,
  makersOf?: (instrument: InstrumentId) => readonly string[] | undefined,
): readonly InstrumentId[] {
  const room = linesCovered(view);
  if (room <= 0) return [];
  const out: InstrumentId[] = [];
  for (const x of view.instruments.all()) {
    if (!x.status.live || !x.market.some || !d.makes.includes(String(x.kind))) continue;
    const makers = makersOf?.(x.id);
    if (makers !== undefined && !makers.includes(String(view.self.id))) continue;
    out.push(x.id);
    if (out.length >= room) break;
  }
  return out;
}

function covers(
  view: ParticipantView,
  d: BankDecl,
  line: InstrumentId,
  makersOf?: (instrument: InstrumentId) => readonly string[] | undefined,
): boolean {
  return coveredLines(view, d, makersOf).some((id) => id === line);
}

/**
 * Securities Lending B1, B4, E1, Dealer Desks D1, D5: WHAT A DESK MUST BORROW TO SELL WHAT IT HAS
 * NOT GOT — the answer to the kernel's `borrowNeeds`, and the reason the borrow book has a side.
 *
 * A market maker that can only offer what it holds is not making a market in a line it thinks is
 * dear: it sells down to nothing and then stands there with a bid nobody hits, which is `binds:
 * 'position'` in every period after the first. What a desk does instead is BORROW THE LINE and
 * offer that, and the position it then has is a short — which is why E1 could hold vacuously for
 * the life of this world (`A-67`): there was no way to be short, so there was nothing for the
 * borrow to be behind, so nothing ever borrowed.
 *
 * Three things decide it, all of them the desk's own and none of them new:
 *  - it thinks the line is DEAR — its own view of what a unit is worth is below what the book last
 *    printed (C1, XI-13). A desk that agrees with the market has no reason to be short.
 *  - how much of the line it is prepared to have a position in at all, which is the same three
 *    constraints its bid is cut to (D1, D4, F1: its own limit, the room in its whole book, and the
 *    money it has), less what it already holds. A position is a position whichever way round it is.
 *  - what a period of it costs it — A5, and the sentence `wantsToBorrow` was reaching for: it will
 *    not pay more for the use of somebody else's paper than a period of its own money and capital
 *    costs it, which is the rate it already prices its own quotes off (`rateOf`).
 */
export function deskBorrows(
  view: ParticipantView,
  rows: readonly BankDecl[],
  makersOf?: (instrument: InstrumentId) => readonly string[] | undefined,
): readonly BorrowNeed[] {
  const d = bankOf(rows, view.self.id);
  if (d === undefined || !view.self.status.alive) return [];
  const state = stateOf(view, d);
  if (state === undefined) return [];
  const out: BorrowNeed[] = [...toCreate(view, d, state)];
  for (const line of coveredLines(view, d, makersOf)) {
    const printed = view.mark(line);
    if (!printed.some) continue;
    const quoted = quoteFor(view, line, state);
    if (!quoted.some) continue;
    const q = quoted.value;
    // XI-13: it is short because it disagrees with the book, and it puts its own money behind that.
    if (q.view >= printed.value) continue;
    const want = subQty(q.bidSize, view.free(line), 'beyond what it already holds');
    if (want <= 0) continue;
    const ccy = view.instruments.get(line).ccy;
    const rate = state.rateIn(ccy);
    if (rate === undefined) continue;
    const pledge = pledges(view, line);
    if (!pledge.some) continue;
    out.push({ instrument: line, units: want, ccy, willPay: rate, collateral: pledge.value });
  }
  return out;
}

/**
 * Fund Shares E3, E3.a, Securities Lending B1, `C-1`'s ETF row (item 9.8): WHOEVER MUST DELIVER MAY
 * BORROW.
 *
 * A creation is delivered IN KIND — a pro-rata slice of the fund's own book — so a desk that does
 * not hold every line of the basket cannot create, whatever the premium. `deliverable` says so in
 * one line: what it can make is what the line it holds LEAST of backs, and a line it holds NONE of
 * makes that zero. Measured: a premium of **0.28 of NAV**, twenty-six times a period of carry, with
 * nobody able to close it, and `E3` running one way for the life of the world because the only
 * desks who could create were the ones who happened to hold the whole basket already.
 *
 * The missing mechanism was the borrow market, and it is there now (item 9.4). What this does is
 * the sentence `C-1` ends on: a desk short of a line it must deliver borrows it, delivers the
 * basket, and takes the shares — and it owes the line back, which the shares it now holds can
 * redeem into. That is the arbitrage as it actually works, and every leg of it is real.
 *
 * It borrows ONLY what it is short of, and only for units it has room for and would actually
 * create: the gap and the room are `inKindGaps`' answer, the same one `arbitrage` posts on, so a desk
 * cannot borrow for a trade it will not do (Law 4).
 */
function toCreate(view: ParticipantView, d: BankDecl, state: DeskState): BorrowNeed[] {
  const out: BorrowNeed[] = [];
  for (const g of inKindGaps(view, d, state, view.venues)) {
    // A discount closes by REDEEMING, which delivers shares it already holds. There is nothing to
    // borrow for that; only a premium asks the desk to deliver a basket.
    if (g.premium <= 0) continue;
    const basket = g.basket;
    if (typeof basket !== 'object' || basket === null) continue;
    const wanted = downTick(subQty(g.room, g.held, 'room it has for more of this line'));
    if (wanted <= 0) continue;
    const ccy = view.instruments.get(g.share).ccy;
    const rate = state.rateIn(ccy);
    if (rate === undefined) continue;
    for (const [line, perShare] of Object.entries(basket as Record<string, unknown>)) {
      if (typeof perShare !== 'number' || perShare <= 0) continue;
      const id = instrumentId(line);
      if (!view.instruments.has(id)) continue;
      // Law 8, E3: a creation unit is a WHOLE unit, so what this line must deliver is a whole
      // number of its own pieces — UP, because a basket short of a piece is a basket it cannot
      // deliver, and what it must find is what it must find.
      const needs = upTick(scale(wanted, asRatio(perShare, 'what one unit draws of this line'), 'units'));
      const short = subQty(needs, view.free(id), 'what it has not got');
      if (short <= 0) continue;
      const pledge = pledges(view, id);
      if (!pledge.some) continue;
      out.push({ instrument: id, units: short, ccy, willPay: rate, collateral: pledge.value });
    }
  }
  return out;
}

/**
 * Securities Lending C1: WHAT IT PLEDGES — the biggest free position it has that is not the line it
 * is borrowing and that the market has printed a price for, because what a lender takes is
 * something it could sell. The lender's haircut is then taken over that (C1), not over this.
 *
 * Nothing when it has no free position anybody has priced, and then it does not borrow: a borrow
 * against nothing is an unsecured loan of a security, which is not this transaction.
 */
function pledges(view: ParticipantView, borrowing: InstrumentId): Option<InstrumentId> {
  let best: { id: InstrumentId; worth: Cash } | undefined;
  for (const h of view.holdings()) {
    if (h.instrument === borrowing) continue;
    const free = view.free(h.instrument);
    if (free <= 0) continue;
    const mark = view.mark(h.instrument);
    if (!mark.some) continue;
    const worth = valueAt(mark.value, free, 'what it has free of that line');
    if (best === undefined || worth > best.worth) best = { id: h.instrument, worth };
  }
  return best === undefined ? none<InstrumentId>() : some(best.id);
}

export function dealingOrders(
  view: ParticipantView,
  m: MarketDecl,
  rows: readonly BankDecl[],
  makersOf?: (instrument: InstrumentId) => readonly string[] | undefined,
): readonly Order[] {
  const d = bankOf(rows, view.self.id);
  if (d === undefined || !view.self.status.alive) return [];
  // Spot FX D1: a pair delivers nothing, so this desk has nothing to quote in it. What a bank does
  // in a pair market is its FX desk's, and that desk is the spot-fx module's own participant.
  const subject = delivers(m);
  if (!subject.some) return [];
  const i = view.instruments.get(subject.value);
  if (!i.status.live || !d.makes.includes(String(i.kind))) return [];
  /**
   * Dealer Desks A3: AND THIS LINE'S OWN MAKERS, when it has any. Dealing in a KIND is what a bank
   * decided to be in the business of; dealing in one NAME is a second decision, drawn per line, and
   * the two are not the same fact. Without this, every bank that deals shares at all quotes every
   * share in the world — which is the "every bank has a view of every firm" that A3 says a dealer
   * is not, and it is what left every desk opening with no inventory in any line (worklist 13b,
   * finding `12d-10`).
   */
  const makers = makersOf?.(i.id);
  if (makers !== undefined && !makers.includes(String(view.self.id))) return [];
  /**
   * Banks Funding D1, XI-2, Money Market A2.b, A-54: THE RUNG BELOW THE MARKET, AND IT IS ABOVE THE
   * COVERAGE GATE. A bank the last session refused sells the paper it has left, and a forced seller
   * does not name a price — the order carries a size and no level (Clearing C3), so it is struck at
   * whatever the other side posted, and the print a fire sale makes reaches every other holder of
   * the same paper through the revaluation.
   *
   * It sat BELOW `covers`, so a bank that could not quote could not be a forced seller either — and
   * a bank in trouble is exactly the one whose desk has stopped covering lines. XI-2's door for a
   * bank could not open. Quoting a market and being made to sell are different acts and only the
   * first of them needs people.
   */
  const urgent = urgentSale(view, d, i.id);
  if (urgent > 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: urgent }];
  /**
   * Dealer Desks D1, D4, XI-13 (13d): AND IT CAN ONLY QUOTE WHAT IT EMPLOYS. Making a market in a
   * line is people watching it, so how many lines this desk can be in is the hours it actually pays
   * for over the hours one line takes — and a desk that sheds staff drops lines, whose books then
   * journal `market.noView` because nobody is standing in them.
   *
   * Which lines it drops is its OWN candidate set in the register's order, never a choice made
   * here: a desk under pressure keeps the lines it has been in and stops quoting the rest.
   */
  if (!covers(view, d, i.id, makersOf)) return [];
  const state = stateOf(view, d);
  if (state === undefined) return [];
  const quoted = quoteFor(view, i.id, state);
  if (!quoted.some) return [];
  const q = quoted.value;
  const offer = view.offer(m.id);
  // Sovereign C3: a session carrying the issuer's own offer is an AUCTION, and at an auction a
  // primary dealer is a bidder. It posts one order, on one side, for one reason — and it does not
  // offer the line the issuer is selling, because underwriting the issue and competing with it are
  // not both what a dealership is.
  if (offer.some) return primaryBid(view, q, state, i.id, offer.value.size);
  const out: Order[] = [];
  if (q.bidSize > 0 && q.bid > 0) {
    out.push({ party: view.self.id, side: 'buy', price: q.bid, qty: q.bidSize });
  }
  if (q.offerSize > 0 && q.offer > 0) {
    out.push({ party: view.self.id, side: 'sell', price: q.offer, qty: q.offerSize });
  }
  return out;
}

/**
 * Banks Funding B7, D1, D2, Money Market A2.b: WHAT IT IS ACTUALLY SHORT OF, and what it sells.
 *
 * A bank asks the session for two different things at once: the money it has to pay out tomorrow,
 * and the buffer it likes to keep behind that. Only the first is an obligation. Its buffer is the
 * thing a bank RUNS DOWN when the market says no — that is what a buffer is for — so the sale
 * covers the refusal LESS the buffer inside it: a bank that could not top up its cushion sells
 * nothing, and a bank that cannot meet a maturity sells until it can.
 *
 * It sells across the lines it makes a market in, in proportion to what each of them is worth to
 * it: a treasurer liquidating a book takes it down evenly rather than emptying one line first. Law
 * 8: it delivers whole pieces and rounds UP, because the point is to cover the shortfall — bounded
 * by what it actually holds, which is not a clamp but the arithmetic of a delivery (Law 6).
 */
function urgentSale(view: ParticipantView, d: BankDecl, line: InstrumentId): Qty {
  const said = view.lastOwn('moneyMarket.refused');
  if (!said.some || said.value.period + 1 !== view.period) return NO_QTY;
  const saidShort = said.value.data['short'];
  const saidBuffer = said.value.data['buffer'];
  if (typeof saidShort !== 'number' || typeof saidBuffer !== 'number') return NO_QTY;
  const owed = minus(
    asCash(saidShort, 'what the session refused it'),
    asCash(saidBuffer, 'the cushion inside it'),
    'what it cannot pay, once the cushion is gone',
  );
  if (owed <= 0) return NO_QTY;
  const worths: Cash[] = [];
  let mine = asCash(0, 'what this line would fetch');
  for (const m of view.markets) {
    const subject = delivers(m);
    if (!subject.some) continue;
    const i = view.instruments.get(subject.value);
    if (!i.status.live || !d.makes.includes(String(i.kind))) continue;
    const mark = view.mark(i.id);
    const free = view.free(i.id);
    if (!mark.some || mark.value <= 0 || free <= 0) continue;
    const worth = valueAt(mark.value, free, 'what this parcel would fetch');
    worths.push(worth);
    if (i.id === line) mine = worth;
  }
  const total = sum(worths).value;
  if (total <= 0 || mine <= 0) return NO_QTY;
  const mark = view.mark(line);
  if (!mark.some || mark.value <= 0) return NO_QTY;
  const share = scale(owed, ratioOf(mine, total, 'what this line carries'), 'raised here');
  const want = upTick(amountOf(share, mark.value, 'units to sell'));
  const free = view.free(line);
  return atMost(want, free, 'it sells what it holds unencumbered and no more');
}

/**
 * Sovereign C3, C3.a, C3.b, E2.a, E5: the primary dealer's bid.
 *
 * Every bank here is a primary dealer: it carries an OBLIGATION to bid a share of what is offered —
 * the share the issuer announced with the line (`auction.announced`), which is a term of the
 * dealership and therefore the ISSUER's number, not the dealer's — and it holds the privileges that
 * come with it. The obligation is what makes an auction hard to fail (C3.a).
 *
 * It does not make failure impossible, and that is the point. The bid is at the dealer's OWN price
 * — the same bid its quote shows in that line every other period: its own view, less what carrying
 * a unit costs it, less what it is already holding. So a dealer that has run its book up bids
 * lower, and a dealer with no money bids for less. It can bid badly and wear it (C3.b), and nobody
 * absorbs the remainder by construction (Treasury D5.a).
 */
function primaryBid(
  view: ParticipantView,
  q: DeskQuote,
  state: DeskState,
  instrument: InstrumentId,
  offered: Qty,
): readonly Order[] {
  if (q.bid <= 0 || q.view <= 0) return [];
  const said = view.lastPublicAbout('auction.announced', instrument);
  const share = said.some && said.value.period === view.period ? said.value.data['dealershipShare'] : undefined;
  const obliged =
    typeof share === 'number'
      ? scale(
          offered,
          asRatio(share, 'the share its dealership obliges it to bid for'),
          'what it must bid for',
        )
      : asAmount<'piece'>(0, 'no dealership, no obligation');
  // E2.a, E5: and what its own treasury is short of in this line — a reason of its own, and the
  // same distance from target its quote is already skewing around, counted in units.
  const gap = minus(
    amountOf(state.targetIn(instrument), q.view, 'units its treasury wants held'),
    view.free(instrument),
    'units it is short of that',
  );
  const wanted = atLeast(obliged, gap, 'its dealership obligation is a floor under its own need');
  // C3.a: it bids out of the money it has. A bank with none bids nothing, and that is how an
  // auction fails: not because a rule allowed it to, but because nobody could pay.
  const affordable = amountOf(state.cash, q.bid, 'what it could pay for');
  // Law 8: both of those are money over a price, so both are fractions of a unit of the paper. It
  // bids for whole ones, and down, because a bid it cannot pay for is not a bid (C3.a).
  const qty = downTick(atMost(wanted, affordable, 'a bid it cannot pay for is not a bid'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: q.bid, qty }];
}

/**
 * Fund Shares E3, E3.a: the bank's reason to close a gap between an exchange-traded fund's two
 * values — and its reason not to. What it costs is one period of carrying the position it is about
 * to take on (D3); what limits it is what it holds and what its own limits leave it room for (D1).
 */
/**
 * Fund Shares E3, E3.a: ONE EXCHANGE-TRADED FUND'S TWO VALUES, AND WHAT THIS DESK COULD DO ABOUT
 * THE GAP — read once, because two things act on it and Law 4 admits one writer of a fact.
 *
 * `arbitrage` posts the creation or the redemption; `deskBorrows` (item 9.8) asks what the desk is
 * SHORT OF to deliver a basket at all. Both are the same read of the same numbers and they must
 * agree about whether a gap is worth closing, or a desk would borrow for a trade it will not do.
 */
interface EtfGap {
  readonly venue: VenueId;
  readonly share: InstrumentId;
  /** What one share of the BOOK is worth, which is what a premium is measured against. */
  readonly perShare: PerPiece;
  /** E3: what the market pays over the book. Negative is a discount and closes the other way. */
  readonly premium: PerPiece;
  /** E3: what a unit is made of, as this fund published it. */
  readonly basket: unknown;
  /** D1: what its own limit leaves it room for, in whole creation units. */
  readonly room: Qty;
  readonly held: Qty;
  /** What the market last printed for the share, and what a period of carrying one costs it. */
  readonly printed: PerPiece;
  readonly worth: PerPiece;
  /** The fund whose book this is, for a record that names it. */
  readonly fund: string;
}

function inKindGaps(
  view: ParticipantView,
  d: BankDecl,
  state: DeskState,
  venues: readonly VenueDecl[],
): EtfGap[] {
  const out: EtfGap[] = [];
  for (const v of venues) {
    // Item 10e: the venue says WHAT HAPPENS THERE — creation and redemption in kind — rather than
    // naming a kind of fund. A desk reads the door, and the door states a fact about itself.
    if (v.key['kind'] !== 'inKind') continue;
    const fund = v.key['fund'];
    const line = v.key['share'];
    if (fund === undefined || line === undefined) continue;
    const share = instrumentId(line);
    if (!view.instruments.has(share) || !view.instruments.get(share).status.live) continue;
    if (!d.makes.includes(String(view.instruments.get(share).kind))) continue;
    const struck = view.lastPublicAbout('fund.listedStruck', fund);
    const nav = struck.some ? struck.value.data['perShare'] : undefined;
    const print = view.print(share);
    if (typeof nav !== 'number' || nav <= 0 || !print.some) continue;
    // Appendix B, Clearing E4: the gap is against a price the market MADE. A mark carried forward
    // because nobody traded is not a level it could sell into, and a bank that delivered a basket
    // against one would be trading on its own carried number.
    if (!wasTraded(print.value)) continue;
    // Law 8, Law 16: it is a PRICE over a price — what one share trades at less what one share of
    // the book is worth — and it is named for that. Called `gap` it read like a quantity, which is
    // the one thing it is not: nothing in this world holds 2,308.77 of anything.
    const perShare = asPerPiece(nav, 'what one share of the book is worth');
    const premium = minus(print.value.price, perShare, 'what the market pays over the book');
    const worth = scale(
      perShare,
      state.ratePerPeriod,
      'what a share costs it to carry for a period',
    );
    if (Math.abs(premium) <= worth) continue;
    // Law 8, D1: what its own limit leaves it room for, in WHOLE creation units — the limit is
    // money and a unit has a price, so the division lands between two of them and the one below is
    // what it has room for.
    const room = downTick(
      amountOf(
        scale(state.limitAggregate, state.concentration, 'the most of the book in one line'),
        perShare,
        'creation units that comes to',
      ),
    );
    out.push({
      venue: v.id,
      share,
      perShare,
      premium,
      basket: struck.some ? struck.value.data['basket'] : undefined,
      room,
      held: view.quantity(share),
      printed: print.value.price,
      worth,
      fund,
    });
  }
  return out;
}

export function arbitrage(ctx: MechanismContext, bank: PartyId, rows: readonly BankDecl[]): void {
  const d = bankOf(rows, bank);
  if (d === undefined || !ctx.parties.has(bank) || !ctx.parties.get(bank).status.alive) return;
  const view = ctx.participant(bank);
  const state = stateOf(view, d);
  if (state === undefined) return;
  for (const g of inKindGaps(view, d, state, ctx.venues)) {
    const shares: Qty =
      g.premium > 0
        ? deliverable(view, g.basket, subQty(g.room, g.held, 'room it has for more of this line'))
        : g.held;
    if (shares <= 0 || !material(shares, 2, g.room)) continue;
    ctx.post(g.venue, {
      party: bank,
      side: g.premium > 0 ? 'buy' : 'sell',
      price: 'market',
      qty: shares,
    });
    ctx.record(
      'bank.arbitrage',
      [bank, g.fund, g.share],
      {
        bank,
        fund: g.fund,
        share: g.share,
        nav: g.perShare,
        price: g.printed,
        premium: g.premium,
        worth: g.worth,
        shares,
        side: g.premium > 0 ? 'create' : 'redeem',
      },
      false,
    );
  }
}

/**
 * E3: how many creation units it could actually deliver, out of what it is holding and the room it
 * has left. A basket it cannot make up is a creation it cannot do (Clearing A3).
 */
function deliverable(view: ParticipantView, basket: unknown, room: Qty): Qty {
  if (typeof basket !== 'object' || basket === null || room <= 0) return NO_QTY;
  let most = room;
  let lines = 0;
  for (const [line, perShare] of Object.entries(basket as Record<string, unknown>)) {
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    lines += 1;
    // Law 8: E3 — a creation unit is a WHOLE unit or it is not one. What this line backs is what it
    // holds over what a unit draws of it, and the fraction above that is a unit it cannot deliver.
    const canMake = downTick(
      over(
        view.free(instrumentId(line)),
        asRatio(perShare, 'what one creation unit draws of this line'),
        'creation units this line backs',
      ),
    );
    if (canMake < most) most = canMake;
  }
  return lines > 0 ? most : NO_QTY;
}

/**
 * D5, E4: what each bank is carrying on its dealing book, what it is quoting and what room it has
 * left, published together — so a period in which spreads widened and inventory did not is visible
 * rather than inferred.
 */
export function publishDealing(
  ctx: MechanismContext,
  bank: PartyId,
  rows: readonly BankDecl[],
): void {
  const d = bankOf(rows, bank);
  if (d === undefined || !ctx.parties.get(bank).status.alive) return;
  const view = ctx.participant(bank);
  const state = stateOf(view, d);
  if (state === undefined) return;
  const lines: Record<string, unknown> = {};
  for (const m of ctx.markets) {
    const subject = delivers(m);
    if (!subject.some) continue;
    const i = ctx.instruments.get(subject.value);
    if (!i.status.live || !d.makes.includes(String(i.kind))) continue;
    const quoted = quoteFor(view, i.id, state);
    if (!quoted.some) continue;
    const q: DeskQuote = quoted.value;
    const mark = view.mark(i.id);
    lines[i.id] = {
      inventory: view.quantity(i.id),
      // C2.a, F2: where its own treasury wants this line, and what it is actually holding of it at
      // the marks — the two numbers that say which part of the holding is a position it took.
      target: state.targetIn(i.id),
      worth: mark.some
        ? valueAt(mark.value, view.quantity(i.id), 'what it holds of this line')
        : 0,
      bid: q.bid,
      offer: q.offer,
      spread: minus(q.offer, q.bid, 'the width it quoted'),
      skew: q.skew,
      bidSize: q.bidSize,
      offerSize: q.offerSize,
      // D4, D5: which limit shrank the bid — the position, the whole book, or the money.
      binds: q.binds,
    };
  }
  ctx.record(
    'bank.dealing',
    [bank],
    {
      bank,
      book: state.bookValue,
      // D1, F1: how much of its capacity is used. Finite and enumerable, and here it is enumerated.
      roomLeft: minus(state.limitAggregate, state.bookValue, 'room in the whole book'),
      ratePerPeriod: state.ratePerPeriod,
      lines,
    },
    true,
  );
}

export type { BankDecl, InstrumentId };
