/**
 * Net asset value: what a fund's book comes to, divided by the claims on it.
 *
 * @spec Fund Shares A2 Fund Shares A3 Fund Shares B1 Fund Shares B2 Fund Shares B2.a Fund Shares B4 Fund Shares D4 Hedge Funds E3 Derivative X1 Derivative D1 XI-3 XI-6 Law 4 Law 6 Law 19
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
 *
 * Item 13.6, Derivative X1: AND THE BOOK IS NOT ONLY THE REGISTER. A contract is on both sides'
 * books at once and nobody holds units of it, so it lives in its own store — and a NAV read out of
 * holdings alone would leave a pool's derivative position out of the very number its holders'
 * claim is measured by, while the pool's EQUITY account moved with every one of its revaluations.
 * The difference between those two answers is a fund with equity, which is the one thing a fund may
 * never have (A3). Nothing in this world reaches it today, because every mandate drawn says
 * `mayWrite: []` — and item 13.2 is about to draw one that does not.
 */
import { atLeastCash, noCash, sumCash } from '../../core/measure.js';
import type { Period } from '../../calendar/calendar.js';
import { Unpriced } from '../../core/errors.js';
import type { PartyId } from '../../core/ids.js';
import { type Cash, minus, negated, type PerPiece, pricedAt, valueAt } from '../../core/measure.js';
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
      throw new Unpriced(
        'Fund Shares B2',
        `${fund} holds ${h.instrument}, which has never marked`,
        {
          instrument: h.instrument,
        },
      );
    }
    // Currency C4.a, A-50: ON THE FUND'S OWN BOOK. A NAV is a price in one money, and this added
    // whatever money each holding happened to be in — a mandate with no currency in it (A-47) put
    // foreign bills here, and their face went straight into the per-share number.
    assets.push(reads.inOwnMoney(fund, worth.value.value, at));
    if (worth.value.from < oldest) oldest = worth.value.from;
  }
  const owed: Cash[] = [];
  /**
   * Derivative X1, D1, A3, B1 (item 13.6): AND WHAT ITS CONTRACTS ARE WORTH TO IT.
   *
   * A book read out of holdings alone stops at the register's edge, and a contract is not in the
   * register: nobody issued it, nobody holds units of it, and it is on both sides' books at once.
   * So a pool with a derivative position carried a mark its own share value had never been told
   * about — and because a fund's share liability follows its book (`owes: 'value'`) while its
   * equity ACCOUNT follows every revaluation including a contract's, the two answers diverged by
   * exactly the contract book. That difference is a fund WITH equity (A3), which is the one thing
   * a fund may never have, and it was measured at 83,247,864 on one vehicle.
   *
   * D1: an asset to one side and a liability to the other at every instant, so the sign decides
   * which side of this read it lands on. Nothing is netted across the two: a pool long one contract
   * and short another has an asset and a liability, which is what its holders own and what they
   * owe, and adding them first would hide half of both (B5's three reads, and Appendix B's "no
   * netting across counterparties").
   *
   * NOTHING HERE IS STALE-CHECKED, and that is not an oversight to fix silently: a mark is computed
   * from this period's public reads at the moment it is asked (`contract-value.ts`), so there is no
   * "period this mark came from" to travel with it the way a print's does. What could be stale is a
   * PRINT the mark reads, which is the underlying's own staleness and is already reported wherever
   * that line is held.
   */
  for (const position of reads.contractsOf(fund, at)) {
    const own = reads.inOwnMoney(fund, position.worth, at);
    if (own.pieces > 0) assets.push(own);
    else if (own.pieces < 0) owed.push(negated(own, 'what this contract is a liability for'));
  }
  for (const other of reads.instruments()) {
    if (other.id === share.id || !other.status.live) continue;
    if (!other.issuer.some || other.issuer.value !== fund) continue;
    if (!reads.kindOf(other.id).liabilityOfIssuer) continue;
    for (const holder of reads.holdersOf(other.id)) {
      const worth = reads.worthOf(holder, other.id, at);
      if (worth.some) owed.push(reads.inOwnMoney(fund, worth.value.value, at));
    }
  }
  const a = sumCash(share.ccy, assets, 'what the fund holds');
  const l = sumCash(share.ccy, owed, 'what the fund owes');
  const net = minus(a.value, l.value, 'what the fund is worth');
  /**
   * A3, Bond N13.a, Hedge Funds E3 (item 13.4): A SHARE RANKS BEHIND EVERYTHING ELSE THE FUND OWES
   * AND TAKES WHAT IS LEFT — and what is left of a book that does not cover its senior claims is
   * NOTHING, not a negative amount.
   *
   * IT IS NOT A FLOOR AND THE DIFFERENCE MATTERS (Law 6). A negative share value is a claim that its
   * HOLDERS OWE THE FUND MONEY, and nothing in this world ever established that: a share is a
   * limited liability, which is a fact about the instrument and not a rule about the number. The
   * `ranking` this kind already declares says so in words — *"a pro-rata share of what is left of
   * the fund once anything else it owes is paid"* — and this is that sentence as arithmetic.
   *
   * AND IT IS WHAT MAKES A LEVERED POOL ABLE TO FAIL, which is the whole of §28 E3. While the claim
   * could go negative, the share liability absorbed every loss exactly: assets minus the loan minus
   * (assets minus the loan) is zero, so the pool's EQUITY ACCOUNT stayed at zero however far
   * underwater it went, the solvency trigger could never fire, and *"a vehicle that absorbs losses
   * indefinitely is the buyer of last resort in a different costume"*. With the claim honest, the
   * account does not net: assets less senior claims is a real negative, the pool is insolvent, its
   * estate opens, and its broker eats the shortfall (Prime Brokerage D2, XI-3's fund row). Nothing
   * was written to make that happen — it stopped being prevented.
   */
  const forShares = atLeastCash(
    net,
    noCash(share.ccy),
    'a share is a limited liability: its holder is not liable to the fund beyond it',
  );
  return {
    // B1: money over the claims on it, which is what a price IS — and `pricedAt` is the only way
    // to make one out of a payment and a count (item 16).
    perShare: pricedAt(forShares, shares, 'net asset value per share'),
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
  return valueAt(
    perShare,
    reads.quantity(holder, share.id),
    share.ccy,
    "what this holder's claim comes to",
  );
}
