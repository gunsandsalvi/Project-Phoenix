/**
 * WHAT A PARTY EXPECTS A LINE TO FETCH — one read, for every party that prices off the tape.
 *
 * @spec Expectations A2 Expectations A2.a Expectations B3 Law 4 Law 19
 *
 * Its own outlook where it has one, formed from what it has itself traded at; otherwise what the
 * market last printed, which is public (Clearing E1) and is all a party with no history of its own
 * has; nothing where the line has never printed, which is a refusal and not a level (Law 3). The
 * firms module, the household basket and the saving ladder each wrote this ladder out for
 * themselves; a small firm would have been the fourth copy (Law 4). It lives beside the other
 * public reads because a print is public and an outlook is the asker's own.
 */
import type { InstrumentId } from '../core/ids.js';
import { asCash, asPerPiece, type Cash, type PerPiece } from '../core/measure.js';
import { none, type Option, some } from '../core/option.js';
import { about, type ParticipantView } from '../world/context.js';
import type { CurrencyCode } from '../core/ids.js';

export function expectedPriceOf(
  view: Pick<ParticipantView, 'outlook' | 'print'>,
  instrument: InstrumentId,
): Option<PerPiece> {
  const own = view.outlook(about({ on: 'price', instrument }));
  if (own.some)
    return some(asPerPiece(own.value.expected, `what it expects ${instrument} to fetch`));
  const print = view.print(instrument);
  return print.some ? some(print.value.price) : none<PerPiece>();
}

/**
 * §46 A2, B1, Labour C1 (12b.3): WHAT THIS PARTY EXPECTS TO TAKE IN a period — its own outlook on
 * its income, formed adaptively from what it was paid, at its own memory. It is what an employer
 * weighs an hour against: a bid for staff was `earned(1)`, last period's equity moves by
 * instructions, which a bank's single good week turned into a wage of sixty-seven billion an hour.
 * A party with no history of being paid has no outlook, and bids for nobody.
 */
export function expectedIncomeOf(view: Pick<ParticipantView, 'outlook'>): Option<Cash> {
  const own = view.outlook(about({ on: 'income' }));
  // Expectations A2, Currency A4: an income was observed in a money, and the outlook says which.
  return own.some
    ? some(
        asCash(
          own.value.expected,
          own.value.unit as CurrencyCode,
          'what it expects to take in a period',
        ),
      )
    : none<Cash>();
}

/**
 * §46 A2, §32 E7, Labour C1 (12b.3): WHAT THIS PARTY EXPECTS TO MAKE a period — its own outlook on
 * its earnings, the result every settled instruction and every mark leaves on its equity account,
 * formed adaptively at its own memory. It is what a bank's or a fund's desk weighs an hour against:
 * what the book MAKES, not what passes through its account (a bank is paid every deposit and every
 * repayment, and none of that is its revenue). A party that has never made anything bids for nobody.
 */
export function expectedEarningsOf(view: Pick<ParticipantView, 'outlook'>): Option<Cash> {
  const own = view.outlook(about({ on: 'earnings' }));
  return own.some
    ? some(
        asCash(
          own.value.expected,
          own.value.unit as CurrencyCode,
          'what it expects to make a period',
        ),
      )
    : none<Cash>();
}
