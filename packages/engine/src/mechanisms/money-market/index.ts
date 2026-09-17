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
import { atMostCash, sumCash } from '../../core/measure.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { Civil } from '../../calendar/civil.js';
import type { Order } from '../../clearing/solver.js';
import { decideRates, POLICY_PARAMS, POLICY_SET } from './policy.js';
import {
  currencyUnit,
  moneyInstrumentId,
  partyId,
  type CurrencyCode,
  type PartyId,
  type RegionId,
} from '../../core/ids.js';
import { atMost, dustOf, material, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { downTick, NO_QTY } from '../../core/tick.js';
import type { OverdraftContext, OverdraftDecision } from '../../registry/kinds.js';
import { BANK, CENTRAL_BANK } from '../../registry/profiles.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { ParamDecl } from '../../registry/params.js';
import { borrowingPower, windowAdvances, type Advance } from './collateral.js';
import {
  collectPremiums,
  DEPOSIT_INSURER,
  insurerOf,
  INSURER_PARAMS,
  insurerKind,
} from './insurer.js';
import { failedBanks, nothingLeftBehind, resolve } from './resolution.js';
import {
  BOOKS,
  DEPOSIT_CLASSES,
  MM_PARAMS,
  POLICY_RATES,
  policyRateOf,
  classOf,
  type BookDecl,
} from './data.js';
import {
  couldLeave,
  depositBase,
  depositsByClass,
  liquidityMetric,
  payDepositInterest,
} from './deposits.js';
import {
  INTERBANK,
  REPO,
  interbankKind,
  isRow,
  repoKind,
  rowTerms,
  securesAnything,
  type Pledged,
} from './rows.js';
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
import { asQty, subQty, type Qty } from '../../core/tick.js';
import {
  type Ratio,
  asAmount,
  asNamed,
  asRatio,
  heldAsMoney,
  minus,
  over,
  plus,
  valueAt,
} from '../../core/measure.js';
import { negQty } from '../../core/tick.js';
import { buffersPublished } from '../../registry/banking.js';

export * from './data.js';
export * from './rows.js';
export * from './collateral.js';
export * from './deposits.js';
export * from './session.js';
export { DEPOSIT_INSURER, insurerOf, INSURER_PARAMS } from './insurer.js';
export { valueBook, failedBanks, type Valuation as BookValuation } from './resolution.js';

/** What the module keeps: the deposit book, and how many rows it has written. */
interface Market {
  next: number;
  /** Money B3.b: the accounts the central bank let go below zero, waiting to become rows. */
  overdrawn: { bank: PartyId; ccy: CurrencyCode; viaSwap?: PartyId }[];
}

function market(ctx: MechanismContext): Market {
  return ctx.state<Market>('market', () => ({ next: 1, overdrawn: [] }));
}

const ccyOf = (ctx: MechanismContext, party: PartyId): CurrencyCode =>
  ctx.registry.currencyOf(ctx.parties.get(party).region);

/**
 * C-3, worklist 13l, Central Bank B1, B2: THE CORRIDOR OF THIS MONEY, and it is one per money.
 *
 * This took no currency at all, so the Fed, the ECB, the Bank of England and the Bank of Japan all
 * administered 2%. With no interest differential between two moneys there is no carry: an FX
 * forward prices flat to spot, covered interest parity says nothing, and the cross-currency basis
 * has nothing to be a basis of — four mechanisms that exist and cannot show anything, because one
 * parameter was shared by four institutions that are the whole reason they differ.
 *
 * The WIDTH is still one policy for every corridor. Two central banks that ran different-width
 * corridors would be a real difference and a real declaration, and nobody has made it: the floor
 * and the ceiling stay one row apiece until somebody does (Law 2, fewest primitives).
 */
function corridor(ctx: MechanismContext, ccy: CurrencyCode): Corridor {
  return corridorOf(
    ctx.params.perAnnum(policyRateOf(ccy)),
    ctx.params.perAnnum(MM_PARAMS.floorSpread),
    ctx.params.perAnnum(MM_PARAMS.ceilingSpread),
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
    if (!isRow(i.terms) || i.terms.collateral.length === 0 || securesAnything(i)) continue;
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
  for (const bank of banksOf(ctx)) payDepositInterest(ctx, bank, ccyOf(ctx, bank));
  // E1: and then the depositors answer, which is the only thing that stops a bank paying less.
  // Each of them answers through the module that owns it, with its own view (Observer A4): this
  // market publishes the boards and asks, and it is told nothing about why anybody moved.
  ctx.chooseBanks();
}

/**
 * Banks Funding A1.a, E1: the segments, PUBLISHED. How depositors are grouped and what the state
 * insures of each are facts about a taxonomy and a regulation, and every bank prices its board
 * against the same ones — so they are said out loud once, by the market that holds them, and read
 * by anybody who needs them (Law 4). A bank keeping its own copy would price against a market only
 * it can see.
 *
 * WHAT IT COSTS ONE OF THEM TO MOVE IS NOT PUBLISHED, because nobody but the depositor faces it: it
 * is an amount of that depositor's own money (A1.d), weighed against that depositor's own balance,
 * and a bank that could read it would be reading a private preference (Observer A4). What a bank
 * does face is the premium, and that is here.
 */
function publishClasses(ctx: MechanismContext): void {
  ctx.record(
    'deposit.classes',
    [],
    {
      // A1.a, Law 8: and what it insures them UP TO, in each money it could be held in, which is
      // the same regulation said in the same breath. A depositor works out what of its own balance
      // nobody covers by reading this; one that had to know the parameter's name to ask would be
      // reading this market's table instead of its board (Law 4).
      limits: Object.fromEntries(
        [...ctx.registry.currencies.keys()].map((code) => [
          code,
          ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(code)),
        ]),
      ),
      classes: DEPOSIT_CLASSES.map((c) => ({
        id: c.id,
        insured: c.insured,
        // Banks Capital D4: and what the guarantee COSTS the bank that has it, which is what makes
        // the same bank pay its classes differently — insured money carries a premium on top of the
        // rate, so a bank paying the same all-in for both offers the insured class less by exactly
        // that. It is published with the class because it is the same regulation, said once (Law 4).
        premium: c.insured ? ctx.params.perAnnum(INSURER_PARAMS.premium) : 0,
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
  const haircut = ctx.params.ratio(MM_PARAMS.haircut);
  for (const bank of banksOf(ctx)) {
    const ccy = ccyOf(ctx, bank);
    const cb = ctx.registry.centralBankOf(ccy);
    const power = borrowingPower(
      windowAdvances(ctx.participant(cb), ctx.participant(bank), on, haircut),
      ccy,
    );
    ctx.record('centralBank.collateral', [bank, cb], { bank, ccy, advance: power.pieces }, true);
  }
}

/** A1, C2.a: where each bank said it stood after the flows. Its number, published, read here. */
interface Standing {
  readonly reserves: Qty;
  readonly buffer: Qty;
  readonly gap: Qty;
}

function standings(ctx: MechanismContext): Map<PartyId, Standing> {
  const out = new Map<PartyId, Standing>();
  for (const [bank, said] of buffersPublished(ctx.journal, ctx.period)) {
    out.set(partyId(bank), {
      reserves: said.reserves,
      buffer: said.buffer,
      gap: subQty(said.reserves, said.buffer, 'its position'),
    });
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
  const banks = banksOf(ctx);
  const pos = standings(ctx);
  declareVenues(ctx, banks);
  const haircut = ctx.params.ratio(MM_PARAMS.haircut);
  for (const borrower of banks) {
    const ccy = ccyOf(ctx, borrower);
    // C-3: the corridor of the money this bank banks in, which is the one its own central bank
    // administers. It used to be one corridor for every bank in the world.
    const c = corridor(ctx, ccy);
    const cb = ctx.registry.centralBankOf(ccy);
    for (const book of BOOKS) {
      const venue = sessionVenue(book, borrower);
      // Clearing B2: the schedules, from whoever has one. Nothing about a bank is decided here.
      ctx.gather(venue);
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
      const raised = clearBook(
        ctx,
        m,
        book,
        borrower,
        ccy,
        on,
        upTo(ctx.posted(venue), borrower, need),
      );
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
function upTo(orders: readonly Order[], borrower: PartyId, need: Qty): readonly Order[] {
  const out: Order[] = [];
  for (const o of orders) {
    if (o.side !== 'buy' || o.party !== borrower) {
      out.push(o);
      continue;
    }
    // Both are counts of money pieces — what it bid and what it still needs — so the smaller of
    // them is one too, and nothing was rounded to get it.
    const qty = atMost(o.qty, need, 'what it still needs is all it is still bidding for');
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
    if (s.amount > room)
      s = {
        ...s,
        amount: ctx.registry.payable(heldAsMoney(room, ccy, 'what it has left to place')),
      };
    let cover: readonly Pledged[] = [];
    let amount = s.amount;
    if (book.secured) {
      cover = coverFor(
        advancesFrom(ctx, s.lender, borrower, on),
        heldAsMoney(s.amount, ccy, 'what it asked for'),
      );
      // B3.c: what its remaining paper covers is what it gets, at THIS lender's valuation of it.
      // Less than it asked for is the constraint biting, not a failure of the session — and what
      // it gets is a whole number of pieces of money (Law 8), because that is what is lent.
      const covered = sumCash(
        ccy,
        cover.map((x) => valueAt(x.valuedAt, x.qty, ccy, 'covered')),
        'covered',
      ).value;
      amount = ctx.registry.payable(
        atMostCash(
          covered,
          heldAsMoney(s.amount, ccy, 'what it asked for'),
          'a guarantee pays no more than was owed',
        ),
      );
      if (amount <= 0) continue;
      cover = coverFor(
        advancesFrom(ctx, s.lender, borrower, on),
        heldAsMoney(amount, ccy, 'what it is lent'),
      );
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
        ctx.params.perAnnum(MM_PARAMS.overdraftPenalty),
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
  const overnight = BOOKS.find((b) => b.tenor === 'overnight' && !b.secured);
  if (overnight === undefined) return;
  for (const [bank, p] of pos) {
    const ccy = ccyOf(ctx, bank);
    // C-3: at the floor OF ITS OWN MONEY. A yen bank does not park at the Fed's floor.
    const c = corridor(ctx, ccy);
    const cb = ctx.registry.centralBankOf(ccy);
    // Law 8: its buffer is a share of what could leave, so what is left over is a fraction of a
    // cent. It places whole ones, and down, because it is what it HAS above the cushion.
    const spare = downTick(
      minus(p.reserves, plus(p.buffer, lentThisPeriod(ctx, bank), 'placed'), 'spare'),
    );
    if (spare <= 0) continue;
    const venue = sessionVenue(overnight, cb);
    const ask: Order = { party: bank, side: 'sell', price: c.floor, qty: spare };
    ctx.post(venue, ask);
    for (const o of floorBid(cb, asQty(offered(ctx.posted(venue)), 'what the session offers'), c))
      ctx.post(venue, o);
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
function capacityOf(ctx: MechanismContext, lender: PartyId, ccy: CurrencyCode): Qty {
  const cb = ctx.registry.centralBankOf(ccy);
  if (lender === cb) {
    return asAmount<'piece'>(Number.MAX_SAFE_INTEGER, 'a central bank places its own money');
  }
  const held = ctx.register.quantity(lender, moneyInstrumentId(ctx.parties.get(lender).bank, ccy));
  return subQty(held, lentThisPeriod(ctx, lender), 'what it has left to place');
}

/** What this bank has already placed in this period's session, so it does not place it twice. */
function lentThisPeriod(ctx: MechanismContext, lender: PartyId): Qty {
  const terms: Qty[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isRow(i.terms) || i.terms.lender !== lender) continue;
    if (ctx.calendar.periodOf(i.terms.drawn) !== ctx.period) continue;
    terms.push(ctx.register.quantity(lender, i.id));
  }
  return sum(terms).value;
}

/**
 * 13j, Money Market A1: A BOOK PER MONEY, and the banks in it are the ones that issue that money.
 *
 * An overnight market is where the banks of ONE system lend each other the reserves of ONE central
 * bank: a euro bank cannot settle a dollar loan on the Fed's books and there is no rate the two
 * systems share. It used to take the first bank's money and open one book, which was right when
 * every bank in this world was American.
 */
function declareVenues(ctx: MechanismContext, banks: readonly PartyId[]): void {
  const byCcy = new Map<CurrencyCode, PartyId[]>();
  for (const b of banks) {
    const ccy = ccyOf(ctx, b);
    const held = byCcy.get(ccy);
    if (held === undefined) byCcy.set(ccy, [b]);
    else held.push(b);
  }
  for (const [ccy, here] of byCcy) {
    const cb = ctx.registry.centralBankOf(ccy);
    for (const v of venuesOf(ccy, [...here, cb])) {
      if (ctx.venues.some((x) => x.id === v.id)) continue;
      ctx.openVenue(v);
    }
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
  const haircut = ctx.params.ratio(MM_PARAMS.haircut);
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
      ccy,
    );
    // C1: and what it lent overnight is liquid too — it comes back into the account tomorrow
    // morning. A bank that parked its spare cash at the floor (C1.a) still holds it, as a claim.
    const overnight = fallsDueToIt(ctx, bank, ccy);
    // C2.a: what it holds against a bad week is the bank's own decision, and the bank says it.
    const said = standings(ctx).get(bank);
    const buffer = said === undefined ? NO_QTY : said.buffer;
    const liquid = plus(
      plus(heldAsMoney(reserves, ccy, 'the cash it holds'), overnight, 'cash and what comes back'),
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
        overnight: overnight.pieces,
        paper: paper.pieces,
        liquid: liquid.pieces,
        buffer,
        move,
        couldLeave: exposed.pieces,
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

/**
 * B2, Central Bank B2: the two levels, declared. They are administered, and this says so.
 *
 * C-3: ONE ANNOUNCEMENT PER CENTRAL BANK, naming the money it is about. There was one event with no
 * currency on it, which is what four institutions administering 2% looks like from the outside.
 */
function publishCorridor(ctx: MechanismContext): void {
  for (const ccy of ctx.registry.currencies.keys()) {
    const cb = ctx.registry.centralBankOf(ccy);
    if (!ctx.parties.has(cb)) continue;
    const c = corridor(ctx, ccy);
    ctx.record(
      'centralBank.corridor',
      [cb],
      { ccy, policy: c.policy, floor: c.floor, ceiling: c.ceiling },
      true,
    );
  }
}

function paramsOf(): ParamDecl[] {
  return [
    ...POLICY_RATES.map((r): ParamDecl => ({
      id: policyRateOf(r.ccy),
      value: r.rate,
      unit: 'per annum',
      dimension: 'perAnnum',
      kind: 'policy',
      owner: 'centralBank',
      why: `Central Bank B1, B2: the rate ${r.ccy}'s own central bank declares. It is administered and not traded (B2), and it is the one price in this world that is not cleared — Law 3 allows exactly this one, because the quantity response is real and booked on both balance sheets. What it is set AGAINST is its mandate (B1.a), and the mandate is parliament (worklist 14). ${r.why}`,
    })),
    ...POLICY_RATES.flatMap((r): ParamDecl[] => [
      {
        id: POLICY_PARAMS.target(r.ccy),
        value: 0.02,
        unit: 'per annum',
        dimension: 'perAnnum',
        kind: 'policy',
        // 19.1: PARLIAMENT'S, not the bank's. What a central bank is FOR is a decision the polity
        // takes and the bank carries out — the bank sets the rate, and what it is aiming at is set
        // for it. Until §47 exists nobody moves it, which is what a standing mandate is, and the
        // bank cannot move it itself: the register refuses a setter that is not the owner.
        owner: 'parliament',
        why: `Central Bank B1.a (18a.1): what ${r.ccy}'s mandate is FOR — the rate of change of the consumer basket the bank is trying to hold to. Two per cent a year, which is the number the institutions this world imports its primitives from actually use, and it is a POLICY: somebody chose it, it can be changed by whoever owns it, and after §47 that owner is parliament.`,
      },
      {
        id: POLICY_PARAMS.step(r.ccy),
        value: 0.0025,
        unit: 'per annum',
        dimension: 'perAnnum',
        kind: 'policy',
        owner: 'centralBank',
        why: `Central Bank B1 (18a.1): the smallest move ${r.ccy}'s bank makes. A decision has a grain and this is the grain — a quarter of a point, the move the committees that do this actually make — and it is NOT a gain: nothing multiplies the gap by anything, and a bank that sees a gap moves one of these and looks again at its next meeting.`,
      },
      {
        id: POLICY_PARAMS.every(r.ccy),
        value: 2,
        unit: 'months',
        dimension: 'months',
        kind: 'policy',
        owner: 'centralBank',
        why: `Central Bank B1 (18a.1): how often ${r.ccy}'s bank meets. Every two months — eight times a year, which is what the committees do — and it is a DATE walked from the day this world opened rather than a remainder on the period index, so what a period is long does not change how often anybody decides (Money G3.a).`,
      },
    ]),
    {
      id: MM_PARAMS.floorSpread,
      value: 0.001,
      unit: 'per annum below the policy rate',
      dimension: 'perAnnum',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Money Market C1, C3: what it pays for cash it takes in, and half of the corridor whose WIDTH is a policy choice. Narrow, because a floor far below the policy rate lets the market rate wander and B4 stops being informative.',
    },
    {
      id: MM_PARAMS.ceilingSpread,
      value: 0.005,
      unit: 'per annum above the policy rate',
      dimension: 'perAnnum',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Money Market C2, C4: what it charges to lend against paper. Above the policy rate so a bank prefers the market and drawing is information (C4.a), and wider than the floor spread because the window is meant to be the dearer answer.',
    },
    {
      id: MM_PARAMS.haircut,
      value: 0.05,
      unit: 'ratio of the market price',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank D2, D3: the haircut the window takes on the paper it lends against. Eligibility and haircuts are its choice and a policy instrument in themselves, which is why this is a policy and not a preference of anybody.',
    },
    {
      id: MM_PARAMS.swapLine,
      value: 50_000_000_000,
      denominated: 'money',
      unit: 'of the lending money, named unit',
      dimension: 'amount',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank A2.b, Currency B4, E4 (16.5): A SWAP LINE IS AN AGREEMENT BETWEEN TWO CENTRAL BANKS, and its size is the term of that agreement — a policy two institutions set, not a bound on a market. A bank short of a foreign money has no window in it (a central bank lends its own system); what it has is ITS OWN central bank, which draws that money on the line, hands its own money across at the rate in force, and lends the foreign money on at the foreign window’s ceiling plus its penalty. Two rows on two central banks’ books, both dated, both priced; the line is finite and a draw past it is refused, which is a state.',
    },
    {
      id: MM_PARAMS.overdraftPenalty,
      value: 0.02,
      unit: 'per annum above the window rate',
      dimension: 'perAnnum',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank D3, D3.b: AT A PENALTY. An account that went below zero at the central bank borrowed from it without asking, and it is charged above the window it did not use — which is what makes the window the thing a bank goes to first and this the thing it goes to never.',
    },
    {
      id: MM_PARAMS.insuranceLimit,
      value: 250_000,
      denominated: 'money',
      unit: 'of the money it is a deposit in, per member',
      dimension: 'amount',
      kind: 'policy',
      owner: 'parliament',
      why: 'Banks Funding A1.a, Banks Capital D4: what is insured, PER MEMBER of a cell (XI-15). It is what makes E4 break the run loop for retail money and not for wholesale, and it is a rule somebody wrote — parliament owns it from item 19. IT WAS 100, AND A `denominated` VALUE IS IN NAMED UNITS: a hundred dollars a member, against an opening balance thousands of times that, so practically nothing in this world was insured. Every bank then read its whole deposit base as money that could leave (C2), wanted more liquid paper than its own balance sheet, and had negative funding room from the first period a liquidity line was published — so no bank lent to anybody after period 0, nothing was quoted, no firm could cost capital and nothing was ever built (item 0, stop 16). A quarter of a million is what the real guarantee is written at, and `test/params.test.ts` now holds every denominated number against what a member actually holds.',
    },
    {
      id: INSURER_PARAMS.premium,
      value: 0.002,
      unit: 'per annum on the insured part of a bank own deposit base',
      dimension: 'perAnnum',
      kind: 'policy',
      owner: 'parliament',
      why: 'Banks Capital D4: what the guarantee costs the banks that have it. It is a rule somebody wrote and parliament owns it from worklist 14, and it is what makes deposit insurance a price a bank pays for taking retail money rather than a free option written by the state.',
    },
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
        // The same test the module that frees the collateral applies (Law 4): a row secures
        // something while it has not ceased and something is outstanding on it. A row that was
        // never drawn secures nothing either — the money never moved, so the paper it would have
        // stood behind was never bound (Register D5).
        if (!isRow(i.terms) || !securesAnything(i)) continue;
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
            : NO_QTY;
          // Law 7: what is bound is a sum over liens and what the row says is a sum over parcels,
          // so the comparison is entitled to the dust of both walks and to nothing else.
          if (!withinDust(bound, c.qty, dustOf(2, Math.abs(bound) + Math.abs(c.qty)))) {
            out.push({
              family: 'ownership',
              spec: 'Money Market B3.c',
              owner: String(i.id),
              size: minus(c.qty, bound, 'unbound collateral'),
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
  nouns: [
    {
      name: 'market',
      kind: 'working',
      holds:
        'the accounts the central bank let go below zero this period, waiting to become repo rows',
      why: 'state one phase hands to a later phase in the same period. The ROW is the fact and it is written to the register; this is the interval between the kernel allowing the drawing and the row existing (Money B3.b).',
    },
  ],
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
  // 19.1: it speaks for the CENTRAL BANK's mandate — its rate and its corridor — and for no other.
  // Parliament's numbers (the target, the tax rates, the transfers) are the polity's to move, and
  // until the polity exists nobody moves them, which is what a standing mandate IS.
  mandates: ['centralBank'],
  phases: [
    {
      name: 'moneyMarket.rates',
      spec: 'Banks Funding B1 Banks Funding B1.a Money Market D2',
      // AFTER the banks have decided their boards: what this phase does is PAY at the rates they
      // announced and let the depositors answer them. It decides nothing about any bank.
      anchor: { after: 'banks.treasury' },
      reads: [
        { kind: 'event', name: 'bank.depositRate', of: 'anyPeriod' },
        { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
      ],
      writes: [
        { kind: 'event', name: 'centralBank.collateral' },
        { kind: 'event', name: 'centralBank.corridor' },
        { kind: 'event', name: 'deposit.classes' },
        { kind: 'event', name: 'insurance.premium' },
      ],
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
      /**
       * Central Bank B1, B1.a (18a.1): THE DECISION. Before the corridor is published and before
       * anything is paid at it, because what the rest of the period runs on is the rate it has just
       * set — a bank that met after its own corridor was published would be setting next week's.
       */
      name: 'moneyMarket.policy',
      spec: 'Central Bank B1 Central Bank B1.a Central Bank B2 Indices D4 Expectations A2',
      anchor: { before: 'moneyMarket.rates' },
      reads: [{ kind: 'event', name: 'environment.state', of: 'thisPeriod' }],
      writes: [{ kind: 'event', name: POLICY_SET }],
      run: (ctx: MechanismContext): void => {
        decideRates(ctx, [...ctx.registry.currencies.values()].map((c) => c.code));
      },
    },
    {
      name: 'moneyMarket.book',
      spec: 'Money B3.b Money B3.c Central Bank D3 Central Bank D3.b',
      // After the session and after the lending module has booked its own drawings: what is still
      // below zero at the central bank at the close of the period is an overdraft, and it becomes
      // a row before anything can die of it or the audit can see it.
      anchor: { before: 'revaluation' },
      reads: [],
      writes: [{ kind: 'event', name: 'moneyMarket.window' }],
      run: (ctx: MechanismContext): void => {
        bookOverdrafts(ctx);
      },
    },
    {
      name: 'moneyMarket.resolve',
      spec: 'Banks Capital C3 Banks Capital D1 Banks Capital D2 Banks Capital D3 Banks Capital D4 Banks Capital D5 Banks Capital D6',
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
      reads: [],
      writes: [
        { kind: 'event', name: 'bank.resolution.bid' },
        { kind: 'event', name: 'bank.resolution.done' },
        { kind: 'event', name: 'bank.resolution.ownClaim' },
        { kind: 'event', name: 'bank.resolution.valued' },
        { kind: 'event', name: 'bank.resolution.writtenDown' },
      ],
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
      anchor: { before: 'lending.book' },
      reads: [
        { kind: 'event', name: 'bank.buffer', of: 'anyPeriod' },
        { kind: 'event', name: 'credit.default', of: 'anyPeriod' },
      ],
      writes: [
        { kind: 'event', name: 'bank.liquidity' },
        { kind: 'event', name: 'centralBank.parked' },
        { kind: 'event', name: 'moneyMarket.print' },
        { kind: 'event', name: 'moneyMarket.refused' },
      ],
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
    // 13j: one book and one guarantee PER MONEY, over the banks that issue it. A world with four
    // banking systems has four overnight markets and four deposit guarantees, and a bank belongs to
    // exactly one of each — which is what its own region says (Money A1).
    const byCcy = new Map<CurrencyCode, { region: RegionId; banks: PartyId[] }>();
    for (const b of ctx.parties.ofKind(BANK)) {
      const ccy = ctx.registry.currencyOf(b.region);
      const held = byCcy.get(ccy);
      if (held === undefined) byCcy.set(ccy, { region: b.region, banks: [b.id] });
      else held.banks.push(b.id);
    }
    for (const [ccy, here] of byCcy) {
      const cb = ctx.registry.centralBankOf(ccy);
      for (const v of venuesOf(ccy, [...here.banks, cb])) ctx.openVenue(v);
      // D4: the fund exists from period zero and opens with NOTHING, because a fund that opened
      // full would be a seed deciding how much of a future failure the banking system had already
      // paid for. What it has when one fails is what the premiums have actually brought in (Seed E1).
      ctx.parties.add({
        id: insurerOf(ccy),
        kind: DEPOSIT_INSURER,
        region: here.region,
        name: `${ctx.registry.currency(ccy).name} Deposit Guarantee`,
        bank: cb,
        representation: 'named',
        status: { alive: true, standing: 'good' },
      });
    }
  },
};

/** What a party of this kind is to a bank that funds itself, for the reads that need it. */
export function depositClassOf(ctx: MechanismContext, party: PartyId): Option<string> {
  const cls = classOf(ctx.registry, ctx.parties.get(party).kind);
  return cls === undefined ? none<string>() : some(cls.id);
}

/** The rate that cleared for a name in a book this period, for a reader (C3, B6.a). */
export function printed(ctx: MechanismContext, borrower: PartyId): Option<number> {
  const rows = ctx.journal
    .ofKind('moneyMarket.print')
    .filter((e) => e.period === ctx.period && e.subjects.includes(borrower));
  const rates = rows
    .map((e) => e.data['rate'])
    .filter((r): r is number => typeof r === 'number')
    // Item 16: what the session printed re-enters here — a rate per annum on what was lent.
    .map((r) => asRatio(r, 'what this row was struck at'));
  return rates.length === 0
    ? none<Ratio>()
    : some(over(sum(rates).value, asRatio(rates.length, 'the rows there were'), 'rate'));
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
  // Currency D4, XI-12, Central Bank D1: A CENTRAL BANK LENDS TO ITS OWN SYSTEM. A bank booked in
  // another region has no reserve account here and no claim on this window — it is not supervised
  // here, it holds none of the collateral this window takes, and there is no resolution authority
  // behind it. So a bank short of a FOREIGN money is not short of reserves: it has to buy that
  // money from somebody who has it, at a rate, in the pair's own session (Spot FX B1), and a bank
  // that cannot does not pay. Without this the second currency arrives as an unlimited foreign
  // overdraft — money issued to a holder with no lender row behind it, which is Money B3.c's
  // finding and the reason a world with two moneys would never need an FX market at all.
  const home = ctx.registry.currencyOf(ctx.parties.get(o.holder).region);
  if (home !== o.ccy) {
    // Central Bank A2.b, Currency E4 (16.5): THE SWAP LINE. The bank's OWN central bank draws the
    // foreign money on its line with the issuer and lends it on; both draws become rows at the
    // close (`bookOverdrafts`). It is allowed only while the line has room — what this issuer has
    // already lent the home central bank on it, read off the rows, against the line's stated size.
    const homeCb = ctx.registry.centralBankOf(home);
    const line = ctx.registry.pieces(currencyUnit(o.ccy), asNamed(ctx.params.amount(MM_PARAMS.swapLine, currencyUnit(o.ccy)), 'the line'));
    const drawn = drawnOnLine(ctx, o.issuer, homeCb, o.ccy);
    if (ctx.parties.has(homeCb) && ctx.parties.get(homeCb).status.alive && drawn + o.shortfall <= line) {
      m.overdrawn.push({ bank: o.holder, ccy: o.ccy, viaSwap: homeCb });
      return { allow: true };
    }
    ctx.record(
      'centralBank.refused',
      [o.issuer, o.holder],
      { bank: o.holder, short: o.shortfall, ccy: o.ccy, foreign: true, swapLineDrawn: drawn, swapLine: line },
      true,
    );
    return { allow: false };
  }
  const on = ctx.calendar.startOf(ctx.period);
  const power = borrowingPower(
    windowAdvances(
      ctx.participant(o.issuer),
      ctx.participant(o.holder),
      on,
      ctx.params.ratio(MM_PARAMS.haircut),
    ),
    o.ccy,
  );
  const solvent = ctx.participant(o.holder).equity().pieces > 0;
  const covered = power.pieces >= o.shortfall;
  if (!solvent || !covered) {
    ctx.record(
      'centralBank.refused',
      [o.issuer, o.holder],
      { bank: o.holder, short: o.shortfall, collateral: power.pieces, solvent, ccy: o.ccy },
      true,
    );
    return { allow: false };
  }
  m.overdrawn.push({ bank: o.holder, ccy: o.ccy });
  return { allow: true };
}

/** Central Bank A2.b (16.5): what this issuer has lent that central bank on their swap line and not yet been repaid, off the rows. */
function drawnOnLine(ctx: MechanismContext, issuer: PartyId, borrower: PartyId, ccy: CurrencyCode): number {
  let out = 0;
  for (const i of ctx.instruments.issuedBy(borrower)) {
    if (!i.status.live || i.ccy !== ccy || !isRow(i.terms) || i.terms.lender !== issuer) continue;
    out += ctx.register.heldTotal(i.id).value;
  }
  return out;
}

/**
 * Central Bank A2.b, Currency E4, XI-5 (16.5): A DRAW ON THE SWAP LINE, as rows on two books.
 *
 * The issuer lends the home central bank the foreign money at its own window's ceiling (a central
 * bank lends a central bank at the rate it lends its system), and the home central bank hands its own
 * money across at the rate in force — that is the swap, two legs in two moneys in one instruction.
 * Then the home central bank lends the money on to its bank at the ceiling plus its penalty, which
 * repays the overdraft the bank ran at the issuer. The line is finite (`swapLine`), both rows are
 * dated overnight and roll like any other, and neither central bank converted anything.
 */
function swapDraw(
  ctx: MechanismContext,
  m: Market,
  issuer: PartyId,
  homeCb: PartyId,
  bank: PartyId,
  need: Qty,
  ccy: CurrencyCode,
  atCeiling: Ratio,
  atPenalty: Ratio,
): void {
  const book = BOOKS.find((b) => b.tenor === 'overnight' && !b.secured);
  if (book === undefined) return;
  const home = ctx.registry.currencyOf(ctx.parties.get(homeCb).region);
  const n = m.next;
  m.next += 1;
  const line = writeRow(ctx, { lender: issuer, borrower: homeCb, amount: need, rate: atCeiling, book }, n, ccy, []);
  if (!line.some) return;
  // The other leg of the swap: the home central bank's own money, at the rate in force, to the issuer.
  const across = ctx.registry.payable(ctx.valuation.inMoney(heldAsMoney(need, ccy, 'what it drew'), home, ctx.period));
  if (across > 0) {
    ctx.settle({
      legs: [{ kind: 'money', from: ctx.accountOf(homeCb, home), to: ctx.accountOf(issuer, home), ccy: home, amount: across }],
      cause: 'transfer',
      reason: `${String(homeCb)} hands ${String(issuer)} its own money against the swap-line draw ${line.value}`,
    });
  }
  // A central bank short abroad ITSELF draws the line and lends it on to nobody: one row, not two.
  let onward: Option<string> = none<string>();
  if (bank !== homeCb) {
    const k = m.next;
    m.next += 1;
    onward = writeRow(ctx, { lender: homeCb, borrower: bank, amount: need, rate: atPenalty, book }, k, ccy, []);
  }
  ctx.record(
    'centralBank.swapLine',
    [issuer, homeCb, bank],
    { issuer, drawnBy: homeCb, lentTo: bank, amount: need, ccy, across, ccyAcross: home, atCeiling, atPenalty, line: line.value, onward: onward.some ? onward.value : null },
    true,
  );
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
  const on = ctx.calendar.startOf(ctx.period);
  const seen = new Set<string>();
  for (const d of drawn) {
    if (seen.has(d.bank)) continue;
    seen.add(d.bank);
    const ccy = d.ccy;
    // C-3: the ceiling of the money it is overdrawn IN, which is the one it is penalised against.
    const c = corridor(ctx, ccy);
    const cb = ctx.registry.centralBankOf(ccy);
    const short = negQty(
      ctx.register.quantity(d.bank, moneyInstrumentId(cb, ccy)),
      'its overdraft',
    );
    const need = ctx.registry.payable(heldAsMoney(short, ccy, 'its overdraft'));
    if (need <= 0) continue;
    const book = BOOKS.find((b) => b.tenor === 'overnight' && b.secured);
    if (book === undefined) continue;
    const rate = plus(
      c.ceiling,
      ctx.params.perAnnum(MM_PARAMS.overdraftPenalty),
      'the penalty rate',
    );
    if (d.viaSwap !== undefined) {
      swapDraw(ctx, m, cb, d.viaSwap, d.bank, need, ccy, c.ceiling, rate);
      continue;
    }
    const cover = coverFor(
      advancesFrom(ctx, cb, d.bank, on),
      heldAsMoney(need, ccy, 'what it is short of'),
    );
    const covered = sumCash(
      ccy,
      cover.map((x) => valueAt(x.valuedAt, x.qty, ccy, 'covered')),
      'covered',
    ).value;
    const amount = ctx.registry.payable(
      atMostCash(
        covered,
        heldAsMoney(need, ccy, 'what it is short of'),
        'a guarantee pays no more than was owed',
      ),
    );
    if (amount <= 0) continue;
    const n = m.next;
    m.next += 1;
    const id = writeRow(
      ctx,
      { lender: cb, borrower: d.bank, amount, rate, book },
      n,
      ccy,
      coverFor(advancesFrom(ctx, cb, d.bank, on), heldAsMoney(amount, ccy, 'what it is lent')),
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
