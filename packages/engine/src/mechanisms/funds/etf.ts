/**
 * The exchange-traded fund: a fund whose shares trade, so it has two values and they are different
 * numbers — and whose investors come and go IN KIND, so it never has to sell anything.
 *
 * @spec Fund Shares E1 Fund Shares E2 Fund Shares E3 Fund Shares E3.a Fund Shares E4 Fund Shares G1.a Fund Shares B1 Fund Shares C3 Fund Shares C5 Fund Shares F1 Equity C2.c Clearing B2 Clearing E1 XI-2 XI-6 Law 4 Law 19
 *
 * TWO VALUES (E2). Its shares are the same claim on the same kind of book as any other fund's, so
 * what a holder's position is worth is the NAV — a read of the book over the claims on it, fresh at
 * every ask (B1). And its shares also TRADE, so a session prints what a third party paid for one.
 * Neither number is the other's approximation: the print is what the market made of the queue of
 * people who wanted in and out this morning, and the NAV is what the fund actually holds.
 *
 * THE GAP IS A REASON, NOT A RULE (E3.a). Nothing here ties the two together. What closes a gap is
 * a party that can turn one into the other — deliver the basket and take shares, or deliver shares
 * and take the basket — and does it because the difference is worth more to it than what the trade
 * costs it. When nobody will, the gap persists, and E4 says a persistently large one is a finding
 * about liquidity rather than a number to clamp. There is no clamp here and no convergence anywhere.
 *
 * IN KIND (G1.a). A creation takes a pro-rata slice of what the fund holds and gives shares against
 * it; a redemption gives the slice back and takes the shares. Nothing is sold and no market is
 * touched, which is exactly why an exchange-traded fund is NOT a forced seller — and why a world
 * whose largest vehicles redeemed only in kind would have no fund-driven forced selling at all, and
 * would need some other vehicle to carry it. That vehicle is the money fund (item 8), and it does.
 */
import type { Period } from '../../calendar/calendar.js';
import { instrumentId, type InstrumentId, type PartyId } from '../../core/ids.js';
import { div, material, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { weightOf } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import type { EtfDecl } from './data.js';

/** One line of a creation unit: how many units of it back one share (E3). */
export interface BasketLine {
  readonly instrument: InstrumentId;
  /** Units of this line per share of the fund. */
  readonly perShare: number;
  /** What one unit of it is marked at, which is what the fund's own NAV is read off (B2). */
  readonly markPerUnit: number;
}

/**
 * E3: what a creation unit is made of.
 *
 * A pro-rata slice of what the fund ACTUALLY HOLDS once it holds anything — so a creation cannot
 * change what the fund is, and a redemption gives back exactly what it took a share of (Law 19:
 * read the book, do not restate it). Before there is a book, it is the basket the fund was launched
 * with, which is the one thing anybody could go on.
 */
export function basketOf(ctx: MechanismContext, d: EtfDecl, share: InstrumentId): BasketLine[] {
  const fund = d.fund as PartyId;
  const issued = ctx.instruments.get(share).issued;
  const out: BasketLine[] = [];
  if (issued > 0) {
    for (const h of ctx.register.holdingsOf(fund)) {
      if (h.instrument === share) continue;
      if (ctx.registry.instrumentKind(ctx.instruments.get(h.instrument).kind).pricing === 'money') {
        continue;
      }
      const units = ctx.register.quantity(fund, h.instrument);
      if (units <= 0) continue;
      const mark = lastMark(ctx, h.instrument);
      // B2, XI-6: a line nobody has priced is not worth zero and is not worth guessing. A basket
      // with an unpriced line in it cannot be handed in or handed out, and that is the answer.
      if (!mark.some) return [];
      out.push({
        instrument: h.instrument,
        perShare: div(units, issued, 'units of this line behind one share'),
        markPerUnit: mark.value,
      });
    }
    return out;
  }
  for (const [line, perShare] of Object.entries(d.basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const mark = lastMark(ctx, id);
    if (!mark.some) return [];
    out.push({ instrument: id, perShare, markPerUnit: mark.value });
  }
  return out;
}

/**
 * Clearing F1: the last thing a market said about a line, which is what anybody acting before this
 * period's session has to go on. A claim on a book is worth what the book comes to whenever anybody
 * asks (Fund Shares B1), so that one is read fresh; everything else is what it last printed.
 */
function lastMark(ctx: MechanismContext, instrument: InstrumentId): Option<number> {
  const kind = ctx.registry.instrumentKind(ctx.instruments.get(instrument).kind);
  if (kind.pricing === 'derived') return some(ctx.valuation.markPerUnit(instrument, ctx.period));
  const p = ctx.prices.latest(instrument, ctx.period);
  return p.some ? some(p.value.price) : none<number>();
}

/** What one share's worth of basket is marked at — which is the NAV when the basket is the book. */
export function basketValue(basket: readonly BasketLine[]): number {
  return sum(basket.map((b) => mul(b.perShare, b.markPerUnit, 'what this line contributes'))).value;
}

/**
 * E3, G1.a, C3: a CREATION. The party delivers a pro-rata slice of the fund's book and takes new
 * shares against it, in one instruction: every leg or none (XI-5). No cash moves, no market is
 * touched, and the fund's composition is exactly what it was.
 *
 * The shares are issued at what the basket delivered is worth, so the fund's assets and the claims
 * on them move by the same amount and its equity stays at zero (A3). That is not an adjustment: it
 * is what "a share is a claim on the book" means when the book grows by a slice of itself.
 */
export function create(
  ctx: MechanismContext,
  d: EtfDecl,
  share: InstrumentId,
  party: PartyId,
  shares: number,
): boolean {
  const basket = basketOf(ctx, d, share);
  if (basket.length === 0 || shares <= 0) return false;
  const perShare = basketValue(basket);
  if (!(perShare > 0)) return false;
  const holder = ctx.parties.get(party);
  const weight = weightOf(holder);
  const legs: Leg[] = [];
  for (const line of basket) {
    const units = mul(shares, line.perShare, 'units of this line it must deliver');
    if (!material(units, 2, units)) return false;
    // F1: it delivers what it holds. A creator that has not got the basket does not create.
    if (mul(ctx.register.free(party, line.instrument), weight, 'what it holds') < units) return false;
    legs.push({
      kind: 'asset',
      from: party,
      to: d.fund as PartyId,
      instrument: line.instrument,
      qty: units,
      pricePerUnit: some(line.markPerUnit),
      accruedPerUnit: none(),
      fromCell: cellOf(holder, div(units, weight, 'per member')),
      toCell: none(),
    });
  }
  legs.push({
    kind: 'asset',
    from: d.fund as PartyId,
    to: party,
    instrument: share,
    qty: shares,
    pricePerUnit: some(perShare),
    accruedPerUnit: none(),
    fromCell: none(),
    toCell: cellOf(holder, div(shares, weight, 'per member')),
  });
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${party} creates ${shares} of ${share} in kind`,
  });
  ctx.record(
    'etf.created',
    [d.fund, party, share],
    { fund: d.fund, by: party, shares, perShare, settled: r.outcome === 'settled' },
    true,
  );
  return r.outcome === 'settled';
}

/**
 * E3, G1.a: a REDEMPTION in kind. The shares come back and the slice goes out. Nothing is sold —
 * which is the whole of why this vehicle is not a forced seller (XI-2 runs through the money fund
 * instead) — and the holders who stay are unaffected, because what left was exactly its own share.
 */
export function redeemInKind(
  ctx: MechanismContext,
  d: EtfDecl,
  share: InstrumentId,
  party: PartyId,
  shares: number,
): boolean {
  const basket = basketOf(ctx, d, share);
  if (basket.length === 0 || shares <= 0) return false;
  const perShare = basketValue(basket);
  if (!(perShare > 0)) return false;
  const holder = ctx.parties.get(party);
  const weight = weightOf(holder);
  const held = mul(ctx.register.free(party, share), weight, 'shares it can give back');
  if (held < shares) return false;
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: party,
      to: d.fund as PartyId,
      instrument: share,
      qty: shares,
      pricePerUnit: some(perShare),
      accruedPerUnit: none(),
      fromCell: cellOf(holder, div(shares, weight, 'per member')),
      toCell: none(),
    },
  ];
  for (const line of basket) {
    const units = mul(shares, line.perShare, 'units of this line it takes back');
    if (!material(units, 2, units)) return false;
    if (ctx.register.free(d.fund as PartyId, line.instrument) < units) return false;
    legs.push({
      kind: 'asset',
      from: d.fund as PartyId,
      to: party,
      instrument: line.instrument,
      qty: units,
      pricePerUnit: some(line.markPerUnit),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: cellOf(holder, div(units, weight, 'per member')),
    });
  }
  const r = ctx.settle({
    legs,
    cause: 'maturity',
    reason: `${party} redeems ${shares} of ${share} in kind`,
  });
  ctx.record(
    'etf.redeemed',
    [d.fund, party, share],
    { fund: d.fund, by: party, shares, perShare, settled: r.outcome === 'settled' },
    true,
  );
  return r.outcome === 'settled';
}

/** XI-15: a cell side carries the per-member amount; a named party carries none. */
function cellOf(holder: { readonly representation: string; readonly weight?: number }, perMember: number): Option<{ readonly perMember: number; readonly weight: number }> {
  if (holder.representation !== 'cell' || holder.weight === undefined) return none();
  return some({ perMember, weight: holder.weight });
}

/**
 * E2, E4: the premium or discount — a READ of two prices, published beside both of them.
 *
 * It is not a target and nothing anywhere tries to close it. A persistently large one is a finding
 * about liquidity: it says the parties who could turn one value into the other would not, and why
 * they would not is their own limits and their own costs (Dealer Desks D1-D3).
 */
export function premiumOf(print: Option<number>, nav: number): Option<number> {
  if (!print.some || nav <= 0) return none();
  return some(div(sub(print.value, nav, 'what the market pays over the book'), nav, 'as a share of it'));
}

/** The period a read is about, so a stale print is visibly a stale premium (Clearing E4). */
export type At = Period;
