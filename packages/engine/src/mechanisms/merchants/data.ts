/**
 * MERCHANTS: the firms whose business is that a thing is worth more somewhere else.
 *
 * @spec Freight C1 Freight C1.a Freight D3 Commodities Spot C6 Firm A1 Firm A3 Seed B1.a Law 2 Law 15
 *
 * A merchant makes nothing. It buys a thing in the place it is standing in, pays somebody to carry
 * it, and sells it where it is dear — and what it earns is the gap between two prints less the
 * freight and less the wait. Until now the only shippers this world had were PRODUCERS with stock
 * they happened to be holding (`freight.toShip`), which closes a basis by accident; a merchant is
 * the party whose whole purpose is to close it, and the one that loses money when it does not.
 *
 * A MERCHANT IS A FIRM, for the reason a carrier is (13c step 5, Law 15): a named party that owns
 * premises, employs people, banks somewhere and can fail. What makes it a merchant is its line —
 * `wholesale`, tonnes distributed, which is the service a shop buys rather than dealing with a
 * hundred makers — and this module gives the firms in that line the one thing the ordinary firm
 * machinery cannot: a reason to buy something it will not use.
 *
 * TWO PREFERENCES AND NOTHING ELSE (Law 2). What margin this management wants before it puts money
 * into a cargo, and how much of its money it will put into any ONE line. Both are drawn per
 * merchant from stated widths, because two merchants facing the same gap do not take the same
 * position — which is what makes the line-up of who closes a basis an outcome rather than a rule.
 */
import type { RegionId } from '../../core/ids.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import { between, type Spread } from '../../rng/spread.js';

export interface MerchantDecl {
  /** The named party. It is a firm and it is already in the world; this says it trades. */
  readonly merchant: string;
  readonly region: RegionId;
  /**
   * Freight D3: the margin on the gap this management wants before it commits money to a cargo. It
   * is what stands between the two prints and what it will pay, so it is also what the basis cannot
   * close below — and it is a PREFERENCE, not a fee: nobody is charged it.
   */
  readonly margin: number;
  /**
   * How much of its money it will put behind ONE line. A merchant that spreads itself over forty
   * lines is a different business from one that is long grain, and which of them is holding when a
   * gap closes the wrong way is the whole of the risk in this trade.
   */
  readonly appetite: number;
  readonly why: string;
}

export const merchantParam = (merchant: string, what: string): ParamId =>
  paramId(`merchant.${what}.${merchant}`);

export const MERCHANT_SPREAD: { readonly margin: Spread; readonly appetite: Spread } = {
  margin: {
    low: 0.02,
    high: 0.12,
    why: 'Freight D3, C1.a: the margin on the gap a merchant wants before it commits money it gets back only when the cargo lands. Two merchants facing one gap do not take the same position, which is why the basis is closed by somebody in particular and not by arithmetic.',
  },
  appetite: {
    low: 0.02,
    high: 0.25,
    why: 'Firm A3: how much of its money goes behind one line. It is the whole of the concentration risk in merchanting — a house that is long one thing when the gap closes the wrong way is a house that fails — and it is a preference, because nothing in the world tells a merchant how to spread itself.',
  },
};

/** Seed A5, B1.a, Audit D3: the merchants of a world, drawn in their own labelled stream. */
export function drawMerchants(
  who: readonly { readonly firm: string; readonly region: RegionId }[],
  seed: string,
): readonly MerchantDecl[] {
  const rng = prng(seed, 'merchants');
  return who.map((w) => ({
    merchant: w.firm,
    region: w.region,
    margin: between(rng, MERCHANT_SPREAD.margin),
    appetite: between(rng, MERCHANT_SPREAD.appetite),
    why: 'Seed B1.a: drawn at the seed from the stated widths, like every other party in this world.',
  }));
}
