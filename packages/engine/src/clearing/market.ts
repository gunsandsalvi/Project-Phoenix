/**
 * A market: a venue where one instrument clears at a stated point in the period, and the phase that
 * runs every market in order.
 *
 * @spec Bond N7 Bond N7.a Bond N9 Bond N9.a Clearing A1 Clearing B1 Clearing B2 Clearing C4.a Clearing C4.b Clearing D2 Clearing D3 Clearing D4 Clearing E1 Clearing E4 Clearing F1 Clearing F2 Register C3 Register C3.a XI-5 Goods C2 Sovereign C1 Sovereign C1.b Sovereign C2 Sovereign C4 Sovereign C5 Sovereign C6 Sovereign C7 Sovereign B3.a Bond N9.b Treasury D2.a Treasury D5.a
 *
 * Trades become instructions with the paper one way and the cash the other, settled together (D3,
 * XI-5). The print is written once per period (F2): traded if anything cleared, otherwise the last
 * print carried and marked stale with the reason it did not clear (C4.b, E4). A market whose
 * instrument has never printed and did not clear writes nothing: it has no price, which is what
 * Unpriced means at the reading site (XI-6).
 *
 * A PRIMARY OFFER (Sovereign C) is the issuer's own supply for one session: a size and a walk-away
 * level, posted by the issuer's module before the session and cleared by this same solver at one
 * uniform price (C2). It is not a second mechanism and not a second print: a new line and a
 * re-opening of an old one (B3.a) are the same act with the same book. Nobody absorbs what is not
 * bid for (Treasury D5.a): the unsold remainder is withdrawn and the withdrawal is an event (C7).
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { assertNever, forbid } from '../core/assert.js';
import { type CurrencyCode, type InstrumentId, type MarketId, type PartyId, type UnitId } from '../core/ids.js';
import {
  add,
  atMost,
  div,
  finite,
  mul,
  sub,
  sum,
  zeroIfNone,
} from '../core/num.js';
import type { Qty } from '../core/tick.js';
import { none, type Option, some } from '../core/option.js';
import { asQty, commonGrain, downTick, downToGrain, downToTick, toGrain, upToTick } from '../core/tick.js';
import type { Journal } from '../journal/journal.js';
import type { AccountRef, InstructionDraft, Leg } from '../ledger/instruction.js';
import { cellSide, type Settlement } from '../ledger/settlement.js';
import { weightOf, type Parties } from '../parties/party.js';
import { struckIn, type PriceStore, type StaleReason } from '../prices/price-store.js';
import type { Registry } from '../registry/registry.js';
import { clear, type Fill, type Order, type Rationing } from './solver.js';
import type { DerivativeKindId } from '../core/ids.js';
import type { ContractTerms, DerivativeKindProfile } from '../registry/derivatives.js';

/**
 * Spot FX A1, C1; Law 15: WHAT KIND OF THING THIS MARKET MOVES, as a dispatch key and never a
 * branch. An `asset` market moves an instrument against money, which is every market this world had
 * until now. An `fx` market moves MONEY AGAINST MONEY: a spot trade is two money legs in two
 * currencies at a rate (A1), so there is no instrument to deliver and no issuer to ask about.
 *
 * A `contract` market strikes a DERIVATIVE: two named parties agree terms at the level the session
 * cleared (Derivative Layer B1) and what comes of it is a row in the contract store, not a delivery
 * — nobody hands anything over, and what moves in cash is the premium if the kind has one and the
 * margin both sides put up (D9).
 *
 * What the three share is everything else — one solver, one book, one print, one rationing rule —
 * and what differs is the instruction a fill becomes. That difference lives in one table
 * (`MARKET_KINDS`) so a fourth kind is a row rather than an `if`.
 */
export type MarketKind = 'asset' | 'fx' | 'contract';

interface MarketBase {
  readonly id: MarketId;
  readonly name: string;
  /**
   * What the print is ABOUT. For an asset market, the instrument that changes hands. For a pair, the
   * pair itself (`fxPairId`) — an id with no instrument behind it, because nobody holds a pair: what
   * it names is the subject of the price, and a rate is a price (Law 3).
   */
  readonly instrument: InstrumentId;
  /** What the price is IN: the quote currency of a pair, the money an asset is paid for in. */
  readonly ccy: CurrencyCode;
  readonly rationing: Rationing;
  /**
   * Spot FX F1.a, Clearing F1: WHERE IN THE PERIOD THIS MARKET RUNS. Markets clear in ascending
   * order, so a payer short of a money can buy it before the market that needs it — which is F1.a's
   * "never inside the trade": the conversion is its own session with its own counterparty, and the
   * goods market that follows finds the money already there. Absent is last, which is where every
   * market that does not care sits.
   */
  readonly order?: number;
}

/** An instrument against money: what a market is, and what every market here was until FX. */
export interface AssetMarketDecl extends MarketBase {
  readonly kind?: 'asset';
}

/**
 * Spot FX A1, A3: the two moneys. The price is what one unit of `base` costs in `quote`, so a BUY
 * takes base and gives quote — which is the same convention a market uses.
 */
export interface CurrencyPair {
  readonly base: CurrencyCode;
  readonly quote: CurrencyCode;
}

export interface FxMarketDecl extends MarketBase {
  readonly kind: 'fx';
  readonly fx: CurrencyPair;
}

/**
 * Derivative D12, Derivative Layer B1: what a fill in THIS book becomes. A book is one contract
 * shape — one underlying, one term, one strike (D12) — so the terms are the book's and what varies
 * per trade is who, how much, and the level it cleared at. The BUYER is `a`, which is the side the
 * terms are stated from and the mark is written for (A3).
 */
export interface ContractBook {
  readonly kind: DerivativeKindId;
  readonly terms: ContractTerms;
  /** C2: the central counterparty both sides face here, or null for a bilateral book. */
  readonly house: PartyId | null;
}

export interface ContractMarketDecl extends MarketBase {
  readonly kind: 'contract';
  readonly contract: ContractBook;
}

/**
 * Law 15: THREE KINDS OF MARKET AS THREE SHAPES, discriminated on `kind`. It was one shape with
 * three optional bags (`kind?`, `contract?`, `fx?`), so a contract book naming no contract and a
 * pair market naming no pair were both states the type allowed and two runtime `throw`s forbade.
 * A state the type can forbid is not a state to check for.
 */
export type MarketDecl = AssetMarketDecl | FxMarketDecl | ContractMarketDecl;

/** An asset market need not say what it is, because it is what a market is (Clearing A1). */
export function kindOf(m: MarketDecl): MarketKind {
  return m.kind ?? 'asset';
}

/**
 * Derivative Layer B1, Law 19: THE CONTRACT THIS BOOK STRIKES, or nothing because this market is
 * not a contract book. A module walking every market asks what KIND a market is, which is the
 * question it means; it used to test a field for absence and infer the kind from that.
 */
export function contractOf(m: MarketDecl | undefined): ContractBook | undefined {
  return m?.kind === 'contract' ? m.contract : undefined;
}

/**
 * Derivative Layer B1: THE CONTRACT BOOK THIS MARKET IS, or nothing because it is not one. The same
 * question as `contractOf` with the market kept, for a caller that has to hand the market on to
 * something only a contract book has — and it is here, in the kernel, so that no module writes the
 * discriminant test itself (Law 15: `phoenix/no-kind-branch` refuses it there, and is right to).
 */
export function asContractMarket(m: MarketDecl | undefined): ContractMarketDecl | undefined {
  return m?.kind === 'contract' ? m : undefined;
}

/** Spot FX A3: the pair this market prices, or nothing because this market is not a pair. */
export function pairOf(m: MarketDecl | undefined): CurrencyPair | undefined {
  return m?.kind === 'fx' ? m.fx : undefined;
}

/**
 * The issuer's supply for one session (Sovereign C1, C2, C5). The size is the issuer's choice
 * (C1.b) and the reservation its walk-away (C5, C7); the market chooses the price (Treasury D2.a).
 */
export interface PrimaryOffer {
  readonly market: MarketId;
  readonly issuer: PartyId;
  /** Law 8: whole units offered this session. An issuer brings pieces, like anybody else. */
  readonly size: Qty;
  /** The least the issuer will accept per unit; below it the paper is withdrawn (C7). */
  readonly reservation: number;
  /** C2: every winner pays the stop-out. */
  readonly allotment: 'uniformPrice';
}

/** Cash on each side settles through the party's own account (Money B1). */
export type AccountResolver = (party: PartyId, ccy: CurrencyCode) => AccountRef;

export interface MarketRunDeps {
  readonly parties: Parties;
  /** Law 8: what the smallest piece of anything is, which every quantity here has to land on. */
  readonly registry: Registry;
  readonly prices: PriceStore;
  readonly settlement: Settlement;
  readonly journal: Journal;
  readonly accountOf: AccountResolver;
  /** Bond N9.b: what has accrued per unit at this session's date, from the instrument's own terms. */
  readonly accruedPerUnit: (instrument: InstrumentId, period: Period) => number;
  /** Who promised it, when somebody did: a physical thing has nobody on that side (Goods A1). */
  readonly instrumentIssuer: (instrument: InstrumentId) => Option<PartyId>;
  /** Register A1.c: what this line is counted in, so its smallest piece can be asked for. */
  readonly unitOf: (instrument: InstrumentId) => UnitId;
  /**
   * Law 8, Clearing C4.c: THE SMALLEST INCREMENT THIS MARKET QUOTES IN, in the terms a price is
   * held in. Asked of the world rather than worked out here, because what a kind's tick is and what
   * a pair's is are two registry rows and this file is not where either lives (Law 15).
   */
  readonly tickOf: (m: MarketDecl) => number;
  /**
   * Law 15: WHAT A KIND OF MARKET NEEDS BEYOND THE BOOK, one row per kind that needs anything.
   * `admits` and `marginLegs` were two fields here, on the deps every market ever run is handed,
   * for the one kind that uses them — and a fourth kind with needs of its own would have been two
   * more. Asset and fx markets need nothing beyond the book, and the absence of their rows says so.
   */
  readonly kinds: MarketKindDeps;
}

export interface MarketKindDeps {
  readonly contract: ContractMarketDeps;
}

/** What a contract book needs that the kernel cannot answer (Derivative Layer E1, E2, D9). */
export interface ContractMarketDeps {
  /** Law 15: what a kind of contract is, asked of its profile and never branched on. */
  readonly derivativeKind: (kind: DerivativeKindId) => DerivativeKindProfile;
  /**
   * Derivative Layer E1, E2: HOW MUCH OF THIS ONE MEMBER CAN TAKE, read once per period from its
   * own liquid cash net of what it has already committed, and drawn down as it is consumed (E3).
   *
   * The kernel cannot answer it: what a member holds back as a buffer is its own preference, and
   * what it has already committed this period is its own module's record. So the module that owns
   * the layer answers, and the market cuts the trade to the smaller of the two sides' answers.
   */
  readonly admits: (
    party: PartyId,
    wanted: number,
    m: ContractMarketDecl,
    struck: number,
  ) => number;
  /**
   * D9, C3.a: the legs that post what this trade requires — an ASSET SWAP, money out and a claim
   * in, never an expense. They are the module's because the claim is the module's instrument, and
   * they are in the SAME instruction as the contract because a contract admitted and margined in
   * two passes was briefly unmargined (E2: the cut happens at the strike).
   */
  readonly marginLegs: (
    party: PartyId,
    against: PartyId,
    size: number,
    m: ContractMarketDecl,
    struck: number,
  ) => readonly Leg[];
}

export interface MarketResult {
  readonly market: MarketId;
  readonly outcome: 'cleared' | StaleReason;
  /** The level that cleared; none when nothing cleared and there was nothing to carry. */
  readonly price: Option<number>;
  readonly settledVolume: number;
  readonly failedTrades: number;
  /** Sovereign C4, when the session carried a primary offer. */
  readonly auction: Option<AuctionResult>;
}

/** What the market reads out of an auction (Sovereign C4). */
export interface AuctionResult {
  readonly issuer: PartyId;
  readonly size: number;
  /** Units the issuer actually placed and settled. */
  readonly allotted: number;
  /** Total demand posted, over the size offered: the cover ratio. */
  readonly cover: number;
  /** The average level the winning bids posted, less the stop-out, per unit (in yield it inverts). */
  readonly tail: Option<number>;
  readonly stopOut: Option<number>;
}

/** A matched trade: two named sides and a quantity (Clearing D2). */
interface Trade {
  readonly buyer: PartyId;
  readonly seller: PartyId;
  /** Law 8: what crossed, as a count of the unit's own smallest piece — on the common grain of
   * the two sides, so each side's share per member is a whole number of pieces too (XI-15). */
  readonly qty: Qty;
}

/**
 * Law 8: the book, on the market's own grid.
 *
 * A `market` order names no level — it takes whatever the book offers (solver.ts) — so there is
 * nothing to put on a grid and it passes through. Everything else is a LIMIT, and a limit has one
 * meaning per side: the most a buyer will pay, the least a seller will accept. So the direction is
 * read off the side and is not a choice anybody gets to make differently.
 */
function onTheGrid(orders: readonly Order[], tick: number): Order[] {
  const out: Order[] = [];
  for (const o of orders) {
    // Law 8, Clearing A2: AND A SIZE BELOW ONE PIECE IS NOT A SIZE. A reservation for less than the
    // smallest deliverable piece of the thing is a party that wants some of it and cannot name an
    // amount anybody could fill — the same answer a bid below one tick gets, for the same reason.
    //
    // It is HERE, with the grid rule, and not at each of the places that post, because it is the
    // same rule: what a book can hold is whole pieces at whole ticks, and a rule applied in one
    // place cannot be forgotten in another (Law 4). Rounding it UP is what must not happen — that
    // posts a size its owner never asked for.
    if (o.qty <= 0) continue;
    if (o.price === 'market') {
      out.push(o);
      continue;
    }
    const level = o.side === 'buy' ? downToTick(o.price, tick) : upToTick(o.price, tick);
    // Law 8, Clearing A2: A LIMIT BELOW THE SMALLEST INCREMENT THIS MARKET QUOTES IS NOT A LIMIT.
    // A buyer that will pay less than one tick has no level it could name here — rounding its bid
    // the way its own side means takes it to nothing, and nothing is not an offer. It is simply not
    // in this book at this session, which is the same answer a party with no size to post gets.
    //
    // It must not be POSTED at zero, which is what this did: a bill printed at nothing, and the
    // first read that divided by that price threw `yield of ust.bill.2026-06-15 is Infinity`. A
    // price of zero is not a cheap price, it is the absence of one (XI-6).
    if (level <= 0) continue;
    out.push({ ...o, price: level });
  }
  return out;
}

export function runMarket(
  m: MarketDecl,
  posted: readonly Order[],
  offer: Option<PrimaryOffer>,
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
): MarketResult {
  const orders: Order[] = [...posted];
  if (offer.some) {
    forbid(
      offer.value.market === m.id,
      'Law 4',
      `offer for ${offer.value.market} posted into ${m.id}`,
    );
    const issuer = deps.instrumentIssuer(m.instrument);
    forbid(
      issuer.some && issuer.value === offer.value.issuer,
      'Sovereign C1.b',
      `${offer.value.issuer} does not issue ${m.instrument}`,
    );
    orders.push({
      party: offer.value.issuer,
      side: 'sell',
      price: offer.value.reservation,
      qty: offer.value.size,
    });
  }
  // Law 8, Clearing C4.c: EVERY LEVEL IN THE BOOK IS ON THIS MARKET'S GRID, and this is the one
  // door into it. A price finer than the tick is not a price anybody can hit, and a solver that
  // cleared at one would print it (C4.c: the level is POSTED, never a bracket) — which is how
  // prints came to carry sixteen significant figures and every `units x price` read in the world
  // inherited them (worklist 12b.1).
  //
  // It is HERE and not at each of the twenty-one places that post, for the reason the print is not
  // rounded either: there is one book, everything that reaches it comes through this function, and
  // a rule applied in one place cannot be forgotten in another (Law 4). What the rule is, is not a
  // decision — a limit means the most a buyer will pay or the least a seller will accept, so the
  // side it was posted on says which way the grid takes it, and the kernel is honouring what the
  // poster promised rather than choosing on its behalf.
  const book = onTheGrid(orders, deps.tickOf(m));
  // Sovereign C2: a session carrying an offer is an auction, and its stated allotment is the
  // stop-out. Without one the venue is an open book and the sellers compete.
  const outcome = clear(book, m.rationing, offer.some ? 'marginalBid' : 'sellersCompete');
  switch (outcome.kind) {
    case 'cleared': {
      const trades = pairFills(m, outcome.fills, (p) => weightOf(deps.parties.get(p)));
      // A pair has no coupon and no instrument to ask about, and neither has a contract book: what
      // accrues on money is nothing, and what accrues on an obligation is what its own terms say
      // falls due (D4), which is the kind's business rather than the kernel's.
      const subject = delivers(m);
      const accrued = subject.some ? deps.accruedPerUnit(subject.value, period) : 0;
      let settledVolume = 0;
      let allotted = 0;
      let failed = 0;
      for (const t of trades) {
        const draft = tradeInstruction(m, t, outcome.price, accrued, deps, period, cycle);
        // Law 8: a quantity so small that what it comes to is less than half a piece of money is
        // not a trade — there is nothing to pay for it. It does not fill, which is a real outcome
        // of a real book and is what `settledVolume` says against the cleared volume.
        if (!draft.some) continue;
        const record = deps.settlement.settle(draft.value, period, cycle);
        if (record.outcome === 'settled') {
          settledVolume = finite(settledVolume + t.qty, 'settled volume');
          if (offer.some && t.seller === offer.value.issuer)
            allotted = finite(allotted + t.qty, 'allotted');
        } else failed += 1;
      }
      // Law 3, Clearing E1: A PRINT IS WHAT SOMEBODY PAID. A session whose book crossed but whose
      // every trade then failed to become a settled instruction has produced a level and no trade,
      // and writing it would be a price with nobody on either side of it — one that the marks, the
      // curve, every holder's equity and the next session's quotes would all then be built on. So
      // it prints nothing of its own and the last real price stands, visibly stale and saying why.
      if (settledVolume <= 0) {
        return carryLast(m, 'nothingSettled', period, cycle, deps, offer, book, outcome.fills);
      }
      deps.prices.write({
        instrument: m.instrument,
        market: m.id,
        period,
        price: outcome.price,
        ccy: m.ccy,
        provenance: { kind: 'traded', qty: settledVolume, trades: trades.length - failed },
      });
      deps.journal.record(
        period,
        cycle,
        'print',
        [m.id, m.instrument],
        {
          price: outcome.price,
          volume: outcome.volume,
          settledVolume,
          failedTrades: failed,
          rationed: outcome.rationed,
        },
        true,
      );
      const auction = offer.some
        ? some(
            auctionResult(offer.value, book, outcome.fills, some(outcome.price), allotted),
          )
        : none<AuctionResult>();
      if (auction.some) journalAuction(m, auction.value, period, cycle, deps);
      return {
        market: m.id,
        outcome: 'cleared',
        price: some(outcome.price),
        settledVolume,
        failedTrades: failed,
        auction,
      };
    }
    case 'noDemand':
    case 'noSupply':
    case 'noOverlap':
      return carryLast(m, outcome.kind, period, cycle, deps, offer, book, []);
    default:
      return assertNever(outcome, 'Outcome');
  }
}

/**
 * Clearing C4.b, E1, XI-6: A SESSION THAT PRODUCED NO TRADE. What stands is the last real price,
 * carried forward and VISIBLY stale with the reason on it — or, when there has never been one,
 * nothing at all, because a price is never invented to fill a gap.
 *
 * One function for every way a session can end without a trade (Law 4): the book had one side, the
 * two sides did not meet, or they met and nothing settled. They are different reasons and each says
 * which it was; what happens to the price is the same thing, and it is written once.
 */
function carryLast(
  m: MarketDecl,
  reason: StaleReason,
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
  offer: Option<PrimaryOffer>,
  orders: readonly Order[],
  fills: readonly Fill[],
): MarketResult {
  const auction = offer.some
    ? some(auctionResult(offer.value, orders, fills, none<number>(), 0))
    : none<AuctionResult>();
  if (auction.some) journalAuction(m, auction.value, period, cycle, deps);
  const last = deps.prices.latest(m.instrument, period);
  if (!last.some) {
    // Nothing traded and nothing to carry: the line has no price at all, which is what a reader is
    // told when it asks (XI-6). A print is never invented to fill the gap.
    deps.journal.record(period, cycle, 'print', [m.id, m.instrument], { printed: false, reason }, true);
    return { market: m.id, outcome: reason, price: none(), settledVolume: 0, failedTrades: 0, auction };
  }
  const from = struckIn(last.value);
  deps.prices.write({
    instrument: m.instrument,
    market: m.id,
    period,
    price: last.value.price,
    ccy: m.ccy,
    provenance: { kind: 'stale', from, reason },
  });
  deps.journal.record(
    period,
    cycle,
    'print',
    [m.id, m.instrument],
    { stale: true, reason, carriedFrom: from, price: last.value.price },
    true,
  );
  return {
    market: m.id,
    outcome: reason,
    price: some(last.value.price),
    settledVolume: 0,
    failedTrades: 0,
    auction,
  };
}

/** Pair buy fills with sell fills, walking both lists; every trade has two named sides (D2). */
function pairFills(
  m: MarketDecl,
  fills: readonly Fill[],
  weight: (party: PartyId) => number,
): Trade[] {
  const buys = fills.filter((f) => f.side === 'buy').map((f) => ({ ...f }));
  const sells = fills.filter((f) => f.side === 'sell').map((f) => ({ ...f }));
  const out: Trade[] = [];
  let b = 0;
  let s = 0;
  let bLeft = zeroIfNone(buys[0]?.qty);
  let sLeft = zeroIfNone(sells[0]?.qty);
  while (b < buys.length && s < sells.length) {
    const buyer = buys[b];
    const seller = sells[s];
    if (buyer === undefined || seller === undefined) break;
    // D2, Clearing A2: NOBODY TRADES WITH ITSELF, AND IT IS A CONTRACT.
    //
    // A party is welcome on both sides of one book — it wants more of a line at one level and will
    // let some go at another, which is what a quote IS (Dealer Desks A2) — but its two orders
    // cannot CROSS, because a schedule whose bid is at or above its own offer is a party paying
    // itself. So a fill between one party's own two orders means the party that posted them is
    // broken, and this refuses it at the site rather than stepping past it: a solver that walked
    // around it would hide the next module that puts two deciders behind one name (Law 12).
    forbid(
      buyer.party !== seller.party,
      'Clearing A2',
      `${buyer.party} is on both sides of ${m.id} at crossing prices`,
      { party: buyer.party, market: m.id },
    );
    const want = atMost(bLeft, sLeft, 'neither side can exchange what the other has not got');
    // Law 8, XI-15: what these two can actually exchange. Between named parties that is the unit's
    // own smallest piece; where one side is a population it is that piece for every member of it,
    // because each member is a real holder and none of them can hold a fraction of one.
    const step = commonGrain(weight(buyer.party), weight(seller.party));
    const q = downToGrain(want, step);
    if (q > 0) out.push({ buyer: buyer.party, seller: seller.party, qty: q });
    bLeft = finite(bLeft - q, 'buy left');
    sLeft = finite(sLeft - q, 'sell left');
    // What is left over is smaller than these two can trade: it is not filled, and the side with
    // less of it steps aside so the other can meet somebody it CAN deal with.
    if (bLeft < step) {
      b += 1;
      bLeft = zeroIfNone(buys[b]?.qty);
    }
    if (sLeft < step) {
      s += 1;
      sLeft = zeroIfNone(sells[s]?.qty);
    }
  }
  return out;
}

/**
 * Bond N9.b: the paper changes hands at the clean price and the buyer pays the seller what has
 * accrued since the last coupon on top. The lot's basis is the clean price, so the coupon that
 * arrives next is not a windfall to whoever holds it on the date: the buyer's equity falls by the
 * accrued now and rises by the coupon then, and the seller's income is what it earned.
 */
function tradeInstruction(
  m: MarketDecl,
  t: Trade,
  price: number,
  accruedPerUnit: number,
  deps: MarketRunDeps,
  period: Period,
  cycle: Cycle,
): Option<InstructionDraft> {
  return MARKET_KINDS[kindOf(m)].trade(m, t, price, accruedPerUnit, deps, period, cycle);
}

/**
 * Spot FX A1, Clearing D1, Law 4: WHAT A MARKET DELIVERS — the instrument that changes hands in it,
 * when what it moves is a thing somebody holds. One writer of that, because every reader that walks
 * the market list and asks the instrument store about the subject would otherwise have to know what
 * a pair is. It reads the market's declared KIND, where it used to test two optional bags for
 * absence and re-derive the kind from them (Law 19: read the source, do not re-derive it).
 */
export function delivers(m: MarketDecl): Option<InstrumentId> {
  return MARKET_KINDS[kindOf(m)].delivers(m);
}

/**
 * Law 15: one row per kind of market, and nothing anywhere branches on which (`MarketKind`).
 *
 * The row's members are METHODS on purpose. A method's parameters are checked bivariantly, and that
 * is what lets the `contract` row hold a function taking a `ContractMarketDecl` — sound HERE, and
 * only here, because this table is keyed by the very field the union is discriminated on: the row
 * reached by `kindOf(m)` is the row written for `m`'s own variant. That is what deletes the two
 * runtime `throw`s, where each handler took the whole union and asked which variant it had.
 */
interface MarketKindTerms<M extends MarketDecl> {
  trade(
    m: M,
    t: Trade,
    price: number,
    accruedPerUnit: number,
    deps: MarketRunDeps,
    period: Period,
    cycle: Cycle,
  ): Option<InstructionDraft>;
  /**
   * A pair delivers nothing: its two legs are money, and the id it prints under names a price
   * rather than an instrument anybody could be handed. A contract book delivers nothing either:
   * what a derivative trade produces is an obligation on two balance sheets (Derivative X1).
   */
  delivers(m: M): Option<InstrumentId>;
}

const MARKET_KINDS: Readonly<Record<MarketKind, MarketKindTerms<MarketDecl>>> = Object.freeze({
  asset: { trade: assetTrade, delivers: (m: AssetMarketDecl) => some(m.instrument) },
  fx: { trade: fxTrade, delivers: () => none<InstrumentId>() },
  contract: { trade: contractTrade, delivers: () => none<InstrumentId>() },
});

/**
 * Derivative D1, D7, D9, X1; Derivative Layer B1, B2, C2, E2: A FILL IN A CONTRACT BOOK IS A ROW ON
 * TWO BALANCE SHEETS, cut to what its two sides can margin, with the margin in the same pass.
 *
 * Bilaterally it is ONE contract, buyer as `a`. Cleared it is TWO — member to house and house to
 * member (C2) — so no member ever faces another, and the house is flat by construction because it
 * holds both sides of the same terms at the same level. Either way nothing is delivered: what the
 * trade produces is an obligation, and the only cash that moves now is the premium the kind states
 * (D7.b: none at all for a contract struck at par) and the margin (D9).
 *
 * THE CUT IS ARITHMETIC AND NOT A BOUND (Law 6, E2, E4). A member's admitted share is a quantity it
 * HAS — liquid cash it has not already committed — so a trade larger than it is a trade one side
 * cannot make, in the way that selling units nobody holds is. What is refused is journaled and
 * measured (E4), and nothing anywhere raises the limit.
 */
function contractTrade(
  m: ContractMarketDecl,
  t: Trade,
  price: number,
  _accruedPerUnit: number,
  deps: MarketRunDeps,
  period: Period,
  cycle: Cycle,
): Option<InstructionDraft> {
  const decl = m.contract;
  const profile = deps.kinds.contract.derivativeKind(decl.kind);
  let size = t.qty;
  for (const side of [t.buyer, t.seller]) {
    // Law 8: what the layer will admit is money over a margin requirement, so it lands between two
    // contracts — and the one below is what this side can actually carry. A contract is whole or it
    // is not one (item 13b.1).
    const room = downTick(deps.kinds.contract.admits(side, size, m, price));
    if (room < size) size = room;
  }
  if (size < t.qty) {
    // E4: what the market struck BEYOND what its members could margin is a measurable quantity, and
    // this is where it is measured. Nothing anywhere raises the limit in response to it.
    deps.journal.record(
      period,
      cycle,
      'derivatives.refused',
      [m.id, t.buyer, t.seller],
      { market: m.id, wanted: t.qty, admitted: size, cut: sub(t.qty, size, 'refused') },
      true,
    );
  }
  if (size <= 0) return none<InstructionDraft>();
  // Law 8: a premium is MONEY, so it is a whole number of the money's own smallest piece.
  const premium = deps.registry.cashFor(
    m.ccy,
    mul(profile.premiumPerUnit(price, decl.terms), size, 'the premium at inception'),
  );
  const house = decl.house;
  const legs: Leg[] = [];
  const open = (a: PartyId, b: PartyId, value: number): Leg => ({
    kind: 'contract',
    act: 'open',
    a,
    b,
    derivative: decl.kind,
    terms: decl.terms,
    ccy: m.ccy,
    notional: size,
    struckAt: price,
    // Clearing E1: the same line this session's print is written against, so a party that struck
    // the contract has observed the price of this book (Expectations A2).
    book: m.instrument,
    value,
    house,
  });
  if (house === null) {
    legs.push(open(t.buyer, t.seller, premium));
  } else {
    // C2: the house is buyer to the seller and seller to the buyer, and no member pays another.
    legs.push(open(t.buyer, house, premium), open(house, t.seller, premium));
  }
  if (premium > 0) {
    const pay = (from: PartyId, to: PartyId): Leg => ({
      kind: 'money',
      from: deps.accountOf(from, m.ccy),
      to: deps.accountOf(to, m.ccy),
      ccy: m.ccy,
      amount: premium,
      fromCell: none(),
      toCell: none(),
    });
    if (house === null) legs.push(pay(t.buyer, t.seller));
    else legs.push(pay(t.buyer, house), pay(house, t.seller));
  }
  legs.push(
    ...deps.kinds.contract.marginLegs(t.buyer, house ?? t.seller, size, m, price),
    ...deps.kinds.contract.marginLegs(t.seller, house ?? t.buyer, size, m, price),
  );
  return some({
    legs,
    cause: 'trade',
    reason: `${m.name}: ${size} @ ${price}${size < t.qty ? ` (cut from ${t.qty})` : ''}`,
  });
}

/**
 * Spot FX A1, C1, XI-5: A SPOT TRADE IS TWO MONEY LEGS, and both settle or neither does.
 *
 * There is no asset here and no issuer: what changes hands is one money for another, at the rate
 * the session struck. The buyer of the pair takes the BASE and gives the QUOTE — one unit of base
 * costs `price` of quote, which is the convention the print is in — and delivery-versus-payment is
 * the atomicity settlement already has: an instruction applies whole or not at all (XI-5), so
 * neither side can be left having paid for money it did not get (Herstatt, C6).
 *
 * Law 8 twice over: each leg lands on the smallest piece of its OWN money, and the two pieces are
 * different sizes. What the quote side comes to is struck on its own grain at the rate, so the rate
 * a trade REALISES can differ from the print by less than one piece — exactly as an asset trade's
 * cash does, and for the same reason.
 */
function fxTrade(
  m: FxMarketDecl,
  t: Trade,
  price: number,
  _accruedPerUnit: number,
  deps: MarketRunDeps,
): Option<InstructionDraft> {
  const pair = m.fx;
  const buyer = deps.parties.get(t.buyer);
  const seller = deps.parties.get(t.seller);
  const grain = commonGrain(weightOf(buyer), weightOf(seller));
  const quote = toGrain(mul(t.qty, price, 'what the base costs in quote'), grain);
  if (quote <= 0) return none<InstructionDraft>();
  const baseOut = cellSide(seller, asQty(t.qty / weightOf(seller), 'its share per member'));
  const baseIn = cellSide(buyer, asQty(t.qty / weightOf(buyer), 'its share per member'));
  const quoteOut = cellSide(buyer, asQty(quote / weightOf(buyer), 'its share per member'));
  const quoteIn = cellSide(seller, asQty(quote / weightOf(seller), 'its share per member'));
  const legs: Leg[] = [
    {
      kind: 'money',
      from: deps.accountOf(t.seller, pair.base),
      to: deps.accountOf(t.buyer, pair.base),
      ccy: pair.base,
      amount: t.qty,
      fromCell: baseOut === undefined ? none() : some(baseOut),
      toCell: baseIn === undefined ? none() : some(baseIn),
    },
    {
      kind: 'money',
      from: deps.accountOf(t.buyer, pair.quote),
      to: deps.accountOf(t.seller, pair.quote),
      ccy: pair.quote,
      amount: quote,
      fromCell: quoteOut === undefined ? none() : some(quoteOut),
      toCell: quoteIn === undefined ? none() : some(quoteIn),
    },
  ];
  return some({ legs, cause: 'trade', reason: `${m.name}: ${t.qty} @ ${price}` });
}

function assetTrade(
  m: AssetMarketDecl,
  t: Trade,
  price: number,
  accruedPerUnit: number,
  deps: MarketRunDeps,
): Option<InstructionDraft> {
  const buyer = deps.parties.get(t.buyer);
  const seller = deps.parties.get(t.seller);
  // Law 8: WHAT IS PAID IS A WHOLE NUMBER OF THE SMALLEST PIECE OF THE MONEY, and where a side is a
  // population, of that piece for each of its members. The quantity was already struck on a grain
  // both sides can hold (pairFills); the cash is struck on the same grain in money, at the level
  // that cleared. So the price a trade REALISES can differ from the print by less than one piece of
  // money per member — which is what rounding a price to real money has always meant, and is why
  // `settledVolume` and the print are two numbers rather than one.
  const cashGrain = commonGrain(weightOf(buyer), weightOf(seller));
  const cash = toGrain(mul(t.qty, add(price, accruedPerUnit, 'dirty price'), 'trade cash'), cashGrain);
  if (cash <= 0) return none<InstructionDraft>();
  const buyerCell = cellSide(buyer, asQty(t.qty / weightOf(buyer), 'its share per member'));
  const sellerCell = cellSide(seller, asQty(t.qty / weightOf(seller), 'its share per member'));
  const buyerCashCell = cellSide(buyer, asQty(cash / weightOf(buyer), 'its share per member'));
  const sellerCashCell = cellSide(seller, asQty(cash / weightOf(seller), 'its share per member'));
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: t.seller,
      to: t.buyer,
      instrument: m.instrument,
      qty: t.qty,
      pricePerUnit: some(price),
      accruedPerUnit: accruedPerUnit === 0 ? none() : some(accruedPerUnit),
      fromCell: sellerCell === undefined ? none() : some(sellerCell),
      toCell: buyerCell === undefined ? none() : some(buyerCell),
    },
    {
      kind: 'money',
      from: deps.accountOf(t.buyer, m.ccy),
      to: deps.accountOf(t.seller, m.ccy),
      ccy: m.ccy,
      amount: cash,
      fromCell: buyerCashCell === undefined ? none() : some(buyerCashCell),
      toCell: sellerCashCell === undefined ? none() : some(sellerCashCell),
    },
  ];
  const issuer = deps.instrumentIssuer(m.instrument);
  const cause = issuer.some && t.seller === issuer.value ? 'issuance' : 'trade';
  return some({ legs, cause, reason: `${m.name}: ${t.qty} @ ${price}` });
}

/** Sovereign C4: the cover ratio and the tail are read off the book, not stated. */
function auctionResult(
  offer: PrimaryOffer,
  orders: readonly Order[],
  fills: readonly Fill[],
  stopOut: Option<number>,
  allotted: number,
): AuctionResult {
  const bids = orders.filter((o) => o.side === 'buy' && o.party !== offer.issuer);
  const demand = sum(bids.map((o) => o.qty)).value;
  const won = fills.filter((f) => f.side === 'buy');
  const wonQty = sum(won.map((f) => f.qty)).value;
  const wonValue = sum(won.map((f) => mul(f.qty, f.at, 'bid value'))).value;
  const tail =
    stopOut.some && wonQty > 0
      ? some(sub(div(wonValue, wonQty, 'average bid'), stopOut.value, 'tail'))
      : none<number>();
  return {
    issuer: offer.issuer,
    size: offer.size,
    allotted,
    cover: div(demand, offer.size, 'cover ratio'),
    tail,
    stopOut,
  };
}

function journalAuction(
  m: MarketDecl,
  a: AuctionResult,
  period: Period,
  cycle: Cycle,
  deps: MarketRunDeps,
): void {
  deps.journal.record(
    period,
    cycle,
    'auction.result',
    [m.id, m.instrument, a.issuer],
    {
      line: m.instrument,
      size: a.size,
      allotted: a.allotted,
      withdrawn: sub(a.size, a.allotted, 'withdrawn'),
      cover: a.cover,
      stopOut: a.stopOut.some ? a.stopOut.value : null,
      tail: a.tail.some ? a.tail.value : null,
    },
    true,
  );
}
