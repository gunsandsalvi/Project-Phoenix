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
import { asPerPiece, type PerPiece } from '../core/measure.js';
import { none, type Option, some } from '../core/option.js';
import { about, type ParticipantView } from '../world/context.js';

export function expectedPriceOf(
  view: Pick<ParticipantView, 'outlook' | 'print'>,
  instrument: InstrumentId,
): Option<PerPiece> {
  const own = view.outlook(about({ on: 'price', instrument }));
  if (own.some) return some(asPerPiece(own.value.expected, `what it expects ${instrument} to fetch`));
  const print = view.print(instrument);
  return print.some ? some(print.value.price) : none<PerPiece>();
}
