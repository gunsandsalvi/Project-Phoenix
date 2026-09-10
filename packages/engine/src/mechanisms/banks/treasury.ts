/**
 * A bank's TREASURY: how much it holds against what could leave, and in what form.
 *
 * @spec Banks Funding C1 Banks Funding C1.a Banks Funding C2 Banks Funding C2.a Banks Funding F4 Sovereign E2 Sovereign E2.a Sovereign E5 Corporate Credit E5 Corporate Credit E5.c Dealer Desks A1 Dealer Desks C2.a Money Market A2 Money Market A2.a Law 2 Law 4 Law 19
 *
 * ONE liquidity decision, where two used to be. The money market held a bank's CASH buffer (its own
 * worst week, C2.a) and the sovereign module held a PAPER buffer (a stated share of what it had
 * issued, a placeholder). They were one treasurer's one question asked twice, and the second answer
 * was a number nobody could defend.
 *
 * The question is C2's: what could leave, and what do I hold against it. What could leave is a read
 * the bank already publishes (`bank.liquidity.couldLeave`: the uninsured money on its own books).
 * What it must hold against that is a POLICY — a coverage rule somebody wrote, which is what makes
 * it a thing a polity can change (worklist 14) — and what it holds ABOVE the rule is its own
 * PREFERENCE, which is why two banks facing the same depositors carry different portfolios.
 *
 * The FORM is the other half of the decision and it is not a second one: the part it holds in cash
 * is the worst week its own account has actually had, because that is what has to be paid on the
 * day; the rest is paper, which earns a yield and can be pledged, and is the liquidity portfolio.
 *
 * THE TREASURY POSTS NOTHING. It hands the dealing line a TARGET (`liquidityTargets`) and the
 * dealing line quotes around it (C2.a: how a book mean-reverts without anybody telling it to). That
 * is how a real treasury sells a liquidity portfolio — through its own desk, whose quote then skews
 * — and it is what keeps one bank showing one face to one market.
 */
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { moneyInstrumentId, partyId } from '../../core/ids.js';
import type { Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import type { Instrument } from '../../register/instruments.js';
import { downTick } from '../../core/tick.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { priceAt } from '../../prices/curve.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { bankParam, type BankDecl } from './data.js';

/** C2: the coverage a bank must hold against what could leave. A rule, and somebody wrote it. */
export const P_COVERAGE: ParamId = paramId('regulation.liquidityCoverage');

/** One day count for what a bank reckons a dated claim is worth, stated once where it is asked. */
export const VALUATION_DAY_COUNT = 'ACT/ACT';

/** C2, C2.a: what this bank wants to be holding, and in what form. */
export interface LiquidityPlan {
  /** F4: the uninsured money on its own books — what a bad week would ask it for. */
  readonly couldLeave: number;
  /** The liquid assets it wants: the rule's coverage plus its own cushion, on what could leave. */
  readonly wanted: number;
  /** C2.a: the part it holds in the account, because that is what has to be paid on the day. */
  readonly cash: number;
  /** The rest: the liquidity portfolio, in its own money. Nothing where the cash already covers it. */
  readonly paper: number;
}

/**
 * The plan, read from what the bank itself last published about its own funding (Law 4, Law 19).
 *
 * A bank that has published nothing has no plan — it has not yet had a week, so it does not know
 * what a bad one costs. That is a real state at the opening of the world and not a missing number:
 * it holds what it was given until it has watched its own account for a period.
 */
export function liquidityPlan(view: ParticipantView, cushion: number): Option<LiquidityPlan> {
  const said = view.lastOwn('bank.liquidity');
  if (!said.some) return none<LiquidityPlan>();
  const couldLeave = said.value.data['couldLeave'];
  const buffer = said.value.data['buffer'];
  if (typeof couldLeave !== 'number' || typeof buffer !== 'number') return none<LiquidityPlan>();
  const wanted = mul(
    couldLeave,
    add(view.params.get(P_COVERAGE), cushion, 'the rule and its own cushion'),
    'the liquid assets it wants',
  );
  const above = sub(wanted, buffer, 'what is left for the portfolio');
  // A bank whose own worst week is bigger than the coverage asks of it holds no portfolio: the cash
  // is already the answer. Nothing is bounded here — a holding of less than nothing is not a
  // smaller target, it is a short position, and it takes a borrow this bank has not made.
  return some({ couldLeave, wanted, cash: buffer, paper: above > 0 ? above : 0 });
}

/**
 * E2.a, E5: where the portfolio is held — the lines its own dealing line makes a market in that are
 * DATED CLAIMS ON A NAME.
 *
 * Not a list of eligible assets (that is a LENDER's question, Money Market B3.a, and the lender
 * answers it): this is the treasury asking what it could hold and get out of. A claim with dated
 * payments can be valued at a yield and pledged; a share cannot. And a line its own desk does not
 * make is a line it would have to go to a rival to sell, which is not a liquidity portfolio.
 */
export function liquidityLines(view: ParticipantView, d: BankDecl): readonly InstrumentId[] {
  const out: InstrumentId[] = [];
  const on = view.calendar.startOf(view.period);
  for (const m of view.markets) {
    const i = view.instruments.get(m.instrument);
    if (!i.status.live || !d.makes.includes(String(i.kind))) continue;
    if (view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar).length === 0) continue;
    out.push(i.id);
  }
  return out;
}

/**
 * The target holding per line, in the bank's own money (Dealer Desks C2.a).
 *
 * Spread evenly over the lines it could hold it in, because between one issuer's lines a treasury
 * holding paper for LIQUIDITY has no reason to prefer one over another: they are the same credit
 * and the same money, and what tells them apart is price risk, which is a view and belongs to the
 * dealing line's quote rather than to the target it quotes around.
 */
export function liquidityTargets(
  view: ParticipantView,
  d: BankDecl,
  plan: Option<LiquidityPlan>,
): ReadonlyMap<InstrumentId, number> {
  const out = new Map<InstrumentId, number>();
  const lines = liquidityLines(view, d);
  // EVERY line it makes is in here, and a line it holds for no liquidity reason has a target of
  // NOTHING — which is an answer and not a missing number. So nothing downstream ever has to decide
  // what an absent target means, because there are none.
  for (const m of view.markets) {
    const i = view.instruments.get(m.instrument);
    if (i.status.live && d.makes.includes(String(i.kind))) out.set(i.id, 0);
  }
  if (!plan.some || plan.value.paper <= 0 || lines.length === 0) return out;
  const each = div(plan.value.paper, lines.length, 'the target in one line');
  for (const id of lines) out.set(id, each);
  return out;
}

/**
 * Corporate Credit E5, E5.a-c: what this bank requires, per annum, to hold a named issuer's paper.
 *
 * Its own cost of funds, its own expected loss on that name and the capital the position consumes —
 * computed once by the module that owns this bank's economics and published under its own name, and
 * READ here (Law 4, Law 19). A second copy of those three terms would be the bank pricing its book
 * against a belief it does not hold.
 */
export function requiredYieldOf(view: ParticipantView, issuer: PartyId): Option<number> {
  const own = view.lastOwn('bank.reservation');
  if (!own.some) return none<number>();
  const required = own.value.data['required'];
  if (typeof required !== 'object' || required === null) return none<number>();
  const rate = (required as Record<string, unknown>)[issuer];
  return typeof rate === 'number' ? some(rate) : none<number>();
}

/**
 * What a dated claim is worth to a party at a yield it has decided for itself: the line's own cash
 * flows, discounted. The yield is an INPUT to the schedule the bank posts and the PRICE is what
 * clears; the curve is then read back out of the prints (Sovereign D2, Bond N7.b). Nothing here
 * reads the print it is about to help set, which is the fixed point XI-13 exists to prevent.
 */
export function priceAtYield(
  view: ParticipantView,
  instrument: InstrumentId,
  y: number,
): Option<number> {
  const i = view.instruments.get(instrument);
  const on = view.calendar.startOf(view.period);
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  if (flows.length === 0) return none<number>();
  return some(priceAt(flows, y, on, VALUATION_DAY_COUNT, `what ${instrument} is worth to it`));
}

/** The money this bank settles in, off its own region (Law 8: a number carries its currency). */
export function ccyOf(view: ParticipantView): CurrencyCode {
  return view.registry.region(view.self.region).ccy;
}

/* ------------------------------------------------------------------------------------------------
 * FUNDING: what it pays for money, what it will lend, and what it is short of.
 *
 * @spec Banks Funding B1 Banks Funding B1.a Banks Funding B2 Banks Funding B2.a Banks Funding B2.b Banks Funding B3 Banks Funding C1 Banks Funding C2 Banks Funding C2.a Banks Funding D1 Banks Funding D3 Banks Funding F1 Banks Funding F2 Banks Funding F4 Money Market A2 Money Market A2.a Money Market A2.b Money Market B1 Money Market B2 Money Market B2.a Money Market B3 Money Market B4 Money Market B5.a Money Market B7 Money Market C4.a Money Market C4.b Observer A5 Law 4
 *
 * ALL OF IT IS THE BANK'S, and none of it is the market's. The interbank market is a venue: it
 * declares the books, it clears what is posted into them, it writes the rows and it publishes the
 * refusal. What a bank will pay for a deposit, what it will lend to a rival and at what, what it
 * holds against a bad week and what it is short of when the week ends are decisions, and a decision
 * belongs to the party that has to live with it (Law 4: one decider, one face).
 * ---------------------------------------------------------------------------------------------- */

/** The two administered levels, as the central bank last published them (Central Bank B2, C2). */
export interface SeenCorridor {
  readonly policy: number;
  readonly floor: number;
  readonly ceiling: number;
}

/**
 * Central Bank B2, B3.a: the corridor, READ. It is administered and published, so a bank looks it
 * up rather than deriving it — a second copy of it in the treasury would be a second central bank.
 *
 * A bank prices its board off the corridor it has SEEN, which is last period's when the rate moved
 * this morning. That is a real lag: a board does not change in the hour a policy rate does.
 */
export function corridorSeen(view: ParticipantView): Option<SeenCorridor> {
  const said = view.lastPublic('centralBank.corridor');
  if (!said.some) return none<SeenCorridor>();
  const { policy, floor, ceiling } = said.value.data;
  if (typeof policy !== 'number' || typeof floor !== 'number' || typeof ceiling !== 'number') {
    return none<SeenCorridor>();
  }
  return some({ policy, floor, ceiling });
}

/** What its own account has done to it lately, and what it holds against the worst of it (C2.a). */
export interface ReserveMemory {
  readonly moves: Record<string, number[]>;
}

/** C2.a: one more week of its own account, remembered as far back as this bank looks. */
export function remember(memory: ReserveMemory, bank: PartyId, move: number, keep: number): void {
  const moves = memory.moves[bank] ?? [];
  moves.push(move);
  while (moves.length > keep) moves.shift();
  memory.moves[bank] = moves;
}

/**
 * A2.a, C2, C2.a: the buffer this bank holds against what could leave — the worst week its own
 * account has had, over its own memory. A bank that has never had a bad week holds nothing against
 * one, which is a real (and dangerous) position and not a missing number.
 */
export function bufferOf(memory: ReserveMemory, bank: PartyId): number {
  const moves = memory.moves[bank] ?? [];
  let worst = 0;
  for (const m of moves) if (m < worst) worst = m;
  return -worst;
}

/** C2.a: what this bank's reserve account actually did this period — the wire's own legs, summed. */
export function reserveFlow(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const terms: number[] = [];
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.reserveLegs) {
      if (leg.bank !== bank || leg.ccy !== ccy) continue;
      terms.push(leg.amount);
    }
  }
  return sum(terms).value;
}

/** Money Market A2: where a bank stands after the flows — what it holds, and what it wanted to. */
export interface TreasuryPosition {
  readonly bank: PartyId;
  readonly reserves: number;
  readonly buffer: number;
  readonly gap: number;
}

/**
 * B1.a, B5.a, B2: what money is worth to this bank — what the market has been charging it over the
 * memory it keeps, weighted by how much it borrowed at each rate, and the floor if it has not
 * borrowed at all (that is what it can get for the spare).
 *
 * It is an AVERAGE OVER ITS OWN MEMORY and not the last session's dearest row, and the difference
 * is the whole stability of the thing. A deposit rate struck off one night's borrowing swings by
 * the width of the corridor every time a bank is short for a day, and then every depositor in the
 * world moves at once — which is not a funding market, it is a metronome.
 *
 * D3, B2.b: AND WHEN THE MARKET WOULD NOT HAVE IT, IT BIDS UP FOR DEPOSITS — IF IT IS WORTH IT. The
 * dollar it could not raise is worth what its remaining alternative costs, which is the window at
 * the top of the corridor. But there is ONE RATE PER LIABILITY: a bank cannot pay up for the next
 * dollar without paying up on every dollar it already has, so bidding up costs it the rise on its
 * whole base and buys it the rise on what it was short of. It does that when the second is the
 * bigger of the two and not otherwise — a decision between two real costs, and the same gap
 * multiplies both, so what it comes down to is whether what it could not raise is bigger than what
 * it already owes.
 */
export function worthOfMoney(
  ctx: MechanismContext,
  bank: PartyId,
  c: SeenCorridor,
  base: number,
): number {
  const short = refusedLastSession(ctx, bank);
  if (short.some && short.value > base) return c.ceiling;
  const memory = ctx.params.get(bankParam(bank, 'bufferMemory'));
  const from = ctx.period > memory ? ctx.period - memory : 0;
  const weights: number[] = [];
  const weighted: number[] = [];
  for (const e of ctx.journal.ofKind('moneyMarket.print')) {
    if (e.period < from || e.period >= ctx.period || e.data['borrower'] !== bank) continue;
    const rate = e.data['rate'];
    const volume = e.data['volume'];
    if (typeof rate !== 'number' || typeof volume !== 'number' || volume <= 0) continue;
    weights.push(volume);
    weighted.push(mul(volume, rate, 'what that money cost it'));
  }
  const total = sum(weights).value;
  if (total <= 0) return c.floor;
  const paid = div(sum(weighted).value, total, 'what its funding has been costing it');
  return paid > c.floor ? paid : c.floor;
}

/**
 * B7, D3: what the last session left this name short of, if anything. It is the session's own
 * public refusal read back (Law 19), not a second count of it.
 */
function refusedLastSession(ctx: MechanismContext, bank: PartyId): Option<number> {
  if (ctx.period === 0) return none<number>();
  const last = ctx.period - 1;
  for (const e of ctx.journal.ofKind('moneyMarket.refused')) {
    if (e.period !== last || !e.subjects.includes(bank)) continue;
    const short = e.data['short'];
    if (typeof short === 'number') return some(short);
  }
  return none<number>();
}

/**
 * B1.a, E2.a, D5.a: AND IT ANSWERS ITS RIVALS, because they are paying in public and its own
 * depositors can read it (E1).
 *
 * A bank that only ever priced its deposits off its own funding cost would sit still while somebody
 * across the road bid a point over it, and lose its base to it week by week as one depositor after
 * another worked out that the gap had cost it more than moving would (E1). That is not a deposit
 * market: it is a bank that cannot see a rival's board its own depositors can read.
 */
export function defended(best: Option<number>, own: number, worth: number): number {
  if (!best.some) return own;
  // B1.a, D1: it matches a rival that is paying more, and it stops at what the money is worth to
  // it — past that it funds itself in the market instead and lets the deposit go, which is the
  // alternative D1 names and not a cap on anything. What it will not do is guess how far under the
  // rival it could sit: that is the depositor's arithmetic (E1) and it is done there.
  return best.value > own && best.value < worth ? best.value : own;
}

/**
 * Money Market C4.b: what THIS bank reckons it could put up — its own free paper, at its own marks.
 *
 * A dated claim on a name can be pledged: there is somebody behind it and it can be valued. A share
 * cannot, and neither can a tonne of grain. What a LENDER will actually advance against it is the
 * lender's own question and the lender answers it (Money Market B3.b); this is the borrower asking
 * how much it could possibly ask for, which is what decides how big a bid it puts in a secured book.
 */
export function pledgeable(view: ParticipantView): number {
  const on = view.calendar.startOf(view.period);
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.status.live || !i.issuer.some) continue;
    if (view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar).length === 0) continue;
    const mark = view.mark(i.id);
    const free = view.free(i.id);
    if (!mark.some || mark.value <= 0 || free <= 0) continue;
    terms.push(mul(free, mark.value, 'what it could put up'));
  }
  return sum(terms).value;
}

/**
 * Banks Funding A2.a: WHOLESALE BORROWING IS SHORT AND IT ROLLS, AND THAT IS WHERE A SQUEEZE BITES.
 *
 * What falls due FROM this bank at the start of the next period — every liability of its own, by
 * that liability's own dates, and never a list of the kinds that happen to be rows. A row struck
 * this period falls due before any session could fund it, so a bank borrows today to repay
 * tomorrow, which is what rolling is.
 */
export function fallsDueNext(view: ParticipantView, ccy: CurrencyCode): number {
  return dueNext(view, ccy, (i) => i.issuer.some && i.issuer.value === view.self.id, (i) => i.issued);
}

/**
 * Banks Funding C1: and the same ladder from the other side — what comes back INTO the account
 * tomorrow morning. A bank that parked its spare cash at the floor has not stopped holding it: it
 * holds an overnight claim that turns back into a balance, and that is as liquid as an asset gets.
 */
export function comesBackNext(view: ParticipantView, ccy: CurrencyCode): number {
  return dueNext(view, ccy, () => true, (i) => view.quantity(i.id));
}

function dueNext(
  view: ParticipantView,
  ccy: CurrencyCode,
  mine: (i: Instrument) => boolean,
  units: (i: Instrument) => number,
): number {
  const next = view.period + 1;
  const on = view.calendar.startOf(view.period);
  const terms: number[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || i.ccy !== ccy || !mine(i)) continue;
    const n = units(i);
    if (n <= 0) continue;
    const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
    const perUnit = sum(
      flows.filter((f) => view.calendar.place(f.date) === next).map((f) => f.perUnit),
    ).value;
    if (perUnit === 0) continue;
    terms.push(mul(n, perUnit, 'what falls due next period'));
  }
  return sum(terms).value;
}

/** Money Market A2: where this bank stands after the flows, as it reckons it itself. */
export function positionOf(view: ParticipantView, ccy: CurrencyCode): TreasuryPosition {
  const reserves = view.cash(ccy);
  const said = view.lastOwn('bank.buffer');
  const held = said.some && said.value.period === view.period ? said.value.data['buffer'] : undefined;
  const buffer = typeof held === 'number' ? held : 0;
  return { bank: view.self.id, reserves, buffer, gap: sub(reserves, buffer, 'its position') };
}

/**
 * B5.a, C1: what a lender will accept. Its alternative is the floor — it can always park there —
 * so it is a decision between two named things it could do, and nothing clamps the answer.
 */
export function lenderReservation(
  view: ParticipantView,
  secured: boolean,
  borrower: PartyId,
  c: SeenCorridor,
): Option<number> {
  // Money Market B3: a secured claim is a claim on the paper. The name costs it nothing it has not
  // already taken a haircut for, so what it wants is what it could get for the cash anyway.
  if (secured) return some(c.floor);
  // B2: unsecured prices the NAME, out of this bank's own credit model — what it expects to lose on
  // that name and what the capital such a claim consumes costs it, both of which it has already
  // published under its own name (Law 4). A bank with no view of the name does not bid at all.
  const own = view.lastOwn('bank.reservation');
  if (!own.some) return none<number>();
  const loss = pick(own.value.data['expectedLoss'], borrower);
  const capital = pick(own.value.data['capitalCost'], borrower);
  if (loss === undefined || capital === undefined) return none<number>();
  return some(add(c.floor, add(loss, capital, 'what the name costs it'), 'what it wants for it'));
}

function pick(map: unknown, about: PartyId): number | undefined {
  if (typeof map !== 'object' || map === null) return undefined;
  const v = (map as Record<string, unknown>)[about];
  return typeof v === 'number' ? v : undefined;
}

/**
 * Money Market A3, B1, B4, C4.a: WHAT THIS BANK POSTS IN ONE BOOK OF ONE SESSION.
 *
 * The venue says which book it is and whose name is borrowing; everything else is the bank's own
 * state. As a LENDER it offers what it has spare — its account less what it holds against a bad
 * week — at what that name is worth to it. As the BORROWER it bids for what it is short of plus
 * what it has to repay tomorrow, and it will pay up to the window, because the window is what it
 * does instead (C4.a); in a secured book it never asks for more than its own paper could cover
 * (C4.b). It posts at most one order on one side, which is what one face to one market means.
 */
export function sessionOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.key['market'] !== 'money' || !view.self.status.alive) return [];
  const borrower = venue.key['borrower'];
  if (borrower === undefined) return [];
  const secured = venue.key['secured'] === 'true';
  const seen = corridorSeen(view);
  if (!seen.some) return [];
  const c = seen.value;
  const ccy = venue.ccy;
  const self = view.self.id;
  const p = positionOf(view, ccy);
  if (String(self) === borrower) {
    const need = downTick(add(-p.gap, fallsDueNext(view, ccy), 'what it has to raise'));
    if (need <= 0) return [];
    const power = secured ? pledgeable(view) : need;
    const size = power < need ? power : need;
    return size > 0 ? [{ party: self, side: 'buy', price: c.ceiling, qty: size }] : [];
  }
  if (p.gap <= 0) return [];
  const wants = lenderReservation(view, secured, partyId(borrower), c);
  if (!wants.some) return [];
  return [{ party: self, side: 'sell', price: wants.value, qty: p.gap }];
}

/** The classes of depositor this world has, as the market publishes them (Banks Funding A1.a). */
export interface DepositClassSeen {
  readonly id: string;
  readonly insured: boolean;
  /** Banks Capital D4: what the guarantee on this class costs the bank that funds itself with it. */
  readonly premium: number;
}

/**
 * A1.a, E1: the segments a bank prices its board against, READ off what the market published.
 *
 * How depositors are grouped is a fact about DEPOSITORS and not about any bank — every bank faces
 * the same segments, which is why a rival's board is something this bank's own depositors can
 * compare with (E2.a). A bank keeping its own copy of the taxonomy would be a bank pricing against
 * a market that only it can see.
 *
 * WHAT IT COSTS ONE OF THEM TO MOVE IS NOT HERE, and that is the point: it is an amount of money
 * the depositor pays, weighed against its own balance (A1.d, E1), and a bank does not know it. What
 * the bank knows is what its own base has done, which is the contested share below.
 */
export function classesSeen(ctx: MechanismContext): readonly DepositClassSeen[] {
  const said = ctx.journal.ofKind('deposit.classes');
  const last = said[said.length - 1];
  const rows = last?.data['classes'];
  if (!Array.isArray(rows)) return [];
  const out: DepositClassSeen[] = [];
  for (const r of rows) {
    if (typeof r !== 'object' || r === null) continue;
    const { id, insured, premium } = r as Record<string, unknown>;
    if (typeof id !== 'string' || typeof insured !== 'boolean') continue;
    if (typeof premium !== 'number') continue;
    out.push({ id, insured, premium });
  }
  return out;
}

/**
 * B1.a, A5, D3, D5.a: THE BOARD. What this bank decides to pay each class of depositor, from what
 * money is worth to it, what it keeps for itself, and what its rivals are paying in public.
 *
 * It is public because that is the whole of why it works: a depositor moves for it (E1), and a
 * rival bank sees it and answers (E2.a).
 */
export function setBoard(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  classes: readonly DepositClassSeen[],
  base: number,
): void {
  const seen = corridorSeen(ctx.participant(bank));
  if (!seen.some) return;
  const worth = worthOfMoney(ctx, bank, seen.value, base);
  const margin = ctx.params.get(bankParam(bank, 'depositMargin'));
  // B1.a, B3: what it will pay is what the money is worth to it, less what it keeps and less what
  // the money costs it besides the rate. IT DOES NOT SUBTRACT ANYBODY'S STICKINESS: what it costs a
  // depositor to move is an amount of that depositor's money, weighed against that depositor's
  // balance (A1.d, E1), and a bank does not know either number. It used to be subtracted here as
  // though it were a rate, which made the board a function of a cost in a different unit and gave
  // every bank a per-class discount nobody had bid for and nobody had paid.
  const net = sub(worth, margin, 'what it keeps');
  const rates: Record<string, number> = {};
  // A1, Banks Capital D4: AND THE THREE CLASSES ARE NOT ONE RATE, for a cost this bank actually
  // bears. Insured money carries a premium the bank pays the insurer on top of what it pays the
  // depositor, so a bank whose all-in cost of funds is the same either way offers the insured class
  // less by exactly the premium. That is why retail is paid under wholesale, and it is a real
  // payment out of this bank's account (`insurance.premium`) rather than a stated stickiness.
  for (const cls of classes) {
    const own = sub(net, cls.premium, `what the guarantee on ${cls.id} costs it`);
    rates[cls.id] = defended(bestRival(ctx, bank, cls.id), own, worth);
  }
  ctx.record('bank.depositRate', [bank], { bank, ccy, rates }, true);
}

/** The keenest board anybody else is showing this class, off what they published (Law 19). */
function bestRival(ctx: MechanismContext, bank: PartyId, cls: string): Option<number> {
  let best: number | undefined;
  for (const e of ctx.journal.ofKind('bank.depositRate')) {
    if (e.subjects.includes(bank)) continue;
    const rates = e.data['rates'];
    if (typeof rates !== 'object' || rates === null) continue;
    const rate = (rates as Record<string, unknown>)[cls];
    if (typeof rate !== 'number') continue;
    if (best === undefined || rate > best) best = rate;
  }
  return best === undefined ? none<number>() : some(best);
}

/**
 * C2.a, F2, Observer A5: what its own account did to it this week, and what it therefore holds
 * against a bad one. Published, because the balance either side of every leg of it already is, and
 * because everything that decides anything about this bank's funding reads this and nothing else.
 */
export function publishBuffer(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  memory: ReserveMemory,
): void {
  const move = reserveFlow(ctx, bank, ccy);
  remember(memory, bank, move, ctx.params.get(bankParam(bank, 'bufferMemory')));
  ctx.record(
    'bank.buffer',
    [bank],
    { bank, ccy, move, buffer: bufferOf(memory, bank), reserves: ctx.register.quantity(bank, moneyInstrumentId(ctx.registry.centralBankOf(ccy), ccy)) },
    true,
  );
}

/**
 * Banks Funding F1: what this bank has issued of its own money and somebody else is holding — its
 * whole deposit base, as one number, read off the instrument rather than summed from a copy.
 */
export function ownDeposits(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): number {
  const money = moneyInstrumentId(bank, ccy);
  if (!ctx.instruments.has(money)) return 0;
  return sub(ctx.instruments.get(money).issued, ctx.register.quantity(bank, money), 'its base');
}
