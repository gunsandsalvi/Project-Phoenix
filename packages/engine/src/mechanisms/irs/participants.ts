/**
 * Who is in a swap book, and WHY.
 *
 * @spec IRS B1 IRS B2 IRS B2.a IRS B3 IRS B4 IRS B5 IRS D1 IRS D2 IRS D3 IRS D3.a IRS D4 Expectations A1 Expectations A2 XI-13 Observer A4 Law 19
 *
 * Three reasons, and each one is a read of the party's own book.
 *
 * B1: AN ISSUER WITH THE WRONG KIND OF DEBT. A borrower that owes a fixed coupon and would rather
 * owe a floating one pays floating and receives fixed, and the other way round. What it swaps is
 * what it actually owes — its own rows, read from the register — and not a number somebody gave it.
 *
 * B3: A BANK MANAGING ITS GAP. What reprices and when is a read of its own book: a bank funded
 * overnight and lent long is short of fixed, and it says so by paying fixed.
 *
 * B4: A VIEW. A party whose outlook of the rate path differs from the curve. This is the side that
 * makes the book a market rather than a queue of hedgers all wanting the same thing (XI-13, §46 A3).
 *
 * B2, B2.a — the pension fund matching a long liability — is DECLARED PARTIAL here and built at
 * 13h, where there is a pension fund to have one.
 */
import {
  type Amount,
  absolute,
  amountOf,
  asPerNamedUnit,
  asPerPiece,
  minus,
  type PerPiece,
  plus,
} from '../../core/measure.js';
import { contractOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { UnitId } from '../../core/ids.js';
import { addQty, asQty, negQty, NO_QTY, type Qty } from '../../core/tick.js';
import type { ParticipantView } from '../../world/context.js';
import { floatingRate, isIrs, type IrsTerms } from './contract.js';
import { irsLineOf } from './data.js';
import { about } from '../../world/context.js';

/**
 * B1: WHAT THIS PARTY OWES AT A FIXED RATE, in this money — its own liabilities, read from the
 * register. A firm that has issued a coupon bond is paying fixed whether or not it wanted to.
 */
function fixedDebtOf(view: ParticipantView, t: IrsTerms): Qty {
  let owed = NO_QTY;
  // Law 18, Law 19 (0g.6): ITS OWN LINES, off the register's index of who promised what. It
  // walked every instrument in the world to find the ones this party issued, and it is asked
  // once per party per swap book per period: 19.8 million instruments in one period of the
  // (24, 96) rung, against 3,367 in the store.
  for (const i of view.instruments.issuedBy(view.self.id)) {
    if (!i.status.live || i.ccy !== t.ccy) continue;
    const kind = view.registry.instrumentKind(i.kind);
    if (!kind.liabilityOfIssuer) continue;
    // A1.d: what makes a liability FIXED is that its own terms name a rate. One that reprices is
    // already floating and does not need a swap to become one.
    owed = addQty(owed, i.issued, 'what it owes at a rate its terms fixed');
  }
  return owed;
}

/** Observer A4: the swapped position it already has in this money, signed by the leg it pays. */
function swapped(view: ParticipantView, t: IrsTerms): Qty {
  let net = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isIrs(c.terms) || c.terms.ccy !== t.ccy) continue;
    const iAmA = c.a === view.self.id;
    const iPayFixed = iAmA === c.terms.paysFixed;
    net = addQty(
      net,
      iPayFixed ? c.notional : negQty(c.notional, 'the other side of it'),
      'fixed it has already agreed to pay',
    );
  }
  return net;
}

export function irsOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isIrs(decl.terms)) return [];
  const t = decl.terms;
  const fixing = floatingRate(t, {
    lastEvent: (kind, subject) => view.lastPublicAbout(kind, subject),
  });
  /**
   * A1.c, E2, Law 3, XI-13: WHAT THIS PARTY NAMES, and it is never read off this book.
   *
   * The FLOATING LEG is the anchor: what the overnight book actually paid this period is a
   * transacted rate both sides can see — a read of ANOTHER market — and a fixed rate is what
   * somebody will swap it for. That is a number this party has whether or not this book has ever
   * printed. E2 is satisfied rather than dodged: nothing here turns a discount curve into a par
   * rate, because there is no discount curve in this module to turn.
   *
   * IT USED TO BE THE FALLBACK, behind this book's own last print. That is XI-13's fixed point —
   * the print moves the outlook, the outlook moves the view, the view moves the quote — and it is
   * the order `banks/dealing-quote.ts` reversed after it walked a bill to a price no yield could
   * discount. A party with nothing of its own to say posted AT the print, so a book whose members
   * had never traded in it printed one number for ever.
   */
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  const outlook = view.outlook(about({ on: 'price', instrument: irsLineOf(t.ccy, t.tenorYears) }));
  // Law 8: a level is held in MONEY PIECES PER PIECE OF THE THING. Two per cent a year on a unit
  // of notional is two cents, and a schedule posted at 0.02 is below this book's own tick.
  const mine: PerPiece = fixing.some
    ? view.registry.priceOf(m.ccy, unit, asPerNamedUnit(fixing.value, 'what the fixing says'))
    : outlook.some
      ? asPerPiece(outlook.value.expected, 'where its own outlook puts this book')
      : asPerPiece(0, 'a party with neither a fixing nor a view has no level');
  if (mine <= 0) return [];
  // Clearing E1: WHERE THE MARKET IS — the comparator that decides which side this party is on and
  // how hard, and never the level it posts.
  const at = view.print(irsLineOf(t.ccy, t.tenorYears));
  const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
  const held = swapped(view, t);
  /**
   * ONE PARTY, ONE POSITION (Law 4, Clearing A2). A borrower hedging its own coupon and a desk
   * with a view on the rate path are two reasons, and when one party has both they are two
   * reasons for ONE position — so what it posts is the distance from where it is to where its
   * own arithmetic says it wants to be, in whichever direction that is.
   *
   * B1: it owes fixed and would rather owe floating, so it wants to RECEIVE fixed — a negative
   * position in "fixed paid". B3, B4: a view that the floating leg will average above this book
   * pulls the other way, and how hard is what its own capital will carry.
   */
  // A-66's shape: what it WANTS is a target and not a grid quantity, so it carries the dimension
  // without asserting the tick — `registry.deliverable` is where a target becomes one (stage 10a).
  let want: Amount<'piece'> = negQty(fixedDebtOf(view, t), 'it owes fixed and would rather not');
  const price = mine;
  if (at.some) {
    const book = at.value.price;
    const conviction = sizeOf(view, unit, mine);
    if (mine > plus(book, tick, 'above the book by a tick it can act on')) {
      want = plus(want, conviction, 'and the fixed it would pay on its own view');
    } else if (mine < minus(book, tick, 'below the book by a tick it can act on')) {
      want = minus(want, conviction, 'and the fixed it would receive on its own view');
    }
  }
  const move = minus(want, held, 'from the fixed it pays to the fixed it wants to pay');
  /**
   * A-66, XI-13, §46 A3: AND A PARTY WITH NOTHING TO CHANGE QUOTES BOTH WAYS AROUND ITS OWN NUMBER.
   *
   * The conviction term stands behind `at.some`, so in a book that has never printed every party is
   * left with `−fixedDebtOf(view, t)` — one sign for everybody — and the session is sell-only and
   * never crosses. The IRS books in this world have never run. A party with no fixed debt to hedge
   * and capital to carry a position makes the market instead: a bid a tick inside its own number
   * and an ask a tick outside, which is a spread and not a crossing. Its number is the FLOATING
   * LEG's — what the overnight book actually paid, a read of another market — and never this
   * book's own print, which is the fixed point this file's header already refuses.
   */
  if (move === 0) {
    const room = sizeOf(view, unit, mine);
    if (room <= 0) return [];
    const bid = minus(price, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(room) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(price, tick, 'a tick outside its own number'),
        qty: asQty(room),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(move, 'either way'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * Derivative Layer E1: what it would carry, from whatever stands behind a position of its own at
 * the rate it is quoting — its equity account, or for a pool its investors' money, answered by the
 * module that runs it (item 13.2b). It is arithmetic and never a notional limit somebody wrote down.
 */
function sizeOf(view: ParticipantView, unit: UnitId, level: PerPiece): Qty {
  const own = view.standsBehind();
  if (own.pieces <= 0 || level <= 0) return NO_QTY;
  // `E-11`: THIS BOOK'S LEVEL IS A RATE EXPRESSED AS MONEY PER PIECE — two per cent a year on a
  // unit of notional is two cents, as the comment at `mine` says. So what its capital carries is
  // money over a LEVEL, which is `amountOf` and gives a notional back; `over` would have divided by
  // a pure number and given money. The book cannot say which of the two its level is, and that is
  // the finding; what this site can do is use the operation that matches what it actually holds.
  return view.registry.deliverable(
    amountOf(own, level, 'what a year of this rate on its capital carries'),
  );
}
