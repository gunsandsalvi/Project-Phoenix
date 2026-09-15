/**
 * Where a household cell banks, and the two things that would make it move.
 *
 * @spec Banks Funding A1.a Banks Funding A1.d Banks Funding B1.a Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Observer A3 Observer A4 XI-15 Law 17
 *
 * RETAIL MONEY IS INSURED, and that is the whole of E4: the state stands behind it up to a stated
 * limit, so what a household is deciding about when its bank looks shaky is only the part above
 * that limit — which for a household is usually nothing. It is why a run is a wholesale phenomenon
 * first (E4.a), and nothing says so: the arithmetic of the guarantee does.
 *
 * SO WHAT MOVES A HOUSEHOLD IS THE RATE, on an AMOUNT rather than on a gap, and B1.a is why that
 * matters: the substitution between one bank and the next is how a policy rate reaches a saver.
 *
 * WHAT IT CAN SEE is a capital ratio somebody published (E2.a) — the slowest of the signals this
 * world produces about a bank and the right one for the depositor furthest from the market. The
 * decision itself is `registry/switching.ts`, which a fund takes too on its own two facts.
 */
import { paramId } from '../../core/ids.js';
import { banksForItsBoard } from '../../registry/switching.js';
import type { Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

export { ownDepositRate } from '../../registry/switching.js';

/** A1.d, E1: what it costs one household to move its account, once, as an amount of its money. */
export const HOUSEHOLD_SWITCHING_COST = paramId('households.switchingCost');

export const householdChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksForItsBoard(view, {
    trouble: 'bank.capital',
    troubled: (e) => e.data['belowRequirement'] === true,
    cost: HOUSEHOLD_SWITCHING_COST,
    insured: true,
  });
