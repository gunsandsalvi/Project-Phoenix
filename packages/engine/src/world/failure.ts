/**
 * Has this party failed? One question, one answer, asked of the kind's own profile.
 *
 * @spec XI-3 Firm D4 Banks Capital C1 Banks Capital C1.a Money E1 Law 4 Law 15
 *
 * XI-3 says nothing is immortal and that the exceptions must be named rather than left out; each
 * party kind states what it can FAIL on (`PartyKindProfile.fails`), and this asks the two questions
 * those answers name. It lives in the kernel because more than one module needs the answer and two
 * modules asking it two ways would be two definitions of death (Law 4): the estate opens on it, and
 * a bank's resolution has to know before the estate does, because a bank does not go to an estate
 * (Banks Capital C3.b — deposits keep working).
 *
 * C1.a is why there are two questions and not one. A party can be SOLVENT AND ILLIQUID — it holds
 * more than it owes and cannot pay what fell due today — or INSOLVENT AND LIQUID, with cash in hand
 * and liabilities past its assets. The two have different triggers and different remedies, and
 * whichever fired is named in the reason so the resolution can say which one it was.
 */
import { sum, withinDust } from '../core/num.js';
import { unpaid } from '../ledger/instruction.js';
import type { MechanismContext, ParticipantView } from './context.js';

/** What it failed to pay this period out of its own balance, and has not since covered (Money E1). */
export function stillOwed(view: ParticipantView): number {
  const mine = view.failedPayments(Number.MAX_SAFE_INTEGER);
  const terms: number[] = [];
  for (const f of mine) {
    if (f.instruction.period !== view.period) continue;
    if (f.reason.kind !== 'overdraftRefused') continue;
    for (const owed of unpaid(f)) if (owed.payer === view.self.id) terms.push(owed.amount);
  }
  return sum(terms).value;
}

/**
 * XI-3, C1.a: the reason this party has failed, or nothing. A kind that names neither trigger cannot
 * die, which is how XI-3's two exceptions — the central bank, and a treasury in its own money — are
 * named consequences of what they ARE rather than omissions.
 */
export function failedWhy(ctx: MechanismContext, view: ParticipantView): string | undefined {
  const can = ctx.registry.partyKind(view.self.kind).fails ?? [];
  const ccy = ctx.registry.region(view.self.region).ccy;
  if (can.includes('cash')) {
    const owed = stillOwed(view);
    if (owed > 0 && owed > view.cash(ccy)) {
      return `it could not pay ${owed} that fell due and still cannot`;
    }
  }
  if (can.includes('solvency')) {
    // Law 7: the equity account is a walk, and what it may call nothing is what that walk has cost
    // it in rounding. A party whose equity is ZERO by construction — a fund (Fund Shares A3) — sits
    // at the dust either side of it every period, and a test that called that insolvency would kill
    // one every week. It is the same tolerance the accounts family compares it against (Law 4).
    const walk = view.equityWalk();
    if (
      walk.value < 0 &&
      !withinDust(walk.value, 0, ctx.valuation.equityDust(view.self.id, walk, ctx.period))
    ) {
      return `its liabilities are past its assets by ${-walk.value}`;
    }
  }
  return undefined;
}
