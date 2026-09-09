/**
 * Where a household puts what it does not spend.
 *
 * @spec Households C2 Households D1 Households D1.a Households D5 Households D5.a Households D6 Sovereign E2.f Money A1 XI-15
 *
 * D5.a: the choice between a deposit and paper bought directly is a real substitution, and it is
 * how a rate reaches a saver. A deposit is a holding of a bank's money — it returns nothing at all
 * here, because paying for deposits is a decision a bank has not been given yet (Banks Funding B1,
 * worklist 11) — so what a household gives up by holding paper instead is access to its money, and
 * what it wants for giving that up is its own liquidity preference. Nothing else enters: it is not
 * offered fund shares pro rata, and it is nobody's residual holder (D6).
 *
 * It bids at the price its OWN required yield gives, never at the print: the price it will pay is
 * the one at which the paper returns what it wants, so a line that is dear to it does not fill and
 * a line that is cheap does. What it does not put into paper stays in its account, which is what
 * saving into a deposit is (C2).
 *
 * It only considers paper that COMES BACK inside its own horizon. Anything longer it would have to
 * sell before maturity at a price nobody can tell it, and what that is worth is the other two
 * reasons D5 names — yield against risk — which a household cannot weigh until something in this
 * world prices risk (worklist 9). Liquidity is the reason it has now, and this is the whole of it.
 */
import { period } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { instrumentId, type InstrumentId, type MarketId, type PartyId, type VenueId } from '../../core/ids.js';
import type { Event } from '../../journal/journal.js';
import { add, div, material, mul } from '../../core/num.js';
import { curveFamilyOf, priceAt } from '../../prices/curve.js';
import type { Instrument } from '../../register/instruments.js';
import { TREASURY } from '../../registry/profiles.js';
import type { ParticipantView } from '../../world/context.js';

/** A bid for paper: a size at the level this cell's own requirement puts on it. */
export interface PaperBid {
  readonly market: MarketId;
  readonly instrument: InstrumentId;
  readonly price: number;
  readonly qty: number;
}

/**
 * D5, D5.a: what it will hold instead of money, and at what price. The curve is a public read of
 * what this paper has been fetching (Sovereign D3); what it says a line returns is what the cell
 * compares against its own requirement, and the price it then bids is its own.
 */
export function paperBids(
  view: ParticipantView,
  required: number,
  horizonPeriods: number,
  spare: number,
): PaperBid[] {
  if (spare <= 0) return [];
  const region = view.self.region;
  const ccy = view.registry.region(region).ccy;
  const on = view.calendar.startOf(view.period);
  // Money G3.a: a horizon is a DATE the calendar places, never a count of periods turned into years.
  const by = view.calendar.startOf(period(view.period + horizonPeriods));
  const eligible: { readonly instrument: Instrument; readonly price: number }[] = [];
  const sovereigns = new Set(
    view.parties.ofKind(TREASURY).filter((t) => t.region === region && t.status.alive).map((t) => t.id),
  );
  for (const i of view.instruments.all()) {
    if (!i.status.live || !i.market.some || i.ccy !== ccy) continue;
    if (!i.issuer.some || !sovereigns.has(i.issuer.value)) continue;
    const family = view.registry.curveFamily(curveFamilyOf(i.issuer.value, ccy));
    const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
    const last = flows[flows.length - 1];
    if (last === undefined) continue;
    // D5: it will tie its money up for its own horizon and no longer.
    if (compareCivil(last.date, by) > 0) continue;
    const price = priceAt(flows, required, on, family.dayCount, `what ${i.id} is worth to a saver`);
    if (price > 0) eligible.push({ instrument: i, price });
  }
  if (eligible.length === 0) return [];
  // It has no reason to prefer one line over another once both clear what it requires, so it
  // spreads what it has across them. The rule is stated once, like a rationing rule (Clearing C3).
  const each = div(spare, eligible.length, 'what it puts into each line');
  const out: PaperBid[] = [];
  for (const e of eligible) {
    // Bond N9.b: what it must find is the clean price plus what has accrued and travels with it.
    const dirty = add(e.price, view.accrued(e.instrument.id), 'what a unit costs it');
    const qty = div(each, dirty, 'units it bids for');
    if (!material(qty, 2, qty) || !e.instrument.market.some) continue;
    out.push({ market: e.instrument.market.value, instrument: e.instrument.id, price: e.price, qty });
  }
  return out;
}

/**
 * D2, D5, D5.a: the third thing a saver can do with its money. A deposit returns nothing; a bill
 * returns what a bill returns and ties the money up; a MONEY FUND is a claim on short paper that
 * can be asked for back at any time, and what it returns is what the fund published it returned.
 *
 * The substitution is a consequence, not an allocation (D5): the cell puts new savings into the
 * fund only when what the fund actually returned clears what it requires of anything it holds
 * instead of money, and it asks for its money back when its own cushion is short — which is the
 * real reason anybody redeems, and the reason a shock to incomes becomes a redemption wave.
 */
export interface FundOrder {
  readonly venue: VenueId;
  readonly side: 'buy' | 'sell';
  /** Per member of the cell (XI-15); the posting carries the cell's weight. */
  readonly sharesPerMember: number;
}

/** What a cell holds in one fund, and what the fund last said a share of it is worth. */
export interface FundPosition {
  readonly venue: VenueId;
  readonly fund: string;
  readonly line: InstrumentId;
  readonly perShare: number;
  /** D2.a: what it offers a saver, after its manager. This is what competes with a deposit. */
  readonly offered: number;
  readonly sharesPerMember: number;
  readonly worthPerMember: number;
}

/**
 * D2, D5: what this cell has in the funds it can reach, at what each of them last published. The
 * venue says which line its shares are and the fund publishes what a share is worth (B1), so a
 * saver reads two public facts and nothing private (Observer A3).
 */
export function fundPositions(
  venues: readonly VenueDecl[],
  struck: readonly Event[],
  view: ParticipantView,
): FundPosition[] {
  const out: FundPosition[] = [];
  for (const v of venues) {
    if (v.key['kind'] !== 'fund') continue;
    const fund = v.key['fund'];
    // The venue says which line its shares are, because the venue is public data about itself and
    // a household has no business knowing how another module names things (Law 15).
    const line = v.key['share'];
    if (fund === undefined || line === undefined) continue;
    const last = struck.filter((e) => e.data['fund'] === fund).pop();
    if (last === undefined) continue;
    const perShare = last.data['perShare'];
    const offered = last.data['offered'];
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    const held = view.quantity(instrumentId(line));
    out.push({
      venue: v.id,
      fund,
      line: instrumentId(line),
      perShare,
      offered: typeof offered === 'number' ? offered : 0,
      sharesPerMember: held,
      worthPerMember: mul(held, perShare, 'what its shares are worth'),
    });
  }
  return out;
}

/**
 * D2, D5, D5.a: what a saver does with a money fund. It is where its CUSHION lives, because a fund
 * of short paper is a substitute for a deposit that pays it something (D2) — so what it holds over
 * what it is about to spend goes in, and what it is short of what it is about to spend comes back
 * out. Nothing here is an allocation: the flow is a consequence of the cell's own budget, and it
 * stops entirely the moment what the fund offers stops clearing what the cell requires (D5).
 */
export function fundOrders(
  positions: readonly FundPosition[],
  required: number,
  toFund: number,
  short: number,
): FundOrder[] {
  const out: FundOrder[] = [];
  for (const p of positions) {
    // C2: it asks for its money back because it needs it. Nothing else in a household's life is a
    // reason to redeem, and a rule that redeemed on a bad return would be a run written into it.
    if (short > 0) {
      if (p.sharesPerMember <= 0) continue;
      const want = div(short, p.perShare, 'shares it must give back');
      out.push({
        venue: p.venue,
        side: 'sell',
        sharesPerMember: want > p.sharesPerMember ? p.sharesPerMember : want,
      });
      continue;
    }
    // D2.a: and it puts money in when what the fund offers clears what it wants for giving up its
    // money — the competition D2 names, against a deposit that pays it nothing.
    if (toFund <= 0 || p.offered < required) continue;
    out.push({ venue: p.venue, side: 'buy', sharesPerMember: div(toFund, p.perShare, 'shares it asks for') });
  }
  return out;
}

/** C2: how much of what it is about to spend its account cannot cover, per member. */
export function shortForSpending(cash: number, spend: number): number {
  const gap = spend - cash;
  return gap > 0 ? gap : 0;
}

/**
 * D2: the part of its money that belongs in a fund rather than in its account — what it holds over
 * what it is about to spend, up to the cushion it wants. Above the cushion it would rather have
 * paper (D5), and below what it is about to spend there is nothing to put anywhere.
 */
export function cushionForFund(cash: number, spend: number, spare: number): number {
  const over = cash - spend - spare;
  return over > 0 ? over : 0;
}

/** C2, D1: what a cell has left over once it has spent and kept its cushion, per member. */
export function sparePerMember(cash: number, spend: number, buffer: number): number {
  const left = cash - spend - buffer;
  return left > 0 ? left : 0;
}

/** The total a cell of this weight commits, from a per-member decision (XI-15). */
export function totalOf(perMember: number, weight: number): number {
  return mul(perMember, weight, 'what the cell commits');
}

export type { PartyId };
