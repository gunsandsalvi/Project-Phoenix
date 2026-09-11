/**
 * Where an assessor banks, and the one thing that would make it move.
 *
 * @spec Banks Funding A1.b Banks Funding A1.d Banks Funding E1 Banks Funding E2.a Banks Funding E4 Ratings A5 Observer A3 Observer A4
 *
 * A1.b: AN ASSESSOR IS A SMALL BUSINESS and banks like one — its account is where the fees arrive
 * and the salaries leave, not an investment, so it does not chase a deposit board. What moves it is
 * the same thing that moves any corporate balance: nothing insures it (E4), so what is at stake is
 * the whole of it, and against that it weighs what moving an operating account costs.
 *
 * It reads the same public fact a firm's treasurer reads — a bank that drew the central bank's
 * window (E2.a) — and nothing private. An assessor that could see a bank's own state before the
 * rest of the world would be an assessor whose grades leak it (A2, Observer A4).
 */
import { currencyUnit, moneyInstrumentId, paramId, type PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK } from '../../registry/profiles.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs one assessor to move the account it is paid through, once. */
export const ASSESSOR_SWITCHING_COST = paramId('ratings.switchingCost');

export function assessorChoosesBank(view: ParticipantView): Option<BankChoice> {
  const self = view.self;
  const ccy = view.registry.region(self.region).ccy;
  const balance = view.quantity(moneyInstrumentId(self.bank, ccy));
  if (balance <= 0) return none();
  const drewTheWindow = (bank: PartyId): boolean =>
    view.lastPublicAbout('moneyMarket.window', String(bank)).some;
  if (!drewTheWindow(self.bank)) return none();
  const cost = view.params.amount(ASSESSOR_SWITCHING_COST, currencyUnit(ccy));
  if (balance <= cost) return none();
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
