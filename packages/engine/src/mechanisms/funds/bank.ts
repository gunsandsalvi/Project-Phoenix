/**
 * Where a fund and its manager bank, and why this is the money that leaves first.
 *
 * @spec Banks Funding A1.c Banks Funding A1.d Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Money Market E1 Observer A3 Observer A4
 *
 * A1.c: FEW, VERY LARGE AND RATE-SENSITIVE, and E4.a says this is the money that goes first. Both
 * come out of the arithmetic in `registry/switching.ts` rather than out of a stated stickiness, and
 * what makes it a fund's is the two facts declared here:
 *
 * - **Nothing insures it** (E4). The whole balance is at stake when its bank looks shaky, and the
 *   whole balance is very large beside what moving costs, so the cost never holds it back.
 * - **It is in the market all day.** What it can see (E2.a) is A SESSION REFUSING ITS BANK — the
 *   earliest signal this world produces about a bank, seen the evening it happens rather than in a
 *   facility draw a cycle later (a firm's) or a capital ratio published afterwards (a household's).
 *   Seeing it first is the whole of why it leaves first.
 */
import { paramId } from '../../core/ids.js';
import { banksForItsBoard } from '../../registry/switching.js';
import type { Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs a fund to move its account, once, as an amount of its own money. */
export const FUND_SWITCHING_COST = paramId('funds.switchingCost');

export const fundChoosesBank = (view: ParticipantView): Option<BankChoice> =>
  banksForItsBoard(view, {
    trouble: 'moneyMarket.refused',
    cost: FUND_SWITCHING_COST,
    insured: false,
  });
