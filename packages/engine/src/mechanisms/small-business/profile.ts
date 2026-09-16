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
import type { InstrumentId, MarketId, PartyId, RegionId } from '../../core/ids.js';
import { paramId, unitId } from '../../core/ids.js';
import { atMost, material, sum } from '../../core/num.js';
import { none, type Option, some } from '../../core/option.js';
import { asQty, downTick, NO_QTY, type Qty, subQty, upTick } from '../../core/tick.js';
import type { Period } from '../../calendar/calendar.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Order, OrderPrice } from '../../clearing/solver.js';
import { keyOf, weightOf } from '../../parties/party.js';
import { costOfDraw } from '../../register/register.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { costOfCapital, plantOffers, project } from '../../registry/capital.js';
import { period } from '../../calendar/calendar.js';
import { toTick } from '../../core/tick.js';
import { capacityFrom, goodId, goodMarketId, goodTerms, type GoodTerms, type PlantNeed, rentedRoom, vintagesHeld, learnedHoursPerUnit,
} from '../../registry/physical.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';
import { OCCUPATION_OF } from '../../registry/occupations.js';
import { ownPayroll, payrollSettledIn, wholePeople } from '../../registry/wages.js';
import { netChange } from '../../register/employment.js';
import { findVenue } from '../../clearing/venue.js';
import { addQty } from '../../core/tick.js';
import { agreementKindId, type AgreementId } from '../../core/ids.js';
import type { AgreementTerms } from '../../register/agreements.js';
import { about, type MechanismContext, type ParticipantView } from '../../world/context.js';

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
  /**
   * 11.0d: WHAT IT KEEPS TO GO ON TRADING — a period of its inputs at the prices it expects, for
   * the batch its hours can make, and the wages it owes. Its own working capital, read off its own
   * plan; the owner draws what stands above it and what falls due on what it issued.
   */
  keeps: Cash;
}

export const nothingDecided = (): DecidedThisPeriod => ({ at: undefined, orders: [], batch: NO_QTY, keeps: heldAsMoney(NO_QTY, 'nothing decided') });

/**
 * Capital Programme B1.d, XI-16 A3 (11.2a.2): THIS POPULATION'S OWN HURDLE AND HORIZON, drawn once
 * from the declared mean and width and kept — the named firm's two management preferences, at a
 * cell: the margin over its cost of capital it insists on before it commits money it cannot get
 * back, and how many periods of a machine's service it counts. Drawn under the population's name
 * (region, bank, line) and not the cell's, so a member who crosses a band keeps its management.
 */
export const TERMS = 'smallBusiness.terms';
interface OwnTerms {
  hurdle: number | undefined;
  horizonPeriods: number | undefined;
}
const notYetDrawn = (): OwnTerms => ({ hurdle: undefined, horizonPeriods: undefined });

export const SMALL_FIRM_TERMS = {
  hurdle: paramId('smallBusiness.hurdle'),
  hurdleDispersion: paramId('smallBusiness.hurdle.dispersion'),
  horizonPeriods: paramId('smallBusiness.horizonPeriods'),
  horizonDispersion: paramId('smallBusiness.horizonPeriods.dispersion'),
} as const;

function ownTerms(view: ParticipantView): { readonly hurdle: Ratio; readonly horizonPeriods: number } {
  const own = view.working(TERMS, notYetDrawn);
  if (own.hurdle === undefined || own.horizonPeriods === undefined) {
    const self = view.self;
    const population =
      self.representation === 'cell'
        ? `${keyOf(self, 'region')}|${keyOf(self, 'bank')}|${keyOf(self, 'line')}`
        : String(self.id);
    const rng = view.rng.derive(`terms/${population}`);
    // Uniform on [mean − spread·mean, mean + spread·mean): centred, and a mean of nothing draws nothing.
    const hurdle = view.params.perAnnum(SMALL_FIRM_TERMS.hurdle);
    const hSpread = view.params.ratio(SMALL_FIRM_TERMS.hurdleDispersion);
    own.hurdle = hurdle + hurdle * hSpread * (2 * rng.next() - 1);
    const horizon = view.params.periods(SMALL_FIRM_TERMS.horizonPeriods);
    const zSpread = view.params.ratio(SMALL_FIRM_TERMS.horizonDispersion);
    // Law 8: a horizon is a count of periods.
    own.horizonPeriods = downTick(horizon + horizon * zSpread * (2 * rng.next() - 1));
  }
  return { hurdle: asRatio(own.hurdle, 'its own hurdle'), horizonPeriods: own.horizonPeriods };
}

/** A2, Goods A2: the line a cell is in, as the registry declares it — its output and its recipe. */
export interface Line {
  readonly output: InstrumentId;
  readonly market: MarketId;
  readonly terms: GoodTerms;
  readonly hoursPerUnit: Ratio;
  readonly inputs: readonly { readonly instrument: InstrumentId; readonly qtyPerUnit: Ratio }[];
  /** Capital Programme A2 (11.2a): the plant a unit of it takes, per kind — the room the table is in. */
  readonly plant: readonly PlantNeed[];
}

export function lineOf(view: ParticipantView): Option<Line> {
  const self = view.self;
  if (self.representation !== 'cell') return none<Line>();
  return lineIn(view, keyOf(self, 'line'), self.region);
}

/** A2, Goods A2 (12.1): the line by name and region, for whoever asks — a founder reads it too. */
export function lineIn(view: Pick<ParticipantView, 'instruments' | 'params' | 'made' | 'self'>, subUnit: string, region: RegionId): Option<Line> {
  const output = goodId(subUnit, region);
  if (!view.instruments.has(output)) return none<Line>();
  const terms = goodTerms(view.instruments.get(output));
  return some({
    output,
    market: goodMarketId(terms.subUnit, terms.region),
    terms,
    // 12c.1: what a MEMBER'S line has learned — the cell's pieces made over its people, against
    // the recipe's rate. A founder has made nothing and takes the recipe's hours.
    hoursPerUnit: learnedHoursPerUnit(
      view.params.ratio(terms.recipe.labourHoursPerUnit),
      // Law 8: a member has made whole pieces; the share of the cell's count that is one member's rounds down.
      downTick(over(asAmount<'piece'>(view.made(output), 'what the cell has made'), asRatio(weightOf(view.self), 'its people'), 'made per member')),
      view.params.ratio(terms.recipe.learningRate),
    ),
    inputs: terms.recipe.inputs.map((i) => ({
      instrument: goodId(i.subUnit, terms.region),
      qtyPerUnit: view.params.ratio(i.qtyPerUnit),
    })),
    plant: terms.recipe.plant.map((r) => ({
      capitalKind: r.capitalKind,
      unitsPerUnitPerPeriod: view.params.ratio(r.unitsPerUnitPerPeriod),
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
  // Capital Programme A2, Goods B1.a (11.2a): AND ITS PLANT, the same read the named firm's start
  // is limited by — a meal needs a room to be served in, and a cell with no room makes none. A line
  // whose recipe needs no plant is not limited by one, which is a different answer from a large
  // number (Law 6).
  const capacity = capacityFrom(line.plant, vintagesHeld(view, view.calendar.startOf(view.period)), rentedRoom(view));
  if (capacity.some) limits.push(capacity.value.perPeriod);
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
  slot.keeps = heldAsMoney(NO_QTY, 'a cell with no line keeps nothing');
  if (!line.some) return;
  const l = line.value;
  /**
   * Firm B1, Expectations C2: WHAT IT MAKES IS WHAT IT EXPECTS TO SELL — its own outlook of its
   * own fills, formed from what it actually sold. A cell that has never sold anything has no
   * expectation of demand and starts ONE PIECE, to find out what it sells; it does not run its
   * members' hours flat out into a book that takes a fraction of it, which is what made the first
   * cut of this kill five cells in six by period one: the unsold perished at cost.
   */
  const sales = view.outlook(about({ on: 'sold', instrument: l.output }));
  const wanted = sales.some
    ? downTick(asAmount<'piece'>(sales.value.expected, 'the units it expects to sell'))
    : asQty(1, 'one piece, to find out what it sells');
  slot.batch = atMost(canStart(view, l, hoursToPlanWith(view)), wanted, 'it makes what it expects to sell, and no more than it can');
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
  // The next batch is what it expects to sell, as far as its hours reach: the inputs are what it is buying.
  const nextBatch = atMost(downTick(over(hoursToPlanWith(view), l.hoursPerUnit, 'what its people can make')), wanted, 'no more than it expects to sell');
  const ccy = view.registry.currencyOf(view.self.region);
  // 11.0d: A PERIOD OF TRADING AT ITS OWN SCALE, at the prices it expects — the inputs for the
  // batch its hours can make, in full, whether or not it already holds some of them, and the wages
  // it owes. That is what it keeps; a draw of everything above what it happened to bid for emptied
  // every cell to the same nothing, and the lattice merged the sector into one cell a line.
  const capacity = downTick(over(hoursToPlanWith(view), l.hoursPerUnit, 'what its people can make'));
  const wages = ownPayroll(view, view.period);
  slot.keeps = sum([
    ...l.inputs.map((input, n) =>
      valueAt(priced[n] ?? asPerPiece(0, 'priced above, one per input'), upTick(scale(capacity, input.qtyPerUnit, 'what a full batch draws')), 'what a period of this input costs'),
    ),
    ...(wages.some ? [wages.value.due] : []),
  ]).value;
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
  /**
   * Capital Programme B1–B4 (11.2a.2): AND ITS PLANT, by the named firm's own arithmetic — one
   * `project`, in the registry, no second copy. What it can put to it is what it retains above a
   * period of trading after this period's bids: the money the owner would otherwise draw. The
   * decision is the same one a management makes: what it is sure enough of to build for, less its
   * own surprises, against what its plant lets it run at, at a price where the contribution of the
   * capacity it adds clears what its money costs it plus its own hurdle. A cell nobody has quoted
   * and whose money none of this world's states borrows in has no cost of capital and decides
   * nothing, rather than being handed a number (Law 2).
   */
  const retained = minus(cash, slot.keeps, 'what it retains above a period of trading');
  const vintages = vintagesHeld(view, view.calendar.startOf(view.period));
  const terms = ownTerms(view);
  const cost = costOfCapital(view, period(view.period - 1), terms.horizonPeriods);
  if (cost.some && retained > 0) {
    const capacity = capacityFrom(l.plant, vintages, rentedRoom(view));
    const inputCost = sum(
      l.inputs.map((i, n) => scale(priced[n] ?? asPerPiece(0, 'priced above, one per input'), asRatio(i.qtyPerUnit, 'what one takes of it'), 'input cost')),
    ).value;
    const decided = project(
      view,
      l.plant,
      vintages,
      capacity.some ? capacity.value.perPeriod : NO_QTY,
      plantOffers(view, l.plant, l.terms.region, price.value),
      wanted,
      toTick(sales.some ? asAmount<'piece'>(sales.value.confidence, 'how wide its surprises about what it sells are') : NO_QTY),
      minus(price.value, inputCost, 'what a unit brings, less what it takes to make'),
      terms.hurdle,
      terms.horizonPeriods,
      cost.value,
      retained,
    );
    if (decided.some) orders.push(...decided.value.orders);
  }
  slot.orders = orders;
  postForHours(ctx, view, l, price.value, priced, wanted);
  /**
   * A5, A5.a, Corporate Credit A1 (11.0e): IT IS BANK-DEPENDENT, AND THIS IS THE DEPENDENCE. What a
   * period of trading at its own scale needs beyond what it holds, it asks its bank for — through
   * the one door every borrower uses, secured on the plant it holds (11.2a.2). The
   * bank reads the ask next period and decides; a cell nobody lends to trades on what it has, which
   * is where a tightening bites first and hardest (A5.a). Default is the kernel's: a coupon it
   * cannot pay is a missed payment like any other, and a cell that cannot cover what fell due
   * fails on cash and goes to its estate (XI-8).
   */
  const shortOfTrading = ctx.registry.payable(minus(slot.keeps, cash, 'what a period of trading needs beyond what it has'));
  // A4, Small-Business Pools B2 (11.2a.2): SECURED ON WHAT IT HAS — its plant, which the bank can
  // take and realise. A cell with none asks unsecured, which is a statement and not an absence.
  if (shortOfTrading > 0) {
    ctx.request(cell, {
      ccy,
      short: heldAsMoney(shortOfTrading, 'what it asks its bank for'),
      security: vintages.map((v) => ({ instrument: v.instrument as InstrumentId, qty: v.units })),
      // C9: working capital, drawn and repaid at its option — one line at its bank, secured.
      repays: 'atOption',
    });
  }
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
  batch: Qty,
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
  // The hours the batch it expects to sell would take, beyond its members' own: the hours it has
  // a use for. Nothing below zero is wanted, and wanting nobody is a real posting.
  const wanted = downTick(
    minus(
      scale(batch, line.hoursPerUnit, 'the hours the batch takes'),
      asAmount<'piece'>(ownHours(view), 'its members\u2019 own hours'),
      'beyond its members',
    ),
  );
  // Labour C3, C5 (12b.2): it posts the CHANGE against what it will have — a bid for more, or a
  // cut given notice. Wanting nobody, or hours worth nothing to it, is the cut of all it has.
  const wantsNobody = wanted <= 0 || perHour <= 0;
  // Law 8 (12b.5, 12c.3): in whole people, like every employer's posting.
  const change = netChange(view.employs(), occupation, view.self.region, wantsNobody ? NO_QTY : wholePeople(view, wanted));
  if (change !== undefined) {
    ctx.post(venue.id, {
      party: view.self.id,
      side: change.side,
      price: change.side === 'buy' ? perHour : 'market',
      qty: change.qty,
    });
  }
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
  const settled = payrollSettledIn(ctx, cell, ctx.period);
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


/* --------------------------------------------------------------------------------------------
 * THE OWNER (11.0d)
 *
 * @spec Small-Business Pools A1 Small-Business Pools A6.a Households B3 XI-8 Law 5
 *
 * A6.a: "every relationship that must be named is either a dimension of the cell's key or a
 * register row, never an attribute averaged inside it." WHO OWNS a small firm is such a
 * relationship, and it is a ROW: one commitment per small-firm cell, from the cell to the household
 * cell its owners live in — the people of its place who bank where it banks, in the first working
 * cohort. It is opened at the seed, because a firm has an owner before it trades, and it binds a
 * going concern: an estate winding the cell up does not run it, and what is left is the estate's
 * to divide (XI-8).
 *
 * WHAT THE OWNER DRAWS is what the cell does not need to keep trading: its cash above what its
 * own decision has committed — the input bids it is about to settle and the wages it owes. That
 * is a read of its own plan and its own book, and nothing else: no payout ratio, no target return
 * (Law 2). It goes to the owner as a dividend in a two-sided instruction (Law 5), which is how a
 * household comes to have income that is not a wage (Households B3).
 * ------------------------------------------------------------------------------------------ */

/** The commitment that names a small-firm cell's owner. */
export const OWNERSHIP = agreementKindId('smallBusiness.ownership');

/**
 * A6.a, Firm Birth A2 (12.1): WHO OWNS IT AND HOW MANY OF THEM RUN ONE — the owners' members who
 * are in a firm. A firm has a person in it, and that person's hours are the firm's (11.0a); a
 * member who runs one cannot found another, and the row is where that fact lives (Law 4).
 */
export interface OwnershipTerms extends AgreementTerms {
  readonly kind: typeof OWNERSHIP;
  readonly members: number;
}
/** Asked only of rows read `ofKind(OWNERSHIP)`: what it tests is that the count is on the row. */
export function isOwnership(t: AgreementTerms): t is OwnershipTerms {
  return typeof (t as { members?: unknown }).members === 'number';
}

/** A6.a: the row that names this cell's owner, read off the kernel's book of commitments by its kind. */
export function ownerOf(ctx: MechanismContext, cell: PartyId): Option<{ readonly row: AgreementId; readonly owner: PartyId }> {
  for (const a of ctx.agreements.ofKind(OWNERSHIP)) {
    if (a.debtor !== cell || a.state !== 'performing') continue;
    return some({ row: a.id, owner: a.creditor });
  }
  return none<{ row: AgreementId; owner: PartyId }>();
}

/**
 * What the cell has to keep: its own working capital (`keeps`, read off its plan) and what falls
 * due on what it has issued — an invoice it took terms on, a loan — which `owedIn` states as its
 * position (due less held), so what is due is that plus what it holds (Law 19: one read).
 */
function needed(view: ParticipantView): Cash {
  const decided = view.working(DECIDED, nothingDecided);
  const ccy = view.registry.currencyOf(view.self.region);
  const due = heldAsMoney(addQty(view.owedIn(ccy), view.cash(ccy), 'what falls due on what it issued'), 'its dues');
  return sum([decided.at === view.period ? decided.keeps : heldAsMoney(NO_QTY, 'no plan, nothing kept'), due]).value;
}

/** The draw: what stands above what the cell has committed goes to its owner, whole pieces. */
export function draw(ctx: MechanismContext, cell: PartyId): void {
  const view = ctx.participant(cell);
  const owner = ownerOf(ctx, cell);
  if (!owner.some || !ctx.parties.get(owner.value.owner).status.alive) return;
  const ccy = view.registry.currencyOf(view.self.region);
  const spare = ctx.registry.payable(
    minus(heldAsMoney(view.cash(ccy), 'what it holds'), needed(view), 'what it does not need to keep trading'),
  );
  if (spare <= 0) return;
  const record = ctx.settle({
    legs: [
      {
        kind: 'money',
        from: ctx.accountOf(cell, ccy),
        to: ctx.accountOf(owner.value.owner, ccy),
        receipt: { of: 'dividend', on: String(owner.value.row) },
        ccy,
        amount: spare,
      },
    ],
    cause: 'transfer',
    reason: `${String(cell)} draws ${String(spare)} to its owners`,
  });
  ctx.record(
    'smallBusiness.drawn',
    [cell, owner.value.owner],
    { cell, owner: owner.value.owner, drew: spare, settled: record.outcome === 'settled' },
    true,
  );
}
