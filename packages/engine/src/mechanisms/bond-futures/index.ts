/**
 * The deliverable bond future, its carry, its net basis, and the trade that makes repo demand real.
 *
 * @spec Sovereign I1 Sovereign I1.a Sovereign I2 Sovereign I3 Sovereign I3.a Derivative D1 Derivative D1.b Derivative D2 Derivative D3 Derivative D3.a Derivative D4 Derivative D7 Derivative D8 Derivative D11 Derivative D11.a Derivative D12 Derivative Layer B1 Derivative Layer C2 Derivative Layer D1 XI-5 Law 3 Law 15 Law 19
 *
 * I1: THE PRICE IS PER UNIT OF FACE and what settles is the BOND. At delivery the short hands over
 * the deliverable line and the long pays for it at that line's own cleared cash price, in one
 * instruction — delivery against payment (XI-5), not a cash difference dressed up as one.
 *
 * I1.a: THE CARRY IS TWO READS. The coupon the bond's own terms promise, and what financing it in
 * the secured book costs — both of them things this world already prints. The NET BASIS is the
 * future's price against that carry, and it is MEASURED: there is no `basis.*` parameter anywhere,
 * nothing targets it, and when it goes somewhere surprising that is a finding about a mechanism.
 *
 * I3: THE BASIS TRADE is long the cash bond, financed, short the future — and I3.a is why it is
 * built rather than described: it is funded, it is margined, and it is CUT ON A DRAWDOWN by the
 * trader's own module when its equity falls past what it will stand. Nothing makes it whole. The
 * repo demand it creates is the largest single source of real demand in a secured market, and here
 * it is a read of that market's own rows rather than a number anybody set.
 */
import { moneyPrint } from '../../prices/price-store.js';
import {
  type Cash,
  type PerPiece,
  absolute,
  asAmount,
  asPerPiece,
  asRatio,
  minus,
  negated,
  noCash,
  over,
  plus,
  pricedAt,
  scale,
  valueAt,
} from '../../core/measure.js';
import { nextCycle, type Period } from '../../calendar/calendar.js';
import { compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { addQty, asQty, negQty, NO_QTY, scaleQty } from '../../core/tick.js';
import { FACE_TICK } from '../../registry/grid.js';
import { issuedBy } from '../../register/instruments.js';
import type {
  Contract,
  ContractPayment,
  ContractMeasure,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { exposedTo, moneyLevel } from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import { contractOf, kindOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { Leg } from '../../ledger/instruction.js';
import type { MechanismContext, ParticipantView, WorldReads } from '../../world/context.js';
import type { DerivativeClassDecl, SystemModule } from '../../world/module.js';
import { about } from '../../world/context.js';

export const BOND_FUTURE = derivativeKindId('bond.future');
/** I1: the notional is a count of CONTRACTS, each for a stated amount of face. */
export const FUTURE_CONTRACTS = unitId('futureContracts');
const FUTURE_DAY_COUNT = 'ACT/ACT';

export const BOND_FUTURE_PARAMS = {
  size: paramId('bond.future.contractSize'),
  life: paramId('bond.future.life.periods'),
  window: paramId('bond.future.margin.window'),
  tolerance: paramId('trader.drawdown.tolerance'),
} as const;

export const bondFutureMarketOf = (deliverable: InstrumentId, expiry: number): MarketId =>
  marketId(`mkt.bond.future.${deliverable}.${expiry}`);

export const bondFutureLineOf = (deliverable: InstrumentId, expiry: number): InstrumentId =>
  instrumentId(`bond.future:${deliverable}:${expiry}`);

export interface BondFutureTerms extends ContractTerms {
  readonly kind: typeof BOND_FUTURE;
  /** I1: a NAMED benchmark line. Not a basket and not a notional bond nobody issued. */
  readonly deliverable: InstrumentId;
  readonly market: MarketId;
  readonly expiry: Period;
  /** I1: how much face one contract delivers. */
  readonly contractSize: number;
  readonly book: InstrumentId;
  readonly long: boolean;
  readonly window: number;
}

export const isBondFuture = (t: ContractTerms): t is BondFutureTerms =>
  'deliverable' in t && 'contractSize' in t && 'long' in t;

/** D8: what it is worth to `a` — the future's own print against the level it was struck at. */
function markOf(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isBondFuture(c.terms)) return noCash(c.ccy);
  const p = reads.print(c.terms.book, at);
  if (!p.some) return noCash(c.ccy);
  const move = minus(
    // 18.0: and the PRINT says it is money too, which is the other half of `moneyLevel`'s check.
    moneyPrint(p.value, 'the price this book last printed'),
    moneyLevel(c.struckAt, 'a bond future is struck at a price'),
    'the future now against the level struck',
  );
  const worth = valueAt(
    move,
    scale(c.notional, asRatio(c.terms.contractSize, 'the face in a contract'), 'per contract'),
    c.ccy,
    'of face each',
  );
  return c.terms.long ? worth : negated(worth, 'and the other side of it');
}

export const bondFutureKind: DerivativeKindProfile = {
  id: BOND_FUTURE,
  unit: FUTURE_CONTRACTS,
  // I1: quoted per unit of FACE, to the same ten-thousandth of par a bond is quoted to.
  priceTick: FACE_TICK,
  // I1: a price of face, so the level a reader sees is money and not a rate.
  quotedAs: 'money',
  underlying: (c) => ({
    kind: 'print',
    market: isBondFuture(c.terms) ? c.terms.market : ('' as MarketId),
    instrument: isBondFuture(c.terms) ? c.terms.deliverable : ('' as InstrumentId),
  }),
  validateTerms: (t) => {
    if (!isBondFuture(t)) throw new Error('not bond future terms');
    if (!(t.contractSize > 0)) throw new Error('a future that delivers no face at all');
  },
  displayName: (c) =>
    isBondFuture(c.terms) ? `${c.terms.deliverable} future ${c.terms.expiry}` : String(c.id),
  mark: markOf,
  flip: (t) => (isBondFuture(t) ? { ...t, long: !t.long } : t),
  // I1, XI-5: what settles at delivery is the BOND against cash, which is an asset leg and a money
  // leg in one instruction — not a payment. This module's own delivery phase writes it, because a
  // periodic payment cannot carry a thing being handed over.
  legs: (): readonly ContractPayment[] => [],
  premiumPerUnit: (): PerPiece => asPerPiece(0, 'a future costs nothing to enter'),
  initialMargin: (c, at, reads): Option<Cash> => {
    if (!isBondFuture(c.terms)) return none<Cash>();
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
          scale(
            c.notional,
            asRatio(c.terms.contractSize, 'the face in a contract'),
            'per contract',
          ),
          c.ccy,
          'of face each',
        ),
        asRatio(Math.sqrt(left > 0 ? left / horizon : 1), 'over the life it has left'),
        'over the life it has left',
      ),
    );
  },
  /**
   * I1, Money Market A2: WHAT TAKING DELIVERY COSTS, said in advance so a treasury can fund it.
   *
   * The long pays the whole face at the deliverable's own cash price on the delivery date. It is
   * nowhere in `legs`, because what settles here is a bond against money and a payment cannot
   * carry a thing being handed over — so the kind says it here, and a bank that must take a
   * hundred thousand of face next week knows this week.
   */
  cashDue: (c, at, reads, party): Cash => {
    const t = c.terms;
    if (!isBondFuture(t) || at < t.expiry) return noCash(c.ccy);
    const long = t.long ? c.a : c.b;
    if (long !== party) return noCash(c.ccy);
    // 18.1: at the price THIS ROW was struck at, which is what it will actually pay — the spot
    // price on the day is what it would have paid without the contract.
    return valueAt(
      moneyLevel(c.struckAt, 'a bond future is struck at a price'),
      scale(c.notional, asRatio(t.contractSize, 'the face in a contract'), 'the face it takes'),
      c.ccy,
      'at the price it struck',
    );
  },
  closeOut: markOf,
  // D11: the term runs out when the module has DELIVERED it. Until then it is open, and the layer's
  // own resolution must not tear it up and pay a cash difference for a contract that delivers.
  expires: () => false,
};

/**
 * I1.a, I2: why a party is in this book, and the two measurements it carries — the carry on what it
 * delivers, and the net basis against it. Both are reads (`netBasis`, `bondCarryOf`) and neither is
 * a target: that the future is not exactly the cash price less the carry is the trade, not a
 * discrepancy.
 */
const bondFutureClass: DerivativeClassDecl = {
  kind: BOND_FUTURE,
  orders: futureOrders,
  measures: (m, reads): readonly ContractMeasure[] => {
    const t = m.contract.terms;
    if (!isBondFuture(t)) return [];
    const basis = netBasis(reads, t.deliverable, t.expiry);
    if (!basis.some) return [];
    return [
      {
        subject: String(t.deliverable),
        measure: 'the future against cash less carry',
        tenorYears: null,
        level: basis.value,
        unit: 'money',
      },
    ];
  },
};

function params(): ParamDecl[] {
  return [
    {
      id: BOND_FUTURE_PARAMS.size,
      value: 100_000,
      unit: 'units of face per contract',
      // `E-8`, item 2: IT IS NOT A PRICE. It is a COUNT of face-units one contract delivers — no
      // money anywhere in it — and it was declared `price` and read as money-per-piece, which is
      // the same mistake `E-8` names for the four levels that ARE money and are stated per named
      // unit. A contract size converts contracts to face; it can never be paid.
      dimension: 'count',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Sovereign I1: how much face one contract delivers. A convention of the exchange, stated with the contract, and what makes a quoted price per unit of face into a size somebody can trade.',
    },
    {
      id: BOND_FUTURE_PARAMS.life,
      value: 13,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Sovereign I1: how long a contract runs to its delivery date. A convention of the exchange.',
    },
    {
      id: BOND_FUTURE_PARAMS.window,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of the deliverable's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
    {
      id: BOND_FUTURE_PARAMS.tolerance,
      value: 0.2,
      unit: 'of its own capital',
      dimension: 'ratio',
      kind: 'preference',
      owner: 'model',
      why: 'Sovereign I3.a: how far a trader lets a position go against it before it closes it. A PREFERENCE — what this party will stand — and it is what makes the basis trade a real position that can be cut rather than one that is carried whatever happens.',
    },
  ];
}

/**
 * I1.a: THE CARRY — what holding the bond to delivery earns and what financing it costs, both read.
 *
 * The coupon comes from the instrument's own terms, which is where a promise lives; the financing
 * comes from what the secured overnight book actually printed. Neither is set here and neither is a
 * parameter: what this function does is subtract two things this world published.
 */
export function bondCarryOf(
  ctx: WorldReads,
  deliverable: InstrumentId,
  to: Period,
): Option<PerPiece> {
  const i = ctx.instruments.get(deliverable);
  const on = ctx.calendar.startOf(ctx.period);
  const flows = ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar, ctx.registry);
  const until = ctx.calendar.startOf(to);
  // What the bond's own terms promise between now and delivery — the coupon, read from the flows
  // rather than from a rate anybody wrote down (Law 19).
  const coupon = sum(
    flows.filter((f) => compareCivil(f.date, until) <= 0).map((f) => f.perUnit),
  ).value;
  const fixing = ctx.journal
    .ofKind('index.benchmark')
    .filter((e) => e.subjects.includes(`${String(i.ccy)}:secured`))
    .at(-1);
  if (fixing === undefined) return none<PerPiece>();
  const rate = fixing.data['rate'];
  if (typeof rate !== 'number') return none<PerPiece>();
  const price = ctx.prices.latest(deliverable, ctx.period);
  if (!price.some) return none<PerPiece>();
  const years = yearFraction(FUTURE_DAY_COUNT, on, until);
  const financing = scale(
    scale(
      price.value.price,
      asRatio(rate, 'what financing it costs a year'),
      'the cost of the money',
    ),
    asRatio(years, 'the year this wait is a fraction of'),
    'to delivery',
  );
  return some(minus(coupon, financing, 'what the coupon earns over what the money costs'));
}

/**
 * I1.a: THE NET BASIS — the future's price against the cash price less the carry. MEASURED, never
 * set: there is no parameter here and nothing anywhere reads this back into a price.
 */
export function netBasis(
  ctx: WorldReads,
  deliverable: InstrumentId,
  expiry: Period,
): Option<PerPiece> {
  const future = ctx.prices.latest(bondFutureLineOf(deliverable, expiry), ctx.period);
  const cash = ctx.prices.latest(deliverable, ctx.period);
  const carry = bondCarryOf(ctx, deliverable, expiry);
  if (!future.some || !cash.some || !carry.some) return none<PerPiece>();
  return some(
    minus(
      future.value.price,
      minus(cash.value.price, carry.value, 'the cash price less the carry'),
      'the net basis',
    ),
  );
}

/**
 * I2: WHO IS ON THE LINE, and each of them for a reason of its own.
 *
 * A holder over its own target in this line SHORTS the future above carry — it is long the thing
 * and would rather not be. A party short of duration goes LONG below carry. And a dealer quotes
 * BOTH WAYS at carry, which is what makes the book a market rather than a queue.
 *
 * I3: the basis trade is the same participant taking both sides of its own arithmetic — long the
 * cash bond and short the future when the net basis pays for the financing — and I3.a is what
 * stops it: its own drawdown tolerance, read from its own equity, with nothing making it whole.
 */
function futureOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isBondFuture(decl.terms)) return [];
  const t = decl.terms;
  const cash = view.print(t.deliverable);
  if (!cash.some) return [];
  /**
   * XI-13, Clearing E1: ITS OWN NUMBER, and it is about the DELIVERABLE rather than about this
   * book — its own outlook of that line where it has one, and what the line last printed
   * otherwise. Both are reads of the cash market, which is where a future's value comes from; a
   * party that posted where THIS book last was would be agreeing with it rather than saying
   * anything, and a book of those prints one number for ever (`banks/dealing-quote.ts` records
   * what that did to a bill, and reversed the same order for the same reason).
   */
  const outlook = view.outlook(about({ on: 'price', instrument: t.deliverable }));
  const mine = outlook.some
    ? asPerPiece(outlook.value.expected, `what it expects ${t.deliverable} to be worth`)
    : cash.value.price;
  /** Where the market is: the comparator that decides the side and the size, never the level. */
  const at = view.print(t.book);
  let position = NO_QTY;
  let worth = noCash(view.registry.currencyOf(view.self.region));
  for (const c of view.contracts.mine()) {
    if (!isBondFuture(c.terms) || c.terms.deliverable !== t.deliverable) continue;
    const iAmA = c.a === view.self.id;
    position = addQty(
      position,
      iAmA === c.terms.long ? c.notional : negQty(c.notional, 'the other side of it'),
      'its position',
    );
    // Currency C4: a row in another money is read in this party's own, at the rate in force.
    worth = plus(worth, view.inOwnMoney(view.contracts.valueOf(c)), 'what its book is worth');
  }
  const own = view.standsBehind();
  /**
   * I3.a: CUT ON A DRAWDOWN, and it is the first thing this party asks. A position that has gone
   * against it past what it will stand is closed — its own equity, its own tolerance, and nothing
   * anywhere makes it whole.
   */
  const tolerance = view.params.ratio(BOND_FUTURE_PARAMS.tolerance);
  if (
    position !== 0 &&
    own.pieces > 0 &&
    worth.pieces < 0 &&
    absolute(worth, 'what it is down').pieces > scale(own, tolerance, 'what it will stand').pieces
  ) {
    const qty = view.registry.deliverable(absolute(position, 'the position it would close'));
    if (qty <= 0) return [];
    return [
      { party: view.self.id, side: position > 0 ? 'sell' : 'buy', price: mine, qty: asQty(qty) },
    ];
  }
  /**
   * I2: ONE PARTY, ONE POSITION. A holder of the line wants to be SHORT the future by what it
   * holds — it is long the thing and would rather not be — and a party with a view that the line
   * is worth more than the book says wants to be LONG. When one party has both reasons they are
   * two terms in one target, and what it posts is the distance from where it is (Clearing A2).
   */
  // 18.3: WHAT IT IS EXPOSED TO, not what it is holding — its stock less what it means to buy and
  // plus what it means to sell, each its own outlook (`exposedTo`, §46 A2). A desk holding a week
  // of the line and buying a week of it every week is not long the line at all, and a party with
  // neither a holding nor a view of its own flow hedges NOTHING, which is what a hedger without an
  // exposure should always have done.
  const held = exposedTo(view, t.deliverable);
  let want = negated(
    over(
      held,
      asRatio(t.contractSize, 'the face one contract delivers'),
      'what its exposure comes to in contracts',
    ),
    'so contracts it wants to be short',
  );
  const price = mine;
  const conviction =
    own.pieces > 0
      ? view.registry.deliverable(
          pricedAt(
            own,
            scale(
              asAmount<'piece'>(t.contractSize, 'the face in a contract'),
              asRatio(cash.value.price, 'at its price'),
              'what one contract commits',
            ),
            'what it can carry',
          ),
        )
      : NO_QTY;
  if (at.some && conviction > 0) {
    const book = at.value.price;
    if (mine > book) {
      want = plus(want, conviction, 'and the duration its own view wants');
    } else if (mine < book) {
      want = minus(want, conviction, 'and the duration its own view would shed');
    }
  }
  const move = minus(want, position, 'from the position it has to the one it wants');
  /**
   * A-66, XI-13, §46 A3: AND A PARTY WITH NOTHING TO CHANGE QUOTES BOTH WAYS AROUND ITS OWN NUMBER.
   *
   * The conviction term above needs a PRINT to know which side of the book this party is on, so in
   * a book that has never printed it drops out and every party is left with its hedging need —
   * which here is `−held / contractSize`, one sign for everybody. The first session of every bond
   * future book was therefore sell-only and never crossed, so it never printed, so the next one was
   * the same. `fx-derivatives` hit this and built the answer: a party with nothing to hedge quotes a
   * bid a tick below and an ask a tick above its own number — a spread, not a crossing — sized by
   * what its own balance sheet has room for. Its number is the DELIVERABLE's, never this book's.
   */
  if (move === 0) {
    if (conviction <= 0) return [];
    const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
    const bid = minus(price, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(conviction) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(price, tick, 'a tick outside its own number'),
        qty: asQty(conviction),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(move, 'the size of the move'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * I1, Clearing C3 (18.4): A SERIES THAT IS OVER IS CLOSED. Its delivery date has passed and nothing
 * is open in it, so it has nothing left to do — and every one of them used to stay open for the
 * rest of the run, printing `noDemand` every period. The kernel refuses one with open interest, so
 * a row that could not deliver keeps its book to be marked against (Derivative D8).
 */
function closeExpired(ctx: MechanismContext): void {
  for (const market of ctx.markets) {
    const decl = contractOf(market);
    if (decl === undefined || !isBondFuture(decl.terms)) continue;
    if (ctx.period <= decl.terms.expiry) continue;
    ctx.closeMarket(market.id, 'its delivery date has passed and nothing is open in it');
  }
}

/** I1: a book on each benchmark line this world prints, cleared where there is a house. */
function openBooks(
  ctx: MechanismContext,
  house: (ccy: CurrencyCode) => PartyId,
  issuer: PartyId,
): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.periods(BOND_FUTURE_PARAMS.window);
  const life = ctx.params.periods(BOND_FUTURE_PARAMS.life);
  const contractSize = ctx.params.count(BOND_FUTURE_PARAMS.size);
  for (const market of ctx.markets) {
    if (kindOf(market) !== 'asset') continue;
    if (!ctx.instruments.has(market.instrument)) continue;
    const i = ctx.instruments.get(market.instrument);
    if (!i.status.live || !issuedBy(i, issuer)) continue;
    if (!ctx.prices.latest(i.id, ctx.period).some) continue;
    // A LADDER, not a new book every period: everything written between two dates on the
    // cycle settles on the same one, into the same book (`nextCycle`).
    const expiry = nextCycle(ctx.period, life);
    const id = bondFutureMarketOf(i.id, expiry);
    if (open.has(String(id))) continue;
    /**
     * I1: THE DELIVERABLE MUST STILL BE THERE ON THE DELIVERY DATE. A line that redeems before it
     * could be handed over is not a deliverable — what would settle is a claim that has ceased,
     * and the register says so (Register B4). What its own terms promise is where the answer is:
     * the last payment it makes is the last day anybody could deliver it.
     */
    const on = ctx.calendar.startOf(ctx.period);
    const flows = ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar, ctx.registry);
    const last = flows[flows.length - 1];
    if (last === undefined) continue;
    if (compareCivil(last.date, ctx.calendar.endOf(expiry)) <= 0) continue;
    const clearer =
      ctx.parties.has(house(i.ccy)) && ctx.parties.get(house(i.ccy)).status.alive
        ? house(i.ccy)
        : null;
    const terms: BondFutureTerms = {
      kind: BOND_FUTURE,
      deliverable: i.id,
      market: market.id,
      expiry,
      contractSize,
      book: bondFutureLineOf(i.id, expiry),
      long: true,
      window,
    };
    ctx.openMarket({
      id,
      name: `${String(i.id)} future`,
      instrument: bondFutureLineOf(i.id, expiry),
      ccy: i.ccy,
      rationing: 'proRata',
      kind: 'contract',
      contract: { kind: BOND_FUTURE, terms, house: clearer },
    });
  }
}

/**
 * I1, XI-5: DELIVERY. The short hands over the face and the long pays for it at the deliverable's
 * own cleared cash price, in one instruction — so either the bond and the money both move or
 * neither does. A short with nothing to deliver FAILS, which is a recorded state and not an excuse
 * to settle in cash instead.
 */
function deliver(ctx: MechanismContext): void {
  const done = new Set<string>();
  for (const c of ctx.contracts.open_()) {
    if (!isBondFuture(c.terms) || done.has(String(c.id))) continue;
    const t = c.terms;
    if (ctx.period < t.expiry) continue;
    const price = ctx.prices.latest(t.deliverable, ctx.period);
    if (!price.some) continue;
    // Register B4: and a line that has ceased since cannot be delivered at all. The row is left to
    // the layer's own resolution, which closes it at its stated value (D11.a).
    if (!ctx.instruments.has(t.deliverable) || !ctx.instruments.get(t.deliverable).status.live)
      continue;
    /**
     * C2, XI-5: A CLEARED DELIVERY IS ONE INSTRUCTION. The house is buyer to the seller and seller
     * to the buyer, which is two rows — and settling them one at a time leaves the house holding a
     * bond between the two, which it then has to be wound up out of if it dies in between. It is
     * flat BY CONSTRUCTION, so the construction has to be one pass: both rows close, the line goes
     * from the short member to the long one through the house, and the money comes back the other
     * way. Either all of it settles or none of it does.
     */
    const rows = c.house === null ? [c] : matching(ctx, c);
    const legs: Leg[] = [];
    let deliverable = true;
    for (const row of rows) {
      if (!isBondFuture(row.terms)) continue;
      const long = row.terms.long ? row.a : row.b;
      const short = row.terms.long ? row.b : row.a;
      // F2, Money E4: a side that has CEASED cannot deliver and cannot be delivered to. The
      // layer's own resolution moves the row to a successor and closes it at its stated value.
      if (!ctx.parties.get(long).status.alive || !ctx.parties.get(short).status.alive) {
        deliverable = false;
        break;
      }
      const face = scaleQty(
        row.notional,
        row.terms.contractSize,
        'the face this contract delivers',
      );
      legs.push({ kind: 'contract', act: 'close', contract: row.id, why: 'delivered' });
      if (face <= 0) continue;
      /**
       * I1, D8 (18.1): AT THE PRICE THE ROW WAS STRUCK AT, and this is the whole of what a future
       * is for. It delivered at the deliverable's SPOT price, so a long that had locked in 98 paid
       * 100 when the bond printed 100 and its contract had bought it nothing — and nothing else
       * gave it back, because margin in this world is POSTED AND HELD (a claim redeemed when the
       * row closes, Derivative Layer C3.a) and never a settled profit. The gain was nowhere.
       *
       * It is per ROW and not per book: each side agreed its own level, and the house's two rows
       * can have been struck at different ones in different sessions.
       */
      const struck = moneyLevel(row.struckAt, 'a bond future is struck at a price');
      legs.push({
        kind: 'asset',
        from: short,
        to: long,
        instrument: t.deliverable,
        qty: ctx.registry.deliverable(face),
        pricePerUnit: some(struck),
        accruedPerUnit: none(),
      });
      legs.push({
        kind: 'money',
        from: ctx.accountOf(long, row.ccy),
        to: ctx.accountOf(short, row.ccy),
        ccy: row.ccy,
        amount: ctx.registry.cashFor(valueAt(struck, face, row.ccy, 'what the face costs')),
      });
    }
    if (!deliverable || legs.length === 0) continue;
    for (const row of rows) done.add(String(row.id));
    const r = ctx.settle({ legs, cause: 'corporateAction', reason: `${c.id} delivers` });
    ctx.record(
      'future.delivered',
      [String(c.id), String(t.deliverable)],
      {
        contract: String(c.id),
        deliverable: String(t.deliverable),
        rows: rows.length,
        face: scaleQty(c.notional, t.contractSize, 'the face this contract delivers'),
        // 18.1: what it delivered AT — the level this row struck — and the cash price on the day
        // beside it, so a reader can see what the contract was worth to whoever held it.
        at: moneyLevel(c.struckAt, 'a bond future is struck at a price'),
        spot: price.value.price,
        settled: r.outcome === 'settled',
      },
      true,
    );
  }
}

/**
 * C2: the other half of a cleared trade — the row the house holds facing the other member, on the
 * same terms at the same level. It is what makes the house flat, and it is what has to settle in
 * the same instruction as this one.
 */
function matching(ctx: MechanismContext, c: Contract): readonly Contract[] {
  const house = c.house;
  const mine = c.terms;
  if (house === null || !isBondFuture(mine)) return [c];
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
  /**
   * Register C4: THE HOUSE RECEIVES BEFORE IT DELIVERS. It is flat, so it never has the line of
   * its own — what it hands to the long member is what the short member just handed it, and legs
   * settle in the order they are written. The other order asks it to deliver units it does not
   * have yet, which is not a fact about the house's balance sheet but about the order of two lines
   * in one instruction.
   */
  const receivesFirst = (x: Contract): number => {
    const t = x.terms;
    if (!isBondFuture(t)) return 1;
    return (t.long ? x.a : x.b) === house ? 0 : 1;
  };
  return [c, other].sort((x, y) => receivesFirst(x) - receivesFirst(y));
}

export function bondFutures(house: (ccy: CurrencyCode) => PartyId, issuer: PartyId): SystemModule {
  return {
    id: 'bond-futures',
    spec: 'Sovereign I',
    requires: ['derivative-layer', 'sovereign-curve', 'money-market', 'indices'],
    instrumentKinds: [],
    derivativeKinds: [bondFutureKind],
    derivativeClasses: [bondFutureClass],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: FUTURE_CONTRACTS, name: 'future contracts', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'bondFutures.books',
        spec: 'Sovereign I1 Derivative Layer B1',
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'print', of: 'anyPeriod' }],
        writes: [],
        run: (ctx): void => {
          openBooks(ctx, house, issuer);
          // 18.4: and the ones whose date has passed stop being books.
          closeExpired(ctx);
        },
      },
      {
        name: 'bondFutures.deliver',
        spec: 'Sovereign I1 XI-5',
        // Before the layer's own resolution: what this contract does at its term is DELIVER, and a
        // cash close-out on top of a delivery would settle it twice.
        anchor: { before: 'derivatives.resolve' },
        reads: [],
        writes: [],
        run: deliver,
      },
    ],
    // Clearing A2, Law 4: A CLASS DECLARES NO PARTICIPANT OF ITS OWN. Its reasons live on its own
    // profile (`DerivativeKindProfile.orders`) and the layer that owns contract books asks for
    // them — one module speaking for one party in one book, which is the assembly half of the
    // kernel's refusal to let anybody cross themselves.
    participants: [],
    families: [],
  };
}
