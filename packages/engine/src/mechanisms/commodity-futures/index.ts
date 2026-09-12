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
import { nextCycle, type Calendar, type Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, MarketId, ParamId, PartyId, UnitId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty } from '../../core/tick.js';
import { CENT_TICK } from '../../registry/grid.js';
import {
  isGoodTerms,
  storageRateIn,
  type GoodTerms,
} from '../../registry/physical.js';
import type {
  Contract,
  ContractMeasure,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import { contractOf, kindOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { Leg } from '../../ledger/instruction.js';
import type { Event, EventKind } from '../../journal/journal.js';
import type { Instrument } from '../../register/instruments.js';
import type { Print } from '../../prices/price-store.js';
import type { MechanismContext, ParticipantView, WorldReads } from '../../world/context.js';
import type { DerivativeClassDecl, SystemModule } from '../../world/module.js';

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
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isCommodityFuture(c.terms)) return 0;
  const p = reads.print(c.terms.book, at);
  if (!p.some) return 0;
  const move = sub(p.value.price, c.struckAt, 'the future now against the level struck');
  const worth = mul(mul(move, c.notional, 'per lot'), c.terms.lotUnits, 'of the grade each');
  return c.terms.long ? worth : -worth;
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
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isCommodityFuture(c.terms)) return none();
    const move = reads.measuredMove(c.terms.deliverable, c.terms.window);
    if (!move.some) return none();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.periods(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['periods']>[0],
    );
    return some(
      mul(
        mul(mul(move.value, c.notional, 'per lot'), c.terms.lotUnits, 'of the grade each'),
        Math.sqrt(left > 0 ? left / horizon : 1),
        'over the life it has left',
      ),
    );
  },
  /**
   * A1, Money Market A2: WHAT TAKING DELIVERY COSTS, said in advance so a treasury can fund it. The
   * long pays for the whole lot at the grade's own spot price on the delivery date.
   */
  cashDue: (c, at, reads, party): number => {
    const t = c.terms;
    if (!isCommodityFuture(t) || at < t.expiry) return 0;
    const long = t.long ? c.a : c.b;
    if (long !== party) return 0;
    const price = reads.print(t.deliverable, at);
    if (!price.some) return 0;
    return mul(mul(c.notional, t.lotUnits, 'the units it takes'), price.value.price, 'at their price');
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
  readonly params: { ratio(id: ParamId): number };
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

export function commodityCarryOf(ctx: CarryReads, deliverable: InstrumentId, to: Period): Option<number> {
  if (!ctx.instruments.has(deliverable)) return none<number>();
  const i = ctx.instruments.get(deliverable);
  if (!isGoodTerms(i.terms)) return none<number>();
  const terms: GoodTerms = i.terms;
  const spot = ctx.price(deliverable);
  if (!spot.some) return none<number>();
  const periods = to > ctx.period ? to - ctx.period : 0;
  if (terms.storagePerUnit === null) return none<number>();
  const rate = storageRateIn(ctx, terms.region);
  if (rate === undefined) return none<number>();
  const room = mul(
    mul(ctx.params.ratio(terms.storagePerUnit), rate, 'what the room for one unit costs a period'),
    periods,
    'over the wait',
  );
  const lost = mul(
    mul(ctx.params.ratio(terms.spoilage), spot.value.price, 'what a period in store spoils'),
    periods,
    'over the wait',
  );
  const fixing = ctx.lastPublic('index.benchmark');
  if (!fixing.some || !fixing.value.subjects.includes(`${String(i.ccy)}:secured`)) {
    return none<number>();
  }
  const secured = fixing.value.data['rate'];
  if (typeof secured !== 'number') return none<number>();
  // Law 8: the benchmark is a rate A YEAR and the wait is in periods, so the two are put in the
  // same unit by the calendar's own day count and never by a number of weeks anybody typed.
  const years = yearFraction(
    CARRY_DAY_COUNT,
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(to),
  );
  const money = mul(
    mul(spot.value.price, secured, 'what the money costs a year'),
    years,
    'over the wait',
  );
  return some(sum([room, lost, money]).value);
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
): Option<number> {
  const future = ctx.prices.latest(commodityFutureLineOf(deliverable, expiry), ctx.period);
  const spot = ctx.prices.latest(deliverable, ctx.period);
  const carry = commodityCarryOf(carryFromWorld(ctx), deliverable, expiry);
  if (!future.some || !spot.some || !carry.some) return none<number>();
  return some(
    sub(future.value.price, add(spot.value.price, carry.value, 'the spot and the carry'), 'the basis'),
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
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  const outlook = view.outlook(`price.${String(t.deliverable)}`);
  const mine = outlook.some ? outlook.value.expected : spot.value.price;
  const at = view.print(t.book);
  let position = 0;
  for (const c of view.contracts.mine()) {
    if (!isCommodityFuture(c.terms) || c.terms.deliverable !== t.deliverable) continue;
    if (c.terms.expiry !== t.expiry) continue;
    const iAmA = c.a === view.self.id;
    position = add(position, iAmA === c.terms.long ? c.notional : -c.notional, 'its position');
  }
  // B1: short by what it is holding. It made the thing, or it bought it; either way it is exposed.
  const held = view.free(t.deliverable);
  let want = -div(held, t.lotUnits, 'what its holding comes to in lots');
  const own = view.equity();
  if (at.some && own > 0) {
    const book = at.value.price;
    const conviction = view.registry.deliverable(
      unit,
      div(own, mul(spot.value.price, t.lotUnits, 'what one lot commits'), 'what it can carry'),
    );
    // B2, B3: its own view against where the book stands, and nothing else decides the side.
    if (mine > book) want = add(want, conviction, 'and the length its own view wants');
    if (mine < book) want = sub(want, conviction, 'and the length its own view would shed');
    // C3: and the carry trade, for a party that has somewhere to put the thing. It is the same
    // conviction pointed at a different fact: the book above spot plus carry is money on the table
    // for whoever can hold the grade, and nothing bounds contango except somebody taking it.
    const carry = commodityCarryOf(carryFromView(view), t.deliverable, t.expiry);
    if (carry.some && book > add(spot.value.price, carry.value, 'spot and the carry')) {
      want = sub(want, conviction, 'and the lots it would sell against room it has');
    }
  }
  const move = sub(want, position, 'from the position it has to the one it wants');
  if (move === 0) return [];
  const qty = view.registry.deliverable(unit, move > 0 ? move : -move);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price: mine, qty: asQty(qty) }];
}

const commodityFutureClass: DerivativeClassDecl = {
  kind: COMMODITY_FUTURE,
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
    let promised = 0;
    for (const c of reads.contracts.open_()) {
      const terms = c.terms;
      if (!isCommodityFuture(terms)) continue;
      if (terms.deliverable !== t.deliverable || terms.expiry !== t.expiry) continue;
      if (!terms.long) continue;
      promised = add(promised, mul(c.notional, terms.lotUnits, 'the units it promises'), 'open interest');
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
      const units = mul(row.notional, row.terms.lotUnits, 'the units this lot delivers');
      legs.push({ kind: 'contract', act: 'close', contract: row.id, why: 'delivered' });
      if (units <= 0) continue;
      legs.push({
        kind: 'asset',
        from: short,
        to: long,
        instrument: t.deliverable,
        qty: ctx.registry.deliverable(ctx.instruments.get(t.deliverable).unit, units),
        pricePerUnit: some(price.value.price),
        accruedPerUnit: none(),
        fromCell: none(),
        toCell: none(),
      });
      legs.push({
        kind: 'money',
        from: ctx.accountOf(long, row.ccy),
        to: ctx.accountOf(short, row.ccy),
        ccy: row.ccy,
        amount: ctx.registry.cashFor(row.ccy, mul(units, price.value.price, 'what the lot costs')),
        fromCell: none(),
        toCell: none(),
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
        units: mul(c.notional, t.lotUnits, 'the units this lot delivers'),
        at: price.value.price,
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
  const other = ctx.contracts.openOf(house).find((x) => {
    const theirs = x.terms;
    if (x.id === c.id || !isCommodityFuture(theirs)) return false;
    return (
      theirs.deliverable === mine.deliverable &&
      theirs.expiry === mine.expiry &&
      x.struckAt === c.struckAt &&
      x.notional === c.notional
    );
  });
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
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          openBooks(ctx, house);
        },
      },
      {
        name: 'commodityFutures.deliver',
        spec: 'Commodity Futures A1 Commodity Futures C4 XI-5',
        cycle: 'anchor',
        // Before the layer's own resolution: what this contract does at its term is DELIVER, and a
        // cash close-out on top of a delivery would settle it twice.
        anchor: { before: 'derivatives.resolve' },
        run: deliver,
      },
    ],
    participants: [],
    families: [],
  };
}
