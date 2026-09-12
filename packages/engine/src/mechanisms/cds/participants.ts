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
import { contractOf, type ContractBook, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { UnitId } from '../../core/ids.js';
import { asQty } from '../../core/tick.js';
import { add, div, mul, sub } from '../../core/num.js';
import type { ParticipantView } from '../../world/context.js';
import { isCds, type CdsTerms } from './contract.js';
import { cdsLineOf } from './data.js';
import { isCdsIndex, seriesLineOf } from './series.js';

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
  t: CdsTerms,
): number | undefined {
  const cash = view.print(t.obligation);
  if (!cash.some || t.tenorYears <= 0) return undefined;
  const belowPar = sub(1, cash.value.price, 'what the cash market discounts this credit by');
  if (belowPar <= 0) return undefined;
  const perAnnum = div(belowPar, t.tenorYears, 'per year of the term it has');
  // Law 8: A LEVEL IS HELD IN MONEY PIECES PER PIECE OF THE THING, which is not the same number as
  // the rate a person says out loud. Three basis points a year on a unit of face is three
  // hundredths of a cent, and a schedule posted at 0.0003 is a schedule below this book's own tick
  // — dropped at the grid, which is how a book with orders in it came to print nothing at all.
  return view.registry.priceOf(
    m.ccy,
    view.registry.derivativeKind(book.kind).unit,
    perAnnum,
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
function exposureTo(view: ParticipantView, t: CdsTerms): number {
  let held = 0;
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !i.issuer.some || i.issuer.value !== t.reference) continue;
    if (!view.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    held = add(held, view.quantity(h.instrument), 'what this name owes it');
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
function coverHeld(view: ParticipantView, t: CdsTerms): number {
  let net = 0;
  for (const c of view.contracts.mine()) {
    if (!isCds(c.terms) || c.terms.reference !== t.reference) continue;
    const iAmA = c.a === view.self.id;
    const iBuy = iAmA === c.terms.buysProtection;
    net = add(net, iBuy ? c.notional : -c.notional, 'protection it already has on this name');
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
function ownView(view: ParticipantView, t: CdsTerms): number | undefined {
  // A2: THE VARIABLE IT HAS ACTUALLY OBSERVED. A party forms a view of a thing by watching it, and
  // what it has watched here is the level THIS BOOK struck when it was in it — which the
  // expectations system records from the fill like any other price it traded at.
  const o = view.outlook(`price.${String(cdsLineOf(t.reference, t.tenorYears))}`);
  return o.some ? o.value.expected : undefined;
}

/**
 * D10.a, E2: WHO YOU FACE IS PART OF WHAT IT IS WORTH — and against this reference in particular.
 *
 * What a buyer will pay is reduced by what it already has with this counterparty (its own read,
 * per pair: C1.a) and by how much of THIS name that counterparty has already written, which is
 * public: the net notional on a reference is a count of rows anybody may read. Protection from
 * somebody already full of the same credit is protection that pays when its writer cannot (E2).
 */
function counterpartyTerm(view: ParticipantView, facing: number, against: string): number {
  const own = view.equity();
  if (own <= 0) return 1;
  // A fraction of its own capital, which is a read and not a limit: the more of one name it
  // already faces through one counterparty, the less the next unit of it is worth.
  return add(
    1,
    -div(facing, add(facing, own, 'against its own capital'), `facing ${against}`),
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
  const term = counterpartyTerm(view, held > 0 ? held : -held, facing);
  const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
  const unit = view.registry.derivativeKind(decl.kind).unit;
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
  let price = mul(mine, term, 'what cover is worth facing this side');
  if (at.some) {
    const book = at.value.price;
    const conviction = sizeOf(view, unit, view.equity(), mine);
    if (mine > add(book, tick, 'wider than the book by a tick it can act on')) {
      want = add(want, conviction, 'and what its own view is worth to it');
    } else if (mine < sub(book, tick, 'tighter than the book by a tick it can act on')) {
      // B2, B3: it would rather WRITE this credit than hold cover on it — the same one position,
      // the other way. A naked seller is capitalised (B3), which is what `conviction` reads, and it
      // does not discount its own offer for the counterparty it is writing TO.
      want = sub(want, conviction, 'against what it would rather write');
      price = mine;
    }
  }
  const move = sub(want, held, 'from the protection it has to the protection it wants');
  if (move === 0) return [];
  const qty = view.registry.deliverable(unit, move > 0 ? move : -move);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * Derivative Layer E1: how much it would write, from its own balance sheet — never a limit, and
 * never more than its own capital could stand behind (B3: a naked seller is capitalised).
 */
function sizeOf(view: ParticipantView, unit: UnitId, own: number, spread: number): number {
  if (own <= 0 || spread <= 0) return 0;
  // What it would carry is what a year of that spread on its own capital comes to at the spread it
  // is quoting: a bigger book on a wider spread is the same risk, which is the arithmetic a party
  // actually does rather than a notional limit somebody wrote down.
  return view.registry.deliverable(
    unit,
    div(own, spread, 'what a year of this spread on its own capital would carry'),
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
  const last = view.print(seriesLineOf(t.series, t.tenorYears));
  if (!last.some) return [];
  const level = last.value.price;
  const unit = view.registry.derivativeKind(decl.kind).unit;
  // B1.a: what it holds of the constituents' own paper, at the weights the series fixed.
  const whole = t.names.reduce((n, x) => add(n, x.weight, 'the whole line'), 0);
  if (whole <= 0) return [];
  let exposed = 0;
  for (const n of t.names) {
    if (n.reference === view.self.id) return [];
    exposed = add(exposed, view.quantity(n.obligation), 'its holding of a constituent');
  }
  let held = 0;
  for (const c of view.contracts.mine()) {
    if (!isCdsIndex(c.terms) || c.terms.series !== t.series) continue;
    const iAmA = c.a === view.self.id;
    held = add(held, iAmA === c.terms.buysProtection ? c.notional : -c.notional, 'cover on the line');
  }
  const want = sub(exposed, held, 'the exposure to this line it has not covered');
  if (want <= 0) return [];
  const qty = view.registry.deliverable(unit, want);
  if (qty <= 0) return [];
  return [
    {
      party: view.self.id,
      side: 'buy',
      price: mul(level, counterpartyTerm(view, held > 0 ? held : -held, 'the line'), 'facing it'),
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
