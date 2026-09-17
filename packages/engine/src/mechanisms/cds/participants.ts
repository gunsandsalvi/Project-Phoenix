/**
 * Who is in a credit default swap book, and WHY.
 *
 * @spec CDS B1 CDS B1.a CDS B2 CDS B3 CDS B4 CDS B5 CDS E1 CDS E2 CDS E4 Derivative D10 Derivative D10.a Derivative Layer E1 Expectations A1 Expectations A2 XI-13 Observer A4 Law 19
 *
 * Four reasons and not one of them is a hedge ratio.
 *
 * A BUYER because it is already exposed and cannot get out (B1.a): a bank holding the reference's
 * paper reads its OWN holding and buys cover for what it could not sell. A BUYER because it
 * disagrees (B1): a party whose outlook of this reference is worse than the spread is asking for
 * thinks protection is cheap. A SELLER because it wants the exposure without funding a bond (B2):
 * the same credit, no balance sheet — which is the whole reason the instrument exists and is also
 * why B3 makes it consume capital. A SELLER because it disagrees the other way (B4).
 *
 * EVERY RESERVATION CARRIES THE COUNTERPARTY (D10.a, E2): what protection is worth depends on who
 * is writing it, and wrong-way risk is the case where the writer's own credit moves with the thing
 * it is writing on. A buyer facing a house prices the house; a buyer facing a seller whose own
 * spread widens with the reference's pays less for the cover, because it is worth less.
 */
import type { InstrumentId } from '../../core/ids.js';
import {
  type Cash,
  type PerPiece,
  type Ratio,
  absolute,
  amountOf,
  asPerNamedUnit,
  asPerPiece,
  asRatio,
  minus,
  over,
  plus,
  ratioOf,
  scale, noCash,
} from '../../core/measure.js';
import { contractOf, type ContractBook, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { asQty, negQty, addQty, NO_QTY, subQty, type Qty } from '../../core/tick.js';
import { add } from '../../core/num.js';
import type { ParticipantView } from '../../world/context.js';
import { isCds, type CdsTerms } from './contract.js';
import { CDS_PARAMS, cdsLineOf } from './data.js';
import { isCdsIndex } from './series.js';
import { cashSpreadOf, spreadsFromView } from './measures.js';
import { about } from '../../world/context.js';

/**
 * A1.c, C3, Law 3, XI-13: WHAT THIS PARTY THINKS PROTECTION ON THIS NAME IS WORTH, and it is never
 * read off this book.
 *
 * Protection on a name is not the only price of that name: its own debt is already trading, and
 * what that debt is worth against par over the years it has left is what the CASH market is
 * charging for the same credit. That is a read of ANOTHER market — the reference's own bond — so it
 * is a number this party has whether or not this book has ever printed, and the difference between
 * the two is C3's basis: a READ, and a real one, because the two sides were never the same number.
 *
 * IT USED TO BE THE LAST RESORT and the book's own print stood in front of it, which is XI-13's
 * fixed point written out — the print moves the outlook, the outlook moves the view, the view moves
 * the quote, the quote moves the print. `banks/dealing-quote.ts` records what that did to a bill
 * and why its own `viewOf` asks the cash-flow number FIRST. This is the same order, for the same
 * reason: what somebody else paid is where the market is, and where the market is, is not a view.
 *
 * A reference whose debt has never printed either has no anchor here — which is the honest answer
 * for a name nobody has ever put a price on, and the party falls back to its own outlook below.
 */
function levelFor(
  view: ParticipantView,
  m: MarketDecl,
  book: ContractBook,
  t: { readonly obligation: InstrumentId; readonly tenorYears: number },
): PerPiece | undefined {
  if (t.tenorYears <= 0) return undefined;
  /**
   * 18.5, Law 4: THE CASH MARKET'S CHARGE FOR THIS CREDIT IS ONE DERIVATION and this is not it any
   * more. It was par less the bond's price over the years — which ignores the coupon, ignores the
   * sovereign, and says a bond at par has no credit risk however dear money is — while the basis
   * measurement two files over derived the honest number from a yield. Two answers to one question
   * is Law 4's defect; the one that stays is `cashSpreadOf`, and both callers ask it.
   */
  /**
   * Law 18 (0g.7): AND IT IS DERIVED ONCE WHILE THE PRINTS AND THE LINES STAND STILL.
   *
   * It is asked once per party per book, and what it asks about is the BOOK — an obligation, a
   * tenor and a money — so every party in a session was inverting the same bond's price to the same
   * yield. Measured at period 8 of the (24, 96) rung: **3,536 `yieldOf` calls for 17 distinct
   * questions**, 3,491 of them from here, each one about fifty present-value passes over the
   * flows. The answer is a function of the prints and the lines and of nothing else, so the key
   * names the question and not the asker, and the versions drop it the instant either moves.
   */
  const at = view.versions();
  const spread = view.memo(
    `cds.cashSpread|${String(t.obligation)}|${String(t.tenorYears)}|${String(m.ccy)}`,
    [at.prices, at.instruments],
    () => cashSpreadOf(spreadsFromView(view), t.obligation, t.tenorYears, m.ccy),
  );
  if (!spread.some || spread.value <= 0) return undefined;
  const perAnnum = asPerPiece(spread.value, 'what a year of this credit costs, per unit of face');
  // Law 8: A LEVEL IS HELD IN MONEY PIECES PER PIECE OF THE THING, which is not the same number as
  // the rate a person says out loud. Three basis points a year on a unit of face is three
  // hundredths of a cent, and a schedule posted at 0.0003 is a schedule below this book's own tick
  // — dropped at the grid, which is how a book with orders in it came to print nothing at all.
  return view.registry.priceOf(
    m.ccy,
    view.registry.derivativeKind(book.kind).unit,
    asPerNamedUnit(perAnnum, 'what a year of protection costs a unit of notional'),
  );
}

/**
 * B1.a, A4: WHAT THIS PARTY IS EXPOSED TO ON THIS NAME — everything the reference owes it, per
 * member, read off its own book.
 *
 * Protection is on a NAME and not on a line. A bank that holds one of a borrower's bonds and buys
 * cover has covered that borrower, and one that holds three and reads only the line the book was
 * written on has measured a third of its own exposure — which is the read being wrong rather than
 * the hedge being small.
 */
function exposureTo(view: ParticipantView, t: CdsTerms): Qty {
  let held = NO_QTY;
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !i.issuer.some || i.issuer.value !== t.reference) continue;
    if (!view.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    held = addQty(held, view.quantity(h.instrument), 'what this name owes it');
  }
  return held;
}

/**
 * B1.a, Observer A4: WHAT COVER IT ALREADY HAS ON THIS NAME — its own rows, signed by the side it
 * is on. Protection bought counts for it, protection written counts against.
 *
 * It is the difference between a reason and a habit. A bank that buys cover for its whole holding
 * every period is not hedging a position, it is buying the same hedge fifty-two times a year: what
 * it wants is the exposure it has LEFT, and a party that already has what it wanted posts nothing.
 */
function coverHeld(view: ParticipantView, t: CdsTerms): Qty {
  let net = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isCds(c.terms) || c.terms.reference !== t.reference) continue;
    const iAmA = c.a === view.self.id;
    const iBuy = iAmA === c.terms.buysProtection;
    net = addQty(
      net,
      iBuy ? c.notional : negQty(c.notional, 'the other side of it'),
      'protection it already has on this name',
    );
  }
  return net;
}

/**
 * B1, B4, Expectations A1: what THIS party thinks the reference's protection is worth, or nothing.
 *
 * The outlook is its own (A2: there is no global one), formed from what it has seen this book
 * print. A party that has never watched this reference has no view of it and says so, which is why
 * a book opens with the parties that are exposed in it and acquires its speculators later.
 */
function ownView(view: ParticipantView, t: CdsTerms): PerPiece | undefined {
  // A2: THE VARIABLE IT HAS ACTUALLY OBSERVED. A party forms a view of a thing by watching it, and
  // what it has watched here is the level THIS BOOK struck when it was in it — which the
  // expectations system records from the fill like any other price it traded at.
  const o = view.outlook(about({ on: 'price', instrument: cdsLineOf(t.reference, t.tenorYears) }));
  // Item 16: its own outlook of this book re-enters here — a spread, which is what it clears in.
  return o.some ? asPerPiece(o.value.expected, 'what it expects this credit to cost') : undefined;
}

/**
 * D10.a, E2: WHO YOU FACE IS PART OF WHAT IT IS WORTH — and against this reference in particular.
 *
 * What a buyer will pay is reduced by what it already has with this counterparty (its own read,
 * per pair: C1.a) and by how much of THIS name that counterparty has already written, which is
 * public: the net notional on a reference is a count of rows anybody may read. Protection from
 * somebody already full of the same credit is protection that pays when its writer cannot (E2).
 */
function counterpartyTerm(view: ParticipantView, facing: Cash, against: string): Ratio {
  const own = view.standsBehind();
  if (own.pieces <= 0) return asRatio(1, 'a party with nothing behind it discounts nothing');
  /**
   * D10.a, E2, 18.5: WHAT IT ALREADY FACES WITH THIS COUNTERPARTY, which is what it was supposed
   * to be reading. It was handed the party's own net COVER on this name — how much protection it
   * had bought, from anybody — so a party that had never traded with this counterparty at all
   * discounted its bid for it, and one that had a book full of it did not. What decides whether
   * the next unit of protection from somebody is worth what it says is the exposure it ALREADY has
   * to that somebody, which its own contract book answers (`contracts.exposureTo`, C1.a's pair).
   */
  const held = absolute(view.inOwnMoney(facing), `facing ${against}`);
  if (held.pieces <= 0) return asRatio(1, 'it faces this counterparty with nothing yet');
  return minus(
    asRatio(1, 'the whole of it'),
    ratioOf(held, plus(held, own, 'against its own capital'), `facing ${against}`),
    'the counterparty term',
  );
}

/**
 * B5, XI-13: SOMEBODY WITH A VIEW IS ON BOTH SIDES OF EVERY BOOK, or the book is one opinion and a
 * price it agreed with itself. What makes the two sides differ here is that they are different
 * parties with different books and different outlooks (§46 A3), not a coefficient.
 */
export function cdsOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isCds(decl.terms)) return [];
  const t = decl.terms;
  if (view.self.id === t.reference) return [];
  // XI-13: its OWN number — the cash market's charge for this credit, and its own outlook of this
  // book only when the reference's debt has never printed. Neither is this book's own last price.
  const anchor = levelFor(view, m, decl, t);
  const expects = ownView(view, t);
  const mine = anchor ?? expects;
  if (mine === undefined) return [];
  const held = coverHeld(view, t);
  const facing = decl.house === null ? 'the other side' : String(decl.house);
  const term = counterpartyTerm(
    view,
    decl.house === null ? noCash(m.ccy) : view.contracts.exposureTo(decl.house),
    facing,
  );
  const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
  // Clearing E1: WHERE THE MARKET IS. It decides which side this party is on and how hard, and it
  // is never the level posted: a party that posted where the market last was would be agreeing with
  // it rather than saying anything, and a book of those prints one number for ever.
  const at = view.print(cdsLineOf(t.reference, t.tenorYears));
  /**
   * ONE PARTY, ONE POSITION (Law 4). A party that both covered a holding and wrote protection
   * on the same name in the same session would be two opinions wearing one name — and the
   * kernel says so (Clearing A2: nobody crosses themselves). So what it posts is the DISTANCE
   * between the protection it wants and the protection it has, in whichever direction that is.
   *
   * What it wants is B1.a's exposure it could not sell, plus B1/B4's view when it has one: a
   * party that thinks the spread should be wider wants more cover than its holding alone.
   */
  let want = exposureTo(view, t);
  let price = scale(mine, term, 'what cover is worth facing this side');
  if (at.some) {
    const book = at.value.price;
    const conviction = sizeOf(view, view.standsBehind(), mine);
    if (mine > plus(book, tick, 'wider than the book by a tick it can act on')) {
      want = addQty(want, conviction, 'and what its own view is worth to it');
    } else if (mine < minus(book, tick, 'tighter than the book by a tick it can act on')) {
      // B2, B3: it would rather WRITE this credit than hold cover on it — the same one position,
      // the other way. A naked seller is capitalised (B3), which is what `conviction` reads, and it
      // does not discount its own offer for the counterparty it is writing TO.
      want = subQty(want, conviction, 'against what it would rather write');
      price = mine;
    }
  }
  const move = subQty(want, held, 'from the protection it has to the protection it wants');
  /**
   * A-66, XI-13, §46 A3: AND A PARTY WITH NOTHING TO COVER QUOTES BOTH WAYS AROUND ITS OWN NUMBER.
   *
   * Everything that could make this party a WRITER stands behind `at.some`, so in a book that has
   * never printed every party is left with `exposureTo(view, t)` — buy-only for everybody — and no
   * session ever crosses: all 60 CDS sessions in a measured run came back `noSupply`. A party with
   * nothing to cover on this name and capital to stand behind a naked position (B3) makes the
   * market: a bid a tick inside its own number and an ask a tick outside. Its number is the CASH
   * MARKET's charge for this credit (`mine`), which the header above already insists is never this
   * book's own print.
   */
  if (move === 0) {
    const room = sizeOf(view, view.standsBehind(), mine);
    if (room <= 0) return [];
    const bid = minus(mine, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(room) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(mine, tick, 'a tick outside its own number'),
        qty: asQty(room),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(move, 'the size of the move'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * Derivative Layer E1: how much it would write, from its own balance sheet — never a limit, and
 * never more than its own capital could stand behind (B3: a naked seller is capitalised).
 */
function sizeOf(view: ParticipantView, own: Cash, spread: PerPiece): Qty {
  if (own.pieces <= 0 || spread <= 0) return NO_QTY;
  /**
   * B3, 18.5: WHAT ITS CAPITAL WILL STAND BEHIND, and the arithmetic is the regulator's own.
   *
   * It was the capital divided by the SPREAD — a hundred times its own capital at a hundred basis
   * points, which is not a balance-sheet constraint at all, and it made a naked writer's size a
   * function of how cheap the credit was: the tighter the spread, the more of it a party would
   * write. What writing protection actually costs a party is CAPITAL, and what a unit of it
   * consumes is the risk weight on protection sold times the capital a unit of weighted exposure
   * takes — both published, both read here, neither this module's to choose.
   */
  const weight = view.params.ratio(CDS_PARAMS.riskWeightSold);
  const ratio = view.params.ratio(CDS_PARAMS.capitalRatio);
  const consumed = scale(weight, ratio, 'the capital a unit of protection sold consumes');
  if (consumed <= 0) return NO_QTY;
  return view.registry.deliverable(
    amountOf(own, asPerPiece(consumed, 'what a unit of it consumes'), 'what its capital will stand behind'),
  );
}

/**
 * A5, A5.b, B1.a: who is in a SERIES book.
 *
 * The reason to be in one is different from the reason to be in a single name, and that difference
 * is the point of the instrument: a party exposed to several of the constituents covers all of them
 * at once, which costs one trade and one margin line instead of twenty. What it wants is cover over
 * the part of the line it is actually exposed to, net of what it already holds — and the residual
 * between that and name-by-name cover is the basis A5.b says is a read.
 */
export function cdsIndexOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isCdsIndex(decl.terms)) return [];
  const t = decl.terms;
  /**
   * A-66, XI-13, §46 A3: ITS OWN NUMBER FOR THE SERIES, AND IT IS NOT THIS BOOK'S LAST PRINT.
   *
   * This read `view.print(seriesLineOf(...))` as a PRECONDITION and then posted at a multiple of
   * it, so a series that had never printed had no orders in it at all — and could therefore never
   * print. It is the same fixed point the single-name book's own header refuses in as many words.
   *
   * What a series is worth is what its CONSTITUENTS are worth: each name's own cash market charge
   * for that credit (`levelFor`, a read of the reference's bond), at the weights the series fixed.
   * A name whose paper has never printed contributes nothing and its weight comes out of the
   * denominator, so the answer is the weighted average of the names this party can actually price.
   */
  const whole = t.names.reduce((n, x) => add(n, x.weight, 'the whole line'), 0);
  if (whole <= 0) return [];
  let priced = 0;
  let blended = asPerPiece(0, 'the series before its names are walked');
  for (const n of t.names) {
    const one = levelFor(view, m, decl, { obligation: n.obligation, tenorYears: t.tenorYears });
    if (one === undefined) continue;
    priced = add(priced, n.weight, 'the part of the line it can price');
    blended = plus(
      blended,
      scale(one, asRatio(n.weight, 'this name in the line'), 'its part of the line'),
      'the line so far',
    );
  }
  if (priced <= 0) return [];
  const level = over(blended, asRatio(priced, 'the part of the line it could price'), 'the series');
  let exposed = NO_QTY;
  for (const n of t.names) {
    if (n.reference === view.self.id) return [];
    exposed = addQty(exposed, view.quantity(n.obligation), 'its holding of a constituent');
  }
  let held = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isCdsIndex(c.terms) || c.terms.series !== t.series) continue;
    const iAmA = c.a === view.self.id;
    held = addQty(
      held,
      iAmA === c.terms.buysProtection ? c.notional : negQty(c.notional, 'the other side of it'),
      'cover on the line',
    );
  }
  const want = subQty(exposed, held, 'the exposure to this line it has not covered');
  const facing = scale(
    level,
    counterpartyTerm(
      view,
      decl.house === null ? noCash(m.ccy) : view.contracts.exposureTo(decl.house),
      'the line',
    ),
    'facing it',
  );
  /**
   * §46 A3: and a party exposed to none of the names makes the market around its own number, sized
   * by what its capital would stand behind a naked position with (B3). A bid a tick inside and an
   * ask a tick outside: a spread, and not a crossing.
   */
  if (want === 0) {
    const room = sizeOf(view, view.standsBehind(), level);
    if (room <= 0) return [];
    const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
    const bid = minus(level, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(room) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(level, tick, 'a tick outside its own number'),
        qty: asQty(room),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(want, 'either way'));
  if (qty <= 0) return [];
  return [
    {
      party: view.self.id,
      side: want > 0 ? 'buy' : 'sell',
      price: want > 0 ? facing : level,
      qty: asQty(qty),
    },
  ];
}

/**
 * Law 15: ONE PARTICIPANT, TWO SHAPES OF BOOK, and the dispatch is on the shape of the terms rather
 * than on an id. A single name and a series are different instruments with different reasons, and
 * which one this book is, is a fact the book itself carries.
 */
export function cdsBookOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const t = contractOf(m)?.terms;
  if (t === undefined) return [];
  if (isCds(t)) return cdsOrders(view, m);
  if (isCdsIndex(t)) return cdsIndexOrders(view, m);
  return [];
}
