/**
 * What a firm decides: how much to make, how many people to employ, what to offer its output at,
 * and what it will pay for what it is made from.
 *
 * @spec Firm B1 Firm B1.a Firm B2 Firm B3 Firm B4 Firm B4.a Firm B5 Firm E1 Firm E2 Firm E3 Firm E6 Firm E7 Firm F1 Firm F2 Goods B1 Goods B1.a Goods B1.b Goods B1.c Goods B1.d Goods B5 Goods C1 Goods C5 Goods E4 Labour C1 Labour C1.a Labour C5 Labour D1 Capital Programme A2 Capital Programme B1 Capital Programme B3 Capital Programme C1 Capital Programme D4 Expectations A2 Expectations A2.a Expectations C2 XI-4 XI-16 Law 2 Law 6
 *
 * Every one of these is a function of the firm's OWN state, its OWN outlook and the prices it faces
 * (E6), and of nothing else. There is no target margin anywhere: the margin is what is left when
 * the market has spoken (B4.a), and a firm whose output does not cover what it takes to make simply
 * stops — it employs nobody and starts nothing, which is a decision with a reason and not a floor.
 *
 * WHAT IT EXPECTS TO SELL is its own outlook of its own fills (§46 C2): a seller sees what it sold,
 * never the book it did not win. A firm that has never sold anything has no expectation of demand
 * and makes no plan at all — it still offers what it holds, because a thing made to be sold that
 * is not sold perishes.
 *
 * WHAT IT OFFERS AT is the value of holding the stock instead: what it expects a unit to fetch,
 * less what perishes before it could (E4). What it cannot hold — what will perish anyway, and what
 * it must turn into cash to make payroll — it offers at whatever the book gives it. That is the
 * whole of the supply schedule, and both steps are reasons rather than levels anybody wrote down.
 *
 * WHAT IT WILL PAY for an hour, or for a tonne of what it is made from, is what that hour or that
 * tonne is worth to it: the output it makes possible, at the price the firm expects, less what the
 * rest of the recipe costs (Labour C1, C1.a). A bid IS the most a buyer will pay, so this is the
 * bid, and the market clears below it whenever supply is ample.
 */
import { Missing } from '../../core/errors.js';
import { marketId, type MarketId, type PartyId } from '../../core/ids.js';
import type { InstrumentId } from '../../core/ids.js';
import {
  add,
  atMost,
  div,
  material,
  mul,
  sub,
  sum,
} from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Order, OrderPrice } from '../../clearing/solver.js';
import { findVenue, type VenueDecl } from '../../clearing/venue.js';
import type { Event } from '../../journal/journal.js';
import type { ParticipantView } from '../../world/context.js';
import { goodId, goodMarketId, goodTerms, type GoodTerms } from '../../registry/physical.js';
import {
  CAPITAL_KINDS,
  capacityFrom,
  capitalChargePerUnit,
  capitalKindOf,
  isPlant,
  lifeParam,
  plantTerms,
  serviceLeft,
  vintagesHeld,
  type PlantNeed,
} from '../../registry/physical.js';
import { firmParam, labourScaleId, type FirmDecl } from './data.js';
import {
  costOfCapital,
  project,
  type CostOfCapital,
  type PlantOffer,
  type Project,
} from './invest.js';
import { downTick, upTick } from '../../core/tick.js';
import { NO_QTY, asQty, type Qty } from '../../core/tick.js';

/** An order the firm has decided to post, in the form the market takes it (Clearing A2). */
export interface PlannedOrder {
  readonly market: MarketId;
  readonly side: 'buy' | 'sell';
  readonly price: OrderPrice;
  readonly qty: number;
}

/**
 * What a firm decided this period. It is published under the firm's own name and read back by its
 * own orders (Law 4: one decision, one writer, read where it is needed rather than taken twice).
 */
export type Plan = Offering | Planned;

/**
 * B1: a firm that could not form a production plan. It needs to know what a unit fetches, what
 * everything the recipe names costs, and what it can expect to sell — and a firm that has never
 * sold anything has no expectation of demand. Without one it decides nothing about employment: it
 * posts no opening, so what it employs does not move (Labour C5). It still offers what it holds,
 * because a thing made to be sold and not sold perishes.
 */
export interface Offering {
  readonly planned: false;
  readonly output: InstrumentId;
  readonly orders: readonly PlannedOrder[];
}

/** What a firm decided when it had everything it needed to decide with. */
export interface Planned {
  readonly planned: true;
  readonly output: InstrumentId;
  /** Its own expectation of what a unit fetches (§46 C3), or what the market last printed. */
  readonly expectedPrice: number;
  /** B5: inputs plus wages, per unit FINISHED — so the yield is in it (Goods B4). Unknown until it has paid a wage. */
  readonly unitCost: Option<number>;
  /** Goods B1: the units it will start this period. The outcome, not a target. */
  /** Law 8: whole pieces it will START. What it plans is what it can do. */
  readonly batch: Qty;
  /** B1.a, B1.c: what stopped it being larger. A binding constraint is a real state. */
  readonly bound: 'demand' | 'labour' | 'margin' | 'capacity';
  /** Capital Programme A2, Goods B1.a: what its plant lets it start a period. None: it needs none. */
  readonly capacity: number | null;
  /** Capital Programme D1: what it will still let it start next period, once this period's wear is off. */
  readonly capacityNext: number | null;
  /** B1.a: the units it would START each period at what it expects to sell, period after period. */
  readonly runRate: number;
  /** Goods B5: what the plant a unit takes wears out by — the capital charge in unit cost. */
  readonly capitalCharge: number;
  /** Firm E3, Capital Programme B1: the project it decided on, if it has one. */
  readonly project: Project | null;
  /** B1.b: what money costs it at the margin now, which is what a project is measured against. */
  readonly costOfCapital: CostOfCapital | null;
  /** E2: the employment it wants, in hours — the labour that makes what it expects to sell. */
  /** Law 8: whole hours. A wage is struck for an hour and never for part of one. */
  readonly hours: Qty;
  /** Labour C1: the most it will pay for an hour, which is what an hour is worth to it. */
  readonly wageBid: number;
  readonly orders: readonly PlannedOrder[];
}

/** The numbers a line's own technology states, read from the good's terms (Goods A2). */
interface Technology {
  readonly terms: GoodTerms;
  readonly spoilage: number;
  /** Firm A3: what a tonne takes at THIS firm — the recipe's hours at its own productivity. */
  readonly hoursPerUnit: number;
  readonly yieldRate: number;
  readonly leadTime: number;
  readonly inputs: readonly { readonly instrument: InstrumentId; readonly qtyPerUnit: number }[];
  /** Goods A2.c, Capital Programme A2: the plant a unit takes, per kind, at the declared numbers. */
  readonly plant: readonly PlantNeed[];
}

export function technologyOf(view: ParticipantView, line: FirmDecl): Technology {
  const output = goodId(line.subUnit, view.self.region);
  const terms = goodTerms(view.instruments.get(output));
  return {
    terms,
    spoilage: view.params.get(terms.spoilage),
    // Goods A2 states what the work takes; Firm A3 states what it takes HERE. This is the one place
    // the two meet, so a firm's own hours-per-unit has one writer and every reader gets the same
    // number — what it bids for an hour, what a unit costs it, and what its people can make.
    hoursPerUnit: mul(
      view.params.get(terms.recipe.labourHoursPerUnit),
      view.params.get(labourScaleId(line.firm)),
      'hours a unit takes this firm',
    ),
    yieldRate: view.params.get(terms.recipe.yieldRate),
    leadTime: view.params.get(terms.recipe.leadTimePeriods),
    inputs: terms.recipe.inputs.map((i) => ({
      instrument: goodId(i.subUnit, terms.region),
      qtyPerUnit: view.params.get(i.qtyPerUnit),
    })),
    plant: terms.recipe.plant.map((r) => ({
      capitalKind: r.capitalKind,
      unitsPerUnitPerPeriod: view.params.get(r.unitsPerUnitPerPeriod),
    })),
  };
}

/**
 * Expectations A2, A2.a: what this firm expects a unit to fetch. Its own outlook when it has one,
 * formed from what it has itself traded at; otherwise what the market last printed, which is public
 * and is all a party with no history of its own has. From its first sale its own outlook leads.
 */
export function expectedPrice(view: ParticipantView, instrument: InstrumentId): Option<number> {
  const own = view.outlook(`price.${instrument}`);
  if (own.some) return some(own.value.expected);
  const print = view.print(instrument);
  return print.some ? some(print.value.price) : none<number>();
}

/** The venue this firm's occupation is struck in (Clearing B2: found by what makes it itself). */
export function venueOf(view: ParticipantView, line: FirmDecl): VenueDecl | undefined {
  return findVenue(view.venues, { region: view.self.region, occupation: line.occupation });
}

/**
 * Goods B5, Labour E2: the wage this firm's own hour costs. Its own wage bill over its own hours is
 * what it actually pays; a firm employing nobody has none of its own and faces what the market
 * published (Labour D1.c, Expectations A2.a) — and one that has neither cannot cost a unit at all.
 */
function wageFacing(view: ParticipantView, venue: VenueDecl): Option<number> {
  const own = view.lastOwn('labour.wages');
  if (own.some) {
    const due = own.value.data['due'];
    const hours = own.value.data['hours'];
    if (typeof due === 'number' && typeof hours === 'number' && hours > 0) {
      return some(div(due, hours, 'own wage per hour'));
    }
  }
  const published = view.lastPublic('labour.goingRate');
  if (!published.some) return none<number>();
  const rates = published.value.data['wagePerHour'];
  if (typeof rates !== 'object' || rates === null) return none<number>();
  const rate = (rates as Record<string, unknown>)[venue.id];
  return typeof rate === 'number' ? some(rate) : none<number>();
}

/**
 * Goods B1.c, Labour C2: the hours it can plan on. Its own last wage bill says what it had under
 * contract at the close of the period before; with a hiring lag of one period those are exactly the
 * hours that can make something now, which is what a plan taken before the period's own work needs
 * to know. Nobody hired since is in it, and that is right: they are not productive yet.
 */
function hoursUnderContract(view: ParticipantView): number {
  const own = view.lastOwn('labour.wages');
  if (!own.some) return 0;
  const hours = own.value.data['hours'];
  return typeof hours === 'number' ? hours : 0;
}

/** D1: what it must pay out that it already knows about — the payroll it is committed to. */
function wagesDue(view: ParticipantView): number {
  const own = view.lastOwn('labour.wages');
  if (!own.some) return 0;
  const due = own.value.data['due'];
  return typeof due === 'number' ? due : 0;
}

/**
 * Goods C1, C5, E4: the supply schedule. What it cannot keep goes at whatever the book gives it —
 * the units that will perish before another session, and the units it must turn into cash to meet
 * a payroll it has already promised. The rest it will part with only above what holding it is
 * worth, which is what it expects a unit to fetch less what perishes in the meantime. Whatever
 * nobody takes stays where it is, which is what illiquidity in goods is.
 */
function sellSchedule(view: ParticipantView, tech: Technology, price: Option<number>): PlannedOrder[] {
  const output = goodId(tech.terms.subUnit, tech.terms.region);
  const stock = view.quantity(output);
  if (!material(stock, 2, stock)) return [];
  const market = goodMarketId(tech.terms.subUnit, tech.terms.region);
  const ccy = view.registry.region(view.self.region).ccy;
  const short = sub(wagesDue(view), view.cash(ccy), 'cash it is short of');
  // A firm with no idea what its stock fetches cannot say how much of it covers a payroll, so what
  // it needs is all of it: it has bills and no view (Firm D1).
  const forced = short <= 0 ? 0 : price.some ? div(short, price.value, 'units it must sell') : stock;
  // Law 8: a piece is the smallest thing there is, so a PART of one it cannot keep is a whole one
  // it cannot keep — a loaf a quarter stale is a stale loaf, and a payroll covered by all but a
  // cent is a payroll not covered. It rounds up for both reasons, and what is left over is exactly
  // what the register says it holds less that, so no fraction survives on either side.
  const cannotKeep = upTick(
    add(mul(stock, tech.spoilage, 'what will perish'), forced, 'what it cannot keep'),
  );
  const atMarket = atMost(cannotKeep, stock, 'it cannot sell stock it does not hold');
  const out: PlannedOrder[] = [];
  if (material(atMarket, 2, stock)) {
    out.push({ market, side: 'sell', price: 'market', qty: atMarket });
  }
  const rest = sub(stock, atMarket, 'what it can hold');
  if (price.some && material(rest, 2, stock)) {
    out.push({
      market,
      side: 'sell',
      // What holding it is worth: what it expects to get, less the part that will not survive.
      price: mul(price.value, sub(1, tech.spoilage, 'what survives'), 'the value of holding'),
      qty: rest,
    });
  }
  return out;
}

/**
 * Firm B1, E1, E2, Goods B1: the decision. It needs a price it expects for what it makes, a price
 * for everything the recipe names, and an expectation of what it will sell; a firm missing any of
 * those plans nothing and only offers what it is holding. It does NOT need to know what a wage
 * costs: what it will pay for an hour is what an hour is worth to it, and posting that bid is how
 * a firm that has never employed anybody finds out what one costs (Labour C1, D1).
 */
export function plan(view: ParticipantView, line: FirmDecl): Option<Plan> {
  const tech = technologyOf(view, line);
  const output = goodId(line.subUnit, view.self.region);
  const price = expectedPrice(view, output);
  const selling = sellSchedule(view, tech, price);
  const venue = venueOf(view, line);
  const wage = venue === undefined ? none<number>() : wageFacing(view, venue);
  const inputPrices = tech.inputs.map((i) => expectedPrice(view, i.instrument));
  const sales = view.outlook(`sold.${output}`);
  if (!price.some || !sales.some || inputPrices.some((p) => !p.some)) {
    return selling.length === 0
      ? none<Plan>()
      : some<Plan>({ planned: false, output, orders: selling });
  }
  const inputCost = sum(
    tech.inputs.map((i, n) => mul(i.qtyPerUnit, priceOf(inputPrices, n), 'input cost')),
  ).value;
  // Capital Programme A2, A4: what its plant lets it make, and what that plant costs it to use.
  const vintages = vintagesHeld(view, view.calendar.startOf(view.period));
  const capacity = capacityFrom(tech.plant, vintages);
  // Capital Programme D1, A6: what its plant will still let it run at next period — this period's
  // stock less the vintages whose life ends before then. It is computed once and both the decision
  // and the record it publishes read the same number (Law 4).
  const surviving = capacityFrom(
    tech.plant,
    vintages.filter((v) => v.periodsLeft > 1),
  );
  // Goods B5, Capital Programme A3: unit cost is inputs plus wages plus a CAPITAL CHARGE, and the
  // charge is the same wear the stock is written down by. A firm with none of a kind it needs wears
  // nothing out, because it has nothing to wear out — and it makes nothing either.
  const charge = capitalChargePerUnit(tech.plant, vintages);
  const capitalCharge = charge.some ? charge.value : 0;
  // Labour C1, C1.a: what an hour is worth to it — the output an hour makes possible at the price
  // it expects, less what the rest of the recipe takes, which now includes what the plant that hour
  // runs on wears out by. It is the most it will pay for one.
  const contribution = sub(
    sub(mul(price.value, tech.yieldRate, 'what a unit of it fetches'), inputCost, 'less its inputs'),
    capitalCharge,
    'less what its plant wears out by',
  );
  const perHour = div(contribution, tech.hoursPerUnit, 'the value of an hour');
  // B5: what a unit costs to start and what a unit that survives the line costs (Goods B4) — known
  // once it has paid a wage. A firm that has never employed anybody knows only what an hour is
  // worth to it, and posting that bid IS how it finds out what one costs (Labour C1, D1).
  const unitCost = wage.some
    ? some(
        div(
          add(
            add(inputCost, mul(tech.hoursPerUnit, wage.value, 'wages per unit'), 'inputs and wages'),
            capitalCharge,
            'and what its plant wears out by',
          ),
          tech.yieldRate,
          'cost per unit finished',
        ),
      )
    : none<number>();
  const worthMaking = wage.some ? perHour > wage.value : perHour > 0;
  // B1: what it expects to sell is what it wants to be able to make, period after period; what it
  // starts THIS period is that less the stock it is already sitting on, grossed up for the yield.
  const stock = view.quantity(output);
  const perPeriod = worthMaking ? div(sales.value.expected, tech.yieldRate, 'started per period') : 0;
  const wanted = worthMaking
    ? div(sub(sales.value.expected, stock, 'what it is short of'), tech.yieldRate, 'batch wanted')
    : 0;
  const fromLabour = div(hoursUnderContract(view), tech.hoursPerUnit, 'what its people can make');
  // Goods B1.a, Capital Programme A2: capacity is one of the reasons, and binding capacity is a
  // real state. A line whose recipe needs no plant is limited by its people and its inputs.
  const limits: readonly { readonly qty: number; readonly bound: Planned['bound'] }[] = [
    { qty: wanted, bound: 'demand' },
    { qty: fromLabour, bound: 'labour' },
    ...(capacity.some ? [{ qty: capacity.value.perPeriod, bound: 'capacity' as const }] : []),
  ];
  const binding = limits.reduce((a, b) => (b.qty < a.qty ? b : a));
  // Law 8: WHAT IT WILL START is whole pieces, decided here and nowhere else. Every limit above
  // is a division — expected sales over a yield, hours over hours-per-unit, plant over what a unit
  // takes — so the binding one is a fraction of a piece, and a part-started unit is not started.
  const batch = !worthMaking || wanted <= 0 ? NO_QTY : downTick(binding.qty);
  const bound = !worthMaking ? 'margin' : binding.bound;
  const orders = [...selling];
  for (const [n, input] of tech.inputs.entries()) {
    // Law 8: a recipe met with the piece below is a recipe not met (`produce.ts` draws the same
    // way), so what it bids for is the whole pieces the batch needs and never the fraction under.
    const need = upTick(mul(batch, input.qtyPerUnit, 'what the batch draws'));
    const buy = sub(need, view.quantity(input.instrument), 'what it must buy');
    if (!material(buy, 2, need) || buy <= 0) continue;
    orders.push({
      market: marketOf(view, input.instrument),
      side: 'buy',
      // What this input is worth to it: the output it makes possible at the price it expects, less
      // the wages, the plant and the other inputs that unit still needs.
      price: div(
        sub(
          sub(
            sub(mul(price.value, tech.yieldRate, 'the output it makes possible'), mul(tech.hoursPerUnit, wage.some ? wage.value : 0, 'its wages'), 'less wages'),
            capitalCharge,
            'less what its plant wears out by',
          ),
          sum(tech.inputs.map((other, m) => (m === n ? 0 : mul(other.qtyPerUnit, priceOf(inputPrices, m), 'other inputs')))).value,
          'less the other inputs',
        ),
        input.qtyPerUnit,
        'what a unit of the input is worth',
      ),
      qty: buy,
    });
  }
  // Firm E3, Capital Programme B: the investment decision. It is taken last because it is measured
  // against what the rest of the plan leaves it — the cash it is not about to need — and it adds
  // its own orders to the same list, because a purchase of plant is a purchase like any other.
  const cost = costOfCapital(view);
  const decided = cost.some
    ? project(
        view,
        tech.plant,
        vintages,
        surviving.some ? surviving.value.perPeriod : 0,
        plantOffers(view, tech, price.value),
        // B1.a: the units it would START each period at what it expects to sell — the same number
        // its employment is decided from, read once and used in both (Law 4).
        perPeriod,
        div(sales.value.confidence, tech.yieldRate, 'how wide its own surprises are, per unit started'),
        contribution,
        view.params.get(firmParam(line.firm, 'hurdle')),
        view.params.get(firmParam(line.firm, 'horizon')),
        cost.value,
        spendable(view, orders),
      )
    : none<Project>();
  if (decided.some) orders.push(...decided.value.orders);
  return some<Plan>({
    planned: true,
    output,
    expectedPrice: price.value,
    unitCost,
    batch,
    bound,
    capacity: capacity.some ? capacity.value.perPeriod : null,
    capacityNext: surviving.some ? surviving.value.perPeriod : null,
    runRate: perPeriod,
    capitalCharge,
    project: decided.some ? decided.value : null,
    costOfCapital: cost.some ? cost.value : null,
    // E2: the labour that makes what it expects to sell, period after period. The stock it happens
    // to hold moves the batch, not the workforce: people are a relationship and letting them go
    // costs severance (Labour C3), so a firm does not shed a soft week's worth of them.
    // Law 8: an hour has a smallest piece like everything else, and what it posts is a whole
    // number of them. Down, because it is what this firm will PAY for: a posting rounded up is a
    // wage bill it did not decide on.
    hours: downTick(mul(perPeriod, tech.hoursPerUnit, 'the employment it wants')),
    wageBid: perHour,
    orders,
  });
}

/**
 * Capital Programme B2: what it can put into plant right now — the cash it holds less the payroll
 * it has promised and the inputs it has just decided to buy. What it wants beyond that is its
 * programme, and a programme is what a lender lends into and a share issue is raised into (Firm
 * E4.a); it is not spendable until the money is actually there.
 */
function spendable(view: ParticipantView, orders: readonly PlannedOrder[]): number {
  const ccy = view.registry.region(view.self.region).ccy;
  const buying = sum(
    orders
      .filter((o) => o.side === 'buy' && o.price !== 'market')
      .map((o) => (typeof o.price === 'number' ? mul(o.price, o.qty, 'what it is about to buy') : 0)),
  ).value;
  return sub(sub(view.cash(ccy), wagesDue(view), 'after its payroll'), buying, 'after what it is buying');
}

/**
 * Capital Programme C1, D3: where the plant it needs could come from — built by a capital-goods
 * producer, or bought second-hand from whoever is selling a vintage that already exists. Both are
 * ordinary markets and both are on the same list, because a project does not care which one filled
 * it; what differs is how much service is left in what is on offer (A6).
 */
function plantOffers(
  view: ParticipantView,
  tech: Technology,
  ownPrice: number,
): PlantOffer[] {
  const out: PlantOffer[] = [];
  for (const need of tech.plant) {
    const d = capitalKindOf(CAPITAL_KINDS, need.capitalKind);
    if (d === undefined) continue;
    const life = view.params.get(lifeParam(d.id));
    const built = goodId(d.madeFrom, tech.terms.region);
    if (!view.instruments.has(built)) continue;
    const asking = expectedPrice(view, built);
    if (!asking.some || asking.value <= 0) continue;
    out.push({
      capitalKind: need.capitalKind,
      unitsPerUnitPerPeriod: need.unitsPerUnitPerPeriod,
      market: goodMarketId(d.madeFrom, tech.terms.region),
      price: asking.value,
      periodsOfService: life,
      newBuild: true,
    });
    for (const i of view.instruments.all()) {
      if (!i.status.live || !isPlant(i) || !i.market.some) continue;
      const terms = plantTerms(i);
      if (terms.capitalKind !== need.capitalKind || terms.region !== view.self.region) continue;
      const left = serviceLeft(terms, view.calendar.startOf(view.period), view.calendar);
      if (left <= 0 || life <= 0) continue;
      // What it expects a second-hand machine to ask: what one that traded went for, and otherwise
      // what a new one costs for the service it has left. It is what this firm expects to have to
      // pay, and it is never what it bids — the bid is its own reservation (Clearing A2).
      const printed = expectedPrice(view, i.id);
      out.push({
        capitalKind: need.capitalKind,
        unitsPerUnitPerPeriod: need.unitsPerUnitPerPeriod,
        market: i.market.value,
        price: printed.some
          ? printed.value
          : mul(asking.value, div(left, life, 'the service it has left'), 'what a used one asks'),
        periodsOfService: left,
        newBuild: false,
      });
    }
  }
  // `ownPrice` is what its own output fetches; a project's return is built from it upstream, and it
  // is named here so the offer list and the return are read from one plan (Law 4).
  return ownPrice > 0 ? out : [];
}

/** The price of the nth input, which the caller has already established this firm knows. */
function priceOf(prices: readonly Option<number>[], n: number): number {
  const p = prices[n];
  if (p?.some !== true) {
    throw new Missing('Goods B5', `the price of input ${n} was there and is not`, { input: n });
  }
  return p.value;
}

function marketOf(view: ParticipantView, instrument: InstrumentId): MarketId {
  const terms = goodTerms(view.instruments.get(instrument));
  return goodMarketId(terms.subUnit, terms.region);
}

/**
 * Law 18, Law 19: the books this firm could be in at all, read off the SAME published plan the
 * orders are read off. Two lists that could disagree would be two writers of one fact (Law 4); this
 * one cannot, because it is the market ids of those orders and nothing else.
 */
export function marketsIn(event: Event): MarketId[] {
  const rows = event.data['orders'];
  if (!Array.isArray(rows)) return [];
  const out = new Set<string>();
  for (const row of rows as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const id = (row as Record<string, unknown>)['market'];
    if (typeof id === 'string') out.add(id);
  }
  return [...out].map((id) => marketId(id));
}

/** The orders this firm decided on, read back from its own published plan (Law 4). */
export function ordersFrom(event: Event, market: MarketId, self: PartyId): Order[] {
  const rows = event.data['orders'];
  if (!Array.isArray(rows)) return [];
  const out: Order[] = [];
  for (const row of rows as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const o = row as Record<string, unknown>;
    const side = o['side'];
    const price = o['price'];
    const qty = o['qty'];
    if (o['market'] !== market) continue;
    if (side !== 'buy' && side !== 'sell') continue;
    if (typeof qty !== 'number') continue;
    if (typeof price !== 'number' && price !== 'market') continue;
    // Law 8, Law 19: read back from the firm's OWN published plan, so what reaches the book is
    // what it decided and never a second derivation of it (Clearing F1). `asQty` throws if the
    // plan it is reading carried a size that is not a whole number of pieces — which is the check
    // that a decision published off the grid cannot quietly become an order.
    out.push({ party: self, side, price, qty: asQty(qty, `${self}'s posted size`) });
  }
  return out;
}
