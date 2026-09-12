/**
 * Where a household cell banks, and the two things that would make it move.
 *
 * @spec Banks Funding A1.a Banks Funding A1.d Banks Funding E1 Banks Funding E2 Banks Funding E2.a Banks Funding E4 Banks Funding E4.a Observer A3 Observer A4 XI-15 Law 17
 *
 * RETAIL MONEY IS INSURED, and that is the whole of E4: the state stands behind it up to a stated
 * limit, so what a household is deciding about when its bank looks shaky is only the part above
 * that limit — which for a household is usually nothing. It is why a run is a wholesale phenomenon
 * first (E4.a), and nothing here says so: the arithmetic of the guarantee does.
 *
 * SO WHAT MOVES A HOUSEHOLD IS THE RATE, and it moves on an AMOUNT rather than on a gap: what
 * staying has ALREADY cost it — its own balance times the difference between the two boards over as
 * long as it has been where it is — against what moving costs it once. A big account gains more
 * from the same quarter point, so it goes first, and a small one may never go at all: the class
 * drains instead of crossing in one instant, which is what it does when the test is a rate every
 * member of a class answers identically (App B: no representative agent where decisions are
 * thresholds). Nothing here is a forecast (Law 17): it is what has already happened to it.
 *
 * WHAT IT CAN SEE is a capital ratio somebody published (E2.a), which is the slowest of the signals
 * this world produces about a bank and the right one for the depositor that is furthest from the
 * market. A fund sees its own session refuse the bank the same evening (§13); a firm sees the
 * window drawn; a household reads the number the bank published afterwards.
 */
import { period as asPeriod } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { currencyUnit, moneyInstrumentId, paramId, type PartyId } from '../../core/ids.js';
import { mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK } from '../../registry/profiles.js';
import type { ParticipantView } from '../../world/context.js';
import type { BankChoice } from '../../world/module.js';

/** A1.d, E1: what it costs one household to move its account, once, as an amount of its money. */
export const HOUSEHOLD_SWITCHING_COST = paramId('households.switchingCost');

/** One day count for how long a depositor has stayed, stated once (Law 8: a rate has a period). */
const STAYED_DAY_COUNT = 'ACT/365F';

export function householdChoosesBank(view: ParticipantView): Option<BankChoice> {
  const self = view.self;
  const ccy = view.registry.currencyOf(self.region);
  const balance = view.quantity(moneyInstrumentId(self.bank, ccy));
  if (balance <= 0) return none();
  const cls = view.registry.partyKind(self.kind).depositClass;
  if (cls === null) return none();

  // E2.a: the one thing a household outside the market can see about a bank — what it published
  // about its own capital. It is public because that is what makes an answer to it possible.
  const shaky = (bank: PartyId): boolean => {
    const said = view.lastPublicAbout('bank.capital', String(bank));
    return said.some && said.value.data['belowRequirement'] === true;
  };

  let best: { bank: PartyId; rate: number } | undefined;
  for (const b of view.parties.ofKind(BANK)) {
    if (b.id === self.bank || b.region !== self.region || !b.status.alive || shaky(b.id)) continue;
    const rate = board(view, b.id, cls);
    if (!rate.some) continue;
    if (best === undefined || rate.value > best.rate) best = { bank: b.id, rate: rate.value };
  }
  if (best === undefined) return none();

  // E4: what nobody insures, per member. The limit is the regulation the market publishes with the
  // classes, so nothing here has to know what the parameter is called.
  if (shaky(self.bank) && uninsured(view, ccy, cls, balance) > 0) {
    return some({
      to: best.bank,
      reason: `${self.id} moves what nobody insures away from ${self.bank}`,
    });
  }

  const own = board(view, self.bank, cls);
  if (!own.some) return none();
  const gap = sub(best.rate, own.value, 'what it would gain');
  if (gap <= 0) return none();
  const cost = view.params.amount(HOUSEHOLD_SWITCHING_COST, currencyUnit(ccy));
  const foregone = mul(
    balance,
    mul(gap, stayed(view), 'over the time it has stayed'),
    'what staying cost it',
  );
  return foregone > cost
    ? some({ to: best.bank, reason: `${self.id} moves to ${best.bank}, which pays more for its money` })
    : none();
}

/** A1.a, E4: the part of its own balance the published guarantee does not cover, per member. */
function uninsured(
  view: ParticipantView,
  ccy: string,
  cls: string,
  balance: number,
): number {
  const said = view.lastPublic('deposit.classes');
  if (!said.some) return balance;
  const classes = said.value.data['classes'];
  const limits = said.value.data['limits'];
  if (!Array.isArray(classes) || typeof limits !== 'object' || limits === null) return balance;
  const row = classes.find(
    (c): c is { id: string; insured: boolean } =>
      typeof c === 'object' && c !== null && (c as { id?: unknown }).id === cls,
  );
  if (row?.insured !== true) return balance;
  const limit = (limits as Record<string, unknown>)[ccy];
  if (typeof limit !== 'number') return balance;
  return balance < limit ? 0 : sub(balance, limit, 'what nobody insures');
}

/**
 * E1: how long this cell has banked where it banks, in years — since it last moved, or since the
 * world opened. Its own move is its own event, so the clock is a read and not a stored counter, and
 * it resets when the depositor moves: one that has just gone somewhere does not go again next week
 * on the same gap.
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
