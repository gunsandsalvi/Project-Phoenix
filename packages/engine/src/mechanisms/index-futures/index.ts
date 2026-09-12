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
import { nextCycle, type Period } from '../../calendar/calendar.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId, UnitId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { add, div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty } from '../../core/tick.js';
import { CENT_TICK } from '../../registry/grid.js';
import type {
  Contract,
  ContractPayment,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

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
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isIndexFuture(c.terms)) return 0;
  const level = reads.index(c.terms.index);
  if (!level.some) return 0;
  const move = sub(level.value.level, c.struckAt, 'the index now against the level struck');
  const worth = mul(mul(move, c.notional, 'per contract'), c.terms.multiplier, 'of the index each');
  return c.terms.long ? worth : -worth;
}

export const indexFutureKind: DerivativeKindProfile = {
  id: INDEX_FUTURE,
  unit: INDEX_CONTRACTS,
  priceTick: CENT_TICK,
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
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isIndexFuture(c.terms)) return none();
    const move = reads.measuredMove(c.terms.book, c.terms.window);
    if (!move.some) return none();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.get(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['get']>[0],
    );
    return some(
      mul(
        mul(mul(move.value, c.notional, 'per contract'), c.terms.multiplier, 'of the index each'),
        Math.sqrt(left > 0 ? left / horizon : 1),
        'over the life it has left',
      ),
    );
  },
  orders: futureOrders,
  closeOut: markOf,
  expires: (c, at): boolean => isIndexFuture(c.terms) && at >= c.terms.expiry,
};

function params(): ParamDecl[] {
  return [
    {
      id: FUTURE_PARAMS.life,
      value: 13,
      unit: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Indices C3: how long a contract runs before it settles against the index read. A convention of the exchange, stated with the contract.',
    },
    {
      id: FUTURE_PARAMS.window,
      value: 8,
      unit: 'periods',
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
  const decl = m.contract;
  if (decl === undefined || !isIndexFuture(decl.terms)) return [];
  const t = decl.terms;
  const level = view.index(t.index);
  if (!level.some || level.value.level <= 0) return [];
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  // E1: the position it TOOK, at the prints its own constituents made.
  let book = 0;
  for (const constituent of level.value.basket) {
    const held = view.free(constituent.instrument);
    if (held <= 0) continue;
    const print = view.print(constituent.instrument);
    if (!print.some) continue;
    book = add(book, mul(held, print.value.price, 'what it holds of this line'), 'its book');
  }
  if (book <= 0) return [];
  const perContract = mul(level.value.level, t.multiplier, 'what one contract covers');
  if (perContract <= 0) return [];
  let hedged = 0;
  for (const c of view.contracts.mine()) {
    if (!isIndexFuture(c.terms) || c.terms.index !== t.index) continue;
    const iAmA = c.a === view.self.id;
    const iAmLong = iAmA === c.terms.long;
    hedged = add(hedged, iAmLong ? -c.notional : c.notional, 'what it has already laid off');
  }
  const want = sub(div(book, perContract, 'contracts its book would take'), hedged, 'left to hedge');
  if (want <= 0) return [];
  const qty = view.registry.deliverable(unit, want);
  if (qty <= 0) return [];
  return [{ party: view.self.id, side: 'sell', price: level.value.level, qty: asQty(qty) }];
}

/** C3, B1: a book per index, cleared where there is a house and bilateral where there is not. */
function openBooks(
  ctx: MechanismContext,
  house: (ccy: CurrencyCode) => PartyId,
  lines: readonly IndexLine[],
): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.get(FUTURE_PARAMS.window);
  const life = ctx.params.get(FUTURE_PARAMS.life);
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
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: INDEX_CONTRACTS, name: 'index contracts', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'futures.books',
        spec: 'Indices C3 Derivative Layer B1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
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
export function hedgedBy(view: ParticipantView, index: string): number {
  let net = 0;
  for (const c of view.contracts.mine()) {
    if (!isIndexFuture(c.terms) || c.terms.index !== index) continue;
    const iAmA = c.a === view.self.id;
    net = add(net, iAmA === c.terms.long ? c.notional : -c.notional, 'its position in the future');
  }
  return net;
}

export type { PartyId };
