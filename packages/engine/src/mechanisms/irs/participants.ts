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
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { UnitId } from '../../core/ids.js';
import { add, div, sub } from '../../core/num.js';
import { asQty } from '../../core/tick.js';
import { issuedBy } from '../../register/instruments.js';
import type { ParticipantView } from '../../world/context.js';
import { floatingRate, isIrs, type IrsTerms } from './contract.js';
import { irsLineOf } from './data.js';

/**
 * B1: WHAT THIS PARTY OWES AT A FIXED RATE, in this money — its own liabilities, read from the
 * register. A firm that has issued a coupon bond is paying fixed whether or not it wanted to.
 */
function fixedDebtOf(view: ParticipantView, t: IrsTerms): number {
  let owed = 0;
  for (const i of view.instruments.all()) {
    if (!i.status.live || i.ccy !== t.ccy) continue;
    if (!issuedBy(i, view.self.id)) continue;
    const kind = view.registry.instrumentKind(i.kind);
    if (!kind.liabilityOfIssuer) continue;
    // A1.d: what makes a liability FIXED is that its own terms name a rate. One that reprices is
    // already floating and does not need a swap to become one.
    owed = add(owed, i.issued, 'what it owes at a rate its terms fixed');
  }
  return owed;
}

/** Observer A4: the swapped position it already has in this money, signed by the leg it pays. */
function swapped(view: ParticipantView, t: IrsTerms): number {
  let net = 0;
  for (const c of view.contracts.mine()) {
    if (!isIrs(c.terms) || c.terms.ccy !== t.ccy) continue;
    const iAmA = c.a === view.self.id;
    const iPayFixed = iAmA === c.terms.paysFixed;
    net = add(net, iPayFixed ? c.notional : -c.notional, 'fixed it has already agreed to pay');
  }
  return net;
}

export function irsOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = m.contract;
  if (decl === undefined || !isIrs(decl.terms)) return [];
  const t = decl.terms;
  const fixing = floatingRate(t, { lastEvent: (kind, subject) => view.lastPublicAbout(kind, subject) });
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
  const outlook = view.outlook(`price.${String(irsLineOf(t.ccy, t.tenorYears))}`);
  // Law 8: a level is held in MONEY PIECES PER PIECE OF THE THING. Two per cent a year on a unit
  // of notional is two cents, and a schedule posted at 0.02 is below this book's own tick.
  const mine = fixing.some
    ? view.registry.priceOf(m.ccy, unit, fixing.value)
    : outlook.some
      ? outlook.value.expected
      : 0;
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
  let want = -fixedDebtOf(view, t);
  const price = mine;
  if (at.some) {
    const book = at.value.price;
    const conviction = sizeOf(view, unit, mine);
    if (mine > add(book, tick, 'above the book by a tick it can act on')) {
      want = add(want, conviction, 'and the fixed it would pay on its own view');
    } else if (mine < sub(book, tick, 'below the book by a tick it can act on')) {
      want = sub(want, conviction, 'and the fixed it would receive on its own view');
    }
  }
  const move = sub(want, held, 'from the fixed it pays to the fixed it wants to pay');
  if (move === 0) return [];
  const qty = view.registry.deliverable(unit, move > 0 ? move : -move);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * Derivative Layer E1: what it would carry, from its own capital at the rate it is quoting. It is
 * arithmetic on its own balance sheet and never a notional limit somebody wrote down.
 */
function sizeOf(view: ParticipantView, unit: UnitId, rate: number): number {
  const own = view.equity();
  if (own <= 0 || rate <= 0) return 0;
  return view.registry.deliverable(unit, div(own, rate, 'what a year of this rate on its capital carries'));
}
