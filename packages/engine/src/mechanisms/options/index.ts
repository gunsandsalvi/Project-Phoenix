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
import {
  absolute,
  amountOf,
  type Amount,
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  negated,
  over,
  type PerPiece,
  plus,
  scale,
  valueAt,
} from '../../core/measure.js';
import { nextCycle, type Period } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../../core/ids.js';
import { derivativeKindId, instrumentId, marketId, paramId, unitId } from '../../core/ids.js';
import { atLeast, mul } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty, negQty, NO_QTY } from '../../core/tick.js';
import { CENT_TICK } from '../../registry/grid.js';
import type {
  Contract,
  ContractPayment,
  ContractMeasure,
  ContractReads,
  ContractTerms,
  DerivativeKindProfile,
} from '../../registry/derivatives.js';
import type { ParamDecl } from '../../registry/params.js';
import { contractOf, type MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule, DerivativeClassDecl } from '../../world/module.js';
import { about } from '../../world/context.js';

export const OPTION = derivativeKindId('option');
export const OPTION_CONTRACTS = unitId('optionContracts');
const OPTION_DAY_COUNT = 'ACT/365F';
/** Expectations A2: how the outlook book names a view of what a line is worth. */

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
export function intrinsic(c: Contract, at: Period, reads: ContractReads): Cash {
  const nothing = asCash(0, 'an option out of the money is worth nothing to exercise');
  if (!isOption(c.terms)) return nothing;
  const t = c.terms;
  const print = reads.print(t.underlying, at);
  if (!print.some) return nothing;
  const strike = asPerPiece(t.strike, 'the level it is struck at');
  const worth =
    t.right === 'call'
      ? minus(print.value.price, strike, 'the print above the strike')
      : minus(strike, print.value.price, 'the strike above the print');
  if (worth <= 0) return nothing;
  return valueAt(
    worth,
    scale(c.notional, asRatio(t.multiplier, 'the multiplier'), 'per contract'),
    'of the thing each',
  );
}

/**
 * D8, D8.a: what it is worth to `a` NOW — what this book says one of these costs today, over what
 * the holder has. A real gain to one side and a real loss to the other, every period.
 *
 * Before expiry it is the PREMIUM the market is printing, which is a price somebody paid. After it,
 * it is what exercise came to. Neither is a model and neither has a volatility in it.
 */
function markOf(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isOption(c.terms)) return asCash(0, 'a row that is not an option is worth nothing here');
  const t = c.terms;
  const worth = at >= t.expiry ? intrinsic(c, at, reads) : premiumNow(c, at, reads);
  return t.holds ? worth : negated(worth, 'and the other side of it');
}

function premiumNow(c: Contract, at: Period, reads: ContractReads): Cash {
  if (!isOption(c.terms)) return asCash(0, 'a row that is not an option is worth nothing here');
  const p = reads.print(c.terms.book, at);
  if (!p.some) return intrinsic(c, at, reads);
  return valueAt(p.value.price, scale(c.notional, asRatio(c.terms.multiplier, 'the multiplier'), 'per contract'), 'of the thing each');
}

export const optionKind: DerivativeKindProfile = {
  id: OPTION,
  unit: OPTION_CONTRACTS,
  priceTick: CENT_TICK,
  // D7.b: what clears is the PREMIUM, which is money a buyer hands over.
  quotedAs: 'money',
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
  premiumPerUnit: (struckAt, terms): PerPiece =>
    isOption(terms)
      ? scale(struckAt, asRatio(terms.multiplier, 'the multiplier'), 'per contract')
      : asPerPiece(0, 'not an option, so nothing is paid for it'),
  initialMargin: (c, at, reads): Option<Cash> => {
    if (!isOption(c.terms)) return none<Cash>();
    const move = reads.measuredMove(c.terms.underlying, c.terms.window);
    if (!move.some) return none<Cash>();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.periods(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['periods']>[0],
    );
    return some(
      scale(
        valueAt(move.value, scale(c.notional, asRatio(c.terms.multiplier, 'the multiplier'), 'per contract'), 'of the thing each'),
        asRatio(Math.sqrt(left > 0 ? left / horizon : 1), 'over the life it has left'),
        'over the life it has left',
      ),
    );
  },
  // D11.a: closed out at what exercise would come to, which is the stated value of the position.
  closeOut: (c, at, reads) =>
    isOption(c.terms) && c.terms.holds
      ? intrinsic(c, at, reads)
      : negated(intrinsic(c, at, reads), 'and the other side of it'),
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
  premium: PerPiece,
  multiplier: number,
  years: number,
): Option<PerPiece> {
  if (!(years > 0) || !(multiplier > 0)) return none<PerPiece>();
  return some(
    over(
      premium,
      asRatio(mul(multiplier, Math.sqrt(years), 'what it buys, over root time'), 'over root time'),
      'the move it implies',
    ),
  );
}

/**
 * D7.a: why a party writes or buys optionality, and IMPLIED VOLATILITY AS A READ — the move this
 * book's own cleared premium is paying for, taken back off the price the market made and never an
 * input to it (Law 3). A book that has not printed has no premium, so it implies nothing, which is
 * the honest answer.
 */
const optionClass: DerivativeClassDecl = {
  kind: OPTION,
  subject: (m) => {
    const t = m.contract.terms;
    return isOption(t) ? String(t.underlying) : '';
  },
  /**
   * Law 18, D7.a: THE LINES THIS PARTY HAS A VIEW OF, which is the whole of why it would be in an
   * option book at all. What optionality is worth to it is its own outlook's confidence about the
   * underlying (`optionOrders`), so a party with no outlook of a line prices no option on it —
   * every path below `mine <= 0` returns nothing.
   *
   * It is the same read `orders` makes, off the same outlook book, so the books this names and the
   * books it posts in cannot disagree (Law 4, Law 19).
   */
  reasons: (view) => {
    // §46 A2: the lines this desk has a view on, asked as SUBJECTS. It used to recover them from
    // the key by `startsWith('price.')` and `slice`, which is a fact taken back out of a string.
    const out: string[] = [];
    for (const s of view.outlookSubjects()) {
      if (s.on === 'price') out.push(String(s.instrument));
    }
    return out;
  },
  orders: optionOrders,
  measures: (m, reads): readonly ContractMeasure[] => {
    const t = m.contract.terms;
    if (!isOption(t)) return [];
    const premium = reads.prices.latest(t.book, reads.period);
    if (!premium.some) return [];
    const move = impliedMove(
      premium.value.price,
      t.multiplier,
      yearFraction(
        OPTION_DAY_COUNT,
        reads.calendar.startOf(reads.period),
        reads.calendar.startOf(t.expiry),
      ),
    );
    if (!move.some) return [];
    return [
      {
        subject: String(t.underlying),
        measure: 'the move its premium pays for',
        tenorYears: null,
        level: move.value,
        unit: 'money',
      },
    ];
  },
};

function params(): ParamDecl[] {
  return [
    {
      id: OPTION_PARAMS.life,
      value: 13,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'The ladder a market is opened on: how long the contracts an exchange lists run for. A convention of the exchange, stated with the contract.',
    },
    {
      id: OPTION_PARAMS.window,
      value: 8,
      unit: 'periods',
      dimension: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of the underlying's own record the initial margin is measured over. A resolution: the answer must not turn on it.",
    },
    {
      id: OPTION_PARAMS.aversion,
      value: 0.1,
      unit: 'of its own capital it will not risk',
      dimension: 'ratio',
      kind: 'preference',
      owner: 'model',
      why: 'How much of a surplus a holder’s own management insists on covering rather than carrying. It is a PREFERENCE — what somebody is willing to live with — and it is why one party buys cover at a price another will not.',
    },
  ];
}

/**
 * Expectations A3, XI-13, D7, §46's second dimension: WHAT THIS PARTY THINKS OPTIONALITY ON THIS
 * CONTRACT IS WORTH, per unit of the underlying, and it is never read off this book.
 *
 * Two terms, and the contract's own terms decide both:
 *
 * **Where exercise stands at the price it expects** (D11) — `expected − strike` for a call and the
 * other way for a put. It is the same distance `intrinsic` takes at a PRINT, taken here at this
 * party's own outlook, so a strike far above where it thinks the line is going is worth less to it
 * than one at the money, and the same party quotes a call and a put on one line differently.
 *
 * **What it thinks the thing MOVES before this expires** — its own outlook's CONFIDENCE, which is
 * the width of its own surprises about the underlying in that underlying's own money, over the root
 * of the time the contract runs. That is `impliedMove` read backwards: what that function takes off
 * a printed premium is what this one puts into a quoted one (Law 4), and the expiry enters here.
 *
 * A-65: this used to be `expected × confidence` — a LEVEL times a WIDTH, which is money² per unit²,
 * posted as a price. The comment above it already said the right term ("its own outlook's
 * CONFIDENCE and not its level") and the code multiplied by the level anyway, so every premium in
 * this world was proportional to the SQUARE of the underlying's price: an option on a line at 100
 * with 1% surprises quoted 100, the whole value of the underlying, and the same 1% on a line at 1
 * quoted 0.01. Neither the strike, nor the right, nor the expiry appeared in it at all, so one
 * party priced every book on a line identically and the strike ladder was a set of books nobody
 * could tell apart.
 *
 * Zero where it has no outlook, where the contract has already expired, or where the distance is
 * further below zero than the move it expects — that last is a DECISION and not a floor (Law 6): a
 * strike it cannot see the price reaching is one it will not pay for, the same answer `intrinsic`
 * gives when nobody exercises.
 */
function worthToIt(view: ParticipantView, t: OptionTerms): PerPiece {
  const nothing = asPerPiece(0, 'it has no view of what optionality on this is worth');
  const outlook = view.outlook(about({ on: 'price', instrument: t.underlying }));
  if (!outlook.some) return nothing;
  const years = yearFraction(
    OPTION_DAY_COUNT,
    view.calendar.startOf(view.period),
    view.calendar.startOf(t.expiry),
  );
  if (!(years > 0)) return nothing;
  const expected = asPerPiece(outlook.value.expected, 'where it thinks the underlying goes');
  const strike = asPerPiece(t.strike, 'the level it is struck at');
  const distance =
    t.right === 'call'
      ? minus(expected, strike, 'the price it expects, above the strike')
      : minus(strike, expected, 'the strike, above the price it expects');
  const width = scale(
    asPerPiece(outlook.value.confidence, 'how wide its own surprises about the line have been'),
    asRatio(Math.sqrt(years), 'over the root of the time this runs'),
    'what it thinks the thing moves before this expires',
  );
  const worth = plus(distance, width, 'what optionality on this contract is worth to it');
  return worth > 0 ? worth : nothing;
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
  const decl = contractOf(m);
  if (decl === undefined || !isOption(decl.terms)) return [];
  const t = decl.terms;
  const print = view.print(t.underlying);
  if (!print.some) return [];
  const aversion = view.params.ratio(OPTION_PARAMS.aversion);
  /**
   * Expectations A3, XI-13, §46's second dimension: WHAT THIS PARTY THINKS OPTIONALITY IS WORTH,
   * and it is never read off this book.
   *
   * What it thinks the thing MOVES is its own outlook's CONFIDENCE and not its level: an outlook
   * carries how wide that party's own surprises about the UNDERLYING have been, which is a view of
   * dispersion formed the same adaptive way as the view of the level, and it is exactly what an
   * option is a price of. Two parties whose prices agree and whose surprises differ disagree about
   * what optionality is worth, which is the disagreement this instrument exists to clear. On top of
   * it is what this party's own capital wants for carrying the position.
   *
   * THE FALLBACK USED TO BE ONE TICK, and it meant the book could never open. With no print the
   * level was the smallest increment the kind quotes in, so every party's own number was above it,
   * every party took the writer's side, and the session was all offers and no bids: `noDemand`, no
   * print, and next period the same tick again. It was also a declared number standing in for a
   * mechanism, written in code rather than in the parameter register (Law 2). There is no fallback
   * now because there is nothing to fall back FROM: the party's own arithmetic is the level, and a
   * party with no view of how much the underlying moves has no view of what optionality on it is
   * worth — which is a real answer (D4) and not a gap to fill with somebody else's number.
   */
  const moves = worthToIt(view, t);
  const mine =
    moves > 0
      ? plus(moves, scale(moves, aversion, 'what its capital wants'), 'its quote')
      : asPerPiece(0, 'it has no view, so it has no level');
  // A party with no view of how much the underlying moves has no view of what optionality on it is
  // worth, and that is the end of it — so nothing below is worked out for one. Law 18: every party
  // in the world is asked about every book, and its own book and its own equity were both walked
  // before this line to arrive at the same nothing.
  if (mine <= 0) return [];
  let covered: Amount<'piece'> = NO_QTY;
  for (const c of view.contracts.mine()) {
    if (!isOption(c.terms) || c.terms.underlying !== t.underlying || c.terms.right !== t.right) continue;
    const iAmA = c.a === view.self.id;
    covered = plus(
      covered,
      iAmA === c.terms.holds ? c.notional : negQty(c.notional, 'the other side of it'),
      'cover it has',
    );
  }
  /** Clearing E1: where the market is — the comparator that decides the side, never the level. */
  const at = view.print(t.book);
  /**
   * ONE PARTY, ONE POSITION (Clearing A2). Demand with a reason: a holder of the line wants a PUT
   * over the part of its book its own management will not carry — its own holding and its own
   * preference, never a hedge ratio. Supply with a reason: a party with capital and a view on how
   * much the thing MOVES writes out of the same balance sheet it runs everything else on.
   */
  const held = view.free(t.underlying);
  let want: Amount<'piece'> =
    t.right === 'put' && held > 0
      ? over(
          scale(held, aversion, 'the part it will not carry'),
          asRatio(t.multiplier, 'the multiplier'),
          'in contracts',
        )
      : NO_QTY;
  /**
   * Law 8, A-65: WHAT A CONTRACT COSTS, because that is what this book quotes. The unit of the
   * venue is `optionContracts` and `impliedMove` divides a printed premium BY the multiplier to get
   * back to money per unit of the underlying — so the print is per contract and this posted money
   * per unit of the underlying, out by the multiplier, into the same book the read comes off.
   */
  const price = scale(mine, asRatio(t.multiplier, 'the multiplier'), 'per contract');
  // What it can write comes off its own balance sheet, and that is walked only where the comparison
  // it feeds actually happens: a book that has not printed has nothing for its number to be above.
  const own = at.some ? view.equity() : asCash(0, 'no book to stand its capital against');
  if (at.some && own > 0) {
    const book = at.value.price;
    const room = view.registry.deliverable(amountOf(own, price, 'what it can write'));
    if (price > book) {
      // It thinks these are dear against what the market last paid: it would rather be the writer.
      want = minus(want, room, 'and what it would write at its own price');
    } else if (price < book) {
      want = plus(want, room, 'and what it would buy at its own price');
    }
  }
  const move = minus(want, covered, 'from the cover it has to the cover it wants');
  if (move === 0) return [];
  const qty = view.registry.deliverable(absolute(move, 'the size of the move'));
  if (qty <= 0) return [];
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
  const window = ctx.params.periods(OPTION_PARAMS.window);
  const life = ctx.params.periods(OPTION_PARAMS.life);
  for (const underlying of lines) {
    if (!ctx.instruments.has(underlying)) continue;
    const i = ctx.instruments.get(underlying);
    const market = ctx.markets.find((m) => m.instrument === underlying && contractOf(m) === undefined);
    if (market === undefined) continue;
    // A LADDER, not a new book every period: everything written between two dates on the cycle
    // settles on the same one, into the same book (`nextCycle`).
    const expiry = nextCycle(ctx.period, life);
    // AND THE STRIKE IS SET WHEN THE BOOK OPENS, from what the line was worth then. An exchange
    // lists strikes around the price and leaves them there; one recomputed every period would put
    // a new book on the ladder every week and leave the last with one trade in it.
    const opened = expiry - life;
    const print = ctx.prices.latest(underlying, atLeast(opened, 0, 'there is no period before the world began') as Period);
    if (!print.some) continue;
    const strike = ctx.registry.onQuoteGrid(i.kind, i.ccy, print.value.price);
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
    derivativeClasses: [optionClass],
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
