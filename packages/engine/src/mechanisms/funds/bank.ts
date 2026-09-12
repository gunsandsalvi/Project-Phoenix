/**
 * Where a fund and its manager bank, and why this is the money that leaves first.
 *
 * @spec Banks Funding A1.c Banks Funding A1.d Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Money Market E1 Observer A3 Observer A4
 *
 * A1.c: FEW, VERY LARGE AND RATE-SENSITIVE, and E4.a says this is the money that goes first. Both
 * come out of arithmetic here rather than out of a stated stickiness:
 *
 * - **Nothing insures it.** The whole balance is at stake when its bank looks shaky, and the whole
 *   balance is very large beside what moving costs, so the cost never holds it back (E4).
 * - **It is in the market all day.** What it can see (E2.a) is a SESSION REFUSING ITS BANK — the
 *   earliest signal this world produces about a bank, seen the evening it happens rather than in a
 *   facility draw a cycle later (a firm's, §32) or a capital ratio published afterwards (a
 *   household's, §41). Seeing it first is the whole of why it leaves first.
 * - **It chases the board.** Its balance is large, so what a quarter point has already cost it
 *   passes what moving costs it in a week rather than in a year — which is what "rate-sensitive"
 *   means when the test is an amount against an amount rather than a rate every member of a class
 *   answers identically.
 */
import { period as asPeriod } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { currencyUnit, moneyInstrumentId, paramId, type PartyId } from '../../core/ids.js';
import { mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK } from '../../registry/profiles.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs a fund to move its account, once, as an amount of its own money. */
export const FUND_SWITCHING_COST = paramId('funds.switchingCost');

/** One day count for how long a depositor has stayed, stated once (Law 8: a rate has a period). */
const STAYED_DAY_COUNT = 'ACT/365F';

export function fundChoosesBank(view: ParticipantView): Option<BankChoice> {
  const self = view.self;
  const ccy = view.registry.currencyOf(self.region);
  const balance = view.quantity(moneyInstrumentId(self.bank, ccy));
  if (balance <= 0) return none();
  const cls = view.registry.partyKind(self.kind).depositClass;
  if (cls === null) return none();

  // E2.a: the session that would not fund it. A fund is in that market and sees the refusal the
  // evening it happens — which is what puts wholesale money out of the door before anybody else's.
  const refused = (bank: PartyId): boolean =>
    view.lastPublicAbout('moneyMarket.refused', String(bank)).some;

  let best: { bank: PartyId; rate: number } | undefined;
  for (const b of view.parties.ofKind(BANK)) {
    if (b.id === self.bank || b.region !== self.region || !b.status.alive || refused(b.id)) continue;
    const rate = board(view, b.id, cls);
    if (!rate.some) continue;
    if (best === undefined || rate.value > best.rate) best = { bank: b.id, rate: rate.value };
  }
  if (best === undefined) return none();

  // E4: nothing insures this money, so there is nothing to weigh — the whole balance is at stake.
  if (refused(self.bank)) {
    return some({ to: best.bank, reason: `${self.id} leaves ${self.bank}, which the session refused` });
  }

  const own = board(view, self.bank, cls);
  if (!own.some) return none();
  const gap = sub(best.rate, own.value, 'what it would gain');
  if (gap <= 0) return none();
  const cost = view.params.amount(FUND_SWITCHING_COST, currencyUnit(ccy));
  const foregone = mul(
    balance,
    mul(gap, stayed(view), 'over the time it has stayed'),
    'what staying cost it',
  );
  return foregone > cost
    ? some({ to: best.bank, reason: `${self.id} moves to ${best.bank}, which pays more for its money` })
    : none();
}

/**
 * E1: how long it has banked where it banks, in years — since it last moved, or since the world
 * opened. Its own move is its own event, so the clock is a read and not a stored counter.
 */
function stayed(view: ParticipantView): number {
  const last = view.lastOwn('deposit.moved');
  const from = last.some ? last.value.period : asPeriod(0);
  return yearFraction(
    STAYED_DAY_COUNT,
    view.calendar.startOf(from),
    view.calendar.startOf(view.period),
  );
}

/** B1.a, E2.a: the rate on a bank's board for this class, which is public because it must be. */
function board(view: ParticipantView, bank: PartyId, cls: string): Option<number> {
  const said = view.lastPublicAbout('bank.depositRate', String(bank));
  if (!said.some) return none<number>();
  const rates = said.value.data['rates'];
  if (typeof rates !== 'object' || rates === null) return none<number>();
  const rate = (rates as Record<string, unknown>)[cls];
  return typeof rate === 'number' ? some(rate) : none<number>();
}
