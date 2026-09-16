/**
 * What a deal costs, what stands behind it, and the borrowing a buyer that cannot pay for one asks
 * for.
 *
 * @spec Private Equity B1 Private Equity B2 Private Equity B2.a Private Equity B2.b Private Equity B3 Private Equity E1 Corporate Credit A1 Corporate Credit C9 Law 2 Law 6 Law 19
 *
 * §29 B2: *"most of the price is debt raised against the target itself"*, and B2.a says whose it is:
 * *"the debt is the TARGET's liability, not the fund's — which is why a failed buyout kills the firm
 * and not the fund."* So the borrowing this file publishes is published IN THE TARGET'S NAME. A
 * request says who will owe the money, and the answer is the company, not the buyer; a fund that
 * borrowed for its deals would be a fund a failed deal can kill, which is the clause read backwards.
 *
 * B2.b is then not a rule anywhere: *"the deal only happens if lenders will lend, at a price — the
 * credit market decides which buyouts occur."* The bank prices the TARGET, out of the target's own
 * published accounts and its own room for that name, and what comes back is a commitment or nothing.
 * A company nobody will commit to is a company nobody can buy with borrowed money, and no line here
 * says so.
 */
import { asCash, minus, type Cash, noCash, sumCash } from '../../core/measure.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { FACILITY, isFacility } from '../../registry/credit.js';
import type { MechanismContext } from '../../world/context.js';

/**
 * E1, Corporate Credit C9 (17b.1): WHAT A LENDER HAS COMMITTED TO THIS NAME AND NOT YET LENT.
 *
 * The commitments are read off the agreement store rather than an event, because what matters is
 * that the promise still STANDS — a lapsed one is a promise that was made and is over (Law 19). A
 * commitment past the period it was made for is not drawn here even though the lender has not yet
 * struck it off its own book: the borrower knows when its commitment papers expired.
 */
export function committedTo(ctx: MechanismContext, target: PartyId, ccy: CurrencyCode): Cash {
  const standing: Cash[] = [];
  for (const a of ctx.agreements.ofKind(FACILITY)) {
    if (a.state !== 'performing' || a.debtor !== target || a.ccy !== ccy) continue;
    const t = a.terms;
    if (!isFacility(t) || ctx.period > t.until) continue;
    standing.push(t.limit);
  }
  if (standing.length === 0) return noCash(ccy);
  return sumCash(ccy, standing, 'what lenders have committed to this name for a deal').value;
}

/**
 * B2, B2.a, Corporate Credit A1 (17b.2): THE COMPANY ASKS FOR THE MONEY THAT WILL BUY IT.
 *
 * One ask per target and not one per bidder: a request is a fact about who will owe the money, and
 * a company cannot owe two deals' debt at once. Where two buyers want the same firm the ask is the
 * biggest of the holes they would have to fill, because that is what the company would have to
 * carry if the deal that happens is the dearest of them — and the two of them meet in the book
 * afterwards (B4), not here.
 *
 * It asks for a COMMITMENT and not for money (`wants`), which is the whole of why a tender can be
 * conditional: what a bank promises here is drawn inside the instruction that completes the
 * purchase, or it is never drawn at all.
 */
export function askToFund(ctx: MechanismContext, target: PartyId, hole: Cash, buyer: PartyId): void {
  if (hole.pieces <= 0) return;
  const who = ctx.parties.get(target);
  if (!who.status.alive) return;
  // Corporate Credit A1: its own money, and a deal in another one is 17b.7's (Cross-Border C4).
  if (ctx.registry.currencyOf(who.region) !== hole.ccy) return;
  /**
   * C9: IT DOES NOT ASK TWICE WHILE ITS LAST ASK IS UNANSWERED. A borrower publishes in one period
   * and hears in the next, so an ask repeated every period is the same money asked for three times
   * over and three lenders' capital held against one deal. What it already has committed is
   * subtracted by the caller; what it asked for last period is still out.
   */
  const asked = ctx.journal.lastOf('control.financing', String(target));
  if (asked !== undefined && asked.period >= ctx.period - 1) return;
  ctx.request(target, {
    ccy: hole.ccy,
    short: hole,
    // B2: a buyout's debt is a term loan the company pays down, never a line it draws at will.
    repays: 'onSchedule',
    // E1: a lender that has agreed to lend and has not lent, so that a deal can be conditional.
    wants: 'commitment',
  });
  ctx.record(
    'control.financing',
    [String(target), String(buyer)],
    {
      target: String(target),
      buyer: String(buyer),
      wanted: hole.pieces,
      ccy: hole.ccy,
    },
    true,
  );
}

/** What the buyer still has to find: the price, less what it holds and what a lender has promised. */
export const holeIn = (cost: Cash, has: Cash, committed: Cash): Cash =>
  minus(
    minus(cost, has, 'what it cannot pay for out of what it holds'),
    committed,
    'and what a lender has already promised the company',
  );

/** A buyer's own money, as money rather than a count — one crossing, named (Law 8). */
export const cashOf = (pieces: number, ccy: CurrencyCode): Cash =>
  asCash(pieces, ccy, 'what the buyer holds toward it');
