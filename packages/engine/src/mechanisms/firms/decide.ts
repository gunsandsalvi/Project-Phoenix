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
import {
  amountOf,
  asAmount,
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  over,
  type PerPiece,
  plus,
  type Ratio,
  scale,
  valueAt,
} from '../../core/measure.js';
import { Missing } from '../../core/errors.js';
import type { MarketId, PartyId } from '../../core/ids.js';
import type { InstrumentId } from '../../core/ids.js';
import { atMost, material, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Order, OrderPrice } from '../../clearing/solver.js';
import { findVenue, type VenueDecl } from '../../clearing/venue.js';
import type { ParticipantView } from '../../world/context.js';
import {
  goodId,
  goodMarketId,
  goodTerms,
  spacePerPiece,
  STORAGE,
  storageRateIn,
  type GoodTerms,
} from '../../registry/physical.js';
import {
  capacityFrom,
  rentedRoom,
  capitalChargePerUnit,
  vintagesHeld,
  type PlantNeed,
} from '../../registry/physical.js';
import { firmParam, labourScaleId, type FirmDecl } from './data.js';
import { ownPayroll, payrollSince, wageFacing as facing } from '../../registry/wages.js';
import {
  costOfCapital,
  plantOffers,
  project,
  type CostOfCapital,
  type Project,
} from '../../registry/capital.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { downTick, upTick } from '../../core/tick.js';
import { NO_QTY, subQty, toTick, type Qty } from '../../core/tick.js';
import { about } from '../../world/context.js';
import type { Period } from '../../calendar/calendar.js';

/** An order the firm has decided to post, in the form the market takes it (Clearing A2). */
export interface PlannedOrder {
  readonly market: MarketId;
  readonly side: 'buy' | 'sell';
  readonly price: OrderPrice;
  readonly qty: Qty;
}

/** The name of the store a firm's decide phase leaves its plan in, declared in the module's nouns. */
export const DECIDED = 'firms.decided';

/**
 * Law 8: WHAT THIS FIRM DECIDED, AND IN WHICH PERIOD. The period is part of the fact: a plan from
 * last period is not a plan to post now, which is what `lastOwnSince(..., view.period)` was saying
 * when the journal was standing in for this store. A firm that has decided nothing has `at`
 * MISSING rather than a period it never decided in.
 */
export interface DecidedThisPeriod {
  at: Period | undefined;
  batch: Qty;
  orders: readonly PlannedOrder[];
}

/** An empty slot: a firm that has not decided yet has no period, no batch and no orders. */
export const nothingDecided = (): DecidedThisPeriod => ({
  at: undefined,
  batch: NO_QTY,
  orders: [],
});

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
  /** Commodities Spot B4: what a period of waiting cost a piece, at the rate the room let for. */
  readonly carry: number;
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
  /** Commodities Spot B4: what a period of waiting cost a piece, at the rate the room let for. */
  readonly carry: number;
}

/** The pure one. Named so a `Ratio` is never built out of a bare literal (`core/` owns the digits). */
const ONE: Ratio = asRatio(1, 'one');

/** The numbers a line's own technology states, read from the good's terms (Goods A2). */
interface Technology {
  readonly terms: GoodTerms;
  readonly spoilage: Ratio;
  /**
   * Firm A3: what a tonne takes at THIS firm — the recipe's hours at its own productivity.
   *
   * Every coefficient here is a `Ratio`, because a recipe states a COUNT OVER A COUNT: hours per
   * unit, units of an input per unit of output, plant per unit per period. So what a stock of one
   * reaches is `over(stock, coefficient)` and what a batch draws is `scale(batch, coefficient)` —
   * the quantity keeps its dimension and the coefficient can never be spent or posted as a level.
   */
  readonly hoursPerUnit: Ratio;
  readonly yieldRate: Ratio;
  readonly leadTime: number;
  readonly inputs: readonly { readonly instrument: InstrumentId; readonly qtyPerUnit: Ratio }[];
  /**
   * Goods A2.a, item 7b: what having the plant costs per PERIOD, whatever it makes — cleaning per
   * site, support per machine. It is a cost and never a limit: a site with no cleaner still runs.
   */
  readonly overheads: readonly {
    readonly instrument: InstrumentId;
    readonly capitalKind: string;
    readonly qtyPerPlantUnitPerPeriod: Ratio;
  }[];
  /** Goods A2.c, Capital Programme A2: the plant a unit takes, per kind, at the declared numbers. */
  readonly plant: readonly PlantNeed[];
}

export function technologyOf(view: ParticipantView, line: FirmDecl): Technology {
  const output = goodId(line.subUnit, view.self.region);
  const terms = goodTerms(view.instruments.get(output));
  return {
    terms,
    spoilage: view.params.ratio(terms.spoilage),
    // Goods A2 states what the work takes; Firm A3 states what it takes HERE. This is the one place
    // the two meet, so a firm's own hours-per-unit has one writer and every reader gets the same
    // number — what it bids for an hour, what a unit costs it, and what its people can make.
    hoursPerUnit: scale(
      view.params.ratio(terms.recipe.labourHoursPerUnit),
      view.params.ratio(labourScaleId(line.firm)),
      'hours a unit takes this firm',
    ),
    yieldRate: view.params.ratio(terms.recipe.yieldRate),
    leadTime: view.params.periods(terms.recipe.leadTimePeriods),
    inputs: terms.recipe.inputs.map((i) => ({
      instrument: goodId(i.subUnit, terms.region),
      qtyPerUnit: view.params.ratio(i.qtyPerUnit),
    })),
    // A2.a, item 7b: what having the plant costs per period, whatever it makes. It is not in the
    // limits below, because a site with no cleaner still runs — an overhead is a cost, not a gate.
    overheads: terms.recipe.overheads.map((o) => ({
      instrument: goodId(o.subUnit, terms.region),
      capitalKind: o.capitalKind,
      qtyPerPlantUnitPerPeriod: view.params.ratio(o.qtyPerPlantUnitPerPeriod),
    })),
    plant: [
      ...terms.recipe.plant.map((r) => ({
        capitalKind: r.capitalKind,
        unitsPerUnitPerPeriod: view.params.ratio(r.unitsPerUnitPerPeriod),
      })),
      // Commodities Spot A3, Capital Programme A2, A4 (13c): ROOM IS PLANT AND IT BINDS LIKE PLANT.
      // A thing that takes covered space needs somewhere to be when the period ends, and the space
      // a line has is one more kind whose stock divides into what it can have — taken by the same
      // arithmetic that already takes the scarcest kind, so nothing here branches and nothing is
      // capped. A good that takes no space names none, and its capacity is what it always was.
      //
      // Law 8: the ratio is declared per NAMED unit on both sides and the capacity arithmetic runs
      // in pieces, so the conversion is done here, once, where the declared number is read.
      ...(terms.storagePerUnit === null
        ? []
        : [
            {
              capitalKind: STORAGE,
              unitsPerUnitPerPeriod: spacePerPiece(
                view,
                view.instruments.get(goodId(terms.subUnit, terms.region)).unit,
                view.params.ratio(terms.storagePerUnit),
              ),
            },
          ]),
    ],
  };
}

/**
 * Expectations A2, A2.a: what this firm expects a unit to fetch. Its own outlook when it has one,
 * formed from what it has itself traded at; otherwise what the market last printed, which is public
 * and is all a party with no history of its own has. From its first sale its own outlook leads.
 */

/** The venue this firm's occupation is struck in (Clearing B2: found by what makes it itself). */
export function venueOf(view: ParticipantView, line: FirmDecl): VenueDecl | undefined {
  return findVenue(view.venues, { region: view.self.region, occupation: line.occupation });
}

/**
 * Item 10e.4: BOTH READS ARE THE REGISTRY'S NOW, and this is only the door onto them.
 *
 * `payrollSince` and `wageFacing` were written here and written again in the bank's staffing file,
 * and the two copies did not agree about which key the going rate is published under (`E-19`). A
 * manager costing a pool would have been the third. They live in `registry/wages.ts`, where every
 * employer in this world reaches the same answer (Law 4).
 */
export { payrollSince };

/**
 * Goods B5, Labour E2: the wage this firm's own hour costs, asked of the venue it hires in.
 */
function wageFacing(view: ParticipantView, venue: VenueDecl): Option<PerPiece> {
  return facing(view, view.period, venue.id);
}

/**
 * Goods B1.c, Labour C2: the hours it can plan on. Its own last wage bill says what it had under
 * contract at the close of the period before; with a hiring lag of one period those are exactly the
 * hours that can make something now, which is what a plan taken before the period's own work needs
 * to know. Nobody hired since is in it, and that is right: they are not productive yet.
 */
function hoursUnderContract(view: ParticipantView): Qty {
  const own = ownPayroll(view, view.period);
  return own.some ? own.value.hours : asAmount<'piece'>(0, 'a firm that has employed nobody has no hours');
}

/** D1: what it must pay out that it already knows about — the payroll it is committed to. */
function wagesDue(view: ParticipantView): Cash {
  const own = ownPayroll(view, view.period);
  return own.some ? own.value.due : asCash(0, 'a firm that has published no payroll owes none');
}

/**
 * Goods C1, C5, E4: the supply schedule. What it cannot keep goes at whatever the book gives it —
 * the units that will perish before another session, and the units it must turn into cash to meet
 * a payroll it has already promised. The rest it will part with only above what holding it is
 * worth, which is what it expects a unit to fetch less what perishes in the meantime. Whatever
 * nobody takes stays where it is, which is what illiquidity in goods is.
 */
function sellSchedule(view: ParticipantView, tech: Technology, price: Option<PerPiece>): PlannedOrder[] {
  const output = goodId(tech.terms.subUnit, tech.terms.region);
  const stock = view.quantity(output);
  if (!material(stock, 2, stock)) return [];
  const market = goodMarketId(tech.terms.subUnit, tech.terms.region);
  const ccy = view.registry.currencyOf(view.self.region);
  const short = minus(
    wagesDue(view),
    heldAsMoney(view.cash(ccy), 'what is in its account'),
    'cash it is short of',
  );
  // A firm with no idea what its stock fetches cannot say how much of it covers a payroll, so what
  // it needs is all of it: it has bills and no view (Firm D1).
  const forced =
    short <= 0 ? NO_QTY : price.some ? amountOf(short, price.value, 'units it must sell') : stock;
  // Law 8: a piece is the smallest thing there is, so a PART of one it cannot keep is a whole one
  // it cannot keep — a loaf a quarter stale is a stale loaf, and a payroll covered by all but a
  // cent is a payroll not covered. It rounds up for both reasons, and what is left over is exactly
  // what the register says it holds less that, so no fraction survives on either side.
  const cannotKeep = upTick(
    plus(
      scale(stock, asRatio(tech.spoilage, 'what will perish'), 'what will perish'),
      forced,
      'what it cannot keep',
    ),
  );
  const atMarket = atMost(cannotKeep, stock, 'it cannot sell stock it does not hold');
  const out: PlannedOrder[] = [];
  if (material(atMarket, 2, stock)) {
    out.push({ market, side: 'sell', price: 'market', qty: atMarket });
  }
  const rest = subQty(stock, atMarket, 'what it can hold');
  if (price.some && material(rest, 2, stock)) {
    out.push({
      market,
      side: 'sell',
      // Commodities Spot B4: WHAT HOLDING IT IS WORTH — what it expects to get, less the part that
      // will not survive the wait, less WHAT THE WAIT COSTS. The carry is not a number anybody
      // wrote down: it is the rate the room cleared at this period (13c), read off the session's
      // own print, so a world where room is scarce is a world where holding is dear and more of
      // every stock comes to market. A world with no session that cleared has no rate to read, and
      // then the reservation is what it always was — which is an answer and not a zero.
      price: minus(
        scale(
          price.value,
          minus(ONE, tech.spoilage, 'what survives'),
          'the value of holding',
        ),
        carryPerPiece(view, tech),
        'less what the wait costs it',
      ),
      qty: rest,
    });
  }
  return out;
}

/**
 * Commodities Spot B4, D3: what holding one piece for one period costs, at the rate the room let
 * for. Nothing for a line nobody stores in bulk, and nothing where no room changed hands — in both
 * cases because there is no such cost, not because a number was missing (Appendix A).
 */
function carryPerPiece(view: ParticipantView, tech: Technology): PerPiece {
  const per = tech.terms.storagePerUnit;
  if (per === null) return asPerPiece(0, 'a good that needs no cover costs nothing to keep');
  const rate = storageRateIn(view, tech.terms.region);
  if (rate === undefined) return asPerPiece(0, 'no session cleared, so there is no rate to read');
  const unit = view.instruments.get(goodId(tech.terms.subUnit, tech.terms.region)).unit;
  return scale(
    rate,
    asRatio(
      spacePerPiece(view, unit, view.params.ratio(per)),
      'the room a piece takes',
    ),
    'what a period under cover costs a piece',
  );
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
  const price = expectedPriceOf(view, output);
  const selling = sellSchedule(view, tech, price);
  const venue = venueOf(view, line);
  const wage = venue === undefined ? none<PerPiece>() : wageFacing(view, venue);
  const inputPrices = tech.inputs.map((i) => expectedPriceOf(view, i.instrument));
  const sales = view.outlook(about({ on: 'sold', instrument: output }));
  if (!price.some || inputPrices.some((p) => !p.some)) {
    return selling.length === 0
      ? none<Plan>()
      : some<Plan>({ planned: false, output, orders: selling, carry: carryPerPiece(view, tech) });
  }
  /**
   * A-55, A-56, B1: WHAT A FIRM THAT HAS NEVER SOLD EXPECTS TO SELL, and it is ONE.
   *
   * `expectations` forms a `sold` outlook only from an asset leg the firm was a side of, so a firm
   * that has never sold has none — and without one this returned `planned: false`, started no
   * batch, and therefore never sold. THREE OF THIS WORLD'S GOODS SAT AT ZERO FROM PERIOD ZERO in
   * exactly that loop (`dwelling`, `facilities`, `itServices`), and every other line only escaped it
   * because the seed put stock on a book for it.
   *
   * The way out is not a number: it is that YOU CANNOT LEARN WHAT YOU CAN SELL WITHOUT MAKING
   * SOMETHING. A firm with a price for its output and a price for everything its recipe names makes
   * ONE unit — the smallest thing that exists (Law 8: the piece is the grid, not a declared number)
   * — and finds out. If it sells, its own outlook leads from the next period and this never runs
   * again. If it does not, it is holding one unit and offers it like anything else it made and did
   * not sell, which is what a firm that guessed wrong actually does.
   *
   * Everything downstream is unchanged: `worthMaking` still has to hold, the labour and capacity
   * limits still bind, and a line whose contribution does not cover a wage still starts nothing.
   */
  const firstBatch = asAmount<'piece'>(1, 'one unit, to find out what it sells');
  // Item 16: what the recipe's inputs cost FOR ONE UNIT of output — money per piece, like the
  // level it will sell at, which is what lets the two meet in the contribution below.
  const inputCost = sum(
    tech.inputs.map((i, n) =>
      scale(priceOf(inputPrices, n), asRatio(i.qtyPerUnit, 'what one takes of it'), 'input cost'),
    ),
  ).value;
  // Capital Programme A2, A4: what its plant lets it make, and what that plant costs it to use.
  const vintages = vintagesHeld(view, view.calendar.startOf(view.period));
  const capacity = capacityFrom(tech.plant, vintages, rentedRoom(view));
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
  const capitalCharge = charge.some ? charge.value : asPerPiece(0, 'no plant, nothing wears out');
  // Labour C1, C1.a: what an hour is worth to it — the output an hour makes possible at the price
  // it expects, less what the rest of the recipe takes, which now includes what the plant that hour
  // runs on wears out by. It is the most it will pay for one.
  const contribution = minus(
    minus(
      scale(price.value, tech.yieldRate, 'what a unit of it fetches'),
      inputCost,
      'less its inputs',
    ),
    capitalCharge,
    'less what its plant wears out by',
  );
  const perHour = over(contribution, tech.hoursPerUnit, 'the value of an hour');
  // B5: what a unit costs to start and what a unit that survives the line costs (Goods B4) — known
  // once it has paid a wage. A firm that has never employed anybody knows only what an hour is
  // worth to it, and posting that bid IS how it finds out what one costs (Labour C1, D1).
  const unitCost = wage.some
    ? some(
        over(
          plus(
            plus(
              inputCost,
              scale(wage.value, tech.hoursPerUnit, 'wages per unit'),
              'inputs and wages',
            ),
            capitalCharge,
            'and what its plant wears out by',
          ),
          tech.yieldRate,
          'cost per unit finished',
        ),
      )
    : none<PerPiece>();
  const worthMaking = wage.some ? perHour > wage.value : perHour > 0;
  // B1: what it expects to sell is what it wants to be able to make, period after period; what it
  // starts THIS period is that less the stock it is already sitting on, grossed up for the yield.
  const stock = view.quantity(output);
  // Item 16: an outlook carries the dimension of the VARIABLE it is about, and this one is about
  // units sold — so it enters as an amount here, once, rather than at each of its three readers.
  const expectsToSell = sales.some
    ? asAmount<'piece'>(sales.value.expected, 'the units it expects to sell')
    : firstBatch;
  const perPeriod = worthMaking
    ? over(expectsToSell, tech.yieldRate, 'started per period')
    : asAmount<'piece'>(0, 'a line not worth making starts nothing');
  const wanted = worthMaking
    ? over(minus(expectsToSell, stock, 'what it is short of'), tech.yieldRate, 'batch wanted')
    : asAmount<'piece'>(0, 'a line not worth making wants no batch');
  const fromLabour = over(hoursUnderContract(view), tech.hoursPerUnit, 'what its people can make');
  // Goods B1.a, Capital Programme A2: capacity is one of the reasons, and binding capacity is a
  // real state. A line whose recipe needs no plant is limited by its people and its inputs.
  const limits: readonly { readonly qty: Qty; readonly bound: Planned['bound'] }[] = [
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
  /**
   * A-34, Missing is Missing: A FIRM THAT CANNOT PRICE AN HOUR DOES NOT BID FOR AN INPUT.
   *
   * The input bid below read `wage.some ? wage.value : asPerPiece(0, …)` — labour priced at ZERO in
   * the one place in this function that reaches a market, while the same `wage` is handled
   * correctly twice above (`unitCost` answers `none()`, `worthMaking` switches its test). What an
   * input is worth to a firm is the output it makes possible LESS the wages that unit still needs,
   * so a zero there makes the bid too high by `hoursPerUnit × wage` — for most recipes the largest
   * term in it. And every firm is in that state until it has employed somebody, so at world open
   * every input market cleared against systematically inflated bids and the firms that had never
   * hired outbid the ones that had.
   *
   * A recipe that takes no hours pays no wages, and that zero is a real answer rather than a
   * default. Anything else: `wageFacing` already falls back to the published going rate, so a firm
   * that knows neither its own wage bill nor what an hour last cleared at knows nothing about the
   * cost of making this — and it does not bid until it does.
   */
  const wagesPerUnit = wage.some
    ? some(scale(wage.value, tech.hoursPerUnit, 'its wages'))
    : tech.hoursPerUnit <= 0
      ? some(asPerPiece(0, 'a recipe that takes no hours pays no wages'))
      : none<PerPiece>();
  for (const [n, input] of tech.inputs.entries()) {
    if (!wagesPerUnit.some) break;
    // Law 8: a recipe met with the piece below is a recipe not met (`produce.ts` draws the same
    // way), so what it bids for is the whole pieces the batch needs and never the fraction under.
    const need = upTick(scale(batch, input.qtyPerUnit, 'what the batch draws'));
    const buy = subQty(need, view.quantity(input.instrument), 'what it must buy');
    if (!material(buy, 2, need) || buy <= 0) continue;
    orders.push({
      market: marketOf(view, input.instrument),
      side: 'buy',
      // What this input is worth to it: the output it makes possible at the price it expects, less
      // the wages, the plant and the other inputs that unit still needs.
      price: over(
        minus(
          minus(
            minus(
              scale(
                price.value,
                tech.yieldRate,
                'the output it makes possible',
              ),
              wagesPerUnit.value,
              'less wages',
            ),
            capitalCharge,
            'less what its plant wears out by',
          ),
          sum(
            tech.inputs.map((other, m) =>
              m === n
                ? asPerPiece(0, 'the input being priced is not one of the others')
                : scale(
                    priceOf(inputPrices, m),
                    asRatio(other.qtyPerUnit, 'what one takes of it'),
                    'other inputs',
                  ),
            ),
          ).value,
          'less the other inputs',
        ),
        input.qtyPerUnit,
        'what a unit of the input is worth',
      ),
      qty: buy,
    });
  }

  /**
   * Goods A2.a, item 7b: AND WHAT HAVING THE PLANT COSTS PER PERIOD. Cleaning per site, support per
   * machine — bought because the plant EXISTS and not because the line ran, which is why it is not
   * in the limits above and not scaled by the batch. `facilities` and `itServices` had a firm, a
   * recipe, a market and no buyer in any period of any run; this is the buyer.
   *
   * What it will pay is what the service is worth to it, which for an overhead is what it costs to
   * do without — and nothing in this world prices that yet. So it bids what it EXPECTS to pay: its
   * own outlook of the line where it has one, the last print where it has not, the same ladder it
   * buys an input on (Expectations A2.a). It is a reservation and not a valuation.
   */
  for (const o of tech.overheads) {
    const units = sum(
      vintages.filter((v) => v.capitalKind === o.capitalKind && v.periodsLeft > 0).map((v) => v.units),
    ).value;
    if (units <= 0) continue;
    const need = upTick(
      scale(asAmount<'piece'>(units, 'the plant it has in service'), o.qtyPerPlantUnitPerPeriod, 'what it takes a period'),
    );
    const buy = subQty(need, view.quantity(o.instrument), 'what it must buy');
    if (!material(buy, 2, need) || buy <= 0) continue;
    const level = expectedPriceOf(view, o.instrument);
    if (!level.some) continue;
    orders.push({ market: marketOf(view, o.instrument), side: 'buy', price: level.value, qty: buy });
  }
  // Firm E3, Capital Programme B: the investment decision. It is taken last because it is measured
  // against what the rest of the plan leaves it — the cash it is not about to need — and it adds
  // its own orders to the same list, because a purchase of plant is a purchase like any other.
  const cost = costOfCapital(view, payrollSince(view.period), view.params.periods(firmParam(line.firm, 'horizon')));
  const decided = cost.some
    ? project(
        view,
        tech.plant,
        vintages,
        surviving.some ? surviving.value.perPeriod : NO_QTY,
        plantOffers(view, tech.plant, tech.terms.region, price.value),
        // B1.a: the units it would START each period at what it expects to sell — the same number
        // its employment is decided from, read once and used in both (Law 4).
        toTick(perPeriod),
        // Law 8: a width in units started is a count, and which way it rounds is a decision with a
        // name — the NEAREST piece, because a width is a measurement rather than something a party
        // can or must do (`toTick`, core/tick.ts).
        toTick(
          over(
            // A firm that has never sold has been surprised about nothing, and that is a real zero:
            // it has no history to have been wrong about. Its first batch is one unit either way.
            sales.some
              ? asAmount<'piece'>(sales.value.confidence, 'how wide its surprises about units sold are')
              : NO_QTY,
            tech.yieldRate,
            'how wide its own surprises are, per unit started',
          ),
        ),
        contribution,
        view.params.perAnnum(firmParam(line.firm, 'hurdle')),
        view.params.periods(firmParam(line.firm, 'horizon')),
        cost.value,
        spendable(view, orders),
      )
    : none<Project>();
  if (decided.some) orders.push(...decided.value.orders);
  return some<Plan>({
    planned: true,
    output,
    expectedPrice: price.value,
    carry: carryPerPiece(view, tech),
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
    hours: downTick(scale(perPeriod, tech.hoursPerUnit, 'the employment it wants')),
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
/**
 * A-35, Law 4: WHAT THIS FIRM'S BUY ORDERS COMMIT, in one place.
 *
 * It was computed twice from the same list — here with `mul`, which names the product and checks
 * it, and in `publishFunding` with a bare `*`. One fact, two implementations, already differing in
 * discipline. Both ternaries were dead as well: the filter above them has already removed every
 * `'market'` price, so the `typeof` was a check on something that could not happen.
 */
export function committedTo(orders: readonly PlannedOrder[]): Cash {
  return sum(
    orders
      .filter((o) => o.side === 'buy' && typeof o.price === 'number')
      .map((o) =>
        valueAt(
          asPerPiece(o.price as number, 'the level it posted'),
          o.qty,
          'what it is about to buy',
        ),
      ),
  ).value;
}

function spendable(view: ParticipantView, orders: readonly PlannedOrder[]): Cash {
  const ccy = view.registry.currencyOf(view.self.region);
  const buying = committedTo(orders);
  return minus(
    minus(
      heldAsMoney(view.cash(ccy), 'what is in its account'),
      wagesDue(view),
      'after its payroll',
    ),
    buying,
    'after what it is buying',
  );
}

/**
 * Capital Programme C1, D3: where the plant it needs could come from — built by a capital-goods
 * producer, or bought second-hand from whoever is selling a vintage that already exists. Both are
 * ordinary markets and both are on the same list, because a project does not care which one filled
 * it; what differs is how much service is left in what is on offer (A6).
 */

/** The price of the nth input, which the caller has already established this firm knows. */
function priceOf(prices: readonly Option<PerPiece>[], n: number): PerPiece {
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
 * Law 15, Law 4: WHAT THIS FIRM DECIDED, kept where a decision belongs.
 *
 * These two used to take the firm's own `firms.plan` EVENT and take it apart — `orders` came back
 * as `unknown[]`, every field was re-checked, and an order whose price did not survive the round
 * trip was silently dropped. The event was recorded PRIVATE and read back by its own writer in the
 * same period, which is a store wearing a log's clothes (0e′.4). `ParticipantView.working` is the
 * store, so the orders never leave the type system and the two parsers are gone.
 */
export function marketsIn(decided: DecidedThisPeriod): MarketId[] {
  const out = new Set<MarketId>();
  for (const o of decided.orders) out.add(o.market);
  return [...out];
}

/** The orders this firm decided on, in the book that is asking (Law 4: one decision, one writer). */
export function ordersFrom(
  decided: DecidedThisPeriod,
  market: MarketId,
  self: PartyId,
): Order[] {
  const out: Order[] = [];
  for (const o of decided.orders) {
    if (o.market !== market) continue;
    out.push({ party: self, side: o.side, price: o.price, qty: o.qty });
  }
  return out;
}

