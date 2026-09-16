/**
 * The report: a READ of the ledger and the register, never a statement management composes.
 *
 * @spec Reporting A1 Reporting A1.a Reporting A2 Reporting A2.a Reporting G2 Reporting G4 Reporting G5 Law 4 Law 9 Law 15 Law 19
 *
 * A2.a is the rule every line here answers to: no reported number the books do not produce. So
 * income is the equity ledger's own entries, the balance sheet is the same read the `accounts`
 * family checks (one implementation, Law 4), and the cash statement is the wire's own money legs
 * with their counterparties. Nothing is assembled by a formula the ledger does not already carry —
 * if a figure needs one, the equity ledger is missing a writer and not the report a calculation.
 */
import type { PartyId } from '../../core/ids.js';
import { issuedBy, type Instrument } from '../../register/instruments.js';
import type { MechanismContext } from '../../world/context.js';

/**
 * A1, A1.a, G4: WHETHER THIS FIRM IS PUBLIC, read from the register and never a flag.
 *
 * A share is a claim on the residual: the one instrument a party issues that is NOT a liability of
 * its issuer and that a market prices. That is asked of the kind's PROFILE and never of its id
 * (Law 15), so a world that one day lists something else is not a world this has to be told about.
 */
export function listedLineOf(ctx: MechanismContext, firm: PartyId): Instrument | undefined {
  for (const i of ctx.instruments.all()) {
    if (!issuedBy(i, firm) || !i.status.live || !i.market.some) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    if (profile.liabilityOfIssuer || profile.pricing !== 'cleared') continue;
    return i;
  }
  return undefined;
}

/**
 * A1.a, 17.0a (the owner's rule): WHO PREPARES ACCOUNTS — a party somebody else's claim or job
 * depends on. It has issued a live claim another party holds, or it employs somebody. Both are
 * reads of the register and the employment rows, in the shape A1.a insists on — *"never a label"* —
 * so there is no kind of party that reports and no flag anywhere saying which are companies. A
 * household cell that has borrowed is in it, and that is right: what its lender underwrites is what
 * it takes in against what it owes, which is this statement.
 *
 * An estate is not: its kind is terminal, it is being wound up, and what it owes is the waterfall's
 * business rather than a going concern's accounts.
 */
export function keepsAccounts(ctx: MechanismContext, party: PartyId): boolean {
  if (ctx.registry.partyKind(ctx.parties.get(party).kind).terminal === true) return false;
  for (const i of ctx.instruments.issuedBy(party)) {
    if (!i.status.live) continue;
    if (!ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    for (const holder of ctx.register.holdersOf(i.id)) if (holder !== party) return true;
  }
  return ctx.employment.by(party).length > 0;
}

/**
 * A1.a: public is a STATE, read every period. A company is public when ANY claim it issued that a
 * market prices — a share or a bond — is held by somebody outside it (17.0a, the owner's rule: a
 * company with public instruments owes its holders its financials, and holders change hands, so
 * what it owes them it owes everybody). A company none of whose paper anybody outside holds
 * publishes nothing, and one that ceases to be public stops publishing in the period it stops.
 */
export function isPublic(ctx: MechanismContext, firm: PartyId): boolean {
  for (const i of ctx.instruments.issuedBy(firm)) {
    if (!i.status.live || !i.market.some) continue;
    if (ctx.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
    for (const holder of ctx.register.holdersOf(i.id)) {
      if (holder !== firm) return true;
    }
  }
  return false;
}
