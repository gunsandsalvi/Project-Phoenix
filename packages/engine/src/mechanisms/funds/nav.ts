/**
 * Net asset value: what a fund's book comes to, divided by the claims on it.
 *
 * @spec Fund Shares A2 Fund Shares A3 Fund Shares B1 Fund Shares B2 Fund Shares B2.a Fund Shares B4 Fund Shares D4 XI-6 Law 19
 *
 * B1: assets at market minus liabilities, over shares outstanding, READ EVERY TIME. There is no NAV
 * series anywhere in this module and nothing stores one: the number below is computed from the
 * register and the marks at the instant somebody asks, and the instant after it may be different.
 *
 * D4 falls out of that rather than being enforced: nothing here can hold the number at one, because
 * there is nothing to hold — if the assets fall, the division falls with them. A constant NAV would
 * take a guarantor, and the guarantor would be nobody.
 *
 * B2.a: the assets are marked at what a market last said, and how old that is travels with the
 * number. A stale mark makes a stale NAV, somebody subscribes or redeems on it, and that is a real
 * transfer between holders — so it is said out loud rather than smoothed away.
 */
import type { Period } from '../../calendar/calendar.js';
import { Unpriced } from '../../core/errors.js';
import type { PartyId } from '../../core/ids.js';
import {
  type Cash,
  minus,
  type PerPiece,
  pricedAt,
  valueAt,
} from '../../core/measure.js';
import { sum } from '../../core/num.js';
import type { Qty } from '../../core/tick.js';
import { issuerOf, type Instrument } from '../../register/instruments.js';
import type { DerivedReads } from '../../registry/kinds.js';

/** What the read found: the value, and the oldest mark that went into it (B2.a). */
export interface NavRead {
  readonly perShare: PerPiece;
  readonly assets: Cash;
  readonly owed: Cash;
  readonly shares: Qty;
  /** The oldest period any of its marks came from; equal to `at` when nothing is stale. */
  readonly oldestMark: Period;
}

/**
 * B1: the read itself. It asks the kernel what each of the fund's positions is worth at the marks
 * everything else is valued at (XI-6) — never a second price system, and never a formula standing
 * in for a market (Law 3).
 */
export function navOf(share: Instrument, at: Period, reads: DerivedReads): NavRead {
  const fund = issuerOf(share);
  const shares = reads.issued(share.id);
  if (shares <= 0) {
    throw new Unpriced('Fund Shares B1', `${share.id} has no shares outstanding to divide by`, {
      instrument: share.id,
    });
  }
  let oldest = at;
  const assets: Cash[] = [];
  for (const h of reads.holdingsOf(fund)) {
    // A fund does not hold its own shares: a claim on itself is not an asset (F2, and the kernel
    // refuses to value a book that depends on its own value).
    if (h.instrument === share.id) continue;
    const worth = reads.worthOf(fund, h.instrument, at);
    if (!worth.some) {
      // B2: assets are marked at cleared prices. A holding nothing has ever priced is not worth
      // zero and is not worth guessing: there is no NAV until it has a mark (XI-6).
      throw new Unpriced('Fund Shares B2', `${fund} holds ${h.instrument}, which has never marked`, {
        instrument: h.instrument,
      });
    }
    // Currency C4.a, A-50: ON THE FUND'S OWN BOOK. A NAV is a price in one money, and this added
    // whatever money each holding happened to be in — a mandate with no currency in it (A-47) put
    // foreign bills here, and their face went straight into the per-share number.
    assets.push(reads.inOwnMoney(fund, worth.value.value, worth.value.ccy, at));
    if (worth.value.from < oldest) oldest = worth.value.from;
  }
  const owed: Cash[] = [];
  for (const other of reads.instruments()) {
    if (other.id === share.id || !other.status.live) continue;
    if (!other.issuer.some || other.issuer.value !== fund) continue;
    if (!reads.kindOf(other.id).liabilityOfIssuer) continue;
    for (const holder of reads.holdersOf(other.id)) {
      const worth = reads.worthOf(holder, other.id, at);
      if (worth.some) owed.push(reads.inOwnMoney(fund, worth.value.value, worth.value.ccy, at));
    }
  }
  const a = sum(assets);
  const l = sum(owed);
  const net = minus(a.value, l.value, 'what the fund is worth');
  return {
    // B1: money over the claims on it, which is what a price IS — and `pricedAt` is the only way
    // to make one out of a payment and a count (item 16).
    perShare: pricedAt(net, shares, 'net asset value per share'),
    assets: a.value,
    owed: l.value,
    shares,
    oldestMark: oldest,
  };
}

/** A2, B4: what one holder's claim on the book comes to — its shares at the value of a share. */
export function claimOf(
  holder: PartyId,
  share: Instrument,
  perShare: PerPiece,
  reads: DerivedReads,
): Cash {
  // Item 16: this was a bare `*` of a count and a level — the one shape `valueAt` exists to stop.
  return valueAt(perShare, reads.quantity(holder, share.id), "what this holder's claim comes to");
}
