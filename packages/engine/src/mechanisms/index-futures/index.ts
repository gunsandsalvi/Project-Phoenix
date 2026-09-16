/**
 * The index future: the one contract a desk can hedge a book of shares with.
 *
 * @spec Indices C3 Dealer Desks E1 Dealer Desks E2 Derivative D1 Derivative D1.b Derivative D3 Derivative D3.a Derivative D7 Derivative D7.b Derivative D8 Derivative D11 Derivative D11.a Derivative Layer B1 Derivative Layer C2 Derivative Layer D1 Equity B4 Law 3 Law 15 Law 19
 *
 * C3: THE INDEX IS A SETTLEMENT PRICE. Everywhere else an index is a read nobody trades (A2), and
 * this is the one place the read is what somebody is paid against — which is exactly why the index
 * must never be an input to its own constituents (A1.a), and why this contract settles against the
 * READ rather than against a stored level.
 *
 * E1, E2: this is what a dealer's hedge IS. A desk long a book of shares is short the market's
 * direction, and the honest way to lay that off is a contract with a named counterparty and its own
 * margin — not a coefficient that makes the position disappear from a report.
 */
import {
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  negated,
  noCash,
  type PerPiece,
  plus,
  ratioOf,
  scale,
  valueAt,
  asAmount,
  absolute,
} from '../../core/measure.js';
import { nextCycle, type Period } from '../../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { addQty, asQty, negQty, NO_QTY, type Qty } from '../../core/tick.js';
import { CENT_TICK } from '../../registry/grid.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import { moneyLevel } from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import { contractOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule, DerivativeClassDecl } from '../../world/module.js';

export const INDEX_FUTURE = derivativeKindId('index.future');
export const INDEX_CONTRACTS = unitId('indexContracts');

export const FUTURE_PARAMS = {
  life: paramId('index.future.life.periods'),
  window: paramId('index.future.margin.window'),
} as const;

export const futureMarketOf = (index: string): MarketId => marketId(`mkt.future.${index}`);
export const futureLineOf = (index: string): InstrumentId => instrumentId(`future:${index}`);

export interface IndexFutureTerms extends ContractTerms {
  readonly kind: typeof INDEX_FUTURE;
  /** D3, C3: the index it settles against — a read of prints this world clears (D3.a). */
  readonly index: string;
  readonly expiry: Period;
  /** D2: how much of the index one contract is. */
  readonly multiplier: number;
  readonly book: InstrumentId;
  readonly long: boolean;
  readonly window: number;
}

export const isIndexFuture = (t: ContractTerms): t is IndexFutureTerms =>
  'index' in t && 'multiplier' in t && 'long' in t;

/** D8, C3: what it is worth to `a` — the index read now against the level it was struck at. */
function markOf(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isIndexFuture(c.terms)) return noCash(c.ccy);
  const level = reads.index(c.terms.index);
  if (!level.some) return noCash(c.ccy);
  // E-10, Law 8: an index future is struck at an INDEX LEVEL, which this world quotes as money —
  // and `moneyLevel` is where that is asserted rather than assumed. `Contract.struckAt` used to be
  // a `PerPiece` for every kind, so a swap's rate and a future's price were the same type and a
  // reader could subtract one from the other.
  const move = minus(
    asPerPiece(level.value.level, 'the index now'),
    moneyLevel(c.struckAt, 'an index future is struck at a level'),
    'the index now against the level struck',
  );
  const worth = valueAt(
    move,
    scale(c.notional, asRatio(c.terms.multiplier, 'the multiplier'), 'per contract'),
    c.ccy,
    'of the index each',
  );
  return c.terms.long ? worth : negated(worth, 'and the other side of it');
}

export const indexFutureKind: DerivativeKindProfile = {
  id: INDEX_FUTURE,
  unit: INDEX_CONTRACTS,
  priceTick: CENT_TICK,
  // D3: a level of the index in money, times the multiplier — a price and not a rate.
  quotedAs: 'money',
  underlying: (c) => ({ kind: 'index', index: isIndexFuture(c.terms) ? c.terms.index : '' }),
  validateTerms: (t) => {
    if (!isIndexFuture(t)) throw new Error('not index future terms');
    if (!(t.multiplier > 0)) throw new Error('a future on none of the index');
  },
  displayName: (c) => (isIndexFuture(c.terms) ? `${c.terms.index} future` : String(c.id)),
  mark: markOf,
  flip: (t) => (isIndexFuture(t) ? { ...t, long: !t.long } : t),
  // D4, D11: nothing falls due before expiry; the mark moves as margin and the payoff IS the mark,
  // which the layer pays when the term runs out. A second payment here would pay it twice.
  legs: (): readonly ContractPayment[] => [],
  premiumPerUnit: (): PerPiece => asPerPiece(0, 'a future costs nothing to enter'),
  initialMargin: (c, at, reads): Option<Cash> => {
    if (!isIndexFuture(c.terms)) return none<Cash>();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none<Cash>();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.periods(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['periods']>[0],
    );
    return some(
      scale(
        valueAt(
          move.value,
          scale(c.notional, asRatio(c.terms.multiplier, 'the multiplier'), 'per contract'),
          c.ccy,
          'of the index each',
        ),
        asRatio(Math.sqrt(left > 0 ? left / horizon : 1), 'over the life it has left'),
        'over the life it has left',
      ),
    );
  },
  closeOut: markOf,
  expires: (c, at): boolean => isIndexFuture(c.terms) && at >= c.terms.expiry,
};

/** D3: why a party is long or short the index, with a future it can actually settle. */
const indexFutureClass: DerivativeClassDecl = {
  kind: INDEX_FUTURE,
  orders: futureOrders,
};

function params(): ParamDecl[] {
  return [
    {
      id: FUTURE_PARAMS.life,
      value: 13,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Indices C3: how long a contract runs before it settles against the index read. A convention of the exchange, stated with the contract.',
    },
    {
      id: FUTURE_PARAMS.window,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of the book's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
  ];
}

/**
 * E1, E2: A DESK LONG A BOOK OF SHARES SELLS THE INDEX.
 *
 * The size is what it is actually holding of the index's own constituents, valued at their own
 * prints and divided by what one contract covers — its own book, read from the register, and never
 * a ratio. What it has already hedged comes off, so a desk that is covered posts nothing.
 */
function futureOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = contractOf(m);
  if (decl === undefined || !isIndexFuture(decl.terms)) return [];
  const t = decl.terms;
  const level = view.index(t.index);
  if (!level.some || level.value.level <= 0) return [];
  // E1: the position it TOOK, at the prints its own constituents made.
  let book = noCash(m.ccy);
  for (const constituent of level.value.basket) {
    const held = view.free(constituent.instrument);
    if (held <= 0) continue;
    const print = view.print(constituent.instrument);
    if (!print.some) continue;
    book = plus(
      book,
      view.inMoney(
        valueAt(
          print.value.price,
          held,
          view.instruments.get(constituent.instrument).ccy,
          'what it holds of this line',
        ),
        m.ccy,
      ),
      'its book',
    );
  }
  // Finding E-10 again: an index LEVEL is a pure number, and what one contract covers is that
  // level in money — which is the dimension the book it hedges is in.
  const perContract = scale(
    asCash(level.value.level, m.ccy, 'the index now'),
    asRatio(t.multiplier, 'the multiplier'),
    'what one contract covers',
  );
  if (perContract.pieces <= 0) return [];
  let hedged = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isIndexFuture(c.terms) || c.terms.index !== t.index) continue;
    const iAmA = c.a === view.self.id;
    const iAmLong = iAmA === c.terms.long;
    hedged = addQty(
      hedged,
      iAmLong ? negQty(c.notional, 'the other side of it') : c.notional,
      'what it has already laid off',
    );
  }
  const want = minus(
    asAmount<'piece'>(ratioOf(book, perContract, 'contracts its book would take'), 'contracts'),
    hedged,
    'left to hedge',
  );
  const mine = asPerPiece(level.value.level, 'the index now');
  /**
   * A-66, E2, XI-13, §46 A3: AND THE OTHER SIDE, which this file did not have.
   *
   * Its docstring is "A DESK LONG A BOOK OF SHARES SELLS THE INDEX" — one true reason, and the only
   * one implemented: `side: 'sell'` was the only side in the file and `book <= 0` sent a party with
   * no shares away with nothing to say. So every session was sell-only and no index future book has
   * ever crossed. §46 A3 and XI-13 need two sides, and the second is a party with CAPITAL and no
   * book of the constituents: it makes a market around its own number, which is the index's own
   * level — a read of the constituents' prints and never of this book (A1.a: an index may not input
   * to its own constituents, and it does not here either).
   *
   * A desk that is OVER-hedged is the third case and was unreachable: `want <= 0` returned nothing,
   * so a desk short more index than its book takes could not buy any of it back.
   */
  const own = view.standsBehind();
  const room =
    own.pieces > 0
      ? view.registry.deliverable(
          ratioOf(view.inMoney(own, m.ccy), perContract, 'what its capital carries'),
        )
      : NO_QTY;
  if (want === 0) {
    if (room <= 0) return [];
    const tick = view.registry.tickForDerivative(decl.kind, m.ccy);
    const bid = minus(mine, tick, 'a tick inside its own number');
    if (bid <= 0) return [];
    return [
      { party: view.self.id, side: 'buy', price: bid, qty: asQty(room) },
      {
        party: view.self.id,
        side: 'sell',
        price: plus(mine, tick, 'a tick outside its own number'),
        qty: asQty(room),
      },
    ];
  }
  const qty = view.registry.deliverable(absolute(want, 'what it has left to do either way'));
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: want > 0 ? 'sell' : 'buy', price: mine, qty: asQty(qty) }];
}

/** C3, B1: a book per index, cleared where there is a house and bilateral where there is not. */
function openBooks(
  ctx: MechanismContext,
  house: (ccy: CurrencyCode) => PartyId,
  lines: readonly IndexLine[],
): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.periods(FUTURE_PARAMS.window);
  const life = ctx.params.periods(FUTURE_PARAMS.life);
  for (const line of lines) {
    // D3.a: an index with no level is one nothing has printed into yet, and a contract on it would
    // settle against nothing. It gets a book the period its constituents first print.
    if (!ctx.index(line.id).some) continue;
    const id = futureMarketOf(line.id);
    if (open.has(String(id))) continue;
    const clearer =
      ctx.parties.has(house(line.ccy)) && ctx.parties.get(house(line.ccy)).status.alive
        ? house(line.ccy)
        : null;
    const terms: IndexFutureTerms = {
      kind: INDEX_FUTURE,
      index: line.id,
      // A LADDER, not a new book every period (`nextCycle`).
      expiry: nextCycle(ctx.period, life),
      multiplier: 1,
      book: futureLineOf(line.id),
      long: true,
      window,
    };
    ctx.openMarket({
      id,
      name: `${line.id} future`,
      instrument: futureLineOf(line.id),
      ccy: line.ccy,
      rationing: 'proRata',
      kind: 'contract',
      contract: { kind: INDEX_FUTURE, terms, house: clearer },
    });
  }
}

/** Which index, in which money: named by whoever assembles the world, like every other list. */
export interface IndexLine {
  readonly id: string;
  readonly ccy: CurrencyCode;
}

export function indexFutures(
  house: (ccy: CurrencyCode) => PartyId,
  lines: readonly IndexLine[],
): SystemModule {
  return {
    id: 'index-futures',
    spec: 'Indices C3 Dealer Desks E1 Dealer Desks E2',
    requires: ['derivative-layer', 'indices', 'equity'],
    instrumentKinds: [],
    derivativeKinds: [indexFutureKind],
    derivativeClasses: [indexFutureClass],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: INDEX_CONTRACTS, name: 'index contracts', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'futures.books',
        spec: 'Indices C3 Derivative Layer B1',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx): void => {
          openBooks(ctx, house, lines);
        },
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

/** E2: what a desk has laid off, for the observer — a position with a counterparty, not a ratio. */
export function hedgedBy(view: ParticipantView, index: string): Qty {
  let net = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isIndexFuture(c.terms) || c.terms.index !== index) continue;
    const iAmA = c.a === view.self.id;
    net = addQty(
      net,
      iAmA === c.terms.long ? c.notional : negQty(c.notional, 'the other side of it'),
      'its position in the future',
    );
  }
  return net;
}

export type { PartyId };
