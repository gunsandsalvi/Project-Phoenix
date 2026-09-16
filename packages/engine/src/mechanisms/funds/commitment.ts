/**
 * Committed capital: money an investor has promised and not yet paid, and the call that makes it pay.
 *
 * @spec Private Equity A1 Private Equity A2 Private Equity A2.a Private Equity A2.b Private Equity A3 Private Equity A4 Private Equity E2 Fund Shares C1 Fund Shares G1.b Money E1 Law 4 Law 6 XI-8
 *
 * ITEM 13.5. §29 A is not about what a private-equity fund BUYS — it is about how it is funded, and
 * that is the one thing in this world that has no analogue anywhere else: **every other payment here
 * is bounded by what the payer has, and this one is not.**
 *
 * A2: *"capital is committed, not paid — it is called when a deal needs it, and the call is a real
 * payment from the investor's account on a date it cannot refuse."*
 *
 * A2.b IS A FORBID AND IT IS THE ONE TO GET RIGHT: *"a call bounded by the investor's spare cash is
 * not an obligation."* So the call goes to the wire FOR THE WHOLE AMOUNT. It is not trimmed to what
 * the investor happens to hold, it is not met in part, and nothing here looks at the investor's
 * balance before demanding it. What an investor that cannot pay gets is a REFUSED INSTRUCTION — a
 * recorded failed payment (Money E1) which is its own cash-failure trigger — and that refusal is the
 * DEFAULT the clause names, with the consequences every other failed payment in this world has.
 *
 * Compare what every other payer here does: a household spends what it has, a fund redeems what its
 * cash reaches, a bank lends what its room allows. Each of those is a budget and correctly so. A
 * capital call is the one obligation in this model that is not, and the difference is the whole
 * clause — so there is no `atMost` anywhere in this file, and its absence is the mechanism.
 *
 * A3: the pool issues shares against what it is PAID, at the NAV, through the ordinary subscription
 * door (Fund Shares C1) — so a call is a subscription the investor agreed in advance to make, and
 * there is one writer of what a share is issued at (Law 4).
 */
import { noCash, sumCash } from '../../core/measure.js';
import {
  agreementKindId,
  type CurrencyCode,
  moneyInstrumentId,
  type PartyId,
} from '../../core/ids.js';
import {
  asCash,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  type PerPiece,
  plus,
  scale,
} from '../../core/measure.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { MechanismContext } from '../../world/context.js';
import type { Option } from '../../core/option.js';

/** A1, A2: what one named investor promised this pool, and what of it has been drawn. */
export const COMMITMENT = agreementKindId('funds.commitment');

export interface CommitmentTerms extends AgreementTerms {
  readonly kind: typeof COMMITMENT;
  /** A2: what it promised. Not what it paid — that is the point of the clause. */
  readonly committed: Cash;
  /**
   * A2: what has actually been called AND PAID of it. It is written here because it is a fact this
   * module produces — the sum of calls that settled — and there is nowhere else to read it: a call
   * is a payment, not an instrument, so the register has no row to count. It is the same shape
   * `BackstopTerms.drawn` has for the same reason.
   */
  readonly drawn: Cash;
}

export const isCommitment = (t: AgreementTerms): t is CommitmentTerms =>
  'committed' in t && 'drawn' in t;

/** One commitment as this module reads it: who promised, to which pool, and how much is left. */
export interface Commitment extends CommitmentTerms {
  readonly id: Agreement['id'];
  readonly investor: PartyId;
  readonly pool: PartyId;
}

export function commitmentOf(a: Agreement): Commitment | undefined {
  return isCommitment(a.terms)
    ? { ...a.terms, id: a.id, investor: a.debtor, pool: a.creditor }
    : undefined;
}

/**
 * A1, Law 19: every commitment made to this pool. The INVESTOR is the debtor — it owes the pool
 * money it has promised — and the pool is the creditor, which is what a commitment IS and which is
 * why it is an agreement rather than a number on the fund's declaration.
 */
export function commitmentsTo(ctx: MechanismContext, pool: PartyId): readonly Commitment[] {
  const out: Commitment[] = [];
  for (const a of ctx.agreements.ofKind(COMMITMENT)) {
    if (a.state !== 'performing' || a.creditor !== pool) continue;
    const row = commitmentOf(a);
    if (row !== undefined) out.push(row);
  }
  return out;
}

/** A2: what this investor has promised and not yet paid — what a call can still be for. */
export const undrawnOn = (c: Commitment): Cash =>
  minus(c.committed, c.drawn, 'what it promised and has not yet paid');

/** A2, A4: what the pool could still call in total, across everybody who committed to it. */
export const undrawnTo = (rows: readonly Commitment[], ccy: CurrencyCode): Cash =>
  sumCash(ccy, rows.map(undrawnOn), 'what is promised and not yet paid').value;

/**
 * A1, Seed A3, XI-8: OPEN THE COMMITMENTS THIS WORLD WAS RAISED WITH.
 *
 * A closed-end fund was RAISED before it existed — that is what a vintage is — so the commitments
 * behind the one this world opens with are an opening condition, the way a bank's balance sheet is.
 * It is done in a phase rather than the seed for one reason and it is not a preference: the
 * investors are parties another module creates, and a module may not seed into another's population
 * (the ordering would decide whether the row could be written at all). The phase is idempotent and
 * runs every period, which is the shape `grantBackstops` already has for the same reason.
 *
 * WHY THE INVESTORS COMMITTED IS NOT MODELLED AND IS NOT PRETENDED TO BE. An institution deciding to
 * lock money up for the life of a fund is a decision with its own reasons, and the ones this world
 * could give it — a pension matching a very long liability (14.1), a deal pipeline worth funding
 * (13.5b) — are not built. What is built is what happens AFTER it committed, which is A2's whole
 * subject.
 */
export function openCommitments(
  ctx: MechanismContext,
  pool: PartyId,
  ccy: CurrencyCode,
  promised: Readonly<Record<string, number>>,
): void {
  const already = new Set(commitmentsTo(ctx, pool).map((c) => String(c.investor)));
  for (const [who, share] of Object.entries(promised)) {
    if (already.has(who) || share <= 0) continue;
    const investor = who as PartyId;
    if (!ctx.parties.has(investor) || !ctx.parties.get(investor).status.alive) continue;
    // 14.1, Seed A2: WHAT IT PROMISED IS A SHARE OF WHAT IT HAD when the promise was opened — read
    // off its own account, so no institution opens owing a call it could never meet. One that has
    // nothing yet promises nothing yet, and is asked again next period (the phase is idempotent).
    const amount = ctx.registry.cashFor(
      scale(
        heldAsMoney(ctx.participant(investor).cash(ccy), ccy, 'what it has to put to work'),
        asRatio(share, 'the share of it promised'),
        'what it promised',
      ),
    );
    if (amount <= 0) continue;
    const terms: CommitmentTerms = {
      kind: COMMITMENT,
      committed: asCash(amount, ccy, 'what it promised this fund'),
      drawn: noCash(ccy),
    };
    ctx.owes({
      debtor: investor,
      creditor: pool,
      ccy,
      // A2: it owes nothing NOW. What it owes is what it is called for, when it is called — which
      // is exactly the difference between a commitment and a debt (Law 2).
      owed: 0,
      terms,
      why: `${investor} committed ${amount} to ${pool} for the life of the fund`,
    });
    ctx.record(
      'fund.committed',
      [pool, investor],
      { fund: pool, investor, committed: amount },
      true,
    );
  }
}

/**
 * A2 (17b.4): WHAT ONE INVESTOR IS CALLED FOR — the whole of what it still promised where the deal
 * needs everything the fund has left to call, and its SHARE of what is still promised otherwise.
 *
 * Two real quantities and a branch between them, and that is the whole of it: nothing here is a
 * bound on either (Law 6) and nothing looks at what the investor holds (A2.b). A call sized by the
 * DEAL is not a call sized by the investor's balance — which is why this is arithmetic on two
 * promises and the balance appears nowhere in it.
 */
export const calledFrom = (mine: Cash, promised: Cash, short: Cash): Cash =>
  short.pieces >= promised.pieces
    ? mine
    : scale(
        mine,
        asRatio(short.pieces / promised.pieces, 'the share of the promises this deal needs'),
        'what this investor is called for',
      );

/**
 * A2, A2.b, A3, E2 (item 13.5): THE CAPITAL CALL — and it is the one payment in this world that is
 * not bounded by what the payer has.
 *
 * *"Capital is committed, not paid — it is called when a deal needs it, and the call is a real
 * payment from the investor's account on a date it cannot refuse."* A2.b makes the FORBID explicit:
 * *"a call bounded by the investor's spare cash is not an obligation."*
 *
 * So the instruction goes to the wire for the WHOLE amount. Nothing here reads the investor's
 * balance first, nothing trims the demand to fit it, and nothing pays it in part. An investor that
 * cannot pay gets a REFUSED instruction — a recorded failed payment, which is its own cash-failure
 * trigger (Money E1) — and that refusal IS the default the clause names. There is no `atMost` in
 * this path and its absence is the mechanism.
 *
 * E2: *"no capital call not paid from a real balance."* What settles came out of a named account
 * and went into the pool's; what did not settle moved nothing at all.
 *
 * A3: and the pool issues SHARES against what it was paid, at the NAV, through the ordinary
 * subscription door — so a call is a subscription its investor agreed in advance to make, and there
 * is one writer of what a share is issued at (Law 4).
 *
 * WHEN IT CALLS IS THE DEAL (17b.4). *"Capital is committed, not paid: it is called when A DEAL
 * NEEDS IT, and the call is a real payment from the investor's account on a date it cannot refuse."*
 * What a deal needs of this pool is handed in — the cheque the buyer published it must bring
 * (`financingBy`) — and a pool with no deal calls NOTHING, which is the clause rather than a
 * placeholder for it. It used to call the whole of what was promised the moment its cash reached
 * zero, which for a fund with nothing to buy was once, at the start, and left the investors' money
 * sitting in the pool from period 1 for the rest of the run.
 *
 * WHAT EACH INVESTOR IS CALLED FOR is its share of what is still promised — `undrawn_i / undrawn`
 * of the deal's need — and a deal bigger than everything still promised calls all of it. That is a
 * branch on two real quantities and not a bound on either (Law 6): what is called is either the
 * whole of the promise or a share of it, never a demand trimmed to fit something.
 */
export function callCapital(
  ctx: MechanismContext,
  pool: PartyId,
  ccy: CurrencyCode,
  perShare: PerPiece,
  /**
   * A2: WHAT THE DEAL NEEDS OF IT. Nothing is a real answer and is the ordinary one: a pool with no
   * deal in front of it calls nothing, because capital is committed and not paid.
   */
  needs: Option<Cash>,
  /**
   * A3: what the pool does with what a call brought in — issue shares for it, at the NAV, through
   * the ordinary subscription door. It is handed in rather than imported so that THIS FILE IS THE
   * WHOLE CALL PATH and can be guarded as one (`tools/check-forbids.ts`, A2.b): a reader checking
   * that nothing here trims a demand to fit a balance has one file to read.
   */
  issue: (investor: PartyId, paid: Cash, at: PerPiece) => void,
): void {
  if (!needs.some || perShare <= 0) return;
  const cash = ctx.register.quantity(pool, moneyInstrumentId(ctx.accountOf(pool, ccy).issuer, ccy));
  // A2: what the deal needs that it has not already got. A pool holding what it last called does
  // not call again for the same deal, and one holding enough for this one calls nothing at all.
  const short = minus(
    needs.value,
    heldAsMoney(cash, ccy, 'what it is already holding toward it'),
    'what the deal needs that it has not got',
  );
  if (short.pieces <= 0) return;
  const rows = commitmentsTo(ctx, pool);
  const promised = undrawnTo(rows, ccy);
  if (promised.pieces <= 0) return;
  for (const c of rows) {
    const mine = undrawnOn(c);
    if (mine.pieces <= 0) continue;
    const wanted = ctx.registry.payable(calledFrom(mine, promised, short));
    if (wanted <= 0) continue;
    const investor = ctx.parties.get(c.investor);
    /**
     * A2.b: FOR THE WHOLE OF WHAT IT WAS CALLED FOR. The one thing this line must not do is look at
     * what the investor holds — a call sized by the DEAL is not a call sized by the investor's
     * balance — and the one thing it must not be is two instructions, because a call met in halves
     * is a call the investor got to size.
     */
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(investor.id, ccy),
          to: ctx.accountOf(pool, ccy),
          ccy,
          amount: wanted,
        },
      ],
      cause: 'transfer',
      reason: `${pool} calls ${wanted} of ${investor.id}'s commitment`,
    });
    const paid = r.outcome === 'settled';
    if (paid) {
      // A2, Law 19: what it has drawn is what its calls have actually settled for, and this is the
      // one writer of that fact.
      const now: CommitmentTerms = {
        kind: COMMITMENT,
        committed: c.committed,
        drawn: plus(
          c.drawn,
          heldAsMoney(wanted, c.drawn.ccy, 'what this call brought in'),
          'what it has paid in',
        ),
      };
      ctx.restate(c.id, now);
      // A3: shares against what it paid, at the NAV, through the door every subscriber uses.
      issue(investor.id, heldAsMoney(wanted, c.drawn.ccy, 'what it paid'), perShare);
    }
    ctx.record(
      'fund.called',
      [pool, investor.id],
      { fund: pool, investor: investor.id, called: wanted, ccy: c.drawn.ccy, paid, perShare },
      true,
    );
  }
}
