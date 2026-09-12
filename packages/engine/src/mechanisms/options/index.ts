/**
 * Options: the price of optionality, cleared on its own book.
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative D2 Derivative D3 Derivative D3.a Derivative D4 Derivative D5 Derivative D7 Derivative D7.a Derivative D7.b Derivative D8 Derivative D8.a Derivative D9 Derivative D10 Derivative D11 Derivative D11.a Derivative D12 Derivative Layer B1 Derivative Layer C2 Derivative Layer D1 Derivative Layer E1 Derivative Layer E2 Derivative Layer E3 Derivative Layer E4 Bond N7.b Bond N11.a Short-Term Debt B4 Expectations A1 Expectations A2 Expectations A3 Law 3 Law 15 Law 19
 *
 * THE PREMIUM IS WHAT CLEARS (D7, D7.a, Law 3). A market per underlying, right, strike and expiry;
 * the print is what a buyer paid a writer for one unit of the thing. Clearing a VOLATILITY and
 * deriving the premium from it is the shape Bond N7.b forbids for a bond — so implied volatility
 * here is the read taken back OFF the premium, the way a spread is taken off a bond price, and
 * there is no parameter, store or field named for one anywhere in this module.
 *
 * WHY IT IS HERE AT ALL. Three places in this specification need the price of optionality and none
 * of them can form one: an early-termination regime an issuer PAYS for (Bond N11.a), prepayment at
 * a borrower's option, and a committed line whose undrawn headroom is otherwise "a free option the
 * lender did not sell" (Short-Term Debt B4). And Expectations A3 needs it for a different reason: parties
 * can disagree about a LEVEL and nothing lets them disagree about DISPERSION, so a world without an
 * option market has assumed every party has the same opinion of how much things move.
 *
 * EXERCISE IS A DECISION, NOT A CLAMP (Law 6). A holder exercises when exercising pays and does not
 * when it does not; what comes out is never "the payoff, floored at zero" — it is the payoff of the
 * choice it made, and the choice is the mechanism.
 */
import type { Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
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

export const OPTION = derivativeKindId('option');
export const OPTION_CONTRACTS = unitId('optionContracts');
const OPTION_DAY_COUNT = 'ACT/365F';

export const OPTION_PARAMS = {
  life: paramId('option.life.periods'),
  window: paramId('option.margin.window'),
  aversion: paramId('party.riskAversion'),
} as const;

export const optionMarketOf = (
  underlying: InstrumentId,
  right: string,
  strike: number,
  expiry: number,
): MarketId => marketId(`mkt.option.${underlying}.${right}.${strike}.${expiry}`);

export const optionLineOf = (
  underlying: InstrumentId,
  right: string,
  strike: number,
  expiry: number,
): InstrumentId => instrumentId(`option:${underlying}:${right}:${strike}:${expiry}`);

export interface OptionTerms extends ContractTerms {
  readonly kind: typeof OPTION;
  /** D3, D3.a: a line this world clears. An option on something that exists only here is forbidden. */
  readonly underlying: InstrumentId;
  readonly market: MarketId;
  readonly right: 'call' | 'put';
  readonly strike: number;
  readonly expiry: Period;
  readonly multiplier: number;
  /** Where the premium prints — this book's own line (D7: the premium is what clears). */
  readonly book: InstrumentId;
  /** D1: which side `a` is. The book writes the buyer as `a`; `flip` writes the writer. */
  readonly holds: boolean;
  readonly window: number;
}

export const isOption = (t: ContractTerms): t is OptionTerms =>
  'right' in t && 'strike' in t && 'holds' in t;

/**
 * D11, D11.a: WHAT EXERCISE PAYS — the decision, taken, and what it is worth.
 *
 * A call is exercised when the print is above the strike and a put when it is below, and the payoff
 * is the distance. Nothing is floored: an option nobody exercises pays nothing because nobody
 * exercised it (Law 6), which is a different statement from a number clamped at zero.
 */
export function intrinsic(c: Contract, at: Period, reads: ContractReads): number {
  if (!isOption(c.terms)) return 0;
  const t = c.terms;
  const print = reads.print(t.underlying, at);
  if (!print.some) return 0;
  const worth = t.right === 'call'
    ? sub(print.value.price, t.strike, 'the print above the strike')
    : sub(t.strike, print.value.price, 'the strike above the print');
  if (worth <= 0) return 0;
  return mul(mul(worth, c.notional, 'per contract'), t.multiplier, 'of the thing each');
}

/**
 * D8, D8.a: what it is worth to `a` NOW — what this book says one of these costs today, over what
 * the holder has. A real gain to one side and a real loss to the other, every period.
 *
 * Before expiry it is the PREMIUM the market is printing, which is a price somebody paid. After it,
 * it is what exercise came to. Neither is a model and neither has a volatility in it.
 */
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isOption(c.terms)) return 0;
  const t = c.terms;
  const worth = at >= t.expiry ? intrinsic(c, at, reads) : premiumNow(c, at, reads);
  return t.holds ? worth : -worth;
}

function premiumNow(c: Contract, at: Period, reads: ContractReads): number {
  if (!isOption(c.terms)) return 0;
  const p = reads.print(c.terms.book, at);
  if (!p.some) return intrinsic(c, at, reads);
  return mul(mul(p.value.price, c.notional, 'per contract'), c.terms.multiplier, 'of the thing each');
}

export const optionKind: DerivativeKindProfile = {
  id: OPTION,
  unit: OPTION_CONTRACTS,
  priceTick: CENT_TICK,
  underlying: (c) => ({
    kind: 'print',
    market: isOption(c.terms) ? c.terms.market : ('' as MarketId),
    instrument: isOption(c.terms) ? c.terms.underlying : ('' as InstrumentId),
  }),
  validateTerms: (t) => {
    if (!isOption(t)) throw new Error('not option terms');
    if (!(t.strike > 0)) throw new Error(`an option struck at ${t.strike}`);
    if (!(t.multiplier > 0)) throw new Error('an option on none of the thing');
  },
  displayName: (c) =>
    isOption(c.terms)
      ? `${c.terms.underlying} ${c.terms.right} ${c.terms.strike}`
      : String(c.id),
  mark: markOf,
  flip: (t) => (isOption(t) ? { ...t, holds: !t.holds } : t),
  // D2: the premium moves in `premiumPerUnit` at inception, in the same instruction as the row.
  // A second payment here would charge it twice; what moves after that is the margin (D9).
  legs: (): readonly ContractPayment[] => [],
  /**
   * D7, D7.a: THE CLEARED PRICE IS THE PREMIUM. Unlike every par-struck contract in this tree, an
   * option is BOUGHT: what the session agreed is what the holder pays the writer, per unit, now.
   */
  premiumPerUnit: (struckAt, terms) => (isOption(terms) ? mul(struckAt, terms.multiplier, 'per contract') : 0),
  initialMargin: (c, at, reads): Option<number> => {
    if (!isOption(c.terms)) return none();
    const move = reads.measuredMove(c.terms.underlying, c.terms.window);
    if (!move.some) return none();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.get(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['get']>[0],
    );
    return some(
      mul(
        mul(mul(move.value, c.notional, 'per contract'), c.terms.multiplier, 'of the thing each'),
        Math.sqrt(left > 0 ? left / horizon : 1),
        'over the life it has left',
      ),
    );
  },
  // D11.a: closed out at what exercise would come to, which is the stated value of the position.
  orders: optionOrders,
  closeOut: (c, at, reads) => (isOption(c.terms) && c.terms.holds ? intrinsic(c, at, reads) : -intrinsic(c, at, reads)),
  expires: (c, at): boolean => isOption(c.terms) && at >= c.terms.expiry,
};

/**
 * THE READ TAKEN OFF THE PREMIUM (Law 3, Bond N7.b): what move in the underlying this premium is
 * paying for, per unit, per root year.
 *
 * It is derived at the read like a yield off a bond price and stored nowhere. It is not a
 * Black-Scholes sigma and does not pretend to be: it is the premium, divided by what it buys, over
 * the root of the time it buys it for — the plainest statement of "how much movement is this price
 * paying for", and the only one that does not import a model this world has no business having.
 */
export function impliedMove(
  premium: number,
  multiplier: number,
  years: number,
): Option<number> {
  if (!(years > 0) || !(multiplier > 0)) return none<number>();
  return some(div(premium, mul(multiplier, Math.sqrt(years), 'what it buys, over root time'), 'the move it implies'));
}

function params(): ParamDecl[] {
  return [
    {
      id: OPTION_PARAMS.life,
      value: 13,
      unit: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'The ladder a market is opened on: how long the contracts an exchange lists run for. A convention of the exchange, stated with the contract.',
    },
    {
      id: OPTION_PARAMS.window,
      value: 8,
      unit: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of the underlying's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
    {
      id: OPTION_PARAMS.aversion,
      value: 0.1,
      unit: 'of its own capital it will not risk',
      kind: 'preference',
      owner: 'model',
      why: 'How much of a surplus a holder’s own management insists on covering rather than carrying. It is a PREFERENCE — what somebody is willing to live with — and it is why one party buys cover at a price another will not.',
    },
  ];
}

/**
 * Demand with a reason and never a hedge ratio: A HOLDER COVERS WHAT ITS OWN SURPLUS MUST ABSORB.
 *
 * What it holds of the line, at its own print, times what its management refuses to have at risk —
 * that is the size, and it is its own book and its own preference and nothing else. Cover it
 * already has comes off. Supply with a reason: a desk writes out of the balance sheet it runs
 * everything else on, and quotes the move it EXPECTS plus what its capital wants for the position.
 */
function optionOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const decl = m.contract;
  if (decl === undefined || !isOption(decl.terms)) return [];
  const t = decl.terms;
  const print = view.print(t.underlying);
  if (!print.some) return [];
  const unit: UnitId = view.registry.derivativeKind(decl.kind).unit;
  const aversion = view.params.get(OPTION_PARAMS.aversion);
  let covered = 0;
  for (const c of view.contracts.mine()) {
    if (!isOption(c.terms) || c.terms.underlying !== t.underlying || c.terms.right !== t.right) continue;
    const iAmA = c.a === view.self.id;
    covered = add(covered, iAmA === c.terms.holds ? c.notional : -c.notional, 'cover it has');
  }
  const last = view.print(t.book);
  const level = last.some ? last.value.price : view.registry.tickForDerivative(decl.kind, m.ccy);
  /**
   * ONE PARTY, ONE POSITION (Clearing A2). Demand with a reason: a holder of the line wants a PUT
   * over the part of its book its own management will not carry — its own holding and its own
   * preference, never a hedge ratio. Supply with a reason: a party with capital and a view on how
   * much the thing MOVES writes out of the same balance sheet it runs everything else on, at what
   * it expects plus what its capital wants for the position.
   */
  const held = view.free(t.underlying);
  let want = t.right === 'put' && held > 0
    ? div(mul(held, aversion, 'the part it will not carry'), t.multiplier, 'in contracts')
    : 0;
  let price = level;
  const outlook = view.outlook(`move.${String(t.underlying)}`);
  const own = view.equity();
  if (outlook.some && own > 0) {
    const asks = add(
      outlook.value.expected,
      mul(outlook.value.expected, aversion, 'what its capital wants'),
      'its quote',
    );
    if (asks > 0) {
      const room = view.registry.deliverable(
        unit,
        div(own, mul(asks, t.multiplier, 'per contract'), 'what it can write'),
      );
      if (asks > level) {
        // It thinks these are dear: it would rather be the writer.
        want = sub(want, room, 'and what it would write at its own price');
        price = asks;
      } else if (asks < level) {
        want = add(want, room, 'and what it would buy at its own price');
        price = asks;
      }
    }
  }
  const move = sub(want, covered, 'from the cover it has to the cover it wants');
  if (move === 0) return [];
  const qty = view.registry.deliverable(unit, move > 0 ? move : -move);
  if (qty <= 0 || price <= 0) return [];
  return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price, qty: asQty(qty) }];
}

/**
 * D3.a, E2: a ladder of books on lines this world already clears, at strikes around what they
 * print. The strikes are where the market is, which is a fact about the underlying and not a
 * number anybody chose.
 */
function openBooks(
  ctx: MechanismContext,
  house: (ccy: CurrencyCode) => PartyId,
  lines: readonly InstrumentId[],
): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.get(OPTION_PARAMS.window);
  const life = ctx.params.get(OPTION_PARAMS.life);
  for (const underlying of lines) {
    if (!ctx.instruments.has(underlying)) continue;
    const i = ctx.instruments.get(underlying);
    const print = ctx.prices.latest(underlying, ctx.period);
    if (!print.some) continue;
    const market = ctx.markets.find((m) => m.instrument === underlying && m.contract === undefined);
    if (market === undefined) continue;
    const strike = ctx.registry.onQuoteGrid(i.kind, i.ccy, print.value.price);
    const expiry = (ctx.period + life) as Period;
    const clearer =
      ctx.parties.has(house(i.ccy)) && ctx.parties.get(house(i.ccy)).status.alive ? house(i.ccy) : null;
    for (const right of ['call', 'put'] as const) {
      const id = optionMarketOf(underlying, right, strike, expiry);
      if (open.has(String(id))) continue;
      const terms: OptionTerms = {
        kind: OPTION,
        underlying,
        market: market.id,
        right,
        strike,
        expiry,
        multiplier: 1,
        book: optionLineOf(underlying, right, strike, expiry),
        holds: true,
        window,
      };
      ctx.openMarket({
        id,
        name: `${String(underlying)} ${right} ${strike}`,
        instrument: optionLineOf(underlying, right, strike, expiry),
        ccy: i.ccy,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: OPTION, terms, house: clearer },
      });
    }
  }
}

/** Which lines carry an option ladder: named by whoever assembles the world (Law 4: one list). */
export type OptionUniverse = (ctx: MechanismContext) => readonly InstrumentId[];

export function options(
  house: (ccy: CurrencyCode) => PartyId,
  universe: OptionUniverse,
): SystemModule {
  return {
    id: 'options',
    spec: 'Derivative D7 Derivative D7.a Bond N11.a Short-Term Debt B4 Expectations A3',
    requires: ['derivative-layer', 'equity', 'indices'],
    instrumentKinds: [],
    derivativeKinds: [optionKind],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: OPTION_CONTRACTS, name: 'option contracts', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'options.books',
        spec: 'Derivative D7 Derivative D3.a Derivative Layer B1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          openBooks(ctx, house, universe(ctx));
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

/** The years an option has left, for a reader taking the implied move off its premium. */
export const lifeInYears = (c: Contract, at: Period, reads: ContractReads): number =>
  isOption(c.terms) && c.terms.expiry > at
    ? yearFraction(OPTION_DAY_COUNT, reads.calendar.startOf(at), reads.calendar.startOf(c.terms.expiry))
    : 0;

export type { PartyId };
