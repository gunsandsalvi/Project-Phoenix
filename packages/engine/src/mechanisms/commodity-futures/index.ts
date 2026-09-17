/**
 * The commodity future: a lot of a named grade, in a named place, on a named date.
 *
 * @spec Commodity Futures A1 Commodity Futures A1.a Commodity Futures A1.b Commodity Futures A2 Commodity Futures A3 Commodity Futures A4 Commodity Futures B1 Commodity Futures B2 Commodity Futures B3 Commodity Futures C1 Commodity Futures C2 Commodity Futures C3 Commodity Futures C4 Commodity Futures D1 Commodity Futures D2 Commodity Futures D3 Commodity Futures E1 Commodity Futures E2 Commodities Spot A1.a Commodities Spot A3 Derivative Layer B1 Derivative Layer D1 XI-5 Law 2 Law 3 Law 19
 *
 * A1: WHAT IS DELIVERED IS THE THING. The short hands over units of the grade at the location the
 * contract names and the long pays for them at that location's own cleared spot price, in one
 * instruction (XI-5) — not a cash difference dressed up as a delivery. A short with nothing to
 * deliver FAILS, and a fail is a recorded state rather than a licence to settle in cash instead.
 *
 * A1.a: A LOT IS A SILO'S WORTH, and it is READ rather than declared (Law 19, Law 2). How much of a
 * grade fills one unit of covered space is already technology — `storagePerUnit`, declared by the
 * good — so the size of a lot is one over it and there is no contract-size parameter anywhere. A
 * world that changes how much room a tonne takes changes the lot, which is what a lot is.
 *
 * C1, C2: THE CARRY IS THREE READS AND NOTHING ELSE. What the room costs, at the rate the STORAGE
 * SESSION printed in that place; what the wait spoils, at the rate the good declares; and what the
 * money costs, at the secured benchmark this world fixes. Nothing is set here, nothing is targeted,
 * and there is no convenience yield: the gap between the future and the spot plus that carry is a
 * MEASUREMENT (`commodityCarryOf`, `commodityBasis`), and where it goes is a finding about a mechanism.
 *
 * C3: CONTANGO IS BOUNDED ONLY THROUGH SOMEBODY DOING THE TRADE. A party that can find room will
 * sell a future that stands above spot plus carry and hold the thing against it; if nobody has
 * room, nothing closes it, and the curve says so. BACKWARDATION IS UNBOUNDED, because there is no
 * trade on the other side: you cannot borrow a tonne that does not exist.
 *
 * C4: CONVERGENCE IS NOT ENFORCED. It happens because delivery is possible — at expiry the future
 * settles into the spot print itself — and if it did not converge that would be a defect in this
 * module rather than a number to correct.
 */
import {
  type Cash,
  type PerPiece,
  type Ratio,
  absolute,
  asAmount,
  asPerPiece,
  asRatio,
  minus,
  negated,
  over,
  plus,
  pricedAt,
  scale,
  valueAt,
} from '../../core/measure.js';
import { moneyPrint } from '../../prices/price-store.js';
import { nextCycle, type Calendar, type Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, MarketId, ParamId, PartyId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { div, mul } from '../../core/num.js';
import { noCash } from '../../core/measure.js';
import { none, some, type Option } from '../../core/option.js';
import { addQty, asQty, negQty, NO_QTY, scaleQty } from '../../core/tick.js';
import { CENT_TICK } from '../../registry/grid.js';
import { isGoodTerms, storageRateIn, type GoodTerms } from '../../registry/physical.js';
import type {
  Contract,
  ContractMeasure,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { exposedTo, moneyLevel } from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import { contractOf, kindOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Event, EventKind } from '../../journal/journal.js';
import type { Instrument } from '../../register/instruments.js';
import type { Print } from '../../prices/price-store.js';
import type { MechanismContext, ParticipantView, WorldReads } from '../../world/context.js';
import type { DerivativeClassDecl, SystemModule } from '../../world/module.js';
import { about } from '../../world/context.js';
import { lastFixing } from '../../registry/notices.js';

/** Law 8: how a wait in periods becomes a fraction of the year the benchmark is quoted for. */
const CARRY_DAY_COUNT = 'ACT/ACT';

export const COMMODITY_FUTURE = derivativeKindId('commodity.future');
/** A1.a: the notional is a count of LOTS, and what a lot is comes off the good's own storage. */
export const COMMODITY_CONTRACTS = unitId('commodityContracts');

export const COMMODITY_FUTURE_PARAMS = {
  /** A2: how many delivery dates stand open at once. A convention of the exchange, and a series. */
  series: paramId('commodity.future.series'),
  /** A2: periods between one delivery date and the next. */
  spacing: paramId('commodity.future.spacing.periods'),
  window: paramId('commodity.future.margin.window'),
} as const;

export const commodityFutureMarketOf = (deliverable: InstrumentId, expiry: number): MarketId =>
  marketId(`mkt.commodity.future.${deliverable}.${expiry}`);

export const commodityFutureLineOf = (deliverable: InstrumentId, expiry: number): InstrumentId =>
  instrumentId(`commodity.future:${deliverable}:${expiry}`);

export interface CommodityFutureTerms extends ContractTerms {
  readonly kind: typeof COMMODITY_FUTURE;
  /** A1: a named grade in a named place. Location is part of the identity (Commodities Spot A1.a). */
  readonly deliverable: InstrumentId;
  readonly market: MarketId;
  readonly expiry: Period;
  /** A1.a: units of the grade one lot delivers, read off what a unit of it takes in store. */
  readonly lotUnits: number;
  readonly book: InstrumentId;
  readonly long: boolean;
  readonly window: number;
}

export const isCommodityFuture = (t: ContractTerms): t is CommodityFutureTerms =>
  'deliverable' in t && 'lotUnits' in t && 'long' in t;

/** D8: what it is worth to `a` — this book's own print against the level the row was struck at. */
function markOf(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isCommodityFuture(c.terms)) return noCash(c.ccy);
  const p = reads.print(c.terms.book, at);
  if (!p.some) return noCash(c.ccy);
  const move = minus(
    // 18.0: and the PRINT says it is money too, which is the other half of `moneyLevel`'s check.
    moneyPrint(p.value, 'the price this book last printed'),
    moneyLevel(c.struckAt, 'a commodity future is struck at a price'),
    'the future now against the level struck',
  );
  const worth = valueAt(
    move,
    scale(c.notional, asRatio(c.terms.lotUnits, 'the units in a lot'), 'per lot'),
    c.ccy,
    'of the grade each',
  );
  return c.terms.long ? worth : negated(worth, 'and the other side of it');
}

export const commodityFutureKind: DerivativeKindProfile = {
  id: COMMODITY_FUTURE,
  unit: COMMODITY_CONTRACTS,
  // A1: quoted per unit of the grade, in money, to the smallest piece of money there is.
  priceTick: CENT_TICK,
  quotedAs: 'money',
  underlying: (c) => ({
    kind: 'print',
    market: isCommodityFuture(c.terms) ? c.terms.market : ('' as MarketId),
    instrument: isCommodityFuture(c.terms) ? c.terms.deliverable : ('' as InstrumentId),
  }),
  validateTerms: (t) => {
    if (!isCommodityFuture(t)) throw new Error('not commodity future terms');
    if (!(t.lotUnits > 0)) throw new Error('a lot that delivers none of the grade at all');
  },
  displayName: (c) =>
    isCommodityFuture(c.terms) ? `${c.terms.deliverable} ${c.terms.expiry}` : String(c.id),
  mark: markOf,
  flip: (t) => (isCommodityFuture(t) ? { ...t, long: !t.long } : t),
  // A1, XI-5: what settles at delivery is the GRADE against cash — an asset leg and a money leg in
  // one instruction, which a periodic payment cannot carry. This module's own phase writes it.
  legs: (): readonly ContractPayment[] => [],
  premiumPerUnit: (): PerPiece => asPerPiece(0, 'a future costs nothing to enter'),
  initialMargin: (c, at, reads): Option<Cash> => {
    if (!isCommodityFuture(c.terms)) return none<Cash>();
    const move = reads.measuredMove(c.terms.deliverable, c.terms.window);
    if (!move.some) return none<Cash>();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.periods(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['periods']>[0],
    );
    return some(
      scale(
        valueAt(
          move.value,
          scale(c.notional, asRatio(c.terms.lotUnits, 'the units in a lot'), 'per lot'),
          c.ccy,
          'of the grade each',
        ),
        asRatio(Math.sqrt(left > 0 ? left / horizon : 1), 'over the life it has left'),
        'over the life it has left',
      ),
    );
  },
  /**
   * A1, Money Market A2: WHAT TAKING DELIVERY COSTS, said in advance so a treasury can fund it. The
   * long pays for the whole lot at the grade's own spot price on the delivery date.
   */
  cashDue: (c, at, reads, party): Cash => {
    const t = c.terms;
    if (!isCommodityFuture(t) || at < t.expiry) return noCash(c.ccy);
    const long = t.long ? c.a : c.b;
    if (long !== party) return noCash(c.ccy);
    // 18.1: at the price THIS ROW was struck at, which is what it will actually pay.
    return valueAt(
      moneyLevel(c.struckAt, 'a commodity future is struck at a price'),
      scale(c.notional, asRatio(t.lotUnits, 'the units in a lot'), 'the units it takes'),
      c.ccy,
      'at the price it struck',
    );
  },
  closeOut: markOf,
  // D11: the term runs out when this module has DELIVERED it. Until then it is open, and the
  // layer's own resolution must not tear up a contract that delivers and pay a difference for it.
  expires: () => false,
};

/**
 * C1, C2: THE CARRY to a delivery date, per unit of the grade. Three reads and a subtraction.
 *
 * The ROOM is what the storage session printed in that place, for as much room as a unit takes and
 * for as many periods as there are to wait. The SPOILAGE is what the good's own terms say leaves the
 * store each period, valued at what the thing is worth. The MONEY is the secured benchmark this
 * world fixes, on the spot price, for the same wait. Nothing here is a parameter of this module, and
 * none of it is a convenience yield: what is left over between the future and this is the
 * MEASUREMENT (Law 3, Law 19).
 */
export interface CarryReads {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly instruments: { has(id: InstrumentId): boolean; get(id: InstrumentId): Instrument };
  readonly params: { ratio(id: ParamId): Ratio };
  price(id: InstrumentId): Option<Print>;
  lastPublic(kind: EventKind): Option<Event>;
}

/**
 * Law 4: ONE carry, asked by two callers that see the world differently. A measure is computed from
 * the world's own reads and an order from one party's view, and neither of them is the other — so
 * what this function needs is stated once and each caller says how it answers it.
 */
export const carryFromWorld = (ctx: WorldReads): CarryReads => ({
  period: ctx.period,
  calendar: ctx.calendar,
  instruments: ctx.instruments,
  params: ctx.params,
  price: (id) => ctx.prices.latest(id, ctx.period),
  lastPublic: (kind) => {
    const e = ctx.journal.ofKind(kind).at(-1);
    return e === undefined ? none<Event>() : some(e);
  },
});

export const carryFromView = (view: ParticipantView): CarryReads => ({
  period: view.period,
  calendar: view.calendar,
  instruments: view.instruments,
  params: view.params,
  price: (id) => view.print(id),
  lastPublic: (kind) => view.lastPublic(kind),
});

export function commodityCarryOf(
  ctx: CarryReads,
  deliverable: InstrumentId,
  to: Period,
): Option<PerPiece> {
  if (!ctx.instruments.has(deliverable)) return none<PerPiece>();
  const i = ctx.instruments.get(deliverable);
  if (!isGoodTerms(i.terms)) return none<PerPiece>();
  const terms: GoodTerms = i.terms;
  const spot = ctx.price(deliverable);
  if (!spot.some) return none<PerPiece>();
  const periods = to > ctx.period ? to - ctx.period : 0;
  if (terms.storagePerUnit === null) return none<PerPiece>();
  const rate = storageRateIn(ctx, terms.region);
  if (rate === undefined) return none<PerPiece>();
  const room = scale(
    scale(
      rate,
      ctx.params.ratio(terms.storagePerUnit),
      'what the room for one unit costs a period',
    ),
    asRatio(periods, 'the periods of waiting'),
    'over the wait',
  );
  const lost = scale(
    scale(spot.value.price, ctx.params.ratio(terms.spoilage), 'what a period in store spoils'),
    asRatio(periods, 'the periods of waiting'),
    'over the wait',
  );
  const fixing = lastFixing(ctx, `${String(i.ccy)}:secured`);
  if (!fixing.some) return none<PerPiece>();
  const secured = fixing.value;
  // Law 8: the benchmark is a rate A YEAR and the wait is in periods, so the two are put in the
  // same unit by the calendar's own day count and never by a number of weeks anybody typed.
  const years = yearFraction(
    CARRY_DAY_COUNT,
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(to),
  );
  const money = scale(
    scale(
      spot.value.price,
      asRatio(secured, 'what the money costs a year'),
      'the cost of the money',
    ),
    asRatio(years, 'the year this wait is a fraction of'),
    'over the wait',
  );
  return some(plus(plus(room, lost, 'room and spoilage'), money, 'and what the money costs'));
}

/**
 * C3, D1: THE BASIS — this book's print against the spot plus the carry. Measured, never set: there
 * is no parameter here and nothing anywhere reads it back into a price. Positive is contango wider
 * than carry, which somebody with room can close; negative is backwardation, which nobody can.
 */
export function commodityBasis(
  ctx: WorldReads,
  deliverable: InstrumentId,
  expiry: Period,
): Option<PerPiece> {
  const future = ctx.prices.latest(commodityFutureLineOf(deliverable, expiry), ctx.period);
  const spot = ctx.prices.latest(deliverable, ctx.period);
  const carry = commodityCarryOf(carryFromWorld(ctx), deliverable, expiry);
  if (!future.some || !spot.some || !carry.some) return none<PerPiece>();
  return some(
    minus(
      future.value.price,
      plus(spot.value.price, carry.value, 'the spot and the carry'),
      'the basis',
    ),
  );
}

/**
 * B1, B2, B3, C3: WHO IS ON THE LINE, each for a reason of its own, and all three in ONE target
 * (Clearing A2: what it posts is the distance from where it is to where it wants to be).
 *
 * A party HOLDING the grade is long the thing and would rather not be, so it wants to be short by
 * what it holds — the producer's hedge, and it is a read of the register rather than a plan anybody
 * published. A party whose own view of the grade is above where this book stands wants to be LONG,
 * and one whose view is below wants to be short: that is the investor, and its size is what its own
 * capital can carry. And a party that can find ROOM sells a future standing above spot plus carry
 * and holds the thing against it — C3's storage arbitrage, which is the only thing that bounds
 * contango, and it is somebody doing a trade rather than an arithmetic identity.
 */
function futureOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isCommodityFuture(decl.terms)) return [];
  const t = decl.terms;
  const spot = view.print(t.deliverable);
  if (!spot.some) return [];
  const outlook = view.outlook(about({ on: 'price', instrument: t.deliverable }));
  const mine = outlook.some
    ? asPerPiece(outlook.value.expected, `what it expects ${t.deliverable} to be worth`)
    : spot.value.price;
  const at = view.print(t.book);
  let position = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isCommodityFuture(c.terms) || c.terms.deliverable !== t.deliverable) continue;
    if (c.terms.expiry !== t.expiry) continue;
    const iAmA = c.a === view.self.id;
    position = addQty(
      position,
      iAmA === c.terms.long ? c.notional : negQty(c.notional, 'the other side of it'),
      'its position',
    );
  }
  // B1, 18.3: short by what it is EXPOSED to, which is not what it is holding. A mill holding a
  // week of grain and buying a week of grain every week is not long grain: what it holds it will
  // use, and what it will buy is a short — a price rise costs it. So the stock, less what it means
  // to buy, plus what it means to sell, each the party's own outlook (`exposedTo`, §46 A2). This is
  // also the missing BUYER: every holder wanting to be short and nobody wanting to be long is why
  // 3,680 sessions of this book cleared nothing.
  const held = exposedTo(view, t.deliverable);
  let want = negated(
    over(held, asRatio(t.lotUnits, 'what one lot is'), 'what its exposure comes to in lots'),
    'so lots it wants to be short',
  );
  const own = view.standsBehind();
  const conviction =
    own.pieces > 0
      ? view.registry.deliverable(
          pricedAt(
            own,
            scale(
              asAmount<'piece'>(t.lotUnits, 'the units in a lot'),
              asRatio(spot.value.price, 'at their price'),
              'what one lot commits',
            ),
            'what it can carry',
          ),
        )
      : NO_QTY;
  if (at.some && conviction > 0) {
    const book = at.value.price;
    // B2, B3: its own view against where the book stands, and nothing else decides the side.
    if (mine > book) want = plus(want, conviction, 'and the length its own view wants');
    if (mine < book) want = minus(want, conviction, 'and the length its own view would shed');
    // C3: and the carry trade, for a party that has somewhere to put the thing. It is the same
    // conviction pointed at a different fact: the book above spot plus carry is money on the table
    // for whoever can hold the grade, and nothing bounds contango except somebody taking it.
    const carry = commodityCarryOf(carryFromView(view), t.deliverable, t.expiry);
    if (carry.some && book > plus(spot.value.price, carry.value, 'spot and the carry')) {
      want = minus(want, conviction, 'and the lots it would sell against room it has');
    }
  }
  const move = minus(want, position, 'from the position it has to the one it wants');
  /**
   * A-66, XI-13, §46 A3: AND A PARTY WITH NOTHING TO CHANGE QUOTES BOTH WAYS AROUND ITS OWN NUMBER.
   *
   * Every term that could make this party a BUYER stands behind `at.some`, so a book that has never
   * printed leaves everybody with `−held / lotUnits` and the session is sell-only: 3,680 commodity
   * future sessions, every one `noDemand`, so no first print ever happened. The spread below is
   * `fx-derivatives`' answer — a bid a tick inside its own number and an ask a tick outside, sized
   * by what its balance sheet carries — and its number is the SPOT line's, never this book's.
   */
  if (move === 0) {
    if (conviction <= 0) return [];
    const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
    const bid = minus(mine, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(conviction) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(mine, tick, 'a tick outside its own number'),
        qty: asQty(conviction),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(move, 'the size of the move'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price: mine, qty: asQty(qty) }];
}

const commodityFutureClass: DerivativeClassDecl = {
  kind: COMMODITY_FUTURE,
  subject: (m) => {
    const t = m.contract.terms;
    return isCommodityFuture(t) ? String(t.deliverable) : '';
  },
  /**
   * Law 18, B1: WHY A PARTY IS IN A FUTURE ON A THING — because it is holding the thing, or because
   * it is already in one. Both are reads of its own state and both are what `futureOrders` reads:
   * with neither, what it would want is its conviction alone, and that needs a level on the book to
   * stand against (`openToAll` below is the other half of the same narrowing).
   *
   * A real period asked every firm in the world about every one of these books — six and a half
   * million questions for thirty thousand orders — and NOT ONE of the 736 books had printed, so not
   * one of those orders came from anywhere but a holding or a position.
   */
  reasons: (view) => {
    const out = new Set<string>();
    for (const h of view.holdings()) out.add(String(h.instrument));
    for (const c of view.contracts.mine()) {
      if (isCommodityFuture(c.terms)) out.add(String(c.terms.deliverable));
    }
    return [...out];
  },
  /**
   * B2, B3: and once the book HAS printed there is a level for anybody's own number to be above or
   * below, so everybody is asked again. It is a fact about the book, settled once for it.
   */
  openToAll: (m, reads) => {
    const t = m.contract.terms;
    return isCommodityFuture(t) && reads.prices.latest(t.book, reads.period).some;
  },
  orders: futureOrders,
  measures: (m, reads): readonly ContractMeasure[] => {
    const t = m.contract.terms;
    if (!isCommodityFuture(t)) return [];
    const out: ContractMeasure[] = [];
    const basis = commodityBasis(reads, t.deliverable, t.expiry);
    if (basis.some) {
      out.push({
        subject: String(t.deliverable),
        measure: 'the future against spot plus carry',
        tenorYears: null,
        level: basis.value,
        unit: 'money',
      });
    }
    /**
     * E2, C4: OPEN INTEREST AGAINST DELIVERABLE SUPPLY. How many units this book has promised to
     * deliver, against how many of them exist in that place at all. It is two reads and a ratio,
     * and it is the number that says whether convergence is arithmetic or a squeeze: a book that
     * has sold three times the crop cannot all deliver, and what happens then is a fail (E1) rather
     * than a price anybody corrects.
     */
    let promised = NO_QTY;
    for (const c of reads.contracts.open_()) {
      const terms = c.terms;
      if (!isCommodityFuture(terms)) continue;
      if (terms.deliverable !== t.deliverable || terms.expiry !== t.expiry) continue;
      if (!terms.long) continue;
      promised = addQty(
        promised,
        scaleQty(c.notional, terms.lotUnits, 'the units it promises'),
        'open interest',
      );
    }
    // Law 19: what exists is what the instrument says is issued, read from the one place that
    // writes it. The two are published side by side rather than as a ratio, because a ratio is a
    // third number and these are the two a reader wants.
    out.push({
      subject: String(t.deliverable),
      measure: 'units this book has promised to deliver',
      tenorYears: null,
      level: promised,
      unit: 'notional',
    });
    out.push({
      subject: String(t.deliverable),
      measure: 'units of the grade that exist to deliver',
      tenorYears: null,
      level: reads.instruments.get(t.deliverable).issued,
      unit: 'notional',
    });
    const carry = commodityCarryOf(carryFromWorld(reads), t.deliverable, t.expiry);
    if (carry.some) {
      out.push({
        subject: String(t.deliverable),
        measure: 'room, spoilage and money to delivery',
        tenorYears: null,
        level: carry.value,
        unit: 'money',
      });
    }
    return out;
  },
};

function params(): ParamDecl[] {
  return [
    {
      id: COMMODITY_FUTURE_PARAMS.series,
      value: 4,
      unit: 'delivery dates',
      dimension: 'count',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Commodity Futures A2: how many delivery dates stand open at once. A convention of the exchange, and what makes the book a CURVE rather than a price — one date is not a term structure.',
    },
    {
      id: COMMODITY_FUTURE_PARAMS.spacing,
      value: 13,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Commodity Futures A2: periods between one delivery date and the next. A convention of the exchange; a quarter, which is what a physical market lists.',
    },
    {
      id: COMMODITY_FUTURE_PARAMS.window,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of the grade's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
  ];
}

/**
 * A2: THE SERIES. A ladder of delivery dates on every grade that can actually be delivered — one
 * that can be stored, because a contract to deliver a thing nobody can hold to the date is not a
 * contract anybody can be short of, and one that has printed, because a book on a grade with no
 * spot price has nothing to converge to.
 */
function openBooks(ctx: MechanismContext, house: (ccy: CurrencyCode) => PartyId): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.periods(COMMODITY_FUTURE_PARAMS.window);
  const series = ctx.params.count(COMMODITY_FUTURE_PARAMS.series);
  const spacing = ctx.params.periods(COMMODITY_FUTURE_PARAMS.spacing);
  for (const market of ctx.markets) {
    if (kindOf(market) !== 'asset') continue;
    if (!ctx.instruments.has(market.instrument)) continue;
    const i = ctx.instruments.get(market.instrument);
    if (!i.status.live || !isGoodTerms(i.terms)) continue;
    const terms = i.terms;
    // A1: deliverable means deliverable. A thing that cannot be moved cannot be handed over at a
    // place it was not made in, and a thing nobody stores cannot be held to the date.
    if (!terms.portable || terms.storagePerUnit === null) continue;
    if (!ctx.prices.latest(i.id, ctx.period).some) continue;
    const perUnit = ctx.params.ratio(terms.storagePerUnit);
    if (!(perUnit > 0)) continue;
    // A1.a: a lot is a silo's worth of it, which is a read and never a number typed here.
    const lotUnits = div(1, perUnit, 'how much of it fills one unit of covered space');
    const clearer =
      ctx.parties.has(house(i.ccy)) && ctx.parties.get(house(i.ccy)).status.alive
        ? house(i.ccy)
        : null;
    for (let n = 1; n <= series; n += 1) {
      const expiry = nextCycle(ctx.period, mul(spacing, n, 'this far out'));
      const id = commodityFutureMarketOf(i.id, expiry);
      if (open.has(String(id))) continue;
      const t: CommodityFutureTerms = {
        kind: COMMODITY_FUTURE,
        deliverable: i.id,
        market: market.id,
        expiry,
        lotUnits,
        book: commodityFutureLineOf(i.id, expiry),
        long: true,
        window,
      };
      ctx.openMarket({
        id,
        name: `${String(i.id)} ${expiry}`,
        instrument: commodityFutureLineOf(i.id, expiry),
        ccy: i.ccy,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: COMMODITY_FUTURE, terms: t, house: clearer },
      });
    }
  }
}

/**
 * A2, Clearing C3 (18.4): A SERIES THAT IS OVER IS CLOSED.
 *
 * Every series this module ever opened stayed open, so a world that had run a year was holding
 * sessions in books whose delivery date had passed months before — each one printing `noDemand`
 * every period for the rest of the run, and each one counted in every reading of how many markets
 * this world has. A series past its expiry with nothing open in it has nothing left to do.
 *
 * The kernel refuses to close a book with open interest, so a row that has not delivered — a fail,
 * a side that has ceased — keeps its book, which is the answer that leaves the row somewhere to be
 * marked (Derivative D8).
 */
function closeExpired(ctx: MechanismContext): void {
  for (const market of ctx.markets) {
    const decl = contractOf(market);
    if (decl === undefined || !isCommodityFuture(decl.terms)) continue;
    if (ctx.period <= decl.terms.expiry) continue;
    ctx.closeMarket(market.id, 'its delivery date has passed and nothing is open in it');
  }
}

/**
 * A1, C4, XI-5: DELIVERY, which is why the thing converges. The short hands over the units and the
 * long pays for them at the grade's own cleared spot price, in one instruction — so either the
 * goods and the money both move or neither does. A short with nothing in the shed FAILS, and the
 * fail is recorded: it is a state this world can be in, not an excuse to settle in cash.
 */
function deliver(ctx: MechanismContext): void {
  const done = new Set<string>();
  for (const c of ctx.contracts.open_()) {
    if (!isCommodityFuture(c.terms) || done.has(String(c.id))) continue;
    const t = c.terms;
    if (ctx.period < t.expiry) continue;
    const price = ctx.prices.latest(t.deliverable, ctx.period);
    if (!price.some) continue;
    if (!ctx.instruments.has(t.deliverable) || !ctx.instruments.get(t.deliverable).status.live) {
      continue;
    }
    const rows = c.house === null ? [c] : matching(ctx, c);
    const legs: Leg[] = [];
    let deliverable = true;
    for (const row of rows) {
      if (!isCommodityFuture(row.terms)) continue;
      const long = row.terms.long ? row.a : row.b;
      const short = row.terms.long ? row.b : row.a;
      if (!ctx.parties.get(long).status.alive || !ctx.parties.get(short).status.alive) {
        deliverable = false;
        break;
      }
      const units = scaleQty(row.notional, row.terms.lotUnits, 'the units this lot delivers');
      legs.push({ kind: 'contract', act: 'close', contract: row.id, why: 'delivered' });
      if (units <= 0) continue;
      // A1, D8 (18.1): AT THE PRICE THE ROW WAS STRUCK AT, per row. It delivered at spot, so a
      // mill that had locked a lot in at 98 paid 100 for it when grain printed 100 — and margin in
      // this world is posted and HELD, so nothing gave the difference back. See `bond-futures`.
      const struck = moneyLevel(row.struckAt, 'a commodity future is struck at a price');
      legs.push({
        kind: 'asset',
        from: short,
        to: long,
        instrument: t.deliverable,
        qty: ctx.registry.deliverable(units),
        pricePerUnit: some(struck),
        accruedPerUnit: none(),
      });
      legs.push({
        kind: 'money',
        // 0i.5: delivery: the short's proceeds of the lot it hands over.
        receipt: { of: 'disposal' },
        from: ctx.accountOf(long, row.ccy),
        to: ctx.accountOf(short, row.ccy),
        ccy: row.ccy,
        amount: ctx.registry.cashFor(valueAt(struck, units, row.ccy, 'what the lot costs')),
      });
    }
    if (!deliverable || legs.length === 0) continue;
    for (const row of rows) done.add(String(row.id));
    const r = ctx.settle({ legs, cause: 'corporateAction', reason: `${c.id} delivers` });
    ctx.record(
      'commodity.delivered',
      [String(c.id), String(t.deliverable)],
      {
        contract: String(c.id),
        deliverable: String(t.deliverable),
        rows: rows.length,
        units: scaleQty(c.notional, t.lotUnits, 'the units this lot delivers'),
        // 18.1: what it delivered AT — the level this row struck — and what the thing was worth on
        // the day beside it, so a reader can see what the contract was worth to whoever held it.
        at: moneyLevel(c.struckAt, 'a commodity future is struck at a price'),
        spot: price.value.price,
        // E1: a delivery that did not settle is a FAIL, and it says so. Nothing pays a difference
        // instead, and the row stays open for the layer to resolve at its stated value.
        settled: r.outcome === 'settled',
      },
      true,
    );
  }
}

/** C2: the other half of a cleared trade — the row the house holds facing the other member. */
function matching(ctx: MechanismContext, c: Contract): readonly Contract[] {
  const house = c.house;
  const mine = c.terms;
  if (house === null || !isCommodityFuture(mine)) return [c];
  /**
   * C2 (18.2): THE OTHER HALF IS NAMED ON THE ROW. It used to be found by looking through the
   * house's open rows for one whose deliverable, expiry, notional and level agreed — and the level
   * was compared with `===` on an object, so two rows struck at the same price in two sessions
   * matched only when they happened to share a reference. The instruction that wrote them says
   * which is which now (`pairedWith`), and a row with no sibling is a row that had none.
   */
  const paired = c.pairedWith;
  if (!paired.some) return [c];
  const other = ctx.contracts.openOf(house).find((x) => x.id === paired.value);
  if (other === undefined) return [c];
  // Register C4: the house receives before it delivers — it is flat, so what it hands the long is
  // what the short just handed it, and legs settle in the order they are written.
  const receivesFirst = (x: Contract): number => {
    const t = x.terms;
    if (!isCommodityFuture(t)) return 1;
    return (t.long ? x.a : x.b) === house ? 0 : 1;
  };
  return [c, other].sort((x, y) => receivesFirst(x) - receivesFirst(y));
}

export function commodityFutures(house: (ccy: CurrencyCode) => PartyId): SystemModule {
  return {
    id: 'commodity-futures',
    spec: 'Commodity Futures',
    requires: ['derivative-layer', 'goods', 'commodities', 'money-market', 'indices'],
    instrumentKinds: [],
    derivativeKinds: [commodityFutureKind],
    derivativeClasses: [commodityFutureClass],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: COMMODITY_CONTRACTS, name: 'lots', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'commodityFutures.books',
        spec: 'Commodity Futures A2 Derivative Layer B1',
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'print', of: 'anyPeriod' }],
        writes: [],
        run: (ctx): void => {
          openBooks(ctx, house);
          // 18.4: and the ones whose date has passed stop being books.
          closeExpired(ctx);
        },
      },
      {
        name: 'commodityFutures.deliver',
        spec: 'Commodity Futures A1 Commodity Futures C4 XI-5',
        // Before the layer's own resolution: what this contract does at its term is DELIVER, and a
        // cash close-out on top of a delivery would settle it twice.
        anchor: { before: 'derivatives.resolve' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [{ kind: 'event', name: 'index.benchmark', of: 'anyPeriod' }],
        writes: [],
        run: deliver,
      },
    ],
    participants: [],
    families: [],
  };
}
