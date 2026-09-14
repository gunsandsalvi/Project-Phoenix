/**
 * WHAT `liquidity: 'listed'` MEANS: the shares trade, so there are two values and they are
 * different numbers — and investors come and go IN KIND, so the pool never has to sell anything.
 *
 * Item 10e: named for the TERM and not for a kind of fund. Coming and going in kind is what makes a
 * vehicle NOT a forced seller (G1.a), and that is a fact about how its investors get out rather
 * than about what it is called — so it is the liquidity term that puts a pool here.
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
import {
  asRatio,
  type Cash,
  type PerPiece,
  valueAt,
  pricedAt,
  type Ratio,
  ratioOf,
  scale,
  minus,
  over,
} from '../../core/measure.js';
import type { Period } from '../../calendar/calendar.js';
import { scaleQty, type Qty } from '../../core/tick.js';
import { instrumentId, type InstrumentId, type PartyId } from '../../core/ids.js';
import { material, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { CellSide, Leg } from '../../ledger/instruction.js';
import { shareFor } from '../../ledger/settlement.js';
import { weightOf, type Party } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import { inKindOf, type FundDecl } from './data.js';

/** One line of a creation unit: how many units of it back one share (E3). */
export interface BasketLine {
  readonly instrument: InstrumentId;
  /** Units of this line per share of the fund. */
  /**
   * Item 16, Law 9: UNITS OF THIS LINE BEHIND ONE SHARE — a count over a count, which is a pure
   * number. It shares a name with the fund's NAV per share (`nav.ts`) and is not the same thing:
   * that one is MONEY per share. The type is what tells them apart now.
   */
  readonly perShare: Ratio;
  /** What one unit of it is marked at, which is what the fund's own NAV is read off (B2). */
  readonly markPerUnit: PerPiece;
}

/**
 * E3: what a creation unit is made of.
 *
 * A pro-rata slice of what the fund ACTUALLY HOLDS once it holds anything — so a creation cannot
 * change what the fund is, and a redemption gives back exactly what it took a share of (Law 19:
 * read the book, do not restate it). Before there is a book, it is the basket the fund was launched
 * with, which is the one thing anybody could go on.
 */
export function basketOf(ctx: MechanismContext, d: FundDecl, share: InstrumentId): BasketLine[] {
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
        perShare: ratioOf(units, issued, 'units of this line behind one share'),
        markPerUnit: mark.value,
      });
    }
    return out;
  }
  for (const [line, perShare] of Object.entries(inKindOf(d).basket)) {
    const id = instrumentId(line);
    if (!ctx.instruments.has(id) || perShare <= 0) continue;
    const mark = lastMark(ctx, id);
    if (!mark.some) return [];
    out.push({
      instrument: id,
      perShare: asRatio(perShare, 'units of this line behind one share'),
      markPerUnit: mark.value,
    });
  }
  return out;
}

/**
 * Clearing F1: the last thing a market said about a line, which is what anybody acting before this
 * period's session has to go on. A claim on a book is worth what the book comes to whenever anybody
 * asks (Fund Shares B1), so that one is read fresh; everything else is what it last printed.
 */
function lastMark(ctx: MechanismContext, instrument: InstrumentId): Option<PerPiece> {
  const kind = ctx.registry.instrumentKind(ctx.instruments.get(instrument).kind);
  if (kind.pricing === 'derived') return some(ctx.valuation.markPerUnit(instrument, ctx.period));
  const p = ctx.prices.latest(instrument, ctx.period);
  return p.some ? some(p.value.price) : none<PerPiece>();
}

/** What one share's worth of basket is marked at — which is the NAV when the basket is the book. */
export function basketValue(basket: readonly BasketLine[]): PerPiece {
  return sum(
    basket.map((b) => scale(b.markPerUnit, b.perShare, 'what this line contributes')),
  ).value;
}

/**
 * Law 8: what a party can actually take or hand over of a line — whole pieces of it, and for a cell
 * whole pieces PER MEMBER, because every member of it is a real holder of a real piece. What the
 * grid leaves over is not created, delivered or redeemed: it stays where it already was.
 */
function onGrid(
  ctx: MechanismContext,
  holder: Party,
  instrument: InstrumentId,
  total: Qty,
): { readonly perMember: Qty; readonly total: Qty } {
  const unit = ctx.instruments.get(instrument).unit;
  return shareFor(
    ctx.registry,
    holder,
    unit,
    over(total, asRatio(weightOf(holder), 'the members it has'), 'per member'),
  );
}

/**
 * E3, G1.a, C3: a CREATION. The party delivers a pro-rata slice of the fund's book and takes new
 * shares against it, in one instruction: every leg or none (XI-5). No cash moves, no market is
 * touched, and the fund's composition is exactly what it was.
 *
 * The shares are issued at what the basket delivered is worth, so the fund's assets and the claims
 * on them move by the same amount and its equity stays at zero (A3). That is not an adjustment: it
 * is what "a share is a claim on the book" means when the book grows by a slice of itself — and it
 * is why the price the shares are struck at is read off what the grid let the creator actually
 * deliver, rather than off what a whole basket would have been worth (Law 8, Law 19).
 */
export function create(
  ctx: MechanismContext,
  d: FundDecl,
  share: InstrumentId,
  party: PartyId,
  wanted: Qty,
): boolean {
  const basket = basketOf(ctx, d, share);
  if (basket.length === 0 || wanted <= 0) return false;
  if (!(basketValue(basket) > 0)) return false;
  const holder = ctx.parties.get(party);
  const weight = weightOf(holder);
  const made = onGrid(ctx, holder, share, wanted);
  const shares = made.total;
  if (shares <= 0) return false;
  const legs: Leg[] = [];
  const delivered: Cash[] = [];
  for (const line of basket) {
    const put = onGrid(ctx, holder, line.instrument, scale(shares, line.perShare, 'units of this line'));
    const units = put.total;
    if (!material(units, 2, units)) return false;
    // F1: it delivers what it holds. A creator that has not got the basket does not create.
    if (scaleQty(ctx.register.free(party, line.instrument), weight, 'what it holds') < units) {
      return false;
    }
    legs.push({
      kind: 'asset',
      from: party,
      to: d.fund as PartyId,
      instrument: line.instrument,
      qty: units,
      pricePerUnit: some(line.markPerUnit),
      accruedPerUnit: none(),
      fromCell: cellOf(holder, put.perMember),
      toCell: none(),
    });
    delivered.push(valueAt(line.markPerUnit, units, 'what this line delivered'));
  }
  const perShare = pricedAt(sum(delivered).value, shares, 'what a share was issued at');
  legs.push({
    kind: 'asset',
    from: d.fund as PartyId,
    to: party,
    instrument: share,
    qty: shares,
    pricePerUnit: some(perShare),
    accruedPerUnit: none(),
    fromCell: none(),
    toCell: cellOf(holder, made.perMember),
  });
  const r = ctx.settle({
    legs,
    cause: 'issuance',
    reason: `${party} creates ${shares} of ${share} in kind`,
  });
  ctx.record(
    'fund.created',
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
  d: FundDecl,
  share: InstrumentId,
  party: PartyId,
  wanted: Qty,
): boolean {
  const basket = basketOf(ctx, d, share);
  if (basket.length === 0 || wanted <= 0) return false;
  if (!(basketValue(basket) > 0)) return false;
  const holder = ctx.parties.get(party);
  const weight = weightOf(holder);
  const back = onGrid(ctx, holder, share, wanted);
  const shares = back.total;
  if (shares <= 0) return false;
  const held = scaleQty(ctx.register.free(party, share), weight, 'shares it can give back');
  if (held < shares) return false;
  const legs: Leg[] = [];
  const taken: Cash[] = [];
  for (const line of basket) {
    const got = onGrid(
      ctx,
      holder,
      line.instrument,
      scale(shares, line.perShare, 'units of this line'),
    );
    const units = got.total;
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
      toCell: cellOf(holder, got.perMember),
    });
    taken.push(valueAt(line.markPerUnit, units, 'what this line gave back'));
  }
  const perShare = pricedAt(sum(taken).value, shares, 'what a share was redeemed at');
  legs.unshift({
    kind: 'asset',
    from: party,
    to: d.fund as PartyId,
    instrument: share,
    qty: shares,
    pricePerUnit: some(perShare),
    accruedPerUnit: none(),
    fromCell: cellOf(holder, back.perMember),
    toCell: none(),
  });
  const r = ctx.settle({
    legs,
    cause: 'maturity',
    reason: `${party} redeems ${shares} of ${share} in kind`,
  });
  ctx.record(
    'fund.redeemed',
    [d.fund, party, share],
    { fund: d.fund, by: party, shares, perShare, settled: r.outcome === 'settled' },
    true,
  );
  return r.outcome === 'settled';
}

/** XI-15: a cell side carries the per-member amount; a named party carries none. */
function cellOf(
  holder: { readonly representation: string; readonly weight?: number },
  perMember: Qty,
): Option<CellSide> {
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
export function premiumOf(print: Option<PerPiece>, nav: PerPiece): Option<Ratio> {
  if (!print.some || nav <= 0) return none();
  return some(
    ratioOf(minus(print.value, nav, 'what the market pays over the book'), nav, 'as a share of it'),
  );
}

/** The period a read is about, so a stale print is visibly a stale premium (Clearing E4). */
export type At = Period;
