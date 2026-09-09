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
import type { InstrumentId, MarketId, PartyId } from '../../core/ids.js';
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
