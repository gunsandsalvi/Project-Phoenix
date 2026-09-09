/**
 * What a firm does with its own shares: pay out, buy back, or sell more.
 *
 * @spec Equity D1 Equity D1.a Equity D1.b Equity D1.c Equity D2 Equity D2.a Equity D2.b Equity D2.c Equity D3 Equity D3.a Equity D3.b Firm E4 Firm E5 Firm E6 Clearing B2 Clearing C4.a XI-15 Law 4
 *
 * ALL THREE ARE ONE DECISION and it has one reason (E6: made from the firm's own state and the
 * prices it faces). What it has spare it distributes, over as many of its own periods as its
 * management is patient; what it is short of it must fund. WHICH way it distributes, and whether it
 * funds with equity at all, turns on one comparison: what the market says a share of it is worth
 * against what its own books say one is worth.
 *
 * - Cheap (the market below its book): it BUYS ITS OWN BACK. The cash is gone and the count falls,
 *   which is D2 exactly: a buyback is a distribution and never an investment (D2.b), and what each
 *   remaining share is a claim on grows (D2.a).
 * - Dear (the market above its book): it PAYS A DIVIDEND, and if it is also short of money it SELLS
 *   NEW SHARES — which is E4's "how to fund itself, and the choice depends on what each costs" with
 *   the only two costs this world can yet compare.
 *
 * The book value is not a price and never becomes one (B3): it is the firm's own reservation, which
 * is what a participant brings to a market (Clearing B2). The market can refuse it (D1.c).
 */
import type { InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { div, material, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Order } from '../../clearing/solver.js';
import type { ParticipantView } from '../../world/context.js';

/** What the firm decided about its own line this period, in the terms the orders are posted in. */
export interface EquityPlan {
  readonly line: InstrumentId;
  readonly market: MarketId;
  /** What its own books say one share is a claim on: its equity account over the count (Firm C3). */
  readonly bookPerShare: number;
  /** D3: cash per share it is paying out to whoever holds it, this period. */
  readonly dividendPerShare: number;
  /** D2: shares it is bidding for, at `bookPerShare`, to cancel. */
  readonly buyback: number;
  /** D1: new shares it is offering, and the least it will take for one (D1.c: it can fail). */
  readonly issue: number;
  readonly reservation: number;
}

/**
 * E5, E4, D2.c: the decision. `spare` is what the firm's own funding read says it has over what it
 * is about to have to pay — negative when it is short — which is one number with one writer (Law 4)
 * and the same one the banks read when they decide whether to lend to it.
 */
export function decideEquity(
  view: ParticipantView,
  line: InstrumentId,
  market: MarketId,
  issued: number,
  spare: number,
  patience: number,
): Option<EquityPlan> {
  if (issued <= 0 || patience < 1) return none();
  const book = div(view.equity(), issued, 'what its books say a share is a claim on');
  // A firm whose liabilities are past its assets has nothing to distribute and nothing to price a
  // share of itself at. What happens to it is the estate's business (XI-3), not this decision's.
  if (book <= 0) return none();
  const print = view.print(line);
  const dear = print.some && print.value.price > book;
  const plan = {
    line,
    market,
    bookPerShare: book,
    dividendPerShare: 0,
    buyback: 0,
    issue: 0,
    reservation: book,
  };
  if (spare < 0) {
    // E4, D1, D1.b: a funding need it prefers to meet with equity — and it prefers to when the
    // market will pay more for a share than the book says one is worth, because what it gives up
    // then is worth less than what it takes in. Short and cheap, it does not sell: it would be
    // handing away more of itself than the money is worth, and it borrows instead (worklist 6).
    if (!print.some || print.value.price <= book) return none();
    const wanted = sub(0, spare, 'what it is short of');
    const size = div(wanted, print.value.price, 'shares it must sell to raise it');
    if (!material(size, 2, size)) return none();
    return some({ ...plan, issue: size });
  }
  const payout = div(spare, patience, 'what it distributes this period');
  if (!material(payout, 2, spare)) return none();
  if (!dear) {
    // D2, D2.a: cheap to itself, so it buys its own back. It bids at its own reservation, and what
    // it gets is what the session gives it — which may be nothing (B6, Clearing C4.a).
    const shares = div(payout, book, 'shares its payout would buy');
    if (!material(shares, 2, shares)) return none();
    return some({ ...plan, buyback: shares });
  }
  // D3, D3.a: cash per share to whoever holds it. D3.b: the number is the decision, and a firm with
  // less to spare this period declares less — which is the cut others react to.
  return some({ ...plan, dividendPerShare: div(payout, issued, 'the dividend per share') });
}

/**
 * D2, Clearing B2: the firm's own order in its own line. It is a bid at its reservation and it is
 * the only reason a firm is in this book at all — an issuer selling is a PRIMARY offer (D1) and
 * goes through the issuer's own supply, not through an order beside its buyers.
 */
export function buybackOrder(
  self: PartyId,
  shares: number,
  reservation: number,
): readonly Order[] {
  if (shares <= 0 || reservation <= 0) return [];
  return [{ party: self, side: 'buy', price: reservation, qty: shares }];
}

/** D3.a: what one holder is owed of a declared dividend, per member of it (XI-15). */
export function dividendFor(perShare: number, perMemberUnits: number): number {
  return mul(perMemberUnits, perShare, 'the dividend on what it holds');
}
