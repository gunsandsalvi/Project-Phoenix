/**
 * Where a household puts what it does not spend.
 *
 * @spec Households C2 Households D1 Households D1.a Households D5 Households D5.a Households D6 Equity B1 Equity B3 Equity B6 Equity C2 Equity C2.a Expectations B3 Sovereign E2.f Money A1 XI-13 XI-15 Law 8
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
import { nextPeriod, period } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { instrumentId, type InstrumentId, type MarketId, type PartyId, type VenueId } from '../../core/ids.js';
import type { Event } from '../../journal/journal.js';
import { add, div, material, mul, sum } from '../../core/num.js';
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

/** A line this cell would hold, and what it thinks one unit of it is worth. */
export interface SavingLine {
  readonly instrument: Instrument;
  readonly price: number;
}

/**
 * D5, D5.a, Equity B3: everywhere this cell's savings could go, in one pass over the lines that
 * exist, with what it thinks each is worth.
 *
 * TWO KINDS OF CLAIM and one traversal, because a line is one or the other by what it promises and
 * the cell has to look at it once to find out (Law 4: one decision, one pass, one budget):
 *
 * - **Paper.** It promises dated payments (Bond N5), so the cell prices it at the level that makes
 *   those payments return what it requires: its liquidity premium, what it wants for giving up
 *   access to its money. It will only tie its money up for its own horizon, so anything that comes
 *   back later is not somewhere its money can go.
 * - **A share.** It promises NOTHING (Equity A4). Nothing about it can be discounted, so the only
 *   thing the cell can go on is what the issuer has actually been paying, which the issuer declares
 *   publicly (Equity D3), capitalised at what it requires of a claim that promises nothing — its
 *   liquidity premium PLUS how wrong its own income has recently been (§46 B3). Two cells with
 *   different histories therefore want different prices for the same firm, and that disagreement is
 *   what gives the book two sides (§46 A3, XI-13).
 *
 * This is the "yield against risk" D5 names, and it is the reason the horizon comment above pointed
 * at worklist 9: until something in this world priced risk, a cell had only liquidity to weigh.
 */
export function savingLines(
  view: ParticipantView,
  required: number,
  uncertainty: number,
  horizonPeriods: number,
): { readonly paper: SavingLine[]; readonly shares: SavingLine[] } {
  const region = view.self.region;
  const ccy = view.registry.region(region).ccy;
  const on = view.calendar.startOf(view.period);
  // Money G3.a: a horizon is a DATE the calendar places, never a count of periods turned into years.
  const by = view.calendar.startOf(period(view.period + horizonPeriods));
  const year = yearFraction('ACT/365F', on, view.calendar.startOf(nextPeriod(view.period)));
  const forShares = add(required, uncertainty, 'what it requires of a claim that promises nothing');
  const sovereigns = new Set(
    view.parties.ofKind(TREASURY).filter((t) => t.region === region && t.status.alive).map((t) => t.id),
  );
  const paper: SavingLine[] = [];
  const shares: SavingLine[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || !i.market.some || i.ccy !== ccy || !i.issuer.some) continue;
    const profile = view.registry.instrumentKind(i.kind);
    if (profile.physical === true) continue;
    const flows = profile.cashFlows(i, on, view.calendar);
    const last = flows[flows.length - 1];
    if (last !== undefined) {
      // Paper. It has to be somebody's whose paper this cell can price off a public curve, and it
      // has to come back inside its own horizon (D5).
      if (!sovereigns.has(i.issuer.value)) continue;
      if (compareCivil(last.date, by) > 0) continue;
      const family = view.registry.curveFamily(curveFamilyOf(i.issuer.value, ccy));
      const price = priceAt(flows, required, on, family.dayCount, `what ${i.id} is worth to a saver`);
      if (price > 0) paper.push({ instrument: i, price });
      continue;
    }
    // A share: what it has been paying, capitalised at what this cell requires of it.
    if (forShares <= 0 || year <= 0) continue;
    const declared = view.lastPublicAbout('equity.dividend', i.issuer.value);
    if (!declared.some) continue;
    const perShare = declared.value.data['perShare'];
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    // Law 8: what it was paid is per period and what it requires is per annum, so the period is
    // turned into the fraction of a year the calendar says it is.
    const value = div(div(perShare, year, 'what it paid, per annum'), forShares, 'what it is worth');
    if (value > 0) shares.push({ instrument: i, price: value });
  }
  return { paper, shares };
}

/**
 * D5, D5.a: the bids for paper, one per line, for the money the cell decided this line gets.
 * `perLine` is its own budget divided by every place its money could go, so the same money is never
 * committed twice and no class of thing is preferred by a rule nobody stated.
 */
export function paperBids(
  view: ParticipantView,
  eligible: readonly SavingLine[],
  perLine: number,
  lines: number,
): PaperBid[] {
  if (perLine <= 0) return [];
  const out: PaperBid[] = [];
  for (const e of eligible) {
    // Bond N9.b: what it must find is the clean price plus what has accrued and travels with it.
    const dirty = add(e.price, view.accrued(e.instrument.id), 'what a unit costs it');
    const qty = div(perLine, dirty, 'units it bids for');
    // Law 7: this line's share against what the whole budget would have bought — a share that
    // small is the rounding of the split, not a bid.
    const whole = div(mul(perLine, lines, 'the whole of it'), dirty, 'what it would buy');
    if (!material(qty, lines + 1, whole) || !e.instrument.market.some) continue;
    out.push({ market: e.instrument.market.value, instrument: e.instrument.id, price: e.price, qty });
  }
  return out;
}

/**
 * §46 B3: the extra this cell wants for holding a claim that promises nothing — how wide its own
 * income surprises have been against what it expects to be paid, as a rate. A read of its own
 * history and its own outlook, in its own units, with nothing stated anywhere: a cell that has
 * never been surprised wants nothing extra, and one whose income has been all over the place wants
 * a great deal, which is why the same firm is worth different amounts to two of them.
 */
export function ownUncertainty(view: ParticipantView): number {
  const income = view.outlook('income');
  if (!income.some || income.value.expected <= 0) return 0;
  const year = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(nextPeriod(view.period)),
  );
  if (year <= 0) return 0;
  return div(
    div(income.value.confidence, income.value.expected, 'how wrong its income has been'),
    year,
    'per annum',
  );
}

/** A bid or an offer in a share line, at the level this cell's own opinion puts on it. */
export interface ShareOrder {
  readonly market: MarketId;
  readonly instrument: InstrumentId;
  readonly side: 'buy' | 'sell';
  /** 'market' only when it is selling because it needs the money (XI-2). */
  readonly price: number | 'market';
  readonly qty: number;
}

/**
 * Equity B1, B6, Clearing A2: the opinion as a two-sided schedule.
 *
 * It BIDS below its opinion, in steps, for the money this line's share of its budget comes to; it
 * OFFERS what it holds AT its opinion. The two can never both fill — every bid is strictly below
 * the ask — so a holder never trades with itself, and a seller with no buyer keeps its shares (B6).
 *
 * A cell that needs its money back is a seller and NOT a buyer: it offers at whatever the market
 * gives, which is what a forced seller posts (XI-2), and it bids for nothing.
 */
export function shareOrders(
  view: ParticipantView,
  lines: readonly SavingLine[],
  perLine: number,
  short: number,
  steps: number,
): ShareOrder[] {
  const out: ShareOrder[] = [];
  const weight = view.self.representation === 'cell' ? view.self.weight : 1;
  for (const line of lines) {
    if (!line.instrument.market.some) continue;
    const market = line.instrument.market.value;
    const units = mul(view.free(line.instrument.id), weight, 'shares it could sell');
    if (short > 0) {
      if (material(units, 2, units)) {
        out.push({ market, instrument: line.instrument.id, side: 'sell', price: 'market', qty: units });
      }
      continue;
    }
    if (material(units, 2, units)) {
      out.push({ market, instrument: line.instrument.id, side: 'sell', price: line.price, qty: units });
    }
    if (perLine <= 0 || steps < 1) continue;
    const each = div(mul(perLine, weight, 'what the cell puts in'), steps, 'at each level');
    for (let step = 1; step <= steps; step += 1) {
      // Clearing A2.a: how much at each price. The ladder runs from just under its opinion down,
      // so what it buys grows as the market asks it for less.
      const price = mul(line.price, div(step, steps + 1, 'this rung'), 'level');
      if (price <= 0) continue;
      const qty = div(each, price, 'shares it bids for');
      if (!material(qty, steps + 1, div(mul(each, steps, 'the whole of it'), price, 'what it would buy'))) continue;
      out.push({ market, instrument: line.instrument.id, side: 'buy', price, qty });
    }
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

/** C2: how much of what it is about to spend its account cannot cover, per member (Law 7 as above). */
export function shortForSpending(cash: number, spend: number): number {
  const gap = spend - cash;
  const scale = sum([cash, spend]);
  return gap > 0 && material(gap, scale.terms, scale.value) ? gap : 0;
}

/**
 * D2: the part of its money that belongs in a fund rather than in its account — what it holds over
 * what it is about to spend, up to the cushion it wants. Above the cushion it would rather have
 * paper (D5), and below what it is about to spend there is nothing to put anywhere.
 */
export function cushionForFund(cash: number, spend: number, spare: number): number {
  const over = cash - spend - spare;
  const scale = sum([cash, spend, spare]);
  return over > 0 && material(over, scale.terms, scale.value) ? over : 0;
}

/**
 * C2, D1: what a cell has left over once it has spent and kept its cushion, per member.
 *
 * Law 7: a residue that is the rounding of the subtraction is not money it has over. A cell that
 * ends up with 1e-310 "spare" would post an order for 1e-313 units, and a quantity that small
 * cannot be a cell's per-member share of anything: multiplied back by the weight it does not give
 * the total again, and the wire refuses it (XI-15). The dust is the magnitudes it came out of.
 */
export function sparePerMember(cash: number, spend: number, buffer: number): number {
  const left = cash - spend - buffer;
  const scale = sum([cash, spend, buffer]);
  return left > 0 && material(left, scale.terms, scale.value) ? left : 0;
}

/** The total a cell of this weight commits, from a per-member decision (XI-15). */
export function totalOf(perMember: number, weight: number): number {
  return mul(perMember, weight, 'what the cell commits');
}

export type { PartyId };
