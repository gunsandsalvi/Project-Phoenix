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
import type { Period } from '../calendar/calendar.js';
import type { CurrencyCode } from '../core/ids.js';
import { sum, withinDust } from '../core/num.js';
import { isArrear } from '../register/arrears.js';
import type { MechanismContext, ParticipantView } from './context.js';

/** What it failed to pay this period out of its own balance, and has not since covered (Money E1). */
/**
 * Money E1, XI-3 (12a.2): WHAT IT STILL OWES ON PAYMENTS IT MISSED — the arrears it issued, in this
 * money, read off the register (Law 19). It was a walk over its failed instructions of THIS period
 * only, so a payer that missed a wage last week and had the money this week was never short and
 * one that missed today was; the rows are what is owed, whenever it fell due.
 */
export function stillOwed(view: ParticipantView, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const i of view.instruments.issuedBy(view.self.id)) {
    if (!i.status.live || !isArrear(i) || i.ccy !== ccy) continue;
    terms.push(i.issued);
  }
  return sum(terms).value;
}

/**
 * XI-3, C1.a: the reason this party has failed, or nothing. A kind that names neither trigger cannot
 * die, which is how XI-3's two exceptions — the central bank, and a treasury in its own money — are
 * named consequences of what they ARE rather than omissions.
 */
/** Money E1.b: every failure this party has ever had — the epoch is where its history starts. */

export function failedWhy(ctx: MechanismContext, view: ParticipantView): string | undefined {
  const can = ctx.registry.partyKind(view.self.kind).fails ?? [];
  const ccy = ctx.registry.currencyOf(view.self.region);
  if (can.includes('cash')) {
    const owed = stillOwed(view, ccy);
    if (owed > 0 && owed > view.cash(ccy)) {
      return `it could not pay ${owed} that fell due and still cannot`;
    }
  }
  if (can.includes('solvency')) {
    /**
     * Law 7: the equity account is a walk, and what it may call nothing is what that walk has cost
     * it in rounding. A party whose equity is ZERO by construction — a fund (Fund Shares A3) — sits
     * at the dust either side of it every period, and a test that called that insolvency would kill
     * one every week. It is the same tolerance the accounts family compares it against (Law 4).
     *
     * ITEM 13.4: AND A LEVERED POOL IS NO LONGER ALWAYS AT THAT DUST. Its equity was zero by
     * construction however far underwater it went, because its share liability absorbed every loss
     * — so this test could never fire for one, and §28 E3's *"no fund that cannot fail"* was not
     * true of any fund in this world. A share cannot be worth less than nothing (`nav.ts`: a
     * limited liability), so a pool whose book no longer covers what it owes has a REAL negative
     * here, and dies of it like anything else. Nothing was added to make that happen.
     */
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
