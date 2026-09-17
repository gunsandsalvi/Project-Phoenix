/**
 * Securities lending: title passes, the economics do not.
 *
 * @spec Securities Lending A1 Securities Lending A2 Securities Lending A3 Securities Lending A4 Securities Lending A5 Securities Lending A5.a Securities Lending A5.b Securities Lending B1 Securities Lending B2 Securities Lending B2.a Securities Lending B4 Securities Lending C1 Securities Lending C2 Securities Lending C2.a Securities Lending C4 Securities Lending D1 Securities Lending D2 Securities Lending D3 Securities Lending E1 Securities Lending E2 Securities Lending E3 Equity C7 Register D5.a Observer A4 Law 3 Law 4 Law 8 Law 19
 *
 * THE DEFINING PROPERTY IS THAT TWO THINGS COME APART. Legal title moves to the borrower — it can
 * sell what it borrowed, and that is the entire point (A2) — while the economics stay with the
 * lender, which gets a manufactured payment equal to anything the security pays (A3). A world that
 * moved one without the other would have a stock loan in which the lender pays a fee to lose its
 * income, which is the transaction inverted.
 *
 * IT IS BUILT OUT OF THINGS THAT ALREADY EXIST. The security moves in the register like any other
 * units. The collateral is a PLEDGE, so it leaves the poster's free balance and cannot be counted as
 * available by both sides (C4, Register D5.a) — the register already refuses to move encumbered
 * units, so nothing here has to remember not to. The fee is a price and it CLEARS (A5, A5.a): scarce
 * paper is expensive to borrow and abundant paper is cheap, and neither is a number anybody wrote.
 *
 * WHAT IT UNLOCKS IS E1: NO SHORT WITHOUT A BORROW. A negative position nobody lent is an invented
 * security, and until there was a borrow market the only honest answer was to forbid the short. The
 * lendable pool is a read of who actually holds the paper and is willing (B4), and it is what caps
 * how large a short can get — a real constraint, and the reason a squeeze is possible (D2).
 *
 * WHY ANYBODY IS SHORT IS NOT THIS MODULE'S BUSINESS (`A-67`, `B-3`). All of the above was built and
 * reachable from nowhere: the two ways in were exported and called by no one, and the one phase
 * walked a book nothing ever pushed to. The reason is that being short is a POSITION a party takes,
 * out of its own view of a line it holds none of, and this module can see neither — so it asks
 * (`ctx.borrowsWanted`, the door `termsOffered` and `chooseBanks` are), collects what comes back
 * per LINE, and clears one book for each. One book per line is also what makes the fee a price: the
 * borrowers in it bid against each other and the holders of the line undercut each other, and what
 * the session strikes is the level where the two curves meet. It used to clear one session per
 * borrower against every lender posting `price: 'market'`, which is a book with a single level in
 * it — the bidder's own reservation, whatever the supply.
 */
import {
  absolute,
  amountOf,
  asPerPiece,
  type PerPiece,
  heldAsMoney,
  asRatio,
  type Cash,
  minus,
  negated,
  plus,
  type Ratio,
  ratioOf,
  scale,
  valueAt,
  atMostCash,
  noCash,
  sumCash,
} from '../../core/measure.js';
import { clear, isCleared, type Fill } from '../../clearing/solver.js';
import {
  agreementKindId,
  paramId,
  venueId,
  type AgreementId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { atMost, sum } from '../../core/num.js';
import { callFor } from '../../registry/margin.js';
import { downTick, subQty, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { LienId } from '../../core/ids.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { Violation, Family } from '../../audit/audit.js';
import { about } from '../../world/context.js';
import type { Borrowing, MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

/** A4: how long the paper stays out before it goes back — a term of the contract (Law 2). */
export const BORROW_TERM = paramId('securitiesLending.borrowTermPeriods');

/** Law 9: one book per line, because what is being priced is the scarcity of THAT paper (A5.a). */
export const borrowVenue = (instrument: InstrumentId): VenueId => venueId(`borrow:${instrument}`);

/** A1, XI-8: a stock loan is a bilateral commitment like an employment or a lease (item 9.1). */
export const STOCK_LOAN = agreementKindId('securitiesLending.stockLoan');

/** A1, Law 15: everything about the loan the kernel has no business understanding. */
export interface StockLoanTerms extends AgreementTerms {
  readonly kind: typeof STOCK_LOAN;
  readonly instrument: InstrumentId;
  readonly units: Qty;
  /** C1: what the borrower put up, and the lien that binds it. */
  readonly collateral: InstrumentId;
  readonly lien: LienId;
  readonly posted: Qty;
  /** A5: per period, as a fraction of what the borrowed paper is worth. It cleared (A5.a). */
  readonly fee: Ratio;
  /**
   * C1, C2 (item 13.8): THE LENDER'S HAIRCUT, kept because the loan is re-marked against it every
   * period. It is a TERM of the contract — what this lender required of this borrower on this pair
   * of lines, struck once when the loan opened — and re-asking the lender's view each period would
   * be C4's *"the broker can raise the requirement when it likes what it sees less"*, which is a
   * different clause and a different event.
   */
  readonly haircut: Ratio;
  /**
   * C2, C2.a (item 13.8): the VARIATION the borrower has posted in cash, net, since the loan
   * opened. The pledge above is the initial collateral and it does not move; what moves as the two
   * marks move is money, which is what C2.a says the margin flow is.
   */
  readonly margined: Cash;
  readonly opened: number;
}

/**
 * Law 15: the structural predicate — what makes these terms a stock loan is that they name a line,
 * a number of units out on it and the lien that binds what was put up against them.
 */
export const isStockLoan = (t: AgreementTerms): t is StockLoanTerms =>
  'units' in t && 'lien' in t && 'posted' in t;

/**
 * A1: one open loan as this module reads it. The BORROWER owes — it has the paper and has to bring
 * it back, and it owes the fee every period — so it is the debtor and the lender the creditor.
 * Everything about it is a read of the register except the fee it struck.
 */
export interface StockLoan extends StockLoanTerms {
  readonly id: AgreementId;
  readonly lender: PartyId;
  readonly borrower: PartyId;
  readonly ccy: CurrencyCode;
}

export function loanOf(a: Agreement): StockLoan {
  if (!isStockLoan(a.terms)) {
    throw new TypeError(`Securities Lending A1: ${a.id} is not a stock loan`);
  }
  return { ...a.terms, id: a.id, borrower: a.debtor, lender: a.creditor, ccy: a.ccy };
}

/** The open loans, for anything that needs to know what is out on loan (Law 19: one writer). */
export const loansOpen = (ctx: MechanismContext): readonly StockLoan[] =>
  ctx.agreements
    .ofKind(STOCK_LOAN)
    .filter((a) => a.state === 'performing')
    .map(loanOf);

/** B2, A5: one holder's offer into a borrow book — what it has free and the least it will take. */
export interface Offer {
  readonly lender: PartyId;
  readonly units: Qty;
  readonly floor: Ratio;
}

/**
 * A5, A5.a, E3, B2: WHAT A LENDER MUST BE PAID, per period, as a share of what the paper is worth.
 *
 * While the paper is out the lender is exposed to a borrower that does not bring it back (D1): it
 * then keeps the collateral and buys the line back at whatever it costs, and what that costs is how
 * far this line moves. So the least it will take for a period of that exposure is how far this line
 * has moved ON IT in a period — its own recent surprises about the line, over what it thinks a unit
 * is worth (§46 B3). It is its OWN number and not the market's: two holders of one line who have
 * been differently wrong about it want differently much to lend it, and that disagreement is what
 * gives the book a supply curve instead of a single level (§46 A3).
 *
 * A holder with no outlook on the line has no number to lend at and does not lend. That is a
 * refusal and not a zero: E3 says a fee of zero is a cleared price only if somebody posted it.
 */
function floorOf(view: ParticipantView, instrument: InstrumentId): Option<Ratio> {
  const own = view.outlook(about({ on: 'price', instrument }));
  if (!own.some || own.value.expected <= 0) return none<Ratio>();
  // 0h.2: a holder whose outlook on the line has never been SCORED is in the same position as one
  // with no outlook — it cannot say how far the line moves on it, so it has no number to lend at.
  const moves = own.value.confidence;
  if (!moves.some || moves.value <= 0) return none<Ratio>();
  return some(
    ratioOf(
      asPerPiece(moves.value, 'how wide its own surprises on this line have been'),
      asPerPiece(own.value.expected, 'what it thinks a unit of it is worth'),
      'what a period of the loan puts at risk, as a share of what is out',
    ),
  );
}

/**
 * B4, E1: THE LENDABLE POOL — who actually holds this paper and is willing to part with title for a
 * fee. It is a READ of the register and never a stored number, and it is what caps how large a short
 * can get, because a borrower cannot borrow what nobody has free.
 *
 * Willing is not a flag: a holder is willing with what it holds FREE, at a price of its own. Units
 * already pledged or already lent out are not in the pool, which the register answers without being
 * asked twice; a holder with no view of the line is not in it either, because it has no level.
 */
export function lendable(ctx: MechanismContext, instrument: InstrumentId): readonly Offer[] {
  const out: Offer[] = [];
  for (const holder of ctx.register.holdersOf(instrument)) {
    const p = ctx.parties.get(holder);
    // B2: the lender has it sitting there. A CELL does not lend: a million households each lending
    // its own two shares is a million loans, and the per-member grid cannot carry a fraction of one
    // (Law 8, XI-15). Who lends in this world is whoever holds paper as a named party.
    if (!p.status.alive || p.representation === 'cell') continue;
    const free = downTick(ctx.register.free(holder, instrument));
    if (free <= 0) continue;
    const floor = floorOf(ctx.participant(holder), instrument);
    if (!floor.some) continue;
    out.push({ lender: holder, units: free, floor: floor.value });
  }
  return out;
}

/**
 * A5, A5.a, E3, Law 3: THE FEE IS A PRICE AND IT CLEARS — ONE BOOK PER LINE, and everybody who
 * wants that line in it.
 *
 * Every holder of the line that has a view of it offers what it holds free at its own floor, and
 * every borrower bids its own reservation, so both sides are CURVES and the level the session
 * strikes is where they meet: scarce paper is dear because the cheap lenders run out before the
 * bidders do, and abundant paper is cheap because the last lender allotted is one of many
 * undercutting each other (`sellersCompete`).
 *
 * The old shape could not do that and is the second half of `A-67`. It cleared a session PER
 * BORROWER against every lender posting `price: 'market'` — a book with exactly one level in it, so
 * `outcome.price` was that borrower's own reservation however much paper was on offer — and,
 * because `post` appends and the book is emptied only at the top of a period, a second borrower of
 * the same line posted every lender's offer AGAIN into the same venue, showing twice the supply
 * that exists. Grouping by line is one fix for both: each lender is posted once, the level is
 * struck once, and the allotment is shared.
 *
 * A5.b: where the collateral is cash the same number is quoted as a REBATE on that cash — the
 * lender keeps the difference between what it earns on the cash and what it pays back. It is one
 * number seen from two sides, so there is one number here and the second form is a read of it.
 */
export function runBorrows(ctx: MechanismContext, wanted: readonly Borrowing[]): void {
  // Law 9, A5.a: what is being priced is the scarcity of THAT paper, so the book is the line's.
  const byLine = new Map<InstrumentId, Borrowing[]>();
  for (const w of wanted) {
    if (w.units <= 0) continue;
    const held = byLine.get(w.instrument);
    if (held === undefined) byLine.set(w.instrument, [w]);
    else held.push(w);
  }
  for (const [instrument, bids] of byLine) {
    const supply = lendable(ctx, instrument);
    if (supply.length === 0) {
      // D2, E3: nothing to borrow is a real answer with a consequence — the short cannot be put on.
      for (const w of bids) {
        ctx.record(
          'borrow.none',
          [w.borrower, instrument],
          { borrower: String(w.borrower), instrument: String(instrument), wanted: w.units },
          true,
        );
      }
      continue;
    }
    // Law 8: one book is one money, and it is the money the line itself is in — a bid in another
    // currency is a bid in another book. A borrower that named a different money is not in this one.
    const ccy = ctx.instruments.get(instrument).ccy;
    const here = bids.filter((w) => w.ccy === ccy);
    if (here.length === 0) continue;
    const venue = borrowVenue(instrument);
    if (!ctx.venues.some((v) => v.id === venue)) {
      ctx.openVenue({
        id: venue,
        name: `${String(instrument)} borrow`,
        clearedBy: 'securities-lending',
        unit: ctx.instruments.get(instrument).unit,
        ccy,
        key: { instrument: String(instrument) },
      });
    }
    // A5.a, E3: EVERY POSTING CARRIES A LEVEL. A lender with no level is not undercutting anybody,
    // which is why `price: 'market'` on the whole supply side made the fee the bidder's own number.
    for (const o of supply)
      ctx.post(venue, { party: o.lender, side: 'sell', price: o.floor, qty: o.units });
    for (const w of here)
      ctx.post(venue, { party: w.borrower, side: 'buy', price: w.willPay, qty: w.units });
    const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
    if (!isCleared(outcome)) continue;
    // A5: what the borrow book cleared at is a FEE — a share of what the paper is worth, per period
    // — and not a price per unit of it. This is the one book in the world whose level is a rate,
    // and it is named where the book that struck it says so.
    const fee = asRatio(outcome.price, 'what the borrow cleared at');
    allot(ctx, instrument, ccy, fee, outcome.fills, here);
  }
}

/**
 * A1, C3 (Clearing C3's sibling): WHICH LENDER LENT TO WHICH BORROWER. The session says what the
 * fee is and how much paper changed hands; a loan is between two named parties, so somebody has to
 * be paired with somebody.
 *
 * The rule is the venue's and it is stated once: the borrower that bid most is served first, out of
 * the lender that asked least — what a borrower desperate enough to outbid the others gets for it
 * is the cheapest paper in the book. Ties go in the parties' own order, which is stable, so a run
 * is the same run twice from one seed (Audit D3).
 */
function allot(
  ctx: MechanismContext,
  instrument: InstrumentId,
  ccy: CurrencyCode,
  fee: Ratio,
  fills: readonly Fill[],
  bids: readonly Borrowing[],
): void {
  const lenders = fills
    .filter((f) => f.side === 'sell' && f.qty > 0)
    .map((f) => ({ lender: f.party, left: downTick(f.qty) }))
    .sort((a, b) => (a.left === b.left ? (a.lender < b.lender ? -1 : 1) : 0));
  const wanted = fills
    .filter((f) => f.side === 'buy' && f.qty > 0)
    .sort((a, b) => (a.at === b.at ? (a.party < b.party ? -1 : 1) : b.at - a.at));
  let next = 0;
  for (const f of wanted) {
    const w = bids.find((b) => b.borrower === f.party);
    if (w === undefined) continue;
    let owed = downTick(f.qty);
    while (owed > 0 && next < lenders.length) {
      const from = lenders[next];
      if (from === undefined) break;
      if (from.left <= 0) {
        next += 1;
        continue;
      }
      const taken = downTick(atMost(owed, from.left, 'it lends what it has in the book'));
      if (taken <= 0) break;
      openLoan(ctx, {
        borrower: w.borrower,
        lender: from.lender,
        instrument,
        units: taken,
        ccy,
        collateral: w.collateral,
        fee,
      });
      from.left = subQty(from.left, taken, 'what it has left to lend');
      owed = subQty(owed, taken, 'what it still has to find');
    }
  }
}

/** C1: what must be posted for a borrow of this value, at this lender's own haircut. */
const collateralFor = (worth: Cash, haircut: Ratio): Cash =>
  scale(
    worth,
    plus(asRatio(1, 'the whole of it'), haircut, 'the margin over what it took'),
    'what the borrower must put up',
  );

/**
 * C1: THE HAIRCUT IS THE LENDER'S, and it is the gap the lender has to be able to cover.
 *
 * It has to be able to sell the collateral and be whole, and it is not whole if the borrowed line
 * has risen or the collateral has fallen before it gets there — so the margin it wants over the
 * loan is how far apart it thinks the two marks can move in a period. Both halves are the same read
 * for the same reason as `floorOf`: this lender's own surprises, on each of the two lines, over
 * what it thinks each is worth.
 *
 * It used to come in on the BORROWER's side, which is the transaction the wrong way round: a
 * borrower naming its own haircut is the party at risk asking the party that owes it how much cover
 * it would like to give. Nothing when it has no view of the collateral — a lender that does not
 * know what it is being given does not take it.
 */
function haircutOf(
  view: ParticipantView,
  instrument: InstrumentId,
  collateral: InstrumentId,
): Option<Ratio> {
  const onLoan = floorOf(view, instrument);
  const onPledge = floorOf(view, collateral);
  if (!onLoan.some || !onPledge.some) return none<Ratio>();
  return some(plus(onLoan.value, onPledge.value, 'how far apart the two marks can move'));
}

/**
 * A1, A2, C1, C4: THE TRANSACTION, and both legs of it in one numbered instruction. The security
 * goes to the borrower and TITLE GOES WITH IT; the collateral is pledged to the lender and leaves
 * the borrower's free balance, so it cannot be counted as available by both sides.
 *
 * C1: the collateral is worth MORE than what was lent, by the lender's own haircut, because the
 * lender has to be able to sell it and be whole. Collateral exactly equal to the loan, re-marked to
 * the same price, is a gap covered by nothing.
 */
function openLoan(
  ctx: MechanismContext,
  d: {
    readonly lender: PartyId;
    readonly borrower: PartyId;
    readonly instrument: InstrumentId;
    readonly units: Qty;
    readonly ccy: CurrencyCode;
    readonly collateral: InstrumentId;
    readonly fee: Ratio;
  },
): void {
  if (d.lender === d.borrower) return;
  // XI-6: what it is worth is what the market printed for it, at a price a reader can look up.
  const mark = lastMarkOf(ctx, d.instrument);
  if (mark === undefined) return;
  const haircut = haircutOf(ctx.participant(d.lender), d.instrument, d.collateral);
  if (!haircut.some) return;
  const worth = valueAt(mark, d.units, d.ccy, 'what the borrowed paper is worth');
  const needed = collateralFor(worth, haircut.value);
  const price = ctx.prices.latest(d.collateral, ctx.period);
  if (!price.some || price.value.price <= 0) return;
  const posted = downTick(amountOf(needed, price.value.price, 'units of collateral'));
  if (posted <= 0 || ctx.register.free(d.borrower, d.collateral) < posted) return;
  const secures = `stockLoan:${String(d.lender)}:${String(d.borrower)}:${String(d.instrument)}`;
  const legs: Leg[] = [
    {
      // A2: legal title passes. The borrower can sell what it borrowed; that is the entire point.
      kind: 'asset',
      from: d.lender,
      to: d.borrower,
      instrument: d.instrument,
      qty: d.units,
      pricePerUnit: some(mark),
      accruedPerUnit: none(),
    },
    {
      kind: 'pledge',
      pledgor: d.borrower,
      beneficiary: d.lender,
      instrument: d.collateral,
      qty: posted,
      secures,
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'transfer',
    reason: `${String(d.borrower)} borrows ${d.units} of ${String(d.instrument)} from ${String(d.lender)}`,
  });
  if (r.outcome !== 'settled') return;
  const lien = lienFor(ctx, d.borrower, d.collateral, secures);
  if (!lien.some) return;
  const terms: StockLoanTerms = {
    kind: STOCK_LOAN,
    instrument: d.instrument,
    units: d.units,
    collateral: d.collateral,
    lien: lien.value,
    posted,
    fee: d.fee,
    // C1, C2 (13.8): what this lender required, kept so the loan can be re-marked against it.
    haircut: haircut.value,
    margined: noCash(d.ccy),
    opened: ctx.period,
  };
  // XI-8: the loan is a COMMITMENT and the kernel keeps it. It owes nothing the instant it is
  // struck — the fee falls due at the end of the period and `charge` moves it then — which is why
  // the store admits a zero (item 9.1a).
  ctx.owes({
    debtor: d.borrower,
    creditor: d.lender,
    ccy: d.ccy,
    owed: 0,
    terms,
    why: `${d.borrower} has ${d.units} of ${d.instrument} from ${d.lender}`,
  });
  ctx.record(
    'borrow.opened',
    [d.lender, d.borrower, d.instrument],
    {
      lender: String(d.lender),
      borrower: String(d.borrower),
      instrument: String(d.instrument),
      units: d.units,
      collateral: String(d.collateral),
      posted,
      fee: d.fee,
    },
    true,
  );
}

/** Register D5.a: the lien the pledge just wrote, found by what it secures. One name, one lien. */
function lienFor(
  ctx: MechanismContext,
  pledgor: PartyId,
  instrument: InstrumentId,
  secures: string,
): Option<LienId> {
  const holding = ctx.register.holding(pledgor, instrument);
  if (!holding.some) return none<LienId>();
  // Law 4 (16.2): a second loan between the same two names on the same line pledges under the same
  // reason, and the FIRST lien answered for every one of them — three rows named lien 101, the
  // register held 102 and 104, and the return of the second threw. The lien this loan created is
  // the one no open loan already holds.
  const taken = new Set(loansOpen(ctx).map((l) => l.lien));
  for (const l of holding.value.liens) {
    if (l.reason.includes(secures) && !taken.has(l.id)) return some(l.id);
  }
  return none<LienId>();
}

/**
 * A3: THE MANUFACTURED PAYMENT, which is the half that makes it a stock loan. The issuer pays the
 * REGISTERED holder, which is now the borrower, and the borrower passes it on — so the lender's cash
 * flows are unchanged and it has not paid a fee to lose its income.
 *
 * It is read off what actually reached the borrower this period on that line (Law 19), never
 * recomputed from the terms: the payment that was made is the payment that is passed on, and if the
 * issuer paid nothing there is nothing to manufacture.
 */
export function manufacture(ctx: MechanismContext): void {
  for (const loan of loansOpen(ctx)) {
    const paid = receivedOn(ctx, loan);
    if (paid.pieces <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          // 0i.5: Securities Lending A3: the coupon passed on, so the economics stay with the lender.
          receipt: { of: 'manufactured' },
          from: ctx.accountOf(loan.borrower, loan.ccy),
          to: ctx.accountOf(loan.lender, loan.ccy),
          ccy: loan.ccy,
          amount: downTick(paid.pieces),
        },
      ],
      cause: 'corporateAction',
      reason: `${String(loan.borrower)} passes on ${String(loan.instrument)} to ${String(loan.lender)}`,
    });
    if (r.outcome !== 'settled') continue;
    ctx.record(
      'borrow.manufactured',
      [loan.lender, loan.borrower, loan.instrument],
      {
        lender: String(loan.lender),
        borrower: String(loan.borrower),
        instrument: String(loan.instrument),
        amount: paid.pieces,
        ccy: paid.ccy,
      },
      true,
    );
  }
}

/** What the issuer actually paid the registered holder this period on this line, off the wire. */
function receivedOn(ctx: MechanismContext, loan: StockLoan): Cash {
  const amounts: Cash[] = [];
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled' || r.instruction.cause !== 'coupon') continue;
    if (!r.instruction.reason.includes(String(loan.instrument))) continue;
    for (const leg of r.instruction.legs) {
      // A3: the money that reached the registered holder on that line. `isMoneyLeg` asks what SHAPE
      // a leg is, which is the kernel's own dispatch and not a question about an instrument kind.
      if (!isMoneyLeg(leg) || leg.to.holder !== loan.borrower || leg.ccy !== loan.ccy) continue;
      amounts.push(heldAsMoney(leg.amount, loan.ccy, 'what reached the registered holder'));
    }
  }
  const total = sumCash(loan.ccy, amounts, 'what reached the registered holder').value;
  // E2: it passes on what the units it BORROWED earned, not what its whole holding earned. A
  // borrower that already owned some of the line keeps its own.
  const held = ctx.register.quantity(loan.borrower, loan.instrument);
  return held <= 0
    ? noCash(loan.ccy)
    : scale(total, ratioOf(loan.units, held, 'the borrowed share of what it holds'), 'passed on');
}

/**
 * A5: THE FEE, every period, real money between two named parties, on what the paper is worth NOW —
 * so a line that has risen costs more to have borrowed, without anybody re-striking anything.
 *
 * C2 and C2.a are `remark`'s, below (item 13.8): this moves the FEE, which is a price the borrower
 * pays for having the paper, and that is a different flow from the margin that keeps the lender
 * covered while it is out. Both are real money between the same two named parties every period, and
 * they are two payments because they are two obligations.
 */
/**
 * XI-6, Clearing F1.a (21.19, fixed at 15.4): WHAT THE PAPER IS WORTH TO A LENDER — the last print
 * at or before now, which is the read a lender can make from a phase that sits before the paper's
 * own market has run this period. Asking for THIS period's mark from here threw `NotYetProduced`
 * and stopped the world (period 55 of the `growth` rig; period 10 of the opens rig once cells'
 * marks were booked per member). Money is worth one of itself; a line nothing has ever printed is
 * worth nothing a lender can lend against, which is a refusal and not a zero.
 */
function lastMarkOf(ctx: MechanismContext, instrument: InstrumentId): PerPiece | undefined {
  const i = ctx.instruments.get(instrument);
  if (ctx.registry.instrumentKind(i.kind).pricing === 'money')
    return asPerPiece(1, 'money is worth one of itself');
  const print = ctx.prices.latest(instrument, ctx.period);
  return print.some && print.value.price > 0 ? print.value.price : undefined;
}

export function charge(ctx: MechanismContext): void {
  for (const loan of loansOpen(ctx)) {
    const mark = lastMarkOf(ctx, loan.instrument);
    if (mark === undefined) continue;
    const fee = downTick(
      scale(
        valueAt(mark, loan.units, loan.ccy, 'what is out on loan'),
        loan.fee,
        'what the borrow cost this period',
      ).pieces,
    );
    if (fee <= 0) continue;
    ctx.settle({
      legs: [
        {
          kind: 'money',
          // 0i.5: Securities Lending: what the borrower pays to borrow the stock.
          receipt: { of: 'fee' },
          from: ctx.accountOf(loan.borrower, loan.ccy),
          to: ctx.accountOf(loan.lender, loan.ccy),
          ccy: loan.ccy,
          amount: fee,
        },
      ],
      cause: 'transfer',
      reason: `${String(loan.borrower)} pays the borrow fee on ${String(loan.instrument)}`,
    });
  }
}

/**
 * C2, C2.a, C1 (item 13.8, `E-14`): BOTH SIDES MARKED EVERY PERIOD, AND THE DIFFERENCE CALLED.
 *
 * *"When the borrowed security rises, the borrower posts more collateral"* (C2), and *"the margin
 * flow is real money moving between two named parties"* (C2.a). Until this, `charge` moved the fee
 * and NOTHING re-marked the collateral: between the strike and the return the borrowed line could
 * double and the lender's cover did not move, so C1's haircut — one period's worth of the two marks
 * moving apart — was all that stood behind the whole term of the loan. It eroded in silence, which
 * is why it is `E-14`.
 *
 * WHAT IS REQUIRED is what C1 already says: the paper at today's mark, plus this lender's own
 * haircut over it, which is a TERM of the loan and not re-asked (C4 is a different event). WHAT IS
 * COVERING IT is the pledged collateral at today's mark, plus whatever cash has been called so far.
 * The difference is `registry/margin.ts`'s `callFor` — the one definition of a margin call in this
 * world, which a prime broker asks of a portfolio and this asks of a loan (Law 4): two callers of
 * one mechanism, which is what this step was written to make sure of.
 *
 * IT MOVES BOTH WAYS. A borrowed line that FELL leaves the lender holding cover it is not owed, and
 * it goes back — a mechanism that took margin and never returned it is a one-sided flow nothing ever
 * fails on (Law 5). The only thing it cannot do is hand back more than was posted, which is
 * arithmetic impossibility and says so.
 *
 * AND IT CAN FAIL. The instruction goes to the wire for the whole call and a borrower that cannot
 * pay gets a REFUSED instruction (Money E1) — the lender is then uncovered, which is a real state
 * with both parties named on it, and what happens next is D1's when the term falls due.
 */
export function remark(ctx: MechanismContext): void {
  for (const loan of loansOpen(ctx)) {
    const onLoan = lastMarkOf(ctx, loan.instrument);
    const onPledge = ctx.prices.latest(loan.collateral, ctx.period);
    if (onLoan === undefined || !onPledge.some || onPledge.value.price <= 0) continue;
    const call = callFor({
      // C1: the paper at today's mark, with this lender's haircut over it.
      required: collateralFor(
        valueAt(onLoan, loan.units, loan.ccy, 'what is out on loan now'),
        loan.haircut,
      ),
      // C2: and what is actually there — the pledge at today's mark, and the cash called so far.
      covering: plus(
        valueAt(onPledge.value.price, loan.posted, loan.ccy, 'what the pledge is worth now'),
        loan.margined,
        'what is covering it',
      ),
    });
    if (call.pieces === 0) continue;
    const owing = call.pieces > 0;
    // Law 6: it cannot hand back more cover than was posted, which is impossible rather than
    // bounded — there is no further cash of the borrower's for the lender to return.
    const wanted = owing
      ? call
      : atMostCash(
          absolute(call, 'what it is over-covered by'),
          loan.margined,
          'it cannot give back more than was posted',
        );
    // Law 8: money moves on the money's own grid, and it is what the call ACTUALLY reaches.
    const moving = downTick(wanted.pieces);
    if (moving <= 0) continue;
    const payer = owing ? loan.borrower : loan.lender;
    const payee = owing ? loan.lender : loan.borrower;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          // 0i.5: collateral moving to keep the loan covered.
          receipt: { of: 'margin' },
          from: ctx.accountOf(payer, loan.ccy),
          to: ctx.accountOf(payee, loan.ccy),
          ccy: loan.ccy,
          amount: moving,
        },
      ],
      cause: 'transfer',
      reason: owing
        ? `${String(loan.borrower)} meets a margin call on ${String(loan.instrument)}`
        : `${String(loan.lender)} returns cover on ${String(loan.instrument)}`,
    });
    const met = r.outcome === 'settled';
    if (met) {
      // Law 19: what has been called is what the calls have actually settled for, and this is the
      // one writer of that fact — the same shape the capital call's `drawn` has.
      const now: StockLoanTerms = {
        kind: STOCK_LOAN,
        instrument: loan.instrument,
        units: loan.units,
        collateral: loan.collateral,
        lien: loan.lien,
        posted: loan.posted,
        fee: loan.fee,
        haircut: loan.haircut,
        margined: owing
          ? plus(
              loan.margined,
              heldAsMoney(moving, loan.ccy, 'what this call brought in'),
              'what it has posted',
            )
          : minus(
              loan.margined,
              heldAsMoney(moving, loan.ccy, 'what went back'),
              'what it still has posted',
            ),
        opened: loan.opened,
      };
      ctx.restate(loan.id, now);
    }
    ctx.record(
      'borrow.margin',
      [loan.lender, loan.borrower, loan.instrument],
      {
        lender: String(loan.lender),
        borrower: String(loan.borrower),
        instrument: String(loan.instrument),
        // C2.a: which way it moved and how much, and whether it actually moved.
        // C2.a: signed, because which way the money went is the fact — a call met and cover
        // returned are the same mechanism and the sign is what tells them apart.
        called: owing ? moving : negated(moving, 'what went back to the borrower'),
        met,
        margined: loan.margined.pieces,
        ccy: loan.margined.ccy,
      },
      true,
    );
  }
}

/**
 * A4, D3: WHICH LOANS COME BACK THIS PERIOD — the ones whose term is up.
 *
 * A borrow that never ends is a position with no cost of staying on and no moment where the
 * borrower has to find the paper, and D2's squeeze is exactly that moment: the term falls due, the
 * shorts must buy, and the pool they are buying out of is the one B4 measures. A borrower that
 * still wants the position bids for the line again in the same book the same period, at whatever
 * the fee has become — which is a roll, and it is not free.
 */
function due(ctx: MechanismContext): readonly StockLoan[] {
  const term = ctx.params.periods(BORROW_TERM);
  return loansOpen(ctx).filter((l) => ctx.period - l.opened >= term);
}

/**
 * A4, D1, D3: IT TERMINATES. The security comes back and the collateral goes back, in one
 * instruction, so there is no instant where the borrower has both.
 *
 * D1: and a borrower that cannot return it has FAILED — the lender keeps the collateral and is left
 * to buy the paper back in the market at whatever it costs. The failed return terminates, which is
 * the point: the loan does not sit open for ever against a borrower who cannot close it.
 */
export function returnLoans(ctx: MechanismContext, closing: readonly StockLoan[]): void {
  for (const loan of closing) {
    const held = downTick(ctx.register.free(loan.borrower, loan.instrument));
    const back = downTick(atMost(held, loan.units, 'it returns what it borrowed'));
    const failed = back < loan.units;
    // Register D5: the lien ends either way. What differs is who has the units when it does — the
    // borrower, because the paper came back, or the lender, because it did not.
    const freed = ctx.settle({
      legs: [
        {
          kind: 'release',
          pledgor: loan.borrower,
          beneficiary: loan.lender,
          instrument: loan.collateral,
          lien: loan.lien,
        },
      ],
      cause: failed ? 'default' : 'transfer',
      reason: `${String(loan.borrower)} ends the borrow of ${String(loan.instrument)}`,
    });
    if (freed.outcome !== 'settled') continue;
    // A4, D1: the paper goes back, or the collateral does not. They are two instructions and not
    // one because the register answers "can this move?" against what is bound BEFORE the
    // instruction runs — units the same instruction is about to free are still bound when it asks.
    const legs: Leg[] = [];
    if (back > 0) {
      const mark = lastMarkOf(ctx, loan.instrument);
      legs.push({
        kind: 'asset',
        from: loan.borrower,
        to: loan.lender,
        instrument: loan.instrument,
        qty: back,
        pricePerUnit: mark === undefined ? none() : some(mark),
        accruedPerUnit: none(),
      });
    }
    /**
     * C2.a, A4 (item 13.8): AND THE CASH MARGIN GOES BACK WITH THE COLLATERAL. What the borrower
     * posted as variation is the borrower's, held against a loan that is ending, so it returns in
     * the same instruction the paper does — a margin flow that only ever went one way would be a
     * flow with one leg (Law 5), and the return is where the other one is.
     *
     * D1: unless the borrower FAILED, and then the lender keeps it for the same reason it keeps the
     * collateral — it is left to buy the line back at whatever it costs, and whether the two come
     * to the same is its outcome and not a number anybody balances (C1).
     */
    if (!failed && loan.margined.pieces > 0) {
      legs.push({
        kind: 'money',
        // 0i.5: the collateral, returned when the loan ends.
        receipt: { of: 'margin' },
        from: ctx.accountOf(loan.lender, loan.ccy),
        to: ctx.accountOf(loan.borrower, loan.ccy),
        ccy: loan.ccy,
        amount: downTick(loan.margined.pieces),
      });
    }
    if (failed) {
      // D1: THE LENDER KEEPS THE COLLATERAL and is left to buy the line back in the market at
      // whatever it costs. Keeping it is title and not a lien: a loan that has terminated cannot go
      // on securing anything, and collateral encumbered to a row that no longer exists is units
      // nobody can reach. Whether the two are worth the same is not asked — that is what a haircut
      // is for, and whether it was enough is the lender's outcome (C1).
      const at = lastMarkOf(ctx, loan.collateral);
      legs.push({
        kind: 'asset',
        from: loan.borrower,
        to: loan.lender,
        instrument: loan.collateral,
        qty: downTick(
          atMost(
            loan.posted,
            ctx.register.free(loan.borrower, loan.collateral),
            'what is there to take',
          ),
        ),
        pricePerUnit: at === undefined ? none() : some(at),
        accruedPerUnit: none(),
      });
    }
    // Money D1: an instruction with no legs is not an instruction. A borrow that came back in full
    // against collateral already freed has nothing left to move, and the row simply closes.
    if (legs.length > 0) {
      const r = ctx.settle({
        legs,
        cause: failed ? 'default' : 'transfer',
        reason: `${String(loan.borrower)} returns ${back} of ${String(loan.instrument)}`,
      });
      if (r.outcome !== 'settled') continue;
    }
    // A4, XI-8: the commitment ends and the record says it existed. `terminated` and not
    // `discharged`, because the paper coming back is the loan running its course and not a debt
    // being settled — and a failed return ends it too (D1), which a discharge could not say.
    ctx.endAgreement(loan.id, failed ? 'the borrower did not bring it back' : 'returned at term');
    ctx.record(
      failed ? 'borrow.failed' : 'borrow.returned',
      [loan.lender, loan.borrower, loan.instrument],
      {
        lender: String(loan.lender),
        borrower: String(loan.borrower),
        instrument: String(loan.instrument),
        returned: back,
        owed: loan.units,
        // C2.a: and what the variation came to over the life of it, which went back or did not.
        margined: loan.margined.pieces,
        ccy: loan.margined.ccy,
      },
      true,
    );
  }
}

/**
 * E1, E2, B4: the three things that must be true while paper is out on loan, MEASURED and never
 * enforced. Each of them breaks silently, which is why the audit is where they live.
 */
function borrows(): Family {
  return {
    name: 'ownership',
    contributor: 'securities-lending',
    spec: 'Securities Lending B4 Securities Lending E1 Securities Lending E2 Equity C7',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live) continue;
        const holders = view.register.holdersOf(i.id);
        if (holders.length === 0) continue;
        for (const h of holders) {
          const units: number = view.register.quantity(h, i.id);
          if (units >= 0) continue;
          // E1: a negative position nobody lent is an invented security. A short is a BORROW and
          // the borrowed units are in somebody's name; there is no other way to be short here.
          out.push({
            family: 'ownership',
            spec: 'Securities Lending E1',
            owner: i.id,
            size: -units,
            unit: i.unit,
            period: view.period,
            message: `${String(h)} is short ${-units} of ${i.id} and nobody lent it`,
          });
        }
      }
      return out;
    },
  };
}

export function securitiesLending(): SystemModule {
  return {
    id: 'securities-lending',
    agreementKinds: [
      {
        id: STOCK_LOAN,
        // XI-8: paper lent is paper owed back. The obligation to return it is a debt in kind and
        // survives either side, which is what keeps the borrow from becoming a naked short (C1).
        binds: 'whoeverSucceeds',
        what: 'a named borrower holding a named lender\u2019s paper against collateral, for a fee',
      },
    ],
    // XI-8, item 9.1: no nouns. The open loans were this module's private book and are agreements.
    spec: 'Securities Lending',
    requires: ['equity'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: BORROW_TERM,
        value: 4,
        unit: 'periods',
        dimension: 'periods',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Securities Lending A4: how long the paper stays out before it goes back. A term of the contract, stated with it, and it is what makes D2 possible: a borrower that still wants the position has to BUY THE LINE BACK before the term is up, which is what "shorts must buy" means. It is not a forecast of when a short is closed — a borrower that still needs the line asks again in the same book, at whatever the fee has become by then.',
      },
    ],
    phases: [
      {
        name: 'borrow.economics',
        spec: 'Securities Lending A3 Securities Lending A4 Securities Lending A5 Securities Lending C1 Securities Lending C2 Securities Lending C2.a Securities Lending D1',
        // A3: after the issuer has paid the registered holder, because what is passed on is what
        // arrived. A phase that manufactured a payment before the payment existed would be
        // inventing the lender's income rather than passing it through (Law 19).
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext): void => {
          manufacture(ctx);
          charge(ctx);
          /**
           * C2 (item 13.8): BOTH SIDES MARKED, before anything comes back. A loan whose term is up
           * this period is marked one last time and then returns what it holds — so the borrower
           * settles what the move cost rather than walking away from it at the door, and the cash
           * that goes back with the collateral is the right number.
           */
          remark(ctx);
          returnLoans(ctx, due(ctx));
        },
      },
      {
        name: 'borrow.session',
        spec: 'Securities Lending A5 Securities Lending A5.a Securities Lending B1 Securities Lending B4 Securities Lending E3',
        // A2, B1: BEFORE THE MARKETS, because the entire point of borrowing paper is to be able to
        // deliver it — a borrow struck after the session it was for is paper nobody could sell. It
        // is after `borrow.economics` so that what went back this period is back in its lender's
        // free balance before the book counts what there is to lend (B4).
        anchor: { after: 'borrow.economics' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext): void => {
          runBorrows(ctx, ctx.borrowsWanted());
        },
      },
    ],
    participants: [],
    families: [borrows()],
  };
}

/**
 * A5, B1, Law 12: THE BORROWER'S RESERVATION IS NOT MADE HERE, and that is the point of the door.
 *
 * `wantsToBorrow` used to be exported from this file for a caller that could not exist: a module
 * never imports another module, so the only party that could have called it was one this module
 * owns, and it owns none. What it computed was `owed/(owed + equity)` — the debt share of funding,
 * a dimensionless ratio with no periodicity, called "what a period of its own money costs it", the
 * identical mistake `A-58` found in `securitisation:priceFor` with the identical comment.
 *
 * The sentence the comment was reaching for is right and is now true where it belongs: a borrower
 * will not pay more for a period of somebody else's paper than a period of its own money costs it,
 * and the party that knows that number is the party, in the module that owns it — a dealing desk
 * already prices its own quotes off it (`banks/dealing-quote.ts:rateOf`). It arrives here through
 * `ctx.borrowsWanted` as the reservation on a `Borrowing`, and this module never forms one.
 */

/** A5.b: the same fee seen from the cash side — what the lender pays back on cash it was given. */
export const rebateOf = (fee: Ratio, earns: Ratio): Ratio =>
  minus(earns, fee, 'what it hands back out of what the cash earned');

/** B4: how large a short in this line could get — a read of the pool, and a real constraint. */
export const poolOf = (supply: readonly { readonly units: Qty }[]): number =>
  sum(supply.map((s) => s.units)).value;
