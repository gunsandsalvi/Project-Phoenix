/**
 * Equity: the residual claim on a firm, priced by opinions that disagree, and what the firm does
 * with it.
 *
 * @spec Equity A1 Equity A2 Equity A3 Equity A4 Equity A5 Equity A6 Equity B1 Equity B2 Equity B3 Equity B4 Equity B4.a Equity B6 Equity C1 Equity C1.a Equity C1.b Equity C2 Equity C2.a Equity C2.e Equity C3 Equity C4 Equity D1 Equity D1.a Equity D1.b Equity D1.c Equity D2 Equity D2.a Equity D2.b Equity D2.c Equity D3 Equity D3.a Equity D3.b Equity D4 Equity E4 Equity F1 Equity F2 Equity F3 Equity F4 Equity G3 Register E1 Register E1.a Register E4 Register E5 Clearing B2 Clearing C4.a Firm E4 Firm E5 XI-13 XI-15 Law 4 Law 9 Law 15
 *
 * WHY IT IS HERE. Nothing in this world had a price that was not a promise: a bond is worth its
 * flows discounted, a bill is worth its face, a good is worth what a buyer paid for a thing it
 * needed. A SHARE PROMISES NOTHING. There is no stream to discount that anybody is entitled to,
 * so the only thing that can price it is what participants think, and the only reason they trade
 * is that they think different things (§46 A3, XI-13). It is the first instrument in this world
 * whose price is nothing but disagreement, and it is what item 10 needs before it can ask what
 * equity costs a firm.
 *
 * WHAT IS NOT HERE. A share is a residual claim on a firm and the firm's own books are not a price
 * of it (B3): what the firm thinks one is worth is its own reservation and it is used as one — to
 * decide whether it would rather buy its own back or pay the money out, and as the level it will
 * not sell below. Every price in this module's markets is what a session cleared at, or it is a
 * stale mark that says so (Clearing E4).
 *
 * WHAT A FIRM'S DEATH DOES TO IT (E4). Nothing here wipes anything. A share ranks LAST (share.ts),
 * so the estate that succeeds a dead firm reaches its shareholders only after every creditor is
 * paid, pays them what is left, and writes off the rest at zero — which is the wipe, arrived at by
 * the waterfall rather than by a special case, and it is also the residual F2 entitles them to when
 * there is one. What this module does is stop pricing the line: a claim on a liquidation is not the
 * claim anybody formed an opinion of, so nobody posts and the print goes visibly stale.
 */
import { type Cash, type PerPiece, amountOf, asAmount, asCash, asNamed, heldAsMoney, minus, negated, plus, pricedAt } from '../../core/measure.js';
import { forbid } from '../../core/assert.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { AuditView } from '../../audit/view.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import {
  agreementKindId,
  paramId,
  type CorporateActionId,
  type InstrumentId,
  type MarketId,
  type PartyId,
} from '../../core/ids.js';
import type { AgreementTerms } from '../../register/agreements.js';
import { combineDust, div, sum, withinDust } from '../../core/num.js';
import { downTick, type Qty } from '../../core/tick.js';
import { none, some } from '../../core/option.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { anchorOf, quarterClosedBy } from '../../calendar/fiscal.js';
import { compareCivil } from '../../calendar/civil.js';
import type { CorporateAction } from '../../register/corporate.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import { weightOf, gridPerMember } from '../../parties/party.js';
import { prng } from '../../rng/prng.js';
import { issuerOf, type Instrument } from '../../register/instruments.js';
import { CENT_TICK, MONEY_PIECES, SHARE_PIECES } from '../../registry/grid.js';
import { displayName } from '../../registry/naming.js';
import type { ParamDecl } from '../../registry/params.js';
import { FIRM, HOUSEHOLD } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  OPENING_SHARE,
  equityParam,
  equityLineOf,
  equityMarketOf,
  type EquityDecl,
} from './data.js';
import { buybackOrder, decideEquity, dividendFor, EQUITY_PLAN, nothingDecided, type EquityPlan } from './decide.js';
import { floatations } from './float.js';
import { freeFloat, marketCapitalisation } from './opinion.js';
import { SHARE, shareKind, shareTerms, votesOf, type ShareTerms } from './share.js';
import { asQty } from '../../core/tick.js';
import { ownFundingThisPeriod } from '../../registry/funding.js';

export * from './data.js';
export * from './share.js';
export { decideEquity, buybackOrder, dividendFor } from './decide.js';
export { wouldFloat, float, floatations, type Flotation } from './float.js';
export type { EquityPlan } from './decide.js';
export { marketCapitalisation, freeFloat } from './opinion.js';

/** What this module remembers between periods: nothing but what it has already said out loud. */
interface Book {
  /** E4: the lines whose issuer has been succeeded, so the wipe is announced once and not weekly. */
  succeeded: string[];
  /** D3, item 10: the quarter each firm last DECLARED for, so a board declares once for one (Law 4). */
  declared: Record<string, string>;
}

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('equity', () => ({ succeeded: [], declared: {} }));
}

/**
 * A6, Register F2: whether the firm that PROMISED this line still owes it. An estate succeeding a
 * dead firm becomes the issuer of record and the terms still name who promised it, so the two
 * disagreeing is exactly the statement "the firm is gone" — read off the instrument rather than
 * inferred from a party's kind or from an event somebody has to remember.
 */
function stillItsIssuer(i: Instrument): boolean {
  return issuerOf(i) === shareTerms(i).issuer;
}

/** D3, item 10: the one number the declaration needs — how long after the record date it pays. */
export const EQUITY_PAYOUT_LAG = paramId('equity.payoutLag');

function paramsOf(rows: readonly EquityDecl[]): ParamDecl[] {
  return [
    {
      id: EQUITY_PAYOUT_LAG,
      value: 2,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'model',
      why: 'Equity D3, Law 2: between the record date and the payable date a company works out who it owes and instructs its bank, and that takes time. It is a settlement convention of the market — a real-world primitive, imported (Law 2) — and it is the reason a declared dividend is a LIABILITY for an interval rather than an instant. Shorten it to zero and the declaration and the payment collapse back into one event, which is what this world used to do.',
    },
    {
      id: OPENING_SHARE,
      // Law 8: ONE TICK OF A SHARE, said in the grid's own terms rather than as a number — a
      // cent a share, in the pieces of money a price is counted in (`Registry.priceOf`).
      value: (CENT_TICK * MONEY_PIECES) / SHARE_PIECES,
      unit: 'pieces of money per share at the seed (one cent)',
      dimension: 'price',
      kind: 'resolution',
      owner: 'model',
      why: 'Seed C4, Law 2: a market that has never traded has no price (XI-6) and a world that opens with shares outstanding has to say what one is. It is a RESOLUTION: double it and halve every share count the seed states and no value, flow or decision moves — which is exactly what a split does (D4), so the invariance is a mechanism in this world and a test of it, not a claim about one. ONE TICK, because a resolution should be the finest the grid allows and this one decides HOW MANY OWNERS A LINE CAN REACH: a share is indivisible and a cell holds whole pieces per member (XI-15), so at a dollar a share a firm’s book came to fewer shares than this world has savers and every firm under thirty million dollars opened with a line nobody held at all (10f.1). A penny a share cuts the same book into a hundred times as many pieces, and by D4 that is the only thing it changes.',
    },
    ...rows.map((r): ParamDecl => ({
      id: equityParam(r.firm, 'payoutPatience'),
      value: r.payoutPatience,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: `Equity D2.c, Firm E5: over how many of its own periods ${r.firm}'s management distributes what it has spare. ${r.why} It is the whole of how fast money leaves the firm for its owners, and two managements that are not equally patient distribute differently out of the same cash.`,
    })),
  ];
}

/**
 * D1, D2, D3: the firm's own decision about its own line, taken with the firm's own view, once,
 * published under its own name, and read back by the order it posts (Law 4).
 *
 * It runs after the firm has planned its period, because what it has spare is what its own funding
 * read said (Firm E4) — one number, one writer — and before the session, because a decision that
 * read the session it is about to be in would be reading its own answer (Clearing A4).
 */
function decide(ctx: MechanismContext, seed: string, row: EquityDecl): void {
  const line = ctx.instruments.get(equityLineOf(row.firm));
  if (!line.status.live) return;
  if (!stillItsIssuer(line)) {
    announceSuccession(ctx, row, line);
    return;
  }
  const firm = row.firm as PartyId;
  if (!ctx.parties.get(firm).status.alive) return;
  const view = ctx.participant(firm);
  const funding = ownFundingThisPeriod(view, ctx.period);
  if (!funding.some) return;
  const short = funding.value.short;
  const decided = decideEquity(
    view,
    line.id,
    /**
     * §29 C5: a private company has no book to post into, and what it may do about its own shares
     * is the whole of the difference between the two (`decideEquity`).
     *
     * Law 19: off the LINE. `EquityDecl.listed` is the opening condition (Seed B1.a) and it goes
     * stale the instant a firm floats (10f.2) or is taken private; whether a line trades is a fact
     * about the line, and the register is where it is kept.
     */
    line.market,
    line.issued,
    negated(asCash(short, 'what it is short of'), 'what it has spare'),
    ctx.params.periods(equityParam(row.firm, 'payoutPatience')),
  );
  if (!decided.some) return;
  const plan = decided.value;
  // D1, D1.c: new shares are the ISSUER's own supply for this session, cleared by the same solver
  // at one price with everybody else's orders in the book — and withdrawn if nobody will pay the
  // least it will take, which is a failed issue and has consequences (Clearing C4.a).
  if (plan.issue > 0 && plan.market.some) {
    ctx.offer({
      market: plan.market.value,
      issuer: firm,
      size: plan.issue,
      reservation: some(plan.reservation),
      allotment: 'uniformPrice',
    });
  }
  // Law 15, 0e′.4: the bid goes in this firm's own working store, which is what its `markets` and
  // `orders` read back. The event below is the record of what it decided; it is written from the
  // same plan and never read back by this module.
  const slot = ctx.workingOf(firm, EQUITY_PLAN, nothingDecided);
  slot.at = ctx.period;
  slot.line = line.id;
  slot.buyback = asQty(plan.buyback, 'the shares it bids for');
  slot.bookPerShare = plan.bookPerShare;
  ctx.record(
    'equity.plan',
    [firm, line.id],
    {
      line: line.id,
      bookPerShare: plan.bookPerShare,
      buyback: plan.buyback,
      issue: plan.issue,
      reservation: plan.reservation,
      dividendPerShare: plan.dividendPerShare,
    },
    false,
  );
  if (plan.dividendPerShare > 0) declareDividend(ctx, seed, row, line, plan);
}

/**
 * D3, D3.b, Reporting A3, item 10: THE BOARD DECLARES, WITH ITS RESULTS, AND THE MONEY MOVES LATER.
 *
 * This function used to be `payDividend` and it paid, in the same pass, to whoever happened to hold
 * the line at that instant — every period, because `decide` runs every period. **Period 5 settled
 * 249,288 instructions and 162,615 of them were dividend payouts: 65% of everything this world did**
 * (C-2). Not because dividends matter that much, but because THERE WAS NO DECLARATION TO BE
 * SEPARATE FROM THE PAYMENT.
 *
 * A board declares on its own fiscal calendar — the same quarters it reports on, which is why the
 * anchor is in the kernel and drawn once per company. Between the declaration and the payment the
 * dividend is a LIABILITY to named holders, and the share trades EX from the ex date, which is why
 * total return and price return are different numbers.
 */
function declareDividend(
  ctx: MechanismContext,
  seed: string,
  row: EquityDecl,
  line: Instrument,
  plan: EquityPlan,
): void {
  const firm = row.firm as PartyId;
  /**
   * A3: WITH ITS RESULTS, on the company's OWN fiscal quarters — the same quarters it reports on,
   * which is why the anchor moved to the kernel calendar and is drawn once per company (Law 4).
   *
   * It is the CALENDAR and not the report: a company that does not publish still has a year end and
   * still pays its owners, and tying the declaration to a published statement would have stopped
   * every dividend in a world where nothing is public yet.
   */
  const today = ctx.calendar.endOf(ctx.period);
  const quarter = quarterClosedBy(anchorOf(seed, firm), today);
  // Seed A2: not for a quarter that began before this world had books in it at all.
  if (compareCivil(quarter.begins, ctx.calendar.epoch) < 0) return;
  // One declaration per quarter: `decide` runs every period, and a board declares once for one set
  // of results. This is what stopped a dividend being declared and paid fifty-two times a year.
  const b = book(ctx);
  if (b.declared[String(firm)] === quarter.label) return;
  b.declared[String(firm)] = quarter.label;
  /**
   * D3.b: the ex date is the next period — on and after it a BUYER does not get this dividend, and
   * the record date is the same period because who holds it when the market opens ex is who is
   * owed. The payable date is later by the one convention this module declares.
   */
  const ex = periodOf(ctx.period + 1);
  ctx.announce({
    issuer: firm,
    line: line.id,
    kind: 'dividend',
    ex,
    record: ex,
    payable: periodOf(Number(ex) + ctx.params.periods(EQUITY_PAYOUT_LAG)),
    perUnit: plan.dividendPerShare,
    ccy: line.ccy,
    why: `declared with the results for ${quarter.label}`,
  });
}

/**
 * D3, Register E1, E1.a: THE RECORD DATE. Whoever the register says holds it now is who is owed, and
 * from here it is not a date any more — it is a set of named parties and an amount each (XI-8).
 *
 * That is what makes a declared dividend a LIABILITY rather than an intention: item 8's noun, used
 * for what it was built for. A holder that dies between here and the payment does not lose it — its
 * estate has a claim, which is exactly what "nothing ranks in an estate" used to mean.
 */
function recordDividends(ctx: MechanismContext): void {
  for (const action of ctx.recordingOn(ctx.period)) {
    const line = ctx.instruments.get(action.line);
    const firm = action.issuer;
    for (const holderId of ctx.register.holdersOf(line.id)) {
      if (holderId === firm) continue;
      const holder = ctx.parties.get(holderId);
      // 0f.1: the register holds the TOTAL; a dividend is declared per member of a cell and
      // the total is that over the members (Law 8: whole pieces for each real holder).
      if (ctx.register.quantity(holderId, line.id) <= 0) continue;
      const perMemberUnits = ctx.register.perMember(holderId, line.id);
      const share = gridPerMember(
        ctx.registry,
        holder,
        dividendFor(action.perUnit, asAmount<'piece'>(perMemberUnits, 'what one member holds')));
      if (share.total <= 0) continue;
      ctx.owes({
        debtor: firm,
        creditor: holderId,
        ccy: line.ccy,
        owed: share.total,
        terms: dividendOwed(action.id, line.id),
        why: `declared on ${String(line.id)} and payable in ${action.payable}`,
      });
    }
    ctx.recordAction(action.id);
  }
}

/**
 * D3.a: THE PAYABLE DATE. What was declared leaves the issuer, to the parties the record date named
 * — and a payment that does not arrive leaves the claim standing, because a payer that cannot pay
 * has not paid (Money E1) and the agreement is where that fact already lives.
 */
function payDividends(ctx: MechanismContext): void {
  for (const action of ctx.payableOn(ctx.period)) {
    const line = ctx.instruments.get(action.line);
    payDividend(ctx, action.issuer, line, action);
    ctx.payAction(action.id);
  }
}

function payDividend(
  ctx: MechanismContext,
  firm: PartyId,
  line: Instrument,
  action: CorporateAction,
): void {
  const paid: number[] = [];
  let failed = 0;
  // Register F2: the ISSUER as it is now. A company that ceased between the declaration and the
  // payable date has an estate, its rows moved there, and matching on the name it had would skip
  // every one of them — so what it declared would simply never be paid.
  const issuer = ctx.parties.resolve(firm).id;
  for (const owed of ctx.agreements.ofKind(DIVIDEND_DECLARED)) {
    if (ctx.parties.resolve(owed.debtor).id !== issuer || owed.state !== 'performing') continue;
    // Law 15: which declaration this claim came from is a FIELD of its terms, asked for as this
    // kind's terms. It used to be `owed.what !== \`dividend ${action.id}\`` — a string built at one
    // end and compared at the other, which is A-52's shape and is why the kind exists.
    if (!isDividendOwed(owed.terms) || owed.terms.action !== action.id) continue;
    /**
     * Register F2, Money E4: EVERY REFERENCE RESOLVES TO SOMEBODY WHO EXISTS, on both ends.
     *
     * A declaration names its holders on the record date and is paid on the payable date, and a
     * company or a holder can cease in between — this world had `firm.2` cease in period 47 with a
     * declaration of its own still to pay and a claim on another firm still to collect. The kernel
     * moves an agreement row at the cease (`world/succession.ts`), so a row that existed then is
     * already on the estate; one written after it, from a record date the register has since
     * resolved away, is not. Both ends are resolved here, which is what `issueCloseOutClaim` does in
     * the derivative layer for the same reason and in the same words.
     *
     * A payment to yourself is not a payment: when the two resolve to one party the claim is not
     * paid and STAYS, which is the estate's to divide (XI-8).
     */
    const holderId = ctx.parties.resolve(owed.creditor).id;
    const payer = ctx.parties.resolve(owed.debtor).id;
    if (holderId === payer) continue;
    const holder = ctx.parties.get(holderId);
    /**
     * Law 19: WHAT IT IS OWED, read from the claim the record date wrote — not recomputed from what
     * it holds today. A shareholder that sold the day after the record date is still owed this
     * dividend and the buyer is not, which is the whole point of there being a record date, and
     * re-deriving the amount from today's register would pay exactly the wrong people.
     *
     * Law 8, XI-15: WHAT IT CAN BE PAID IN is another question. A cell is a count of people and
     * every one of them has an account, so the claim is paid in whole pieces to each — and between
     * the record date and now the cell may have split, aged or lost members, so what was a whole
     * number of pieces per member then need not be one now. What will not divide stays OWED: the
     * agreement carries it and it is paid when the cell can take it, rather than being rounded
     * away into a residual with no holder (Appendix B).
     */
    const share = gridPerMember(
      ctx.registry,
      holder,
      owed.owed / weightOf(holder));
    const total = Number(share.total);
    if (total <= 0) continue;
    const leg: Leg = {
      kind: 'money',
      from: ctx.accountOf(payer, line.ccy),
      to: ctx.accountOf(holderId, line.ccy),
      ccy: line.ccy,
      // Treasury C1: the payer says what this is. A dividend is not a wage and not a disposal.
      receipt: { of: 'dividend' },
      amount: asQty(total),
    };
    const r = ctx.settle({
      legs: [leg],
      cause: 'corporateAction',
      reason: `payout on ${line.id} to ${holderId}`,
    });
    // Money E1: a payer that cannot pay has not paid, and the claim STAYS — which is the state that
    // used to be a number in an event and is now something the holder's estate can divide (XI-8).
    if (r.outcome === 'settled') {
      paid.push(total);
      ctx.paidOn(owed.id, total);
    } else failed += 1;
  }
  ctx.record(
    'payout.declared',
    [firm, line.id],
    {
      line: line.id,
      action: action.id,
      perShare: action.perUnit,
      shares: line.issued,
      paid: sum(paid).value,
      failedPayments: failed,
    },
    true,
  );
}

/**
 * E4, Register F2: the firm that promised this line is gone and an estate owes it now. Said once,
 * publicly, because everybody holding it needs to know that what they hold is a claim on a
 * liquidation — and then this module stops pricing the line: nobody posts an opinion of a firm that
 * no longer exists, the session finds no orders, and the print goes visibly stale (Clearing E4)
 * until the estate pays what it can and writes off the rest at zero.
 */
function announceSuccession(ctx: MechanismContext, row: EquityDecl, line: Instrument): void {
  const b = book(ctx);
  if (b.succeeded.includes(row.firm)) return;
  b.succeeded.push(row.firm);
  ctx.record(
    'equity.succeeded',
    [row.firm, line.id, issuerOf(line)],
    {
      line: line.id,
      promisedBy: row.firm,
      owedBy: issuerOf(line),
      shares: line.issued,
      why: 'the firm has ceased: its shares rank last on its estate and are worth what is left, which may be nothing (Equity E4, F2)',
    },
    true,
  );
}

/**
 * D3.a, Fund Shares B3: what was DECLARED is what left the issuer, or a payment failed and is on
 * the record as one.
 *
 * A payout that is declared and does not arrive is an issuer that could not pay it, which is a real
 * state (Money E1) and the beginning of a great deal else. One that is declared and quietly never
 * leaves anybody's account is money credited to holders out of nothing, and it would look exactly
 * like an issuer that paid.
 *
 * It checks every declared payout and not only a firm's. What an issuer said it would pay per unit
 * is one public fact with one shape whoever said it — a firm declaring a dividend (D3) or a fund
 * passing on what it received (B3) — so it is one check, over the events and the ledger, both of
 * which are the kernel's and neither of which belongs to either module.
 */
function declaredIsPaid(): Family {
  return {
    name: 'flows',
    contributor: 'equity',
    spec: 'Equity D3 Equity D3.a Fund Shares B3 Register E5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const paidOut = dividendLegs(view);
      for (const e of view.journal.ofKind('payout.declared')) {
        if (e.period !== view.period) continue;
        const line = e.data['line'];
        const paid = e.data['paid'];
        const firm = e.subjects[0];
        if (typeof line !== 'string' || typeof paid !== 'number' || firm === undefined) continue;
        // Item 16: a published number re-enters the type system here, through its own door.
        const said = asCash(paid, 'what the declaration said was paid');
        const moved = sum(paidOut.get(`${line}\u0000${firm}`) ?? []);
        if (withinDust(said, moved.value, combineDust(sum([said]), moved))) continue;
        out.push({
          family: 'flows',
          spec: 'Equity D3.a',
          owner: firm,
          size: minus(said, moved.value, 'recorded paid against what left the firm'),
          unit: view.registry.currencyOf(view.parties.get(firm as PartyId).region),
          period: view.period,
          message: `${line}: ${paid} of payout was recorded paid and ${moved.value} left ${firm}`,
        });
      }
      return out;
    },
  };
}

/**
 * What actually left each issuer this period on a payout instruction (Law 19: read the legs).
 *
 * Law 18: ONE WALK OF THE PERIOD, not one per declared payout. A dividend is a payment to every
 * holder by name, so the period a world of four countries pays them in carries a quarter of a
 * million instructions and two thirds of them are payouts — and this was walking all of them again
 * for every line that declared one. The sums are the same sums over the same legs in the same
 * order; what changes is that the ledger is read once.
 */
function dividendLegs(view: AuditView): ReadonlyMap<string, Cash[]> {
  const out = new Map<string, Cash[]>();
  for (const r of view.ledger.inPeriod(view.period)) {
    if (r.outcome !== 'settled') continue;
    const reason = r.instruction.reason;
    if (!reason.startsWith('payout on ')) continue;
    // The reason names the line it is a payout ON, which is what the declaration named.
    const line = reason.slice('payout on '.length, reason.indexOf(' ', 'payout on '.length));
    if (line === '') continue;
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg)) continue;
      const key = `${line}\u0000${String(leg.from.holder)}`;
      const held = out.get(key);
      const paidOnIt = heldAsMoney(leg.amount, 'what left the firm on this line');
      if (held === undefined) out.set(key, [paidOnIt]);
      else held.push(paidOnIt);
    }
  }
  return out;
}

/**
 * C1.b, F3, B4: the reads. Published every period so the observer has them and so the numbers a
 * later item needs — a takeover's majority (A5.a, worklist 13g), an index's float weight (G1,
 * worklist 12) — are one derivation with one writer. G3: every one is computed FROM the cleared
 * price and none of them is ever used to set it.
 */
function publishReads(ctx: MechanismContext, rows: readonly EquityDecl[]): void {
  for (const row of rows) {
    const line = ctx.instruments.get(equityLineOf(row.firm));
    if (!line.status.live) continue;
    const terms = shareTerms(line);
    const voteTerms: number[] = [];
    // C1.b: what is NOT tradeable is what is bound — a block somebody has undertaken not to sell
    // is units the register says are encumbered, and only free units can move (Register D5.a). No
    // holder in this world has bound any yet: the party that does is a founder, and a founder is
    // a party that funded a firm's entry (Firm Birth A, worklist 13g).
    const boundTerms: number[] = [];
    for (const holder of ctx.register.holdersOf(line.id)) {
      const party = ctx.parties.get(holder);
      voteTerms.push(votesOf(party, ctx.register.quantity(holder, line.id), terms));
      // 0f.1: the register holds the cell's TOTAL.
      boundTerms.push(ctx.register.encumbered(holder, line.id));
    }
    const strategic = sum(boundTerms).value;
    const print = ctx.prices.latest(line.id, ctx.period);
    ctx.record(
      'equity.reads',
      [row.firm, line.id],
      {
        line: line.id,
        shares: line.issued,
        // C1.b: what is genuinely tradeable — the count less what is bound and cannot move.
        freeFloat: freeFloat(line.issued, asAmount<'piece'>(strategic, 'what is bound and cannot move')),
        strategic,
        // B4, B4.a: a read. Nothing compares it against its own two inputs and calls that a check.
        marketCapitalisation: print.some
          ? marketCapitalisation(line.issued, print.value.price)
          : null,
        // F3, A5.a: how many votes there are, and how many holders there are to cast them. Who
        // holds a majority is the register's to answer when somebody asks it (worklist 13g).
        votes: sum(voteTerms).value,
        holders: ctx.register.holdersOf(line.id).length,
      },
      true,
    );
  }
}

/**
 * The module. `rows` is which firms this world listed (Law 15: the data says). A firm that is not
 * in it has no shares at all, which is a real state and not an omission.
 */
/**
 * Equity D3, XI-8: WHAT A BOARD DECLARED AND HAS NOT YET PAID.
 *
 * A declaration is a commitment: the record date fixes who is owed and the payable date is when it
 * is due, and between them the holder has a claim whoever it sells the share to (D3). It carries
 * the ACTION it came from, because a second declaration on the same line is a second claim and the
 * two are told apart by which action wrote them — it used to be told apart by comparing a free-text
 * `what` against the string `dividend ${action.id}`, which is a fact recovered from a string (A-52)
 * and would have matched nothing the moment either side spelled it differently.
 */
export const DIVIDEND_DECLARED = agreementKindId('equity.dividendDeclared');

export interface DividendOwed extends AgreementTerms {
  readonly kind: typeof DIVIDEND_DECLARED;
  readonly action: CorporateActionId;
  readonly line: InstrumentId;
}

/**
 * Law 15: the module that declared the kind narrows a row back to it, and the test is STRUCTURAL
 * and not a comparison of kind ids — what makes these terms a declared dividend is that they name
 * the action that declared it and the line it was declared on.
 */
export const isDividendOwed = (t: AgreementTerms): t is DividendOwed =>
  'action' in t && 'line' in t;

/** Law 4: one writer of the terms of a declared dividend. */
export const dividendOwed = (action: CorporateActionId, line: InstrumentId): DividendOwed => ({
  kind: DIVIDEND_DECLARED,
  action,
  line,
});

export function equity(rows: readonly EquityDecl[], seed: string): SystemModule {
  return {
    id: 'equity',
    agreementKinds: [
      {
        id: DIVIDEND_DECLARED,
        what: 'a dividend a board declared and has not yet paid',
        // Equity E4, XI-8: declared is owed, and whoever succeeds the payer owes it.
        binds: 'whoeverSucceeds',
      },
    ],
    nouns: [
      {
        name: EQUITY_PLAN,
        kind: 'working',
        holds:
          'what each firm decided about its own shares this period — the line, the shares it bids to cancel, and its book per share',
        why:
          'it is how this module gets from its decide phase to its own `markets` and `orders`, and nothing outside it has an opinion about a bid nobody has posted yet (0e\u2032.4). It was a PRIVATE `equity.plan` event read back by its own writer in the same period; the event stays as the record of the decision, and the door that says a size is a COUNT now sits at the WRITE.',
      },
      {
        name: 'equity',
        kind: 'working',
        holds:
          'the lines whose issuer has been succeeded, so the wipe is announced once and not every week',
        why:
          'an idempotence marker within the module’s own corporate-actions phase. It is not a fact about the world; it is how this module avoids saying the same thing twice (E4).',
      },
    ],
    spec: 'Equity',
    // The firm whose residual claim it is decides about it (Firm E4, E5) and publishes what it has
    // spare; the estate is what a share ranks last on; and the firms it lists are parties somebody
    // else's seed created, so that seed runs first. Its buyers decide in their own modules and need
    // nothing from this one but the line, the market and what the issuer declared.
    requires: ['firms', 'estate', 'households', 'seed.foundation'],
    instrumentKinds: [shareKind],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: paramsOf(rows),
    phases: [
      {
        name: 'equity.decide',
        spec: 'Equity D1 Equity D2 Equity D3 Firm E4 Firm E5',
        // After the firm has planned its period, because what it has spare is its own funding read;
        // before the session, because everything it decides it decides on what has already
        // happened (Clearing F1).
        anchor: { after: 'firms.decide' },
        reads: [],
        writes: [
          { kind: 'event', name: 'equity.plan' },
          { kind: 'event', name: 'equity.succeeded' },
          { kind: 'event', name: 'payout.declared' },
        ],
        run: (ctx: MechanismContext) => {
          /**
           * D3, item 10: THE THREE DATES, in order, in one phase — because they are one story and
           * splitting them across phases would let a record date and its own payment land in
           * different orders in different periods.
           *
           * Paying first is deliberate: what is payable today was recorded in an earlier period, so
           * a line that records and pays in the same period pays the claim the earlier record made
           * and not the one written a moment ago (Law 19, and Clearing A4's "never read your own
           * answer"). The declaration comes last for the same reason.
           */
          payDividends(ctx);
          recordDividends(ctx);
          for (const row of rows) decide(ctx, seed, row);
        },
      },
      {
        name: 'equity.float',
        spec: 'Equity D1 Equity D1.b Equity D1.c Equity E3 Firm E4 Reporting A2 Private Equity D1 Private Equity D2',
        // Clearing F1: after everything it reads has been published this period — what it is short
        // of (`firms.decide`) and what its bank quoted it (`lending.write`) — and before the
        // session, because an offer that arrives after the book has cleared is not an offer. It is
        // the same slot an issuer of paper stands in (`bond.issue`), for the same reason.
        anchor: { before: 'markets' },
        reads: [{ kind: 'event', name: 'firms.funding', of: 'thisPeriod' }],
        writes: [],
        run: (ctx: MechanismContext) => {
          floatations(ctx, rows);
        },
      },
      {
        name: 'equity.reads',
        spec: 'Equity B4 Equity C1.b Equity F3 Equity G3',
        // After the session has printed and the marks are in the books: a read of a price is taken
        // once the price exists, never before (Clearing F1.a).
        anchor: { after: 'revaluation' },
        reads: [{ kind: 'print', of: 'thisPeriod' }],
        writes: [{ kind: 'event', name: 'equity.reads' }],
        run: (ctx: MechanismContext) => {
          publishReads(ctx, rows);
        },
      },
    ],
    participants: [
      {
        // D2: the ONE reason a firm is in its own book. Every other reason to be there belongs to
        // the party that has it, in that party's own module (Households D5): a module that wrote
        // other people's schedules would be handing the market its answer (Clearing A3, XI-13).
        partyKind: FIRM,
        // Law 18: THE ONE BOOK IT COULD BE IN, which is its own. Without this every firm was asked
        // about every market in the world — three thousand firms against two hundred and sixty
        // books is three quarters of a million questions a cycle, and it was twenty-two of the
        // twenty-five seconds a period cost. It is a TRAVERSAL and nothing else: the answer below
        // already returns nothing for any other book, so the same firms post the same orders.
        markets: (view: ParticipantView): readonly MarketId[] => {
          const decided = view.working(EQUITY_PLAN, nothingDecided);
          // D2, §29 C5: the ONE order it has is a bid for its own shares, so a period in which it
          // is not buying any is a period in which it is in no book at all — and a private company
          // is never buying any, because there is no book to buy them in.
          if (decided.at !== view.period || decided.buyback <= 0) return [];
          return [equityMarketOf(view.self.id)];
        },
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const decided = view.working(EQUITY_PLAN, nothingDecided);
          if (decided.at !== view.period || decided.line !== m.instrument) return [];
          return buybackOrder(view.self.id, decided.buyback, decided.bookPerShare);
        },
      },
    ],
    families: [declaredIsPaid()],
    seed(ctx: SeedContext): void {
      /**
       * Seed A3, Law 19, Law 2, XI-15: EVERY FIRM'S LINE, AND WHO OPENS HOLDING IT.
       *
       * THE SAVERS HOLD IT. A share is a claim on the residual (A1), and the parties in this
       * world with a reason to hold a claim on a firm's earnings are its households (B3). A
       * DEALER'S INVENTORY IS NOT A REASON — it is a working position a desk takes by trading,
       * so a seed that opens the desks holding the whole float of every line they make has
       * stated an outcome (Seed E1) and given ownership to the one party whose holding of it is
       * supposed to be the RESULT of a market.
       *
       * Item 9 opened them holding it, for a reason that was true then: "a bank that opens making
       * a market with nothing to sell can only ever bid", and the one alternative tried — a
       * FOUNDER, a named party that receives dividends and has nothing to spend them on — drained
       * the sector's money into a hole (the item 9 record). A saver is not that hole: it consumes,
       * it banks, and from item 10 it has a portfolio decision of its own, so it is on the other
       * side of the desk's bid rather than absorbing the circuit.
       *
       * What that cost, measured at 11.5: a desk opened carrying six hundred million more than
       * its own limit on the first morning, every desk in every seed (11.5's finding 12-14); and
       * because a SHARE RAISES NOTHING AT THE CENTRAL BANK'S WINDOW (Banks Funding C1, C1.a),
       * the float sat on the one balance sheet whose whole liquidity turns on what its assets
       * raise there. What cannot run (capital and insured deposits) has to fund what cannot be
       * turned into cash at par, and the float was four times the capital: the two banks in a
       * six-bank world that made a market opened at a liquidity metric of 0.84 and 0.89 while the
       * four that made none opened at 1.04 to 1.07.
       */
      const cells = ctx.parties.ofKind(HOUSEHOLD);
      const members = cells.reduce((t, c) => t + weightOf(c), 0);
      forbid(
        rows.length === 0 || members > 0,
        'Equity B3',
        'this world has firms in it and no saver, so no residual in it has a holder with a reason to hold one',
        { firms: rows.length },
      );
      /**
       * Seed B1.a: WHICH SAVERS, and it is drawn per line rather than taken in order. The cells a
       * line reaches are the first of them (below), so a fixed order would make one cell the owner
       * of every small company in the world — a concentration nothing in this seed states and
       * nobody chose. The list is rotated instead, which spreads WHOSE the small firms are without
       * saying anything about which saver prefers which firm (Seed E1).
       */
      const rng = prng(seed, 'equity.float');
      for (const row of rows) {
        const firm = ctx.parties.get(row.firm as PartyId);
        const ccy = ctx.registry.currencyOf(firm.region);
        // A5: one vote per share. It is a TERM of the instrument and not a parameter: what a share
        // of this line carries is its structure, fixed at issue (Seed C4.b), and a line with two
        // classes of vote is two lines (Register F1.a).
        const terms: ShareTerms = { kind: SHARE, issuer: row.firm as PartyId, votesPerShare: 1 };
        const id = equityLineOf(row.firm);
        /**
         * Equity E3, §29 C5, C5.a: A LINE FOR EVERY FIRM, AND A MARKET FOR THE ONES THAT ARE
         * PUBLIC. Every firm has a residual and somebody owns it; what a listing adds is a place
         * to sell it. A private line has no market, so it never prints, so its holders carry it at
         * what it cost them and say so — "marked, not cleared" — which is C5.a kept by there being
         * no price to mistake for one rather than by a rule against mistaking it.
         */
        const market = row.listed ? some(equityMarketOf(row.firm)) : none<MarketId>();
        ctx.instruments.add({ id, kind: SHARE, issuer: some(row.firm as PartyId), ccy, terms, market });
        // Seed C4: the line and the level it opens at.
        const price = ctx.params.price(OPENING_SHARE);
        if (market.some) {
          ctx.openMarket({
            id: market.value,
            name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
            instrument: id,
            ccy,
            rationing: 'proRata',
          });
          ctx.prices.write({
            instrument: id,
            market: market.value,
            period: ctx.period,
            price,
            ccy,
            provenance: { kind: 'opening' },
          });
        }
        /**
         * WHAT THE LINE COMES TO IS READ OFF THE FIRM (Law 19) and never stated. A share is a claim
         * on the residual (A1), and at period zero a firm's residual is everything it holds:
         * nobody has lent it anything and it owes nobody. So the line is its own opening book, in
         * shares of whatever a share opens at — which is why a larger firm has a larger line
         * without one number being written down beside its name, and why doubling the opening share
         * price halves every count here and moves nothing (D4).
         */
        let book = asCash(0, 'nothing walked yet');
        for (const h of ctx.register.holdingsOf(firm.id)) {
          // Currency C4.a, A-51: a firm's residual is one number in the money it keeps its books
          // in. A firm opening with imported stock holds it in the seller's money and this added
          // the two, so its line came out in a total of two currencies.
          book = plus(
            book,
            ctx.valuation.inOwnMoney(
              firm.id,
              ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period),
              ctx.instruments.get(h.instrument).ccy,
              ctx.period,
            ),
            'its book',
          );
        }
        const shares = ctx.registry.pieces(
          ctx.instruments.get(id).unit,
          asNamed(Math.round(amountOf(book, price, 'the shares its book comes to')), 'what is issued'),
        );
        /**
         * XI-15, Law 6, Law 8: AND HOW FAR IT REACHES. A cell is homogeneous and holds WHOLE pieces
         * per member, so a line can only be held by as many people as it has pieces to give one
         * each — and what stood here divided every line across every saver in the world and then
         * dropped it entirely when the division came out below one piece: `if (perMember > 0)`, a
         * bound (Law 6) behind which every firm whose book was under thirty million dollars opened
         * with a line NOBODY HELD. A residual with no holder is a defect Appendix B names outright,
         * and this one was silent — the line existed, the market opened, and nothing was issued
         * into it.
         *
         * So the holders are AS MANY CELLS AS THE COUNT CAN FILL: a line big enough reaches every
         * saver in the world, a smaller one reaches one cell of them, and the bound is gone because
         * there is nothing left for it to drop. It is also the true statement — FEWER PEOPLE OWN A
         * SMALLER COMPANY — and it is the shape C1.b's free float and C2.e's founders take on once
         * a firm is born rather than seeded (10f.2).
         *
         * EVERY MEMBER THE LINE REACHES HOLDS THE SAME SLICE OF IT, because the seed has nothing to
         * say about which saver prefers which firm: that is what the portfolio decision is for, and
         * a seed that answered it would be seeding the outcome (Seed E1). It is the same statement
         * the foundation already makes about government paper, made about equity.
         */
        const from = rng.int(cells.length);
        const holders: PartyId[] = [];
        let reached = 0;
        for (let n = 0; n < cells.length; n += 1) {
          const cell = cells[(from + n) % cells.length];
          if (cell === undefined) continue;
          const w = weightOf(cell);
          if (reached + w > shares) break;
          reached += w;
          holders.push(cell.id);
        }
        if (holders.length === 0) continue;
        // Law 8: whole pieces per member, and the line is what the members hold. What the division
        // leaves below one piece a member is never issued, so every piece of this line has a named
        // holder from the instant it exists.
        const perMember = downTick(div(shares, reached, "one member's share"));
        for (const holder of holders) ctx.endowUnits(holder, id, perMember, price);
      }
    },
  };
}

/** C1.b: how much of a line is bound and therefore not float — a read of the register (D5.a). */
export function strategicOf(ctx: MechanismContext, firm: string): Qty {
  const line = equityLineOf(firm);
  const terms: Qty[] = [];
  for (const holder of ctx.register.holdersOf(line)) {
    terms.push(
      ctx.register.encumbered(holder, line),
    );
  }
  return sum(terms).value;
}

/** B4: shares times price, read once here so nobody derives it a second way (Law 4). */
export function capitalisationOf(ctx: MechanismContext, firm: string): Cash | null {
  const line = ctx.instruments.get(equityLineOf(firm));
  const print = ctx.prices.latest(line.id, ctx.period);
  return print.some ? marketCapitalisation(line.issued, print.value.price) : null;
}

/** D2.a, D1.a: the count, which is what a dilution raises and a cancellation lowers. */
export function sharesOf(ctx: MechanismContext, firm: string): Qty {
  return ctx.instruments.get(equityLineOf(firm)).issued;
}

/** Law 7: a per-share number and the count it came from, for readers that need both. */
export function perShare(total: Cash, shares: Qty, what: string): PerPiece {
  return pricedAt(total, shares, what);
}

export type { InstrumentId };
