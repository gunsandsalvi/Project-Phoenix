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
import {
  agreementKindId,
  type CurrencyCode,
  moneyInstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { asCash, type Cash, heldAsMoney, minus, type PerPiece, plus } from '../../core/measure.js';
import { none } from '../../core/option.js';
import { sum } from '../../core/num.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { MechanismContext } from '../../world/context.js';

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
export const undrawnTo = (rows: readonly Commitment[]): Cash =>
  sum(rows.map(undrawnOn)).value;

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
  for (const [who, amount] of Object.entries(promised)) {
    if (already.has(who) || amount <= 0) continue;
    const investor = who as PartyId;
    if (!ctx.parties.has(investor) || !ctx.parties.get(investor).status.alive) continue;
    const terms: CommitmentTerms = {
      kind: COMMITMENT,
      committed: asCash(amount, 'what it promised this fund'),
      drawn: asCash(0, 'and it has paid none of it yet'),
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
 * WHEN IT CALLS is the part this world cannot yet pace honestly, and it is said rather than dressed
 * up. A closed-end fund calls when a DEAL needs the money (A2), and there are no deals: §29 B's
 * buyout is 13.5b. What is here is the limit case — it calls what it has not called when it has run
 * out of cash to invest, which for a fund with nothing to buy yet is once, at the start. The
 * SCHEDULE arrives with the deals; the OBLIGATION is what this step is about and it is whole.
 */
export function callCapital(
  ctx: MechanismContext,
  pool: PartyId,
  ccy: CurrencyCode,
  perShare: PerPiece,
  /**
   * A3: what the pool does with what a call brought in — issue shares for it, at the NAV, through
   * the ordinary subscription door. It is handed in rather than imported so that THIS FILE IS THE
   * WHOLE CALL PATH and can be guarded as one (`tools/check-forbids.ts`, A2.b): a reader checking
   * that nothing here trims a demand to fit a balance has one file to read.
   */
  issue: (investor: PartyId, paid: Cash, at: PerPiece) => void,
): void {
  const cash = ctx.register.quantity(pool, moneyInstrumentId(ctx.accountOf(pool, ccy).issuer, ccy));
  // A2: it calls when it needs the money and not before. A fund still holding what it last called
  // has not needed any since — which is as close to deal-paced as a world with no deals can be.
  if (cash > 0 || perShare <= 0) return;
  for (const c of commitmentsTo(ctx, pool)) {
    const wanted = ctx.registry.payable(undrawnOn(c));
    if (wanted <= 0) continue;
    const investor = ctx.parties.get(c.investor);
    /**
     * A2.b: FOR THE WHOLE OF IT. The one thing this line must not do is look at what the investor
     * holds — and the one thing it must not be is two instructions, because a call met in halves is
     * a call the investor got to size.
     */
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(investor.id, ccy),
          to: ctx.accountOf(pool, ccy),
          ccy,
          amount: wanted,
          fromCell: none(),
          toCell: none(),
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
        drawn: plus(c.drawn, heldAsMoney(wanted, 'what this call brought in'), 'what it has paid in'),
      };
      ctx.restate(c.id, now);
      // A3: shares against what it paid, at the NAV, through the door every subscriber uses.
      issue(investor.id, heldAsMoney(wanted, 'what it paid'), perShare);
    }
    ctx.record(
      'fund.called',
      [pool, investor.id],
      { fund: pool, investor: investor.id, called: wanted, paid, perShare },
      true,
    );
  }
}
