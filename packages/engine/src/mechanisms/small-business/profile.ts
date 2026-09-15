/**
 * What a cell of small firms decides: what to make from its members' hours and the inputs on hand,
 * what to bid for the inputs the next batch needs, and what to offer of what it made.
 *
 * @spec Small-Business Pools A1 Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A4 Goods A2 Goods A2.a Goods B1 Goods B1.c Goods C1 Goods E1 Firm B1 Firm E4 Labour C1 Labour C1.a Expectations A2 XI-15 Law 2 Law 6 Law 8 Law 19
 *
 * A SMALL FIRM IS A FIRM (A1) AND IT IS ITS OWNER'S HOURS. Its members work in it themselves —
 * that is what "one person and a van" means — so the hours it has are its members' hours, at the
 * recipe the line names (Goods A2: physical quantities per unit, never a share of cost), and no
 * wage leaves it for them. What it employs beyond its members arrives at 11.0c, through the same
 * venue a named firm hires in.
 *
 * EVERY DECISION IS PER CELL FROM ITS OWN STATE: what it holds, what it expects, what its people
 * can make. There is no representative small firm (A2.a): two cells in one line with different
 * cash bid for different quantities of the same input, and the one that cannot fund its batch
 * makes less, which is where a credit tightening bites first (A5.a) once there is credit to
 * tighten (11.0e).
 *
 * WHAT IT MAKES IS SOLD THE PERIOD IT IS MADE. The small tier's lines are the ones whose output is
 * made where it is bought (`GOODS` with `output: 'capacity'`): a meal, a journey, an hour of care.
 * Nothing of that can be held, so what was made is offered at whatever the book gives (Firm E4,
 * XI-2's forced seller, and the same rule a named firm applies to what will perish) — a reason
 * and not a level, and the whole of its supply schedule.
 *
 * WHAT IT WILL PAY for an input is what that input is worth to it: the output it makes possible
 * at the price it expects, less the other inputs that unit still needs (Labour C1, C1.a; the same
 * arithmetic `firms/decide.ts` runs, with no wage term because its hours are its own). It buys
 * with the money it has (C1.d): a bid it could not settle is a trade that fails, and the failed
 * instruction is a recorded state nobody wanted.
 */
import { amountOf, asAmount, asPerPiece, asRatio, type Cash, heldAsMoney, minus, over, type PerPiece, pricedAt, type Ratio, scale, valueAt } from '../../core/measure.js';
import type { InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { unitId } from '../../core/ids.js';
import { atMost, material, sum } from '../../core/num.js';
import { none, type Option, some } from '../../core/option.js';
import { asQty, downTick, NO_QTY, type Qty, subQty, upTick } from '../../core/tick.js';
import type { Period } from '../../calendar/calendar.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Order, OrderPrice } from '../../clearing/solver.js';
import { keyOf, weightOf } from '../../parties/party.js';
import { costOfDraw } from '../../register/register.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { goodId, goodMarketId, goodTerms, type GoodTerms } from '../../registry/physical.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';
import { OCCUPATION_OF } from '../../registry/occupations.js';
import { ownPayroll, payrollSettledIn } from '../../registry/wages.js';
import { findVenue } from '../../clearing/venue.js';
import { addQty } from '../../core/tick.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';

/** The name of the store a cell's decision waits in between its phases (a `working` noun). */
export const DECIDED = 'smallBusiness.decided';

/** Law 8: an hour is the labour venue's unit, and a cell's hours are counted in it. */
const HOURS = unitId('hours');

export interface PlannedOrder {
  readonly market: MarketId;
  readonly side: 'buy' | 'sell';
  readonly price: OrderPrice;
  readonly qty: Qty;
}

interface DecidedThisPeriod {
  at: Period | undefined;
  /** The input bids for the next batch. What it sells is read off the register when asked. */
  orders: readonly PlannedOrder[];
  /** What it will start this period, in whole pieces of its output. */
  batch: Qty;
}

export const nothingDecided = (): DecidedThisPeriod => ({ at: undefined, orders: [], batch: NO_QTY });

/** A2, Goods A2: the line a cell is in, as the registry declares it — its output and its recipe. */
export interface Line {
  readonly output: InstrumentId;
  readonly market: MarketId;
  readonly terms: GoodTerms;
  readonly hoursPerUnit: Ratio;
  readonly inputs: readonly { readonly instrument: InstrumentId; readonly qtyPerUnit: Ratio }[];
}

export function lineOf(view: ParticipantView): Option<Line> {
  const self = view.self;
  if (self.representation !== 'cell') return none<Line>();
  const output = goodId(keyOf(self, 'line'), self.region);
  if (!view.instruments.has(output)) return none<Line>();
  const terms = goodTerms(view.instruments.get(output));
  return some({
    output,
    market: goodMarketId(terms.subUnit, terms.region),
    terms,
    hoursPerUnit: view.params.ratio(terms.recipe.labourHoursPerUnit),
    inputs: terms.recipe.inputs.map((i) => ({
      instrument: goodId(i.subUnit, terms.region),
      qtyPerUnit: view.params.ratio(i.qtyPerUnit),
    })),
  });
}

/** Goods B1.c, XI-15: its members' hours — every member's, because they work in it. */
function ownHours(view: ParticipantView): Qty {
  const each = view.params.amount(PEOPLE_PARAMS.hoursPerMember, HOURS);
  return asQty(each * weightOf(view.self), 'the hours its members have between them');
}

/**
 * Labour C2, 11.0c: AND THE HOURS IT EMPLOYS beyond its members, under contract at its last
 * payroll — the same read a named firm makes of its own wage bill (`registry/wages.ts`).
 */
function hoursUnderContract(view: ParticipantView): Qty {
  const own = ownPayroll(view, view.period);
  return own.some ? own.value.hours : NO_QTY;
}

/** The hours that can make something this period: its members' and what it employs. */
function hoursToPlanWith(view: ParticipantView): Qty {
  return addQty(ownHours(view), hoursUnderContract(view), 'the hours it has');
}

/**
 * Goods B1, B1.c: what a cell CAN start — the least of what its hours reach and what each input on
 * hand reaches. Every limit is a count of the output; the binding one is the state (Law 6: a real
 * shortage, never a cap).
 */
function canStart(view: ParticipantView, line: Line, hours: Qty): Qty {
  const limits: number[] = [over(hours, line.hoursPerUnit, 'what its people can make')];
  for (const input of line.inputs) {
    limits.push(over(view.free(input.instrument), input.qtyPerUnit, 'what the stock on hand reaches'));
  }
  const least = limits.reduce((a, b) => atMost(a, b, 'a batch is no bigger than its scarcest limit'));
  return downTick(asAmount<'piece'>(least, 'what it can start'));
}

/**
 * The decision, once per cell per period: the batch it starts out of what it holds, and the bids
 * for what the NEXT batch needs, at what each input is worth to it.
 */
export function decide(ctx: MechanismContext, cell: PartyId): void {
  const view = ctx.participant(cell);
  const line = lineOf(view);
  const slot = ctx.workingOf(cell, DECIDED, nothingDecided);
  slot.at = ctx.period;
  slot.orders = [];
  slot.batch = NO_QTY;
  if (!line.some) return;
  const l = line.value;
  slot.batch = canStart(view, l, hoursToPlanWith(view));
  // Expectations A2: what a unit of its output fetches, by its own outlook or the tape. A cell that
  // has never seen a price for what it makes cannot say what an input is worth to it, and does not
  // bid — it makes what it can out of what it holds and learns the price by selling.
  const price = expectedPriceOf(view, l.output);
  if (!price.some) return;
  const priced: PerPiece[] = [];
  for (const i of l.inputs) {
    const p = expectedPriceOf(view, i.instrument);
    // An input it has never seen a price for is an input it cannot value, and it does not bid.
    if (!p.some) return;
    priced.push(p.value);
  }
  // The next batch is what its hours reach: the inputs are what it is buying.
  const nextBatch = downTick(over(hoursToPlanWith(view), l.hoursPerUnit, 'what its people can make'));
  const ccy = view.registry.currencyOf(view.self.region);
  let cash: Cash = heldAsMoney(view.cash(ccy), 'the money it has to buy with');
  const orders: PlannedOrder[] = [];
  for (const [n, input] of l.inputs.entries()) {
    const need = upTick(scale(nextBatch, input.qtyPerUnit, 'what the batch draws'));
    const buy = subQty(need, view.quantity(input.instrument), 'what it must buy');
    if (!material(buy, 2, need) || buy <= 0) continue;
    const others = sum(
      l.inputs.map((other, m) =>
        m === n
          ? asPerPiece(0, 'the input being priced is not one of the others')
          : scale(priced[m] ?? asPerPiece(0, 'priced above, one per input'), asRatio(other.qtyPerUnit, 'what one takes of it'), 'other inputs'),
      ),
    ).value;
    const worth = over(
      minus(price.value, others, 'less the other inputs'),
      input.qtyPerUnit,
      'what a unit of the input is worth',
    );
    if (worth <= 0) continue;
    // C1.d: it buys with the money it has. What its cash reaches is an arithmetic limit on the
    // order and not a bound on the decision; the rest of its need goes unmet, which is a state.
    const affordable = downTick(amountOf(cash, worth, 'what its money reaches'));
    const qty = atMost(buy, affordable, 'it bids for what it can pay for');
    if (qty <= 0) continue;
    cash = minus(cash, valueAt(worth, qty, 'what this bid commits'), 'what is left for the next input');
    orders.push({ market: goodMarketId(goodTerms(view.instruments.get(input.instrument)).subUnit, l.terms.region), side: 'buy', price: worth, qty });
  }
  slot.orders = orders;
  postForHours(ctx, view, l, price.value, priced);
  ctx.record(
    'smallBusiness.plan',
    [cell],
    { cell, line: l.terms.subUnit, batch: slot.batch, bids: orders.length },
    true,
  );
}

/**
 * Labour C1, C1.a, C5, D1 (11.0c): WHAT IT WILL PAY FOR AN HOUR, AND HOW MANY IT WANTS. An hour is
 * worth what the output it makes possible fetches, less the rest of the recipe — the most it will
 * pay, and the same arithmetic a named firm posts (Law 4). How many it wants is what its stock of
 * inputs can use beyond its members' own hours: a cell whose people already out-run its stock
 * wants nobody, and posts that — an empty opening, which the venue reads against the hours it has
 * under contract and sheds the difference at the cell's cost (Labour C3). The venue it posts in is
 * the trade its line employs, in its region (Labour A3): a fact of the registry, never a branch.
 */
function postForHours(
  ctx: MechanismContext,
  view: ParticipantView,
  line: Line,
  priceOut: PerPiece,
  inputPrices: readonly PerPiece[],
): void {
  const occupation = OCCUPATION_OF[line.terms.subUnit];
  if (occupation === undefined) return;
  const venue = findVenue(view.venues, { region: String(view.self.region), occupation });
  if (venue === undefined) return;
  const inputCost = sum(
    line.inputs.map((i, n) =>
      scale(inputPrices[n] ?? asPerPiece(0, 'priced above, one per input'), asRatio(i.qtyPerUnit, 'what one takes of it'), 'input cost'),
    ),
  ).value;
  const perHour = over(minus(priceOut, inputCost, 'less its inputs'), line.hoursPerUnit, 'the value of an hour');
  // What its inputs on hand could make, over what its members' hours already make: the hours it
  // has a use for. Nothing below zero is wanted, and wanting nobody is a real posting.
  const fromStock = line.inputs.map((i) => over(view.free(i.instrument), i.qtyPerUnit, 'what the stock reaches'));
  const stockBound = fromStock.length === 0 ? undefined : fromStock.reduce((a, b) => atMost(a, b, 'the scarcest input'));
  const wanted = stockBound === undefined ? NO_QTY : downTick(scale(minus(asAmount<'piece'>(stockBound, 'what its stock makes'), asAmount<'piece'>(over(ownHours(view), line.hoursPerUnit, 'what its members make'), 'its members\u2019 share'), 'beyond its members'), line.hoursPerUnit, 'the hours that would take'));
  const wantsNobody = wanted <= 0 || perHour <= 0;
  ctx.post(venue.id, {
    party: view.self.id,
    side: 'buy',
    price: wantsNobody ? 0 : perHour,
    qty: wantsNobody ? NO_QTY : wanted,
  });
}

/** Goods B5, E1: the batch, started — inputs drawn at what they cost, output created at that cost. */
export function produce(ctx: MechanismContext, cell: PartyId): void {
  const view = ctx.participant(cell);
  const slot = ctx.workingOf(cell, DECIDED, nothingDecided);
  if (slot.at !== ctx.period || slot.batch <= 0) return;
  const line = lineOf(view);
  if (!line.some) return;
  const l = line.value;
  // Labour C2: what can be started NOW is bounded by the hours actually paid for this period —
  // its members' and what its payroll settled — and by what it decided; a hire found this period
  // is paid and not yet working, and that is a real shortage rather than a plan gone wrong.
  const settled = payrollSettledIn(ctx.journal, String(cell), ctx.period);
  const hoursNow = addQty(ownHours(view), settled.some ? settled.value.productive : NO_QTY, 'the hours it has');
  const batch = atMost(slot.batch, canStart(view, l, hoursNow), 'it starts what its hours and its stock reach');
  if (batch <= 0) return;
  const legs: Leg[] = [];
  // Goods B5: what the period's labour cost, capitalised into the batch like a named firm's wages.
  const costs: Cash[] = [settled.some ? settled.value.paid : heldAsMoney(NO_QTY, 'it employed nobody this period')];
  for (const input of l.inputs) {
    const qty = upTick(scale(batch, input.qtyPerUnit, 'what the recipe draws'));
    if (qty <= 0) continue;
    const held = ctx.register.holding(cell, input.instrument);
    costs.push(held.some ? costOfDraw(held.value.lots, qty) : heldAsMoney(NO_QTY, 'nothing held cost nothing'));
    legs.push({ kind: 'destroy', party: cell, instrument: input.instrument, qty, why: 'consumed' });
  }
  const cost = sum(costs).value;
  legs.push({
    kind: 'create',
    party: cell,
    instrument: l.output,
    qty: batch,
    // E1: what the inputs it drew cost, over the batch. Its members' hours cost it no cash.
    costPerUnit: pricedAt(cost, batch, 'what a unit cost to make'),
  });
  const record = ctx.settle({ legs, cause: 'production', reason: `${String(cell)} made ${String(batch)} of ${l.terms.subUnit}` });
  ctx.record(
    'smallBusiness.produced',
    [cell],
    { cell, line: l.terms.subUnit, batch, cost, settled: record.outcome === 'settled' },
    true,
  );
}

/** The books this cell is in this period: its own line's, and the inputs it is bidding for. */
export function marketsOf(view: ParticipantView): readonly MarketId[] {
  const decided = view.working(DECIDED, nothingDecided);
  const line = lineOf(view);
  const out = new Set<MarketId>();
  if (line.some) out.add(line.value.market);
  if (decided.at === view.period) for (const o of decided.orders) out.add(o.market);
  return [...out];
}

/**
 * Firm E4, XI-2: what it made, offered at whatever the book gives — it cannot be held — and the
 * bids it decided. The sell is read off the register when the market asks, because what it holds
 * is what it made this period and nothing decided earlier could know that.
 */
export function ordersIn(view: ParticipantView, market: MarketId): readonly Order[] {
  const out: Order[] = [];
  const line = lineOf(view);
  if (line.some && line.value.market === market) {
    const free = view.free(line.value.output);
    if (free > 0) out.push({ party: view.self.id, side: 'sell', price: 'market', qty: free });
  }
  const decided = view.working(DECIDED, nothingDecided);
  if (decided.at !== view.period) return out;
  for (const o of decided.orders) {
    if (o.market === market) out.push({ party: view.self.id, side: o.side, price: o.price, qty: o.qty });
  }
  return out;
}

