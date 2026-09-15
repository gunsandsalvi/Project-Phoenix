/**
 * Where an assessor banks, and the one thing that would make it move.
 *
 * @spec Banks Funding A1.b Banks Funding A1.d Banks Funding E1 Banks Funding E2.a Banks Funding E4 Ratings A5 Observer A3 Observer A4
 *
 * A1.b: AN ASSESSOR IS A SMALL BUSINESS and banks like one — its account is where the fees arrive
 * and the salaries leave, not an investment, so it does not chase a deposit board. It reads the
 * same public fact a firm's treasurer reads, a bank that drew the central bank's window (E2.a), and
 * nothing private: an assessor that could see a bank's own state before the rest of the world would
 * be an assessor whose grades leak it (A2, Observer A4).
 *
 * What is its own is the cost of moving an account it is paid through; the decision is
 * `registry/switching.ts` (Law 4: one writer of what a depositor does).
 */
import { paramId } from '../../core/ids.js';
import { banksAwayFromTrouble } from '../../registry/switching.js';
import type { Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs one assessor to move the account it is paid through, once. */
export const ASSESSOR_SWITCHING_COST = paramId('ratings.switchingCost');

export const assessorChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksAwayFromTrouble(view, ASSESSOR_SWITCHING_COST, 'moneyMarket.window');
