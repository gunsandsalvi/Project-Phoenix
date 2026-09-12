/**
 * Where a firm banks, and the one thing that would make it move.
 *
 * @spec Banks Funding A1.b Banks Funding A1.d Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Money C2.b Observer A3 Observer A4
 *
 * A1.b: A FIRM BANKS WHERE IT TRANSACTS. Its account is not an investment, it is the thing every
 * wage, every invoice and every receipt goes through, and the deposit rate on it is not what it is
 * for — which is why this money is called operational and why it does NOT chase a board. A firm
 * that moved for a quarter point would be a firm that reads its bank the way a fund does, and then
 * corporate money and wholesale money are one class wearing two names (A1.d: a model with one
 * deposit type cannot have a run).
 *
 * WHAT DOES MOVE IT is the money it would lose. Nothing insures a corporate balance (A1.b, E4), so
 * what is at stake when its bank looks shaky is the whole of it, and against that it weighs what
 * moving an operational account costs — the payments to redirect, the counterparties to tell. An
 * amount against an amount: a firm with little in the account stays, a firm with its whole float
 * there goes.
 *
 * WHAT IT CAN SEE (E2.a) is the facility drawn. A firm is not in the money market and does not see
 * a session refuse anybody, but a bank at the central bank's window is public and a treasurer reads
 * it — which puts a firm between the fund that saw the refusal the same evening (§13) and the
 * household that reads the capital ratio published afterwards.
 */
import { currencyUnit, moneyInstrumentId, paramId, type PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK } from '../../registry/profiles.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs one firm to move the account it transacts through, once. */
export const FIRM_SWITCHING_COST = paramId('firms.switchingCost');

export function firmChoosesBank(view: ParticipantView): Option<BankChoice> {
  const self = view.self;
  const ccy = view.registry.currencyOf(self.region);
  const balance = view.quantity(moneyInstrumentId(self.bank, ccy));
  if (balance <= 0) return none();

  // E2.a: the facility drawn — public, because that is what makes an answer to it possible (D5.a).
  const drewTheWindow = (bank: PartyId): boolean =>
    view.lastPublicAbout('moneyMarket.window', String(bank)).some;
  if (!drewTheWindow(self.bank)) return none();

  // A1.b, E4: nothing insures this money, so what is at stake is the whole balance — and it is
  // weighed against what moving the account it transacts through actually costs it.
  const cost = view.params.amount(FIRM_SWITCHING_COST, currencyUnit(ccy));
  if (balance <= cost) return none();

  // Observer A3: who the other banks are is public. It goes to one that has not drawn the window,
  // and it is not looking at anybody's board: it wants the money somewhere, not a better rate.
  for (const b of view.parties.ofKind(BANK)) {
    if (b.id === self.bank || b.region !== self.region || !b.status.alive) continue;
    if (drewTheWindow(b.id)) continue;
    return some({
      to: b.id,
      reason: `${self.id} banks away from ${self.bank}, which drew the window`,
    });
  }
  return none();
}
