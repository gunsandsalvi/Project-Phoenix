/**
 * The money market: where a bank's reserve position, which nobody chose, meets somebody else's.
 *
 * @spec Money Market A1 Money Market A1.a Money Market A1.b Money Market A2 Money Market A2.a Money Market A3 Money Market A3.a Money Market B1 Money Market B2 Money Market B2.a Money Market B3 Money Market B3.a Money Market B3.b Money Market B3.c Money Market B4 Money Market B5 Money Market B5.a Money Market B6 Money Market B7 Money Market C1 Money Market C1.a Money Market C2 Money Market C4 Money Market C4.a Money Market C4.b Money Market C5 Money Market D1 Money Market D2 Money Market D3 Money Market E1 Banks Funding A1 Banks Funding A2 Banks Funding A2.a Banks Funding A5 Banks Funding B1 Banks Funding B1.a Banks Funding B1.b Banks Funding B2 Banks Funding B2.b Banks Funding C1 Banks Funding C2 Banks Funding C2.a Banks Funding C4 Banks Funding D1 Banks Funding D3 Banks Funding F1 Banks Funding F2 Banks Funding F4 Central Bank B1 Central Bank B2 Central Bank B3 Central Bank B3.a Central Bank D1 Central Bank D2 Law 2 Law 4 Law 15 XI-14
 *
 * E1, Central Bank B3: THE POLICY RATE REACHES THE ECONOMY THROUGH THIS MARKET. Nothing here
 * assigns it to anything. The central bank declares two levels — what it pays for cash it takes in
 * and what it charges for cash it lends against paper — and takes both sides at those levels for
 * real quantities on its own balance sheet. Every other participant then has an alternative it can
 * actually take, and the rate that clears sits between them because of that and not because
 * anything clamps it (B3.a: the policy rate is never a cleared rate; C3 is a read, at 16).
 */
import type { Family, Violation } from '../../audit/audit.js';
import { period as asPeriod } from '../../calendar/calendar.js';
import type { Civil } from '../../calendar/civil.js';
import type { Order } from '../../clearing/solver.js';
import {
  currencyUnit,
  moneyInstrumentId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { add, div, dustOf, material, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK, CENTRAL_BANK } from '../../registry/profiles.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { ParamDecl } from '../../registry/params.js';
import { pledgeable, windowAdvances, type Advance } from './collateral.js';
import {
  BOOKS,
  DEPOSIT_CLASSES,
  FUNDERS,
  MM_PARAMS,
  classOf,
  funderOf,
  mmParam,
  switchingCost,
  type BookDecl,
} from './data.js';
import {
  bufferOf,
  depositBase,
  rateFor,
  depositsByClass,
  emptyDeposits,
  liquidityMetric,
  moveDeposits,
  payDepositInterest,
  rememberReserves,
  setRates,
  type DepositBook,
} from './deposits.js';
import { interbankKind, isRow, repoKind, rowTerms, INTERBANK, REPO } from './rows.js';
import {
  averageRate,
  banksOf,
  bankOrders,
  collateralFor,
  corridorOf,
  coverFor,
  floorBid,
  netReserveFlow,
  offered,
  positionOf,
  sessionVenue,
  stillNeeded,
  strike,
  venuesOf,
  windowOffer,
  writeRow,
  type Corridor,
  type Position,
} from './session.js';

export * from './data.js';
export * from './rows.js';
export * from './collateral.js';
export * from './deposits.js';
export * from './session.js';

/** What the module keeps: the deposit book, and how many rows it has written. */
interface Market {
  readonly deposits: DepositBook;
  next: number;
}

function market(ctx: MechanismContext): Market {
  return ctx.state<Market>('market', () => ({ deposits: emptyDeposits(), next: 1 }));
}

const ccyOf = (ctx: MechanismContext, party: PartyId): CurrencyCode =>
  ctx.registry.region(ctx.parties.get(party).region).ccy;

function corridor(ctx: MechanismContext): Corridor {
  return corridorOf(
    ctx.params.get(MM_PARAMS.policyRate),
    ctx.params.get(MM_PARAMS.floorSpread),
    ctx.params.get(MM_PARAMS.ceilingSpread),
  );
}

/**
 * B3.c, Register D5: a row that has been repaid does not still hold its collateral. The kernel
 * redeems the row where every other dated claim is redeemed (Register E2); the lien it stood
 * behind is this module's to end, because this module is the one that bound it, and it ends in the
 * same cycle the money came back so the paper can stand behind something else the same period.
 */
function freeRepaidCollateral(ctx: MechanismContext): void {
  for (const i of ctx.instruments.all()) {
    if (!isRow(i.terms) || i.terms.collateral.length === 0 || i.status.live) continue;
    const borrower = i.terms.borrower;
    for (const c of i.terms.collateral) {
      const holding = ctx.register.holding(borrower, c.instrument);
      if (!holding.some) continue;
      for (const lien of holding.value.liens.filter((l) => l.reason === String(i.id))) {
        ctx.settle({
          legs: [
            {
              kind: 'release',
              pledgor: borrower,
              beneficiary: i.terms.lender,
              instrument: c.instrument,
              lien: lien.id,
            },
          ],
          cause: 'transfer',
          reason: `${i.id} is repaid and ${borrower} paper is its own again`,
        });
      }
    }
  }
}

/**
 * B1.a, D2: every bank decides what it pays each class of depositor, and pays it. Its alternative
 * — what the market charged IT last time it borrowed — is read off its own rows, so the comparison
 * is against a number somebody actually charged it and never against an assumption.
 */
function setAndPayDeposits(ctx: MechanismContext): void {
  const m = market(ctx);
  const c = corridor(ctx);
  const banks = banksOf(ctx);
  for (const bank of banks) {
    const ccy = ccyOf(ctx, bank);
    payDepositInterest(ctx, m.deposits, bank, ccy);
    setRates(ctx, m.deposits, bank, ccy, worthOfMoney(ctx, bank, c));
    const rates: Record<string, number> = {};
    for (const cls of DEPOSIT_CLASSES) {
      const rate = rateFor(m.deposits, bank, cls);
      if (rate.some) rates[cls.id] = rate.value;
    }
    // D5.a, E2.a: A RATE PAID UP IS AN OBSERVABLE. It is public because that is the whole of why it
    // works: a depositor moves for it, and a rival bank sees it and answers.
    ctx.record('bank.depositRate', [bank], { bank, ccy, rates }, true);
  }
  // E1: and then the depositors answer, which is the only thing that stops a bank paying less.
  moveDeposits(ctx, banks, ctx.params.get(MM_PARAMS.insuranceLimit));
}

/**
 * B1.a, B5.a: what money is worth to this bank — the dearest rate the market charged it at the last
 * session if it borrowed there, and the floor if it did not. A deposit it keeps is a row it does
 * not have to write, so what it will pay for one is what the row would have cost.
 *
 * It is read from the session's own prints (Law 19) rather than from the rows, because an overnight
 * row has already been repaid by the time a bank sets its rates: the row is gone and what it cost
 * is not.
 */
export function worthOfMoney(ctx: MechanismContext, bank: PartyId, c: Corridor): number {
  if (ctx.period === 0) return c.floor;
  const last = asPeriod(ctx.period - 1);
  let dearest: number | undefined;
  for (const e of ctx.journal.ofKind('moneyMarket.print')) {
    if (e.period !== last || e.data['borrower'] !== bank) continue;
    const rate = e.data['rate'];
    if (typeof rate !== 'number') continue;
    if (dearest === undefined || rate > dearest) dearest = rate;
  }
  return dearest ?? c.floor;
}

/** The banks' positions after the flows, and what each one's own week has taught it (A1, C2.a). */
function positions(ctx: MechanismContext): Map<PartyId, Position> {
  const m = market(ctx);
  const out = new Map<PartyId, Position>();
  for (const bank of banksOf(ctx)) {
    const ccy = ccyOf(ctx, bank);
    const decl = funderOf(bank);
    if (decl === undefined) continue;
    rememberReserves(
      m.deposits,
      bank,
      netReserveFlow(ctx, bank, ccy),
      ctx.params.get(mmParam(bank, 'bufferMemory')),
    );
    out.set(bank, positionOf(ctx, bank, ccy, bufferOf(m.deposits, bank)));
  }
  return out;
}

/**
 * A3, B1, B4: the session itself. Every bank posts into every name's book — what it will lend and
 * what it wants for it — the window takes its seat, and each name's books clear cheapest first,
 * because a treasurer funds where the money is cheapest and stops when it has enough.
 */
function runSession(ctx: MechanismContext): void {
  const m = market(ctx);
  const on = ctx.calendar.startOf(ctx.period);
  const c = corridor(ctx);
  const pos = positions(ctx);
  const banks = banksOf(ctx);
  declareVenues(ctx, banks);

  for (const borrower of banks) {
    const p = pos.get(borrower);
    if (p === undefined || p.gap >= 0) continue;
    const ccy = ccyOf(ctx, borrower);
    const cb = ctx.registry.centralBankOf(ccy);
    let need = -p.gap;
    // Every lender's schedule for this name goes into the book before anything clears, so what a
    // borrower chooses between is what was actually posted (Clearing C5) and not what it guessed.
    for (const book of BOOKS) {
      const venue = sessionVenue(book, borrower);
      for (const lender of banks) {
        if (lender === borrower) continue;
        const lenderPos = pos.get(lender);
        if (lenderPos === undefined) continue;
        for (const o of bankOrders(ctx.participant(lender), book, borrower, lenderPos, 0, c, 0)) {
          ctx.post(venue, o);
        }
      }
      const haircut = ctx.params.get(MM_PARAMS.overdraftPenalty);
      for (const o of windowOffer(
        ctx.participant(cb),
        ctx.participant(borrower),
        book,
        c,
        on,
        haircut,
      )) {
        ctx.post(venue, o);
      }
    }
    for (const book of cheapestFirst(ctx, borrower)) {
      if (need <= 0) break;
      const venue = sessionVenue(book, borrower);
      // C4.b: it asks for what its own paper could cover, because in a secured book that is the
      // most it could possibly raise — and in an unsecured one this number is not consulted at all.
      const power = pledgeable(ctx.participant(borrower), on);
      const bid = bankOrders(ctx.participant(borrower), book, borrower, p, need, c, power);
      for (const o of bid) ctx.post(venue, o);
      const raised = clearBook(ctx, m, book, borrower, ccy, on, ctx.posted(venue));
      need = stillNeeded(need, raised);
    }
    if (need > 0) {
      // B7, B2.a: the market did not clear for this name. It is an outcome of real schedules — no
      // lender would have it at a rate it would pay, or it had nothing left to pledge — and the
      // consequence lands where it falls due (D4).
      ctx.record(
        'moneyMarket.refused',
        [borrower],
        { borrower, short: need, ccy, reserves: p.reserves, buffer: p.buffer },
        true,
      );
    }
  }
  parkTheRest(ctx, pos);
}

/** The books this borrower can reach, cheapest posted ask first: the treasurer's own preference. */
function cheapestFirst(ctx: MechanismContext, borrower: PartyId): readonly BookDecl[] {
  const priced = BOOKS.map((book) => ({ book, best: bestAsk(ctx.posted(sessionVenue(book, borrower))) }));
  return priced
    .filter((x) => x.best.some)
    .sort((a, b) => (a.best.some && b.best.some ? a.best.value - b.best.value : 0))
    .map((x) => x.book);
}

function bestAsk(orders: readonly Order[]): Option<number> {
  let best: number | undefined;
  for (const o of orders) {
    if (o.side !== 'sell' || o.price === 'market') continue;
    if (best === undefined || o.price < best) best = o.price;
  }
  return best === undefined ? none<number>() : some(best);
}

/** What one book struck for one name, written as rows and published as a print (B4, C3). */
function clearBook(
  ctx: MechanismContext,
  m: Market,
  book: BookDecl,
  borrower: PartyId,
  ccy: CurrencyCode,
  on: Civil,
  posted: readonly Order[],
): number {
  const struck = strike(posted, borrower, book, ctx.registry.tick(currencyUnit(ccy)));
  if (struck.length === 0) return 0;
  const raised: number[] = [];
  for (const s of struck) {
    // Law 7: a row for the dust of the clearing is not a row. Writing one would put an instrument
    // in the register, a lien on a rounding and a payment of nothing on the wire.
    if (!material(s.amount, struck.length + 1, sum(struck.map((x) => x.amount)).value)) continue;
    let cover: readonly { instrument: InstrumentId; qty: number; valuedAt: number }[] = [];
    let amount = s.amount;
    if (book.secured) {
      cover = coverFor(advancesFrom(ctx, s.lender, borrower, on), s.amount);
      // B3.c: what its remaining paper covers is what it gets, at THIS lender's valuation of it.
      // Less than it asked for is the constraint biting, not a failure of the session — and what
      // it gets is a whole number of pieces of money (Law 8), because that is what is lent.
      const covered = sum(cover.map((x) => mul(x.qty, x.valuedAt, 'covered'))).value;
      amount = ctx.registry.payable(ccy, covered > s.amount ? s.amount : covered);
      if (amount <= 0) continue;
      cover = coverFor(advancesFrom(ctx, s.lender, borrower, on), amount);
    }
    const n = m.next;
    // The count moves whether or not the row settles: a row that failed still took its name, and
    // handing the same name to the next one would be two rows with one identity (Law 4).
    m.next += 1;
    const id = writeRow(ctx, { ...s, amount }, n, ccy, cover);
    if (!id.some) continue;
    raised.push(amount);
  }
  const total = sum(raised).value;
  if (total > 0) {
    ctx.record(
      'moneyMarket.print',
      [borrower],
      {
        book: book.id,
        tenor: book.tenor,
        secured: book.secured,
        borrower,
        rate: struck[0]?.rate,
        volume: total,
        lenders: struck.length,
        ccy,
      },
      true,
    );
  }
  return total;
}

/**
 * Whose valuation the collateral is taken at. There are two collateral policies in this world and
 * they belong to different parties: a market lender advances what the paper is worth TO IT at its
 * own required yield (B3.b), and the central bank advances the market's price less the haircut it
 * has DECLARED, because eligibility and haircuts are its choice and an instrument of policy in
 * themselves (Central Bank D2). Which one applies is not a question about what kind of party the
 * lender is: it is whether the lender is the issuer of the money being lent, which is the same
 * distinction settlement itself makes when it decides whether reserves move (Money C2.a).
 */
function advancesFrom(
  ctx: MechanismContext,
  lender: PartyId,
  borrower: PartyId,
  on: Civil,
): readonly Advance[] {
  const cb = ctx.registry.centralBankOf(ccyOf(ctx, borrower));
  return lender === cb
    ? windowAdvances(
        ctx.participant(cb),
        ctx.participant(borrower),
        on,
        ctx.params.get(MM_PARAMS.overdraftPenalty),
      )
    : collateralFor(ctx, lender, borrower, on);
}

/**
 * C1, C1.a: what nobody borrowed is parked at the floor, and parking it TAKES IT OUT OF THE
 * BANKING SYSTEM: the money leg's payee is the issuer of the money, so the reserves are destroyed
 * where they land (Money C4). The central bank owes it back tomorrow with the floor's interest,
 * which is a row on both balance sheets and not a bookkeeping move.
 */
function parkTheRest(ctx: MechanismContext, pos: ReadonlyMap<PartyId, Position>): void {
  const m = market(ctx);
  const c = corridor(ctx);
  const overnight = BOOKS.find((b) => b.tenor === 'overnight' && !b.secured);
  if (overnight === undefined) return;
  for (const [bank, p] of pos) {
    const ccy = ccyOf(ctx, bank);
    const cb = ctx.registry.centralBankOf(ccy);
    const spare = sub(p.reserves, add(p.buffer, lentThisPeriod(ctx, bank), 'placed'), 'spare');
    if (spare <= 0) continue;
    const venue = sessionVenue(overnight, cb);
    const ask: Order = { party: bank, side: 'sell', price: c.floor, qty: spare };
    ctx.post(venue, ask);
    for (const o of floorBid(cb, offered(ctx.posted(venue)), c)) ctx.post(venue, o);
    const struck = strike(ctx.posted(venue), cb, overnight, ctx.registry.tick(currencyUnit(ccy)));
    for (const s of struck) {
      if (s.lender !== bank || !material(s.amount, 2, p.reserves)) continue;
      const n = m.next;
      m.next += 1;
      const id = writeRow(ctx, s, n, ccy, []);
      if (!id.some) continue;
      ctx.record(
        'centralBank.parked',
        [bank, cb],
        { bank, amount: s.amount, rate: s.rate, ccy },
        true,
      );
    }
  }
}

/** What this bank has already placed in this period's session, so it does not place it twice. */
function lentThisPeriod(ctx: MechanismContext, lender: PartyId): number {
  const terms: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isRow(i.terms) || i.terms.lender !== lender) continue;
    if (ctx.calendar.place(i.terms.drawn) !== ctx.period) continue;
    terms.push(ctx.register.quantity(lender, i.id));
  }
  return sum(terms).value;
}

function declareVenues(ctx: MechanismContext, banks: readonly PartyId[]): void {
  const ccy = banks[0] === undefined ? undefined : ccyOf(ctx, banks[0]);
  if (ccy === undefined) return;
  const cb = ctx.registry.centralBankOf(ccy);
  for (const v of venuesOf(ccy, [...banks, cb])) {
    if (ctx.venues.some((x) => x.id === v.id)) continue;
    ctx.openVenue(v);
  }
}

/**
 * F1, F2, F4: what a bank's funding actually looks like, published every period. F2 is the reserve
 * balance as a READ OF ITS ACCOUNT — one row at the central bank, moved only by settlement legs,
 * never a mirrored copy — and F4 is the metric somebody outside can see, which is what D5.a's
 * depositor and E2.a's rival bank are watching.
 */
function publishFunding(ctx: MechanismContext): void {
  const m = market(ctx);
  for (const bank of banksOf(ctx)) {
    const ccy = ccyOf(ctx, bank);
    const cb = ctx.registry.centralBankOf(ccy);
    const byClass = depositsByClass(ctx, bank, ccy);
    const reserves = ctx.register.quantity(bank, moneyInstrumentId(cb, ccy));
    const buffer = bufferOf(m.deposits, bank);
    const metric = liquidityMetric(reserves, depositBase(byClass));
    ctx.record(
      'bank.liquidity',
      [bank],
      {
        bank,
        ccy,
        reserves,
        buffer,
        deposits: Object.fromEntries(byClass),
        base: depositBase(byClass),
        metric: metric.some ? metric.value : null,
      },
      true,
    );
  }
}

/** B2, Central Bank B2: the two levels, declared. They are administered, and this says so. */
function publishCorridor(ctx: MechanismContext): void {
  const c = corridor(ctx);
  ctx.record(
    'centralBank.corridor',
    [],
    { policy: c.policy, floor: c.floor, ceiling: c.ceiling },
    true,
  );
}

function paramsOf(): ParamDecl[] {
  return [
    {
      id: MM_PARAMS.policyRate,
      value: 0.02,
      unit: 'per annum',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank B1, B2: the rate it declares. It is administered and not traded (B2), and it is the one price in this world that is not cleared — Law 3 allows exactly this one, because the quantity response is real and booked on both balance sheets. What it is set AGAINST is its mandate (B1.a), and the mandate is parliament (worklist 14).',
    },
    {
      id: MM_PARAMS.floorSpread,
      value: 0.001,
      unit: 'per annum below the policy rate',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Money Market C1, C3: what it pays for cash it takes in, and half of the corridor whose WIDTH is a policy choice. Narrow, because a floor far below the policy rate lets the market rate wander and B4 stops being informative.',
    },
    {
      id: MM_PARAMS.ceilingSpread,
      value: 0.005,
      unit: 'per annum above the policy rate',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Money Market C2, C4: what it charges to lend against paper. Above the policy rate so a bank prefers the market and drawing is information (C4.a), and wider than the floor spread because the window is meant to be the dearer answer.',
    },
    {
      id: MM_PARAMS.overdraftPenalty,
      value: 0.05,
      unit: 'ratio of the market price',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank D2, D3: the haircut the window takes on the paper it lends against. Eligibility and haircuts are its choice and a policy instrument in themselves, which is why this is a policy and not a preference of anybody.',
    },
    {
      id: MM_PARAMS.insuranceLimit,
      value: 0.1,
      unit: 'PHX per member',
      kind: 'policy',
      owner: 'parliament',
      why: 'Banks Funding A1.a, Banks Capital D4: what is insured, PER MEMBER of a cell (XI-15). It is what makes E4 break the run loop for retail money and not for wholesale, and it is a rule somebody wrote — parliament owns it from worklist 14.',
    },
    ...DEPOSIT_CLASSES.map((c) => ({
      id: switchingCost(c.id),
      value: c.switchingCost,
      unit: 'per annum',
      kind: 'preference' as const,
      owner: 'model' as const,
      why: `Banks Funding A1.d, E1: what it costs a ${c.id} depositor to move its account. ${c.why}`,
    })),
    ...FUNDERS.flatMap((f) => [
      {
        id: mmParam(f.bank, 'depositMargin'),
        value: f.depositMargin,
        unit: 'per annum',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Banks Funding B1.a, B3: what ${f.bank} keeps for itself out of what the money it takes in is worth to it. ${f.why}`,
      },
      {
        id: mmParam(f.bank, 'bufferMemory'),
        value: f.bufferMemory,
        unit: 'periods',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Money Market A2.a, Banks Funding C2.a: how far back ${f.bank} looks at its own account when it decides what to hold against what could leave. The buffer is derived from what it has actually seen, never from a ratio of its deposits.`,
      },
    ]),
  ];
}

/**
 * B3.c: every lien this module bound stands behind a live row, and every live secured row's
 * collateral is still bound. Collateral that outlived its row is paper somebody cannot sell for no
 * reason; a row whose collateral was released is an unsecured claim calling itself secured.
 */
function collateralHolds(): Family {
  return {
    name: 'ownership',
    contributor: 'money-market',
    spec: 'Money Market B3.c Register D5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const live = new Set<string>();
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isRow(i.terms) || i.terms.collateral.length === 0) continue;
        // A row that was never drawn secures nothing: the money never moved, so the paper it would
        // have stood behind was never bound (Register D5).
        if (i.issued <= 0) continue;
        live.add(String(i.id));
        for (const c of i.terms.collateral) {
          const holding = view.register.holding(i.terms.borrower, c.instrument);
          const bound = holding.some
            ? sum(
                holding.value.liens.filter((l) => l.reason === String(i.id)).map((l) => l.qty),
              ).value
            : 0;
          // Law 7: what is bound is a sum over liens and what the row says is a sum over parcels,
          // so the comparison is entitled to the dust of both walks and to nothing else.
          if (!withinDust(bound, c.qty, dustOf(2, Math.abs(bound) + Math.abs(c.qty)))) {
            out.push({
              family: 'ownership',
              spec: 'Money Market B3.c',
              owner: String(i.id),
              size: sub(c.qty, bound, 'unbound collateral'),
              unit: 'units',
              period: view.period,
              message: `${i.id} says it is secured on ${c.qty} of ${c.instrument} and ${bound} is bound`,
            });
          }
        }
      }
      for (const h of view.register.allHoldings()) {
        for (const l of h.liens) {
          if (!l.reason.startsWith('repo:') || live.has(l.reason)) continue;
          out.push({
            family: 'ownership',
            spec: 'Register D5',
            owner: h.holder,
            size: l.qty,
            unit: 'units',
            period: view.period,
            message: `${h.holder} still has ${l.qty} of ${h.instrument} bound to ${l.reason}, which is not a live row`,
          });
        }
      }
      return out;
    },
  };
}

export const moneyMarket: SystemModule = {
  id: 'money-market',
  spec: 'Money Market, Banks Funding, Central Bank B, D',
  // It reads what a bank published about its own economics (its cost of funds, what it requires of
  // a name) and it lends against sovereign paper. Both arrive as public events and prints, so what
  // it needs is that those modules are there — not their code (Law 15).
  requires: ['bank-lending', 'sovereign-instruments'],
  instrumentKinds: [interbankKind, repoKind],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: paramsOf(),
  phases: [
    {
      name: 'moneyMarket.rates',
      spec: 'Banks Funding B1 Banks Funding B1.a Money Market D2',
      cycle: 0,
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        freeRepaidCollateral(ctx);
        publishCorridor(ctx);
        setAndPayDeposits(ctx);
      },
    },
    {
      name: 'moneyMarket.clear',
      spec: 'Money Market A3 Money Market B1 Money Market B4 Money Market C1 Money Market C2',
      // A3.a: THE SESSION IS AFTER THE FLOWS — the markets, the wages that settle, the invoices,
      // the fund subscriptions that move money between banks — because the need is not knowable
      // until they have happened. It sits in the last cycle of the period, at the seam just ahead
      // of the lending module's own booking phase, which is a phase this module already depends on
      // being there (`requires`).
      cycle: 'anchor',
      anchor: { before: 'lending.book' },
      run: (ctx: MechanismContext): void => {
        runSession(ctx);
        publishFunding(ctx);
      },
    },
  ],
  participants: [],
  families: [collateralHolds()],
  seed(ctx: SeedContext): void {
    const banks = ctx.parties.ofKind(BANK).map((b) => b.id);
    const first = banks[0];
    if (first === undefined) return;
    const ccy = ctx.registry.region(ctx.parties.get(first).region).ccy;
    for (const v of venuesOf(ccy, [...banks, ctx.registry.centralBankOf(ccy)])) ctx.openVenue(v);
  },
};

/** What a party of this kind is to a bank that funds itself, for the reads that need it. */
export function depositClassOf(ctx: MechanismContext, party: PartyId): Option<string> {
  const cls = classOf(ctx.parties.get(party).kind);
  return cls === undefined ? none<string>() : some(cls.id);
}

/** The rate that cleared for a name in a book this period, for a reader (C3, B6.a). */
export function printed(ctx: MechanismContext, borrower: PartyId): Option<number> {
  const rows = ctx.journal
    .ofKind('moneyMarket.print')
    .filter((e) => e.period === ctx.period && e.subjects.includes(borrower));
  const rates = rows
    .map((e) => e.data['rate'])
    .filter((r): r is number => typeof r === 'number');
  return rates.length === 0 ? none<number>() : some(div(sum(rates).value, rates.length, 'rate'));
}

export { averageRate, CENTRAL_BANK, INTERBANK, REPO, rowTerms };
