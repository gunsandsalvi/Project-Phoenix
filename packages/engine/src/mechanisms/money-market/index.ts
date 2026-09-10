/**
 * The money market: where a bank's reserve position, which nobody chose, meets somebody else's.
 *
 * @spec Money Market A1 Money Market A1.a Money Market A1.b Money Market A2 Money Market A2.a Money Market A3 Money Market A3.a Money Market B1 Money Market B2 Money Market B2.a Money Market B3 Money Market B3.a Money Market B3.b Money Market B3.c Money Market B4 Money Market B5 Money Market B5.a Money Market B6 Money Market B7 Money Market C1 Money Market C1.a Money Market C2 Money Market C4 Money Market C4.a Money Market C4.b Money Market C5 Money Market D1 Money Market D2 Money Market D3 Money Market C5 Money Market D4 Money Market D5 Money Market D5.a Money Market D6 Money Market E1 Money Market E3 Banks Funding A1 Banks Funding A2 Banks Funding A2.a Banks Funding A5 Banks Funding B1 Banks Funding B1.a Banks Funding B1.b Banks Funding B2 Banks Funding B2.b Banks Funding C1 Banks Funding C2 Banks Funding C2.a Banks Funding C4 Banks Funding D1 Banks Funding D3 Banks Funding F1 Banks Funding F2 Banks Funding F4 Central Bank B1 Central Bank B2 Central Bank B3 Central Bank B3.a Central Bank B4 Central Bank D1 Central Bank D2 Central Bank D3.a Central Bank D4 Money B3.c Law 2 Law 4 Law 15 XI-14
 *
 * E1, Central Bank B3: THE POLICY RATE REACHES THE ECONOMY THROUGH THIS MARKET. Nothing here
 * assigns it to anything. The central bank declares two levels — what it pays for cash it takes in
 * and what it charges for cash it lends against paper — and takes both sides at those levels for
 * real quantities on its own balance sheet. Every other participant then has an alternative it can
 * actually take, and the rate that clears sits between them because of that and not because
 * anything clamps it (B3.a: the policy rate is never a cleared rate; C3 is a read, at 16).
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { Civil } from '../../calendar/civil.js';
import type { Order } from '../../clearing/solver.js';
import {
  currencyUnit,
  moneyInstrumentId,
  partyId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { add, div, dustOf, material, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { downTick } from '../../core/tick.js';
import type { OverdraftContext, OverdraftDecision } from '../../registry/kinds.js';
import { BANK, CENTRAL_BANK } from '../../registry/profiles.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { ParamDecl } from '../../registry/params.js';
import { borrowingPower, windowAdvances, type Advance } from './collateral.js';
import { collectPremiums, DEPOSIT_INSURER, INSURER, INSURER_PARAMS, insurerKind } from './insurer.js';
import { failedBanks, nothingLeftBehind, resolve } from './resolution.js';
import {
  BOOKS,
  DEPOSIT_CLASSES,
  MM_PARAMS,
  classOf,
  switchingCost,
  type BookDecl,
} from './data.js';
import {
  couldLeave,
  depositBase,
  depositsByClass,
  liquidityMetric,
  moveDeposits,
  payDepositInterest,
} from './deposits.js';
import { interbankKind, isRow, repoKind, rowTerms, INTERBANK, REPO } from './rows.js';
import {
  averageRate,
  banksOf,
  fallsDueToIt,
  collateralFor,
  corridorOf,
  coverFor,
  floorBid,
  netReserveFlow,
  offered,
  sessionVenue,
  stillNeeded,
  strike,
  venuesOf,
  windowOffer,
  writeRow,
  type Corridor,
} from './session.js';

export * from './data.js';
export * from './rows.js';
export * from './collateral.js';
export * from './deposits.js';
export * from './session.js';
export { DEPOSIT_INSURER, INSURER, INSURER_PARAMS } from './insurer.js';
export { valueBook, failedBanks, type Valuation as BookValuation } from './resolution.js';

/** What the module keeps: the deposit book, and how many rows it has written. */
interface Market {
  next: number;
  /** Money B3.b: the accounts the central bank let go below zero, waiting to become rows. */
  overdrawn: { bank: PartyId; ccy: CurrencyCode }[];
}

function market(ctx: MechanismContext): Market {
  return ctx.state<Market>('market', () => ({ next: 1, overdrawn: [] }));
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
    // Register F2, Money E4: the borrower can have CEASED since it pledged — a resolution moves a
    // bank's whole book, collateral and all, to whoever succeeded it — and the lien is on the
    // successor's holding now. A leg addressed to the dead party would be a defect in this module.
    const borrower = ctx.parties.resolve(i.terms.borrower).id;
    const beneficiary = ctx.parties.resolve(i.terms.lender).id;
    // Register D5, Banks Capital D6: and both ends of the row can resolve to the SAME party, when
    // one bank's resolution put the borrower's book into the lender's hands. There is nothing left
    // to free: a party does not hold security over its own paper, and the resolution ended the lien
    // when it took the book (nobody pledges to itself).
    if (borrower === beneficiary) continue;
    for (const c of i.terms.collateral) {
      const holding = ctx.register.holding(borrower, c.instrument);
      if (!holding.some) continue;
      for (const lien of holding.value.liens.filter((l) => l.reason === String(i.id))) {
        ctx.settle({
          legs: [
            {
              kind: 'release',
              pledgor: borrower,
              beneficiary,
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
function payDeposits(ctx: MechanismContext): void {
  const banks = banksOf(ctx);
  for (const bank of banks) payDepositInterest(ctx, bank, ccyOf(ctx, bank));
  // E1: and then the depositors answer, which is the only thing that stops a bank paying less.
  moveDeposits(ctx, banks);
}

/**
 * Banks Funding A1.a, E1: the segments, PUBLISHED. How depositors are grouped and what it costs one
 * of them to move are facts about depositors, and every bank prices its board against the same
 * ones — so they are said out loud once, by the market that holds the taxonomy, and read by anybody
 * who needs them (Law 4). A bank keeping its own copy would price against a market only it can see.
 */
function publishClasses(ctx: MechanismContext): void {
  ctx.record(
    'deposit.classes',
    [],
    {
      classes: DEPOSIT_CLASSES.map((c) => ({
        id: c.id,
        insured: c.insured,
        switchingCost: ctx.params.get(switchingCost(c.id)),
      })),
    },
    true,
  );
}

/**
 * Central Bank D2, Money Market C4.b: WHAT THE WINDOW WOULD ADVANCE each bank today, against the
 * paper it has free. It is the lender's own valuation and therefore the lender's to publish; a bank
 * that had to guess it would be guessing at somebody else's balance sheet (Observer A4).
 */
function publishCollateral(ctx: MechanismContext): void {
  const on = ctx.calendar.startOf(ctx.period);
  const haircut = ctx.params.get(MM_PARAMS.haircut);
  for (const bank of banksOf(ctx)) {
    const ccy = ccyOf(ctx, bank);
    const cb = ctx.registry.centralBankOf(ccy);
    const power = borrowingPower(
      windowAdvances(ctx.participant(cb), ctx.participant(bank), on, haircut),
    );
    ctx.record('centralBank.collateral', [bank, cb], { bank, ccy, advance: power }, true);
  }
}

/** A1, C2.a: where each bank said it stood after the flows. Its number, published, read here. */
interface Standing {
  readonly reserves: number;
  readonly buffer: number;
  readonly gap: number;
}

function standings(ctx: MechanismContext): Map<PartyId, Standing> {
  const out = new Map<PartyId, Standing>();
  for (const e of ctx.journal.ofKind('bank.buffer')) {
    if (e.period !== ctx.period) continue;
    const { bank, reserves, buffer } = e.data;
    if (typeof bank !== 'string' || typeof reserves !== 'number' || typeof buffer !== 'number') {
      continue;
    }
    out.set(partyId(bank), { reserves, buffer, gap: sub(reserves, buffer, 'its position') });
  }
  return out;
}

/**
 * A3, B1, B4: the session itself. Every bank's schedule for every name's book comes in through the
 * kernel's door (Clearing B2) — this market never asks a bank what it wants, it opens the books and
 * lets the schedules arrive — the window takes its seat, and each name's books clear cheapest
 * first, because a treasurer funds where the money is cheapest and stops when it has enough.
 *
 * WHAT IT IS FILLING IS ONE ORDER. A borrower's bid stands in every book at once, and it is one
 * bid for one amount: so as each book strikes, what is left of that bid is what goes into the next
 * one. The market is not deciding how much the bank wants — the bank said — it is filling what was
 * asked for without filling it four times over (Law 4).
 */
function runSession(ctx: MechanismContext): void {
  const m = market(ctx);
  const on = ctx.calendar.startOf(ctx.period);
  const c = corridor(ctx);
  const banks = banksOf(ctx);
  const pos = standings(ctx);
  declareVenues(ctx, banks);
  const haircut = ctx.params.get(MM_PARAMS.haircut);
  for (const borrower of banks) {
    const ccy = ccyOf(ctx, borrower);
    const cb = ctx.registry.centralBankOf(ccy);
    for (const book of BOOKS) {
      const venue = sessionVenue(book, borrower);
      // Clearing B2: the schedules, from whoever has one. Nothing about a bank is decided here.
      ctx.gather(venue);
      for (const o of windowOffer(ctx.participant(cb), ctx.participant(borrower), book, c, on, haircut)) {
        ctx.post(venue, o);
      }
    }
  }
  for (const borrower of banks) {
    const ccy = ccyOf(ctx, borrower);
    // Law 8: IN WHOLE PIECES OF MONEY. It can only borrow pieces and it can only be short of
    // pieces, so what the arithmetic leaves below one is not a shortfall — and a session that
    // recorded it as a refusal would publish a funding squeeze made of rounding, which every
    // uninsured depositor in the world then reads as a reason to leave (B7, D5.a, E1).
    const wanted = downTick(asked(ctx, borrower));
    let need = wanted;
    if (need <= 0) continue;
    for (const book of cheapestFirst(ctx, borrower)) {
      if (need <= 0) break;
      const venue = sessionVenue(book, borrower);
      const raised = clearBook(ctx, m, book, borrower, ccy, on, upTo(ctx.posted(venue), borrower, need));
      need = downTick(stillNeeded(need, raised));
    }
    // Law 7: what the walk left behind is dust of the clearing, not a shortfall. A session that
    // published one piece of money as a funding squeeze would have every uninsured depositor in the
    // world reading a rounding as a reason to leave (B7, D5.a, E1).
    if (need > 0 && material(need, BOOKS.length + 1, wanted)) {
      // B7, B2.a: the market did not clear for this name. It is an outcome of real schedules — no
      // lender would have it at a rate it would pay, or it had nothing left to pledge — and the
      // consequence lands where it falls due (D4).
      const p = pos.get(borrower);
      ctx.record(
        'moneyMarket.refused',
        [borrower],
        {
          borrower,
          short: need,
          ccy,
          reserves: p === undefined ? 0 : p.reserves,
          buffer: p === undefined ? 0 : p.buffer,
        },
        true,
      );
    }
  }
  parkTheRest(ctx, pos);
}

/** A2.a: what this name asked the session for — the biggest bid it put in any of its own books. */
function asked(ctx: MechanismContext, borrower: PartyId): number {
  let most = 0;
  for (const book of BOOKS) {
    for (const o of ctx.posted(sessionVenue(book, borrower))) {
      if (o.side !== 'buy' || o.party !== borrower || o.price === 'market') continue;
      if (o.qty > most) most = o.qty;
    }
  }
  return most;
}

/** The book as it stands with the borrower's own bid cut back to what it has still not raised. */
function upTo(orders: readonly Order[], borrower: PartyId, need: number): readonly Order[] {
  const out: Order[] = [];
  for (const o of orders) {
    if (o.side !== 'buy' || o.party !== borrower) {
      out.push(o);
      continue;
    }
    const qty = o.qty < need ? o.qty : need;
    if (qty > 0) out.push({ ...o, qty });
  }
  return out;
}

/** The books this borrower can reach, cheapest posted ask first: the treasurer's own preference. */
function cheapestFirst(ctx: MechanismContext, borrower: PartyId): readonly BookDecl[] {
  const priced = BOOKS.map((book) => ({
    book,
    best: bestAsk(ctx.posted(sessionVenue(book, borrower))),
  }));
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
  const struck = strike(posted, borrower, book);
  if (struck.length === 0) return 0;
  const raised: number[] = [];
  for (let s of struck) {
    // Law 7: a row for the dust of the clearing is not a row. Writing one would put an instrument
    // in the register, a lien on a rounding and a payment of nothing on the wire.
    if (!material(s.amount, struck.length + 1, sum(struck.map((x) => x.amount)).value)) continue;
    // B1: A LENDER CANNOT LEND THE SAME MONEY TWICE. Its schedule stands in every name's book, and
    // what it has already placed in this session comes off what it can still place in the next one.
    const room = capacityOf(ctx, s.lender, ccy);
    if (room <= 0) continue;
    if (s.amount > room) s = { ...s, amount: ctx.registry.payable(ccy, room) };
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
function parkTheRest(ctx: MechanismContext, pos: ReadonlyMap<PartyId, Standing>): void {
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
    const struck = strike(ctx.posted(venue), cb, overnight);
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

/**
 * B1, B5: what this lender can still place. For a bank it is what it had above its buffer less what
 * it has already lent this session; for anybody else it is the money in its account, which is the
 * only thing it could hand over. The central bank is the one party this does not bind: it issues
 * the money it lends, and what stops it is the borrower's collateral (C4.b) and nothing else.
 */
function capacityOf(ctx: MechanismContext, lender: PartyId, ccy: CurrencyCode): number {
  const cb = ctx.registry.centralBankOf(ccy);
  if (lender === cb) return Number.MAX_SAFE_INTEGER;
  const held = ctx.register.quantity(lender, moneyInstrumentId(ctx.parties.get(lender).bank, ccy));
  return sub(held, lentThisPeriod(ctx, lender), 'what it has left to place');
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
  const on = ctx.calendar.startOf(ctx.period);
  const haircut = ctx.params.get(MM_PARAMS.haircut);
  for (const bank of banksOf(ctx)) {
    const ccy = ccyOf(ctx, bank);
    const limit = ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
    const cb = ctx.registry.centralBankOf(ccy);
    const byClass = depositsByClass(ctx, bank, ccy);
    const reserves = ctx.register.quantity(bank, moneyInstrumentId(cb, ccy));
    // C1, C1.a: LIQUID ASSETS ARE NOT JUST RESERVES. They are the account plus what its own
    // unencumbered eligible paper would actually raise — at the haircut the central bank declared,
    // which is C1.a's "how surely it converts" as a real number and not an adjective. Paper it has
    // already pledged is not here: it is somebody's collateral (C4.b).
    const paper = borrowingPower(
      windowAdvances(ctx.participant(cb), ctx.participant(bank), on, haircut),
    );
    // C1: and what it lent overnight is liquid too — it comes back into the account tomorrow
    // morning. A bank that parked its spare cash at the floor (C1.a) still holds it, as a claim.
    const overnight = fallsDueToIt(ctx, bank, ccy);
    // C2.a: what it holds against a bad week is the bank's own decision, and the bank says it.
    const said = standings(ctx).get(bank);
    const buffer = said === undefined ? 0 : said.buffer;
    const liquid = add(
      add(reserves, overnight, 'cash and what comes back'),
      paper,
      'liquid assets',
    );
    const exposed = couldLeave(ctx, bank, ccy, limit);
    // C2.a: what its account actually did to it this period — the reserve legs of the wire, summed.
    // It is public because the balance either side of it is (F2), and it is here because the buffer
    // above is derived from a run of these and from nothing else.
    const move = netReserveFlow(ctx, bank, ccy);
    const metric = liquidityMetric(liquid, exposed);
    ctx.record(
      'bank.liquidity',
      [bank],
      {
        bank,
        ccy,
        reserves,
        overnight,
        paper,
        liquid,
        buffer,
        move,
        couldLeave: exposed,
        deposits: Object.fromEntries(byClass),
        base: depositBase(byClass),
        // F4: what somebody outside can see. Liquid assets over what could leave — both of them
        // reads of this bank's own two sides, and neither of them a ratio anybody stated.
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
      id: MM_PARAMS.haircut,
      value: 0.05,
      unit: 'ratio of the market price',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank D2, D3: the haircut the window takes on the paper it lends against. Eligibility and haircuts are its choice and a policy instrument in themselves, which is why this is a policy and not a preference of anybody.',
    },
    {
      id: MM_PARAMS.overdraftPenalty,
      value: 0.02,
      unit: 'per annum above the window rate',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank D3, D3.b: AT A PENALTY. An account that went below zero at the central bank borrowed from it without asking, and it is charged above the window it did not use — which is what makes the window the thing a bank goes to first and this the thing it goes to never.',
    },
    {
      id: MM_PARAMS.insuranceLimit,
      value: 100,
      denominated: true,
      unit: 'of the money it is a deposit in, per member',
      kind: 'policy',
      owner: 'parliament',
      why: 'Banks Funding A1.a, Banks Capital D4: what is insured, PER MEMBER of a cell (XI-15). It is what makes E4 break the run loop for retail money and not for wholesale, and it is a rule somebody wrote — parliament owns it from worklist 14.',
    },
    {
      id: INSURER_PARAMS.premium,
      value: 0.002,
      unit: 'per annum on the insured part of a bank own deposit base',
      kind: 'policy',
      owner: 'parliament',
      why: 'Banks Capital D4: what the guarantee costs the banks that have it. It is a rule somebody wrote and parliament owns it from worklist 14, and it is what makes deposit insurance a price a bank pays for taking retail money rather than a free option written by the state.',
    },
    ...DEPOSIT_CLASSES.map((c) => ({
      id: switchingCost(c.id),
      value: c.switchingCost,
      unit: 'per annum',
      kind: 'preference' as const,
      owner: 'model' as const,
      why: `Banks Funding A1.d, E1: what it costs a ${c.id} depositor to move its account. ${c.why}`,
    })),
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
          // Register F2, Banks Capital D6: the borrower can have CEASED since the row was written —
          // a resolution moves a bank's whole book, collateral and all, to whoever succeeded it —
          // and a reference to it resolves there. The security did not change; who is behind it did.
          const holding = view.register.holding(
            view.parties.resolve(i.terms.borrower).id,
            c.instrument,
          );
          const bound = holding.some
            ? sum(holding.value.liens.filter((l) => l.reason === String(i.id)).map((l) => l.qty))
                .value
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
  requires: ['banks', 'sovereign-instruments'],
  instrumentKinds: [interbankKind, repoKind],
  // Banks Capital D4: the guarantee behind the deposits this module prices is a PARTY, with an
  // account and an income of its own, because a guarantee nobody funded is one the treasury makes
  // silently every time (see `insurer.ts`).
  partyKinds: [insurerKind],
  curveFamilies: [],
  units: [],
  params: paramsOf(),
  phases: [
    {
      name: 'moneyMarket.rates',
      spec: 'Banks Funding B1 Banks Funding B1.a Money Market D2',
      cycle: 0,
      // AFTER the banks have decided their boards: what this phase does is PAY at the rates they
      // announced and let the depositors answer them. It decides nothing about any bank.
      anchor: { after: 'banks.treasury' },
      run: (ctx: MechanismContext): void => {
        freeRepaidCollateral(ctx);
        publishCorridor(ctx);
        publishClasses(ctx);
        publishCollateral(ctx);
        payDeposits(ctx);
        collectPremiums(ctx, banksOf(ctx));
      },
    },
    {
      name: 'moneyMarket.book',
      spec: 'Money B3.b Money B3.c Central Bank D3 Central Bank D3.b',
      // After the session and after the lending module has booked its own drawings: what is still
      // below zero at the central bank at the close of the period is an overdraft, and it becomes
      // a row before anything can die of it or the audit can see it.
      cycle: 'anchor',
      anchor: { before: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        bookOverdrafts(ctx);
      },
    },
    {
      name: 'moneyMarket.resolve',
      spec: 'Banks Capital C3 Banks Capital D1 Banks Capital D2 Banks Capital D3 Banks Capital D4 Banks Capital D5 Banks Capital D6',
      cycle: 'anchor',
      // BEFORE revaluation, and after this module's own booking phase — so what is still below
      // zero has already become a row, and the whole book moves at what it is carried at and is
      // re-marked on the acquirer's balance sheet in the same period it lands there. Moving it
      // after the marks were taken would crystallise a gain the revaluation had already booked.
      //
      // D1's valuation is unaffected: what the assets are WORTH is read off the prints the market
      // made this period, and a print is there the moment the session ends. What IS one period
      // stale here is the solvency trigger, which reads the equity account — so a bank whose assets
      // fell this period is found insolvent next period rather than this one. That is a lag and is
      // stated as one; the cash trigger, which is the one that fires in practice, has no such lag.
      //
      // It does not anchor to the estate's phase, because a world can have this module and no
      // estate at all — the estate leaves a bank alone by asking the kernel whether some module
      // resolves the kind (`resolvesItsOwn`), which is this one saying `resolves: [BANK]`.
      anchor: { before: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        for (const f of failedBanks(ctx)) resolve(ctx, f.bank, f.why);
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
  // Money Market A3, B1, Law 4: this market decides nothing for a bank and posts nothing under a
  // bank's name. Its own seat is the WINDOW's, which is the central bank's offer, and its books are
  // filled by the schedules the banks post through the kernel's door.
  participants: [],
  creditDecisions: [{ partyKind: CENTRAL_BANK, decide: reserveOverdraft }],
  // C3.b: a bank does not go to an estate. This module takes charge of what happens instead.
  resolves: [BANK],
  families: [collateralHolds(), nothingLeftBehind()],
  seed(ctx: SeedContext): void {
    const banks = ctx.parties.ofKind(BANK).map((b) => b.id);
    const first = banks[0];
    if (first === undefined) return;
    const region = ctx.parties.get(first).region;
    const ccy = ctx.registry.region(region).ccy;
    for (const v of venuesOf(ccy, [...banks, ctx.registry.centralBankOf(ccy)])) ctx.openVenue(v);
    // D4: the fund exists from period zero and opens with NOTHING, because a fund that opened full
    // would be a seed deciding how much of a future failure the banking system had already paid
    // for. What it has when one fails is what the premiums have actually brought in (Seed E1).
    ctx.parties.add({
      id: INSURER,
      kind: DEPOSIT_INSURER,
      region,
      name: 'North Deposit Guarantee',
      bank: ctx.registry.centralBankOf(ccy),
      representation: 'named',
      status: { alive: true },
    });
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
  const rates = rows.map((e) => e.data['rate']).filter((r): r is number => typeof r === 'number');
  return rates.length === 0 ? none<number>() : some(div(sum(rates).value, rates.length, 'rate'));
}

export { averageRate, CENTRAL_BANK, INTERBANK, REPO, rowTerms };

/**
 * Money B3.b, Central Bank D3, D3.a, D3.b: WHAT THE CENTRAL BANK DOES WHEN A BANK'S ACCOUNT WOULD
 * GO BELOW ZERO. It is the lender of last resort, and D6 names all four conditions at once:
 *
 *   - FREELY: it does not ration by size. What it will lend is what the paper covers, and that is a
 *     constraint on the borrower rather than a quota of its own.
 *   - AGAINST GOOD COLLATERAL: only paper it declared eligible, at the haircut it declared (D2),
 *     and only what is still unencumbered — so a bank that has pledged everything cannot draw
 *     (C4.b), and running out of collateral is what stops a solvent bank borrowing (B3.c).
 *   - AT A PENALTY: the row is written at the top of the corridor plus the penalty (D3.b), which is
 *     dearer than the window the bank did not use and far dearer than the market it did not reach.
 *   - TO THE SOLVENT: a bank whose own equity is gone is refused here and goes to resolution
 *     (D3.a). That refusal is the one that must exist, because a facility that lends to anybody is
 *     the subsidy C5 forbids and it deletes the whole of Money Market D.
 *
 * The decision is taken here and the ROW is written at the close (`bookOverdrafts`), which is the
 * same shape as a customer's overdraft becoming a loan (Money B3.c): during the period the account
 * is a negative balance, and by the end of it there is a lender, a rate and a date.
 */
function reserveOverdraft(ctx: MechanismContext, o: OverdraftContext): OverdraftDecision {
  const m = market(ctx);
  // Only a bank settles in reserves; anybody else overdrawn at the central bank is the treasury
  // asking for an advance, and there is none (Central Bank E2, Treasury D3).
  if (!o.holderIssuesMoney) return { allow: false };
  const on = ctx.calendar.startOf(ctx.period);
  const power = borrowingPower(
    windowAdvances(
      ctx.participant(o.issuer),
      ctx.participant(o.holder),
      on,
      ctx.params.get(MM_PARAMS.haircut),
    ),
  );
  const solvent = ctx.participant(o.holder).equity() > 0;
  const covered = power >= o.shortfall;
  if (!solvent || !covered) {
    ctx.record(
      'centralBank.refused',
      [o.issuer, o.holder],
      { bank: o.holder, short: o.shortfall, collateral: power, solvent, ccy: o.ccy },
      true,
    );
    return { allow: false };
  }
  m.overdrawn.push({ bank: o.holder, ccy: o.ccy });
  return { allow: true };
}

/**
 * D3.b: an overdrawn reserve account is a real overdraft and it STANDS AS THE NEGATIVE IT IS UNTIL
 * REPAID — so at the close of the period it becomes a row with a lender, a rate and a date, and the
 * money that repays it is the money the central bank lends. What was an account below zero during
 * the period is a priced, collateralised claim by the end of it, and the money family stops having
 * anything to report.
 */
function bookOverdrafts(ctx: MechanismContext): void {
  const m = market(ctx);
  const drawn = [...m.overdrawn];
  m.overdrawn = [];
  const c = corridor(ctx);
  const on = ctx.calendar.startOf(ctx.period);
  const seen = new Set<string>();
  for (const d of drawn) {
    if (seen.has(d.bank)) continue;
    seen.add(d.bank);
    const ccy = d.ccy;
    const cb = ctx.registry.centralBankOf(ccy);
    const short = -ctx.register.quantity(d.bank, moneyInstrumentId(cb, ccy));
    const need = ctx.registry.payable(ccy, short);
    if (need <= 0) continue;
    const book = BOOKS.find((b) => b.tenor === 'overnight' && b.secured);
    if (book === undefined) continue;
    const rate = add(c.ceiling, ctx.params.get(MM_PARAMS.overdraftPenalty), 'the penalty rate');
    const cover = coverFor(advancesFrom(ctx, cb, d.bank, on), need);
    const covered = sum(cover.map((x) => mul(x.qty, x.valuedAt, 'covered'))).value;
    const amount = ctx.registry.payable(ccy, covered > need ? need : covered);
    if (amount <= 0) continue;
    const n = m.next;
    m.next += 1;
    const id = writeRow(
      ctx,
      { lender: cb, borrower: d.bank, amount, rate, book },
      n,
      ccy,
      coverFor(advancesFrom(ctx, cb, d.bank, on), amount),
    );
    if (!id.some) continue;
    // C4.a: DRAWING THE FACILITY IS INFORMATION, and this is the dearest way of drawing it. It is
    // public because that is what makes a depositor's answer to it possible (D5.a, E2.a).
    ctx.record(
      'moneyMarket.window',
      [d.bank, cb],
      { bank: d.bank, amount, rate, overdraft: true, ccy },
      true,
    );
  }
}
