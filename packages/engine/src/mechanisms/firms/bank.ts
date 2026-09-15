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
 * So what is declared here is what is a FIRM's: what moving costs it, and what it can see. The
 * arithmetic is `registry/switching.ts`, because four other modules take the same decision and none
 * of them may import this file (Law 4, `phoenix/no-cross-module-import`).
 *
 * WHAT IT CAN SEE (E2.a) is the facility drawn. A firm is not in the money market and does not see
 * a session refuse anybody, but a bank at the central bank's window is public and a treasurer reads
 * it — which puts a firm between the fund that saw the refusal the same evening (§13) and the
 * household that reads the capital ratio published afterwards (§41).
 */
import { paramId } from '../../core/ids.js';
import { banksAwayFromTrouble } from '../../registry/switching.js';
import type { Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs one firm to move the account it transacts through, once. */
export const FIRM_SWITCHING_COST = paramId('firms.switchingCost');

export const firmChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksAwayFromTrouble(view, FIRM_SWITCHING_COST, 'moneyMarket.window');
