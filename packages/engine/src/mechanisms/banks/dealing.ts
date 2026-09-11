/**
 * A bank's DEALING LINE: the only face it shows any market.
 *
 * @spec Dealer Desks A1 Dealer Desks A2 Dealer Desks A3 Dealer Desks A4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C5 Dealer Desks D1 Dealer Desks D2 Dealer Desks D3 Dealer Desks D4 Dealer Desks D5 Dealer Desks E3 Dealer Desks E4 Dealer Desks F1 Dealer Desks F2 Dealer Desks F3 Fund Shares E3 Fund Shares E3.a Clearing A3 Clearing B2 XI-4 Law 4 Law 15
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
import { instrumentId, type CurrencyCode, type InstrumentId, type PartyId } from '../../core/ids.js';
import { Missing } from '../../core/errors.js';
import { add, div, material, mul, sub, sum } from '../../core/num.js';
import { upTick } from '../../core/tick.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { wasTraded } from '../../prices/price-store.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import {
  bankOf,
  bankParam,
  dealingParam,
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

/**
 * D1, XI-4: its own appetite, and what its treasury allotted it to GROW BY. A line's limit is what
 * it is already carrying plus the room it was given — a bank with no room left keeps the book it
 * has and stops adding to it, which is what scarce capital does to a dealer and is arithmetic
 * rather than a bound (Law 6: it cannot spend room that does not exist). Before its treasury has
 * allotted anything it has its own appetite and nothing else to go on.
 */
function allotted(view: ParticipantView, appetite: number, carried: number): number {
  const room = roomFor(view, DEALING);
  if (!room.some) return appetite;
  const may = add(carried, room.value, 'what it carries plus the room it was given');
  return may < appetite ? may : appetite;
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
  targets: ReadonlyMap<InstrumentId, number>,
): number {
  const terms: number[] = [];
  for (const [id, want] of targets) {
    const mark = view.mark(id);
    if (!mark.some) continue;
    const held = mul(view.quantity(id), mark.value, 'what this line is worth');
    terms.push(Math.abs(sub(held, want, 'how far from the target it is')));
  }
  return sum(terms).value;
}

/** Banks Funding C1: what this bank could pay with, as the bank itself last published it (F4). */
function liquidOf(view: ParticipantView, ccy: CurrencyCode): number {
  const said = view.lastOwn('bank.liquidity');
  const liquid = said.some ? said.value.data['liquid'] : undefined;
  return typeof liquid === 'number' && liquid > 0 ? liquid : view.cash(ccy);
}

/**
 * D1, F2: the capital this bank has, as the bank itself last published it (Banks Capital B3). Its
 * dealing book stands behind that capital and nothing else — a limit read off anything but its own
 * published position would be a second answer to what this bank is worth (Law 4, Law 19).
 */
function capitalOf(view: ParticipantView): number {
  const said = view.lastOwn('bank.capital');
  const capital = said.some ? said.value.data['capital'] : undefined;
  return typeof capital === 'number' && capital > 0 ? capital : 0;
}

/** C5: how many books it is making a market in this period — its money is spread over them. */
function linesQuoted(view: ParticipantView, d: BankDecl): number {
  let n = 0;
  for (const m of view.markets) {
    const i = view.instruments.get(m.instrument);
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
export function carryRate(view: ParticipantView): number | undefined {
  const said = view.lastOwn('bank.costOfFunds');
  const perAnnum = said.some ? said.value.data['perAnnum'] : undefined;
  if (typeof perAnnum !== 'number') return undefined;
  return rateOf(
    perAnnum,
    view.params.get(TRADING_BOOK_CAPITAL_RATIO),
    view.params.get(TRADING_BOOK_RISK_WEIGHT),
    view.params.get(bankParam(view.self.id, 'returnOnCapital')),
    periodOfYear(view.calendar, view.period),
  );
}

/** The state the quote is priced from: all of it the bank's own, read at the moment it quotes. */
export function stateOf(view: ParticipantView, d: BankDecl): DeskState | undefined {
  const rate = carryRate(view);
  if (rate === undefined) return undefined;
  const ccy = ccyOf(view);
  const targets = liquidityTargets(
    view,
    d,
    liquidityPlan(view, view.params.get(bankParam(view.self.id, 'liquidityCushion'))),
  );
  return {
    // C2.a: where its own treasury wants each line held. The treasury posts nothing; this is how
    // what it decided reaches the market (Law 4: one decider, one face).
    targetIn: (instrument: InstrumentId): number => {
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
      mul(
        capitalOf(view),
        view.params.get(dealingParam(view.self.id, 'capitalAtRisk')),
        'what it will put behind its dealing book',
      ),
      bookValue(view, targets),
    ),
    concentration: view.params.get(dealingParam(view.self.id, 'concentration')),
    ratePerPeriod: rate,
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
export function dealingOrders(
  view: ParticipantView,
  m: MarketDecl,
  rows: readonly BankDecl[],
): readonly Order[] {
  const d = bankOf(rows, view.self.id);
  if (d === undefined || !view.self.status.alive) return [];
  const i = view.instruments.get(m.instrument);
  if (!i.status.live || !d.makes.includes(String(i.kind))) return [];
  const state = stateOf(view, d);
  if (state === undefined) return [];
  const quoted = quoteFor(view, i.id, state);
  if (!quoted.some) return [];
  const q = quoted.value;
  // Banks Funding D1, XI-2: THE RUNG BELOW THE MARKET. A bank the last session refused sells the
  // paper it has left, and a forced seller does not name a price — the order carries a size and no
  // level (Clearing C3), so it is struck at whatever the other side posted, and the print a fire
  // sale makes reaches every other holder of the same paper through the revaluation.
  const urgent = urgentSale(view, d, i.id);
  if (urgent > 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: urgent }];
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
  const short = said.value.data['short'];
  const buffer = said.value.data['buffer'];
  if (typeof short !== 'number' || typeof buffer !== 'number') return NO_QTY;
  const owed = sub(short, buffer, 'what it cannot pay, once the cushion is gone');
  if (owed <= 0) return NO_QTY;
  let total = 0;
  let mine = 0;
  for (const m of view.markets) {
    const i = view.instruments.get(m.instrument);
    if (!i.status.live || !d.makes.includes(String(i.kind))) continue;
    const mark = view.mark(i.id);
    const free = view.free(i.id);
    if (!mark.some || mark.value <= 0 || free <= 0) continue;
    const worth = mul(free, mark.value, 'what this parcel would fetch');
    total += worth;
    if (i.id === line) mine = worth;
  }
  if (total <= 0 || mine <= 0) return NO_QTY;
  const mark = view.mark(line);
  if (!mark.some || mark.value <= 0) return NO_QTY;
  const share = mul(owed, div(mine, total, 'what this line carries'), 'raised here');
  const want = upTick(div(share, mark.value, 'units to sell'));
  const free = view.free(line);
  return want < free ? want : free;
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
  offered: number,
): readonly Order[] {
  if (q.bid <= 0 || q.view <= 0) return [];
  const said = view.lastPublicAbout('auction.announced', instrument);
  const share = said.some && said.value.period === view.period ? said.value.data['dealershipShare'] : undefined;
  const obliged = typeof share === 'number' ? mul(offered, share, 'what it must bid for') : 0;
  // E2.a, E5: and what its own treasury is short of in this line — a reason of its own, and the
  // same distance from target its quote is already skewing around, counted in units.
  const gap = sub(
    div(state.targetIn(instrument), q.view, 'units its treasury wants held'),
    view.free(instrument),
    'units it is short of that',
  );
  const wanted = obliged > gap ? obliged : gap;
  // C3.a: it bids out of the money it has. A bank with none bids nothing, and that is how an
  // auction fails: not because a rule allowed it to, but because nobody could pay.
  const affordable = div(state.cash, q.bid, 'what it could pay for');
  // Law 8: both of those are money over a price, so both are fractions of a unit of the paper. It
  // bids for whole ones, and down, because a bid it cannot pay for is not a bid (C3.a).
  const qty = downTick(wanted < affordable ? wanted : affordable);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: q.bid, qty }];
}

/**
 * Fund Shares E3, E3.a: the bank's reason to close a gap between an exchange-traded fund's two
 * values — and its reason not to. What it costs is one period of carrying the position it is about
 * to take on (D3); what limits it is what it holds and what its own limits leave it room for (D1).
 */
export function arbitrage(ctx: MechanismContext, bank: PartyId, rows: readonly BankDecl[]): void {
  const d = bankOf(rows, bank);
  if (d === undefined || !ctx.parties.has(bank) || !ctx.parties.get(bank).status.alive) return;
  const view = ctx.participant(bank);
  const state = stateOf(view, d);
  if (state === undefined) return;
  for (const v of ctx.venues) {
    if (v.key['kind'] !== 'etf') continue;
    const fund = v.key['fund'];
    const line = v.key['share'];
    if (fund === undefined || line === undefined) continue;
    const share = instrumentId(line);
    if (!ctx.instruments.has(share) || !ctx.instruments.get(share).status.live) continue;
    if (!d.makes.includes(String(ctx.instruments.get(share).kind))) continue;
    const struck = view.lastPublicAbout('etf.struck', fund);
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
    const premium = sub(print.value.price, nav, 'what the market pays over the book');
    const worth = mul(nav, state.ratePerPeriod, 'what a share costs it to carry for a period');
    if (Math.abs(premium) <= worth) continue;
    const held = view.quantity(share);
    // Law 8, D1: what its own limit leaves it room for, in WHOLE creation units — the limit is
    // money and a unit has a price, so the division lands between two of them and the one below is
    // what it has room for.
    const room = downTick(
      div(
        mul(state.limitAggregate, state.concentration, 'the most of the book in one line'),
        nav,
        'creation units that comes to',
      ),
    );
    const shares: Qty =
      premium > 0
        ? deliverable(
            view,
            struck.some ? struck.value.data['basket'] : undefined,
            subQty(room, held, 'room it has for more of this line'),
          )
        : held;
    if (shares <= 0 || !material(shares, 2, room)) continue;
    ctx.post(v.id, { party: bank, side: premium > 0 ? 'buy' : 'sell', price: 'market', qty: shares });
    ctx.record(
      'bank.arbitrage',
      [bank, fund, share],
      {
        bank,
        fund,
        share,
        nav,
        price: print.value.price,
        premium,
        worth,
        shares,
        side: premium > 0 ? 'create' : 'redeem',
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
    const canMake = downTick(div(view.free(instrumentId(line)), perShare, 'creation units this line backs'));
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
    const i = ctx.instruments.get(m.instrument);
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
      worth: mark.some ? mul(view.quantity(i.id), mark.value, 'what it holds of this line') : 0,
      bid: q.bid,
      offer: q.offer,
      spread: sub(q.offer, q.bid, 'the width it quoted'),
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
      roomLeft: sub(state.limitAggregate, state.bookValue, 'room in the whole book'),
      ratePerPeriod: state.ratePerPeriod,
      lines,
    },
    true,
  );
}

export type { BankDecl, InstrumentId };
