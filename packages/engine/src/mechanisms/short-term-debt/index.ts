/**
 * Short-term debt: money borrowed for weeks, and the asking-again that is the whole risk of it.
 *
 * @spec Short-Term Debt A1 Short-Term Debt A2 Short-Term Debt A2.a Short-Term Debt A3 Short-Term Debt B1 Short-Term Debt B2 Short-Term Debt B3 Short-Term Debt B3.a Short-Term Debt B3.b Short-Term Debt B5 Short-Term Debt C1 Short-Term Debt C2 Short-Term Debt C3 Short-Term Debt D1 Short-Term Debt E1 Short-Term Debt E2 Short-Term Debt E3 Clearing C4 Clearing C4.a Law 3 Law 4 Law 8 Law 19
 *
 * EVERY FAILURE IN THIS WORLD IS A SOLVENCY FAILURE, AND THAT IS WHAT THIS ITEM IS FOR. A firm here
 * fails because what it owes exceeds what it holds — never because it could not find the money on a
 * Tuesday. B3.b is the other kind, and it is the kind that actually happens: buyers decline, the
 * issuer must repay maturing paper out of cash it does not have, and it must find the money
 * somewhere. Until this module there was nowhere in the model where that could occur.
 *
 * THE ROLL IS NOT A SEPARATE MECHANISM, AND THAT IS THE POINT OF B3.a. A rollover is a NEW ISSUE
 * INTO A MARKET THAT MUST CLEAR: the issuer is asking the market to lend again, and it may not. So
 * there is one phase that brings paper, and what maturing paper does is enlarge the need it brings
 * paper against — a roll is the ordinary case of the ordinary mechanism, not a renewal with its own
 * rules. E1 is the FORBID that names the alternative: paper that always rolls at a written rate is
 * not debt, it is a permanent liability with a coupon, and it removes the only risk the instrument
 * has.
 *
 * IT ISSUES THROUGH THE PATH THAT EXISTS (item 10b.0). The issuer brings a size and a walk-away to
 * a KERNEL market and the one solver strikes it — the same doors the treasury and a corporate
 * issuer use. This module owns the REASON to issue, the shape of the paper and what happens when
 * the book says no; it does not own a book, and a module that cleared its own would be the third
 * issuance mechanism this world is in the middle of deleting (item 10d).
 */
import { boardPosted, bufferOf, creditQuoteThisPeriod } from '../../registry/banking.js';
import {
  amountOf,
  asCash,
  atMostCash,
  noCash,
  sumCash,
  asPerNamedUnit,
  asPerPiece,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  type PerPiece,
  over,
  plus,
  type Ratio,
  scale,
  valueAt,
} from '../../core/measure.js';
import { addDays, addMonths } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import {
  agreementKindId,
  currencyCode,
  instrumentId,
  partyId,
  marketId,
  moneyInstrumentId,
  paramId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type PartyKindId,
} from '../../core/ids.js';
import { Missing } from '../../core/errors.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty, downTick, upTick } from '../../core/tick.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { displayName } from '../../registry/naming.js';
import { BANK, FIRM } from '../../registry/profiles.js';
import type { EventKind } from '../../journal/journal.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import {
  COMMERCIAL_PAPER,
  PAPER_PAR,
  commercialPaper,
  isPaper,
  paperId,
  type PaperTerms,
} from './paper.js';
import { fundingPublishedBy } from '../../registry/funding.js';
import { LOAN, type LoanTerms } from '../../registry/credit.js';

export const PAPER_PARAMS = {
  /** A1.b: how long paper runs for. Weeks to months — a convention of the market, stated with it. */
  tenor: paramId('shortTermDebt.tenor'),
  /** C3: the most of one issuer's name a cash investor is willing to be exposed to. A taste. */
  concentration: paramId('shortTermDebt.concentration'),
  /** C3, §46 B1: how long a cash investor remembers having seen a name fail. Memory is a taste. */
  memory: paramId('shortTermDebt.memory'),
  /** B4: what undrawn headroom costs its holder per annum. A term of the line it agreed. */
  commitmentFee: paramId('shortTermDebt.commitmentFee'),
  /** B4: how big the line an issuer opens with is, against its own book. A PLACEHOLDER (17.2). */
  line: paramId('shortTermDebt.line'),
  /** B4 (12a.7): how long a drawing on the line runs for. A convention of the facility. */
  lineMonths: paramId('shortTermDebt.lineMonths'),
};

/** A2.a: the convention this world's short paper is quoted on, stated because it is material. */
const PAPER_DAY_COUNT = 'ACT/360' as const;

/** A1.c, N13.a: senior unsecured — the same rank a firm's senior bond carries, not a near one. */
const SENIOR = 1;

/* --------------------------------------------------------------------------------------------
 * WHY AN ISSUER ISSUES
 * ------------------------------------------------------------------------------------------ */

/**
 * B1, B2, B3.a, E1: AN ISSUER WITH A DATED NEED BRINGS PAPER, AND SO DOES ONE WITH PAPER TO REPAY.
 *
 * B1 is a short, KNOWN need — a payroll, a delivery it has committed to — and `firms.funding`
 * publishes exactly that as `shortNow`: what the issuer cannot pay of what falls due soon, after
 * its own cash has gone to the near need first. Nothing here recomputes it (Law 19).
 *
 * B3, B3.a: AND WHAT MATURES IS PART OF THE NEED. Paper falling due this period is money that must
 * leave, so it enlarges what the issuer is short of and it comes to the same book as everything
 * else. That is the whole of the roll and it is deliberately not a mechanism of its own: E1 forbids
 * the alternative, and an issuer whose paper renewed itself would never have to ask.
 */
export function issuePaper(ctx: MechanismContext): void {
  for (const { party: issuer, need } of issuers(ctx)) {
    const { ccy } = need;
    // B3.a: what it must repay, and what it is short of besides. Both are money it needs on a date
    // inside the tenor, so there is one number and one trip to the market.
    const rolling = maturingIn(ctx, issuer, ctx.period, ccy);
    const want = plus(need.shortNow, rolling, 'what it needs, including what it must repay');
    if (want.pieces <= 0) continue;
    place(ctx, issuer, ccy, want, rolling);
  }
}

/**
 * A3: THE STATE, A BANK, A FIRM — THE SAME INSTRUMENT, AND THE TYPE IS THE CREDIT.
 *
 * What each type is SHORT OF is its own fact, published in its own words by whoever owns it, so
 * this is a TABLE and not a branch (Law 15): a firm says what it cannot pay of what falls due soon,
 * a bank says what it keeps back against a bad week. Adding the state is a row here — its short
 * paper is the `sovereign.bill` it already issues against its own programme, which is the same
 * contract on a different credit, and is why A3 is one clause and not three.
 */
const NEEDS: readonly {
  readonly kind: PartyKindId;
  readonly need: (ctx: MechanismContext, party: PartyId) => Option<Need>;
}[] = [
  { kind: FIRM, need: firmNeed },
  { kind: BANK, need: bankNeed },
];

interface Need {
  readonly shortNow: Cash;
  readonly ccy: CurrencyCode;
}

function issuers(ctx: MechanismContext): readonly { party: PartyId; need: Need }[] {
  const out: { party: PartyId; need: Need }[] = [];
  for (const row of NEEDS) {
    for (const p of ctx.parties.ofKind(row.kind)) {
      if (!p.status.alive) continue;
      const need = row.need(ctx, p.id);
      if (need.some) out.push({ party: p.id, need: need.value });
    }
  }
  return out;
}

/** B1: what this firm published it cannot pay of what falls due soon (Law 19: it said it). */
function firmNeed(ctx: MechanismContext, issuer: PartyId): Option<Need> {
  const said = fundingPublishedBy(ctx.journal, String(issuer), ctx.period);
  if (!said.some) return none();
  return some({
    shortNow: asCash(
      said.value.shortNow,
      said.value.ccy,
      'what it cannot pay of what falls due soon',
    ),
    ccy: said.value.ccy,
  });
}

/**
 * B1, C1, Banks Funding C2.a: WHAT A BANK IS SHORT OF — what it wanted to be holding and is not.
 *
 * Its BUFFER is its own published want (`bank.buffer`: what it keeps back against a bad week, read
 * off a run of its own reserve flows and nothing else). Its RESERVES are read LIVE off the register
 * rather than out of that same event, and the difference between those two reads is the point:
 * this phase runs after the money market has sat, so the published number is what it wanted BEFORE
 * the session and the register is what it holds AFTER it.
 *
 * That is also why nothing has to stand down for anything (Law 4). The overnight books took what
 * they took first and this reads the residue, not a second claim on one gap: a bank that funded
 * itself overnight is short of nothing here and brings no paper, and one that could not is exactly
 * the issuer B1 describes — a dated need, and somewhere else to ask.
 */
function bankNeed(ctx: MechanismContext, issuer: PartyId): Option<Need> {
  const said = bufferOf(ctx.journal, String(issuer), ctx.period);
  if (!said.some) return none();
  const buffer = said.value.buffer;
  const money = currencyCode(said.value.ccy);
  const held = ctx.register.quantity(
    issuer,
    moneyInstrumentId(ctx.registry.centralBankOf(money), money),
  );
  return some({
    shortNow: minus(
      heldAsMoney(asQty(buffer, 'what it published it keeps back'), money, 'the buffer it wants'),
      heldAsMoney(held, money, 'the reserves it actually holds'),
      'what it is short of its own buffer',
    ),
    ccy: money,
  });
}

/**
 * B3, E3: WHAT THIS ISSUER MUST REPAY THIS PERIOD, read off its own outstanding paper.
 *
 * A read of the register and the terms, never a stored total (Law 19, Appendix B: no stored
 * aggregate). E3's "no maturity that passes without cash moving" is the kernel's — a maturity is a
 * `DueAction` and settlement moves it — and what this answers is the other half: the issuer has to
 * KNOW it is coming, which is what makes a run a thing it can see arriving rather than a surprise.
 */
function maturingIn(
  ctx: MechanismContext,
  issuer: PartyId,
  period: number,
  ccy: CurrencyCode,
): Cash {
  const due: Cash[] = [];
  for (const i of ctx.instruments.issuedBy(issuer)) {
    if (!i.status.live || !isPaper(i.terms) || i.ccy !== ccy) continue;
    if (ctx.calendar.periodOf(i.terms.maturity) !== period) continue;
    // N10: it repays PAR on every unit outstanding, which is a read of the register.
    const face = ctx.register.heldTotal(i.id).value;
    due.push(valueAt(par(ctx, i.ccy), face, i.ccy, 'the par it must repay'));
  }
  return sumCash(ccy, due, 'the par falling due').value;
}

/** N10: par on the piece grid, in this money. One named unit of money for one of face. */
const par = (ctx: MechanismContext, ccy: CurrencyCode): PerPiece =>
  ctx.registry.priceOf(ccy, PAPER_PAR, asPerNamedUnit(1, 'par'));

/**
 * B2, A2, C4, Clearing C4.a: THE PAPER IS BROUGHT — a line, a market, a size and a walk-away.
 *
 * B2 says an issuer uses this because it is CHEAP when the curve is upward-sloping, and that is a
 * comparison it can only make against what borrowing longer costs it. Its walk-away is therefore
 * the price at which this paper costs it what its bank quoted for the same money: above that the
 * short end is the cheaper of the two and it takes it, below it the paper is withdrawn and it
 * borrows instead. Nothing invents a discount (Law 3, E2) — the price is what the book strikes,
 * and what is written here is only the level at which the issuer stops preferring it.
 *
 * AN ISSUER NOBODY HAS QUOTED HAS NO ALTERNATIVE TO COMPARE AGAINST, and it does not guess one: it
 * brings the paper at the least anybody has said they would hold its name for, which is the same
 * read item 10 makes and for the same reason.
 */
function place(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  want: Cash,
  rolling: Cash,
): void {
  const on = ctx.calendar.startOf(ctx.period);
  const maturity = addDays(on, ctx.params.days(PAPER_PARAMS.tenor));
  const alternative = costOfBorrowing(ctx, issuer);
  if (!alternative.some) return;
  const id = paperId(issuer, maturity);
  const standing = ctx.instruments.has(id) ? ctx.instruments.get(id) : undefined;
  if (standing !== undefined && !standing.status.live) return;
  const terms: PaperTerms = {
    kind: COMMERCIAL_PAPER,
    issuer,
    seniority: SENIOR,
    issueDate: on,
    maturity,
    dayCount: PAPER_DAY_COUNT,
  };
  // A2, A2.a: the price at which the discount over this term costs what the alternative costs. It
  // is the yield read backwards into a price ONCE, to say what the issuer would refuse — never to
  // produce a print, which is the book's (E2).
  const term = asRatio(yearFraction(PAPER_DAY_COUNT, on, maturity), 'the term as a year fraction');
  if (term <= 0) return;
  const walkAway = priceCosting(alternative.value, term);
  if (walkAway <= 0) return;
  const units = upTick(amountOf(want, walkAway, 'units offered'));
  if (units <= 0) return;
  if (standing === undefined && !openLine(ctx, issuer, ccy, id, terms)) return;
  ctx.offer({
    market: marketOf(ctx, id),
    issuer,
    size: units,
    reservation: some(walkAway),
    allotment: 'uniformPrice',
  });
  ctx.record(
    'paper.offered',
    [issuer, id],
    {
      issuer,
      line: id,
      size: units,
      reservation: walkAway,
      // B3: how much of this trip is asking for the same money again, so a reader can see a roll.
      rolling: rolling.pieces,
      want: want.pieces,
      ccy: rolling.ccy,
      maturity: ctx.calendar.periodOf(maturity),
      alternative: alternative.value,
    },
    true,
  );
}

/**
 * B2: WHAT BORROWING THE SAME MONEY COSTS IT OTHERWISE, per annum — the keenest quote a bank gave
 * it, or failing that the least any holder said it requires of the name (Corporate Credit E5).
 */
function costOfBorrowing(ctx: MechanismContext, issuer: PartyId): Option<Ratio> {
  const quoted = creditQuoteThisPeriod(ctx.journal, String(issuer), ctx.period);
  if (quoted.some) return some(quoted.value.rate);
  // Corporate Credit E5: failing a quote of its own, the least anybody said they require of the
  // name. One read, on the kernel, because three modules asked it and each scanned for itself.
  return ctx.requiredOf(issuer);
}

/**
 * A2: THE PRICE AT WHICH A DISCOUNT OVER THIS TERM COSTS `perAnnum` — the yield definition
 * rearranged, used ONCE, by the issuer, to say what it would refuse.
 *
 * It is not a price anybody pays and it never reaches the store: E2 forbids a discount computed off
 * a curve nobody traded, and this is not one — it is the issuer's own alternative expressed in the
 * units the book quotes in, so that a walk-away and a bid can be compared at all (Law 8).
 */
function priceCosting(perAnnum: Ratio, term: Ratio): PerPiece {
  const grossed = plus(
    asRatio(1, 'par'),
    scale(perAnnum, term, 'what the alternative costs over this term'),
    'par and the cost of carrying it',
  );
  return asPerPiece(
    over(asRatio(1, 'par'), grossed, 'what a unit must cost to come to par'),
    'the price at which this paper costs what borrowing otherwise costs',
  );
}

/** A1, Law 9: a new line, and the market it trades in after issue (D1). */
function openLine(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  id: InstrumentId,
  terms: PaperTerms,
): boolean {
  const market = marketId(`mkt.${id}`);
  ctx.issue({ id, kind: COMMERCIAL_PAPER, issuer: some(issuer), ccy, terms, market: some(market) });
  // D1: IT TRADES AFTER ISSUE, at a cleared price, so a holder can get out early. That is a market
  // like any other and not a favour anybody does the holder.
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy,
    rationing: 'proRata',
  });
  return true;
}

function marketOf(ctx: MechanismContext, id: InstrumentId): MarketId {
  const inst = ctx.instruments.get(id);
  if (!inst.market.some) {
    throw new Missing('Clearing D1', `${id} names no market`, { instrument: id });
  }
  return inst.market.value;
}

/* --------------------------------------------------------------------------------------------
 * WHY A BUYER BUYS
 * ------------------------------------------------------------------------------------------ */

/**
 * C1, C2, C2.a, C3: A CASH INVESTOR WITH A HORIZON, and its reason is what else it could do.
 *
 * C2 names the alternatives — a deposit, a repo, a central bank facility — and the one every cash
 * holder in this world actually has is a DEPOSIT, whose rate the banks publish every period
 * (`bank.depositRate`). So the most a buyer will pay for this paper is the price at which the paper
 * returns what the keenest board would pay it; above that it leaves the money on deposit. That is
 * C2.a in one line: short paper is a real substitute for a deposit, so the policy rate travels down
 * this channel WITHOUT anything tying the two — the buyer moved because its alternative moved.
 *
 * ITS RESERVATION AND THE ISSUER'S ARE THE SAME ARITHMETIC ON DIFFERENT ALTERNATIVES, which is why
 * there are two sides to the book at all (§46 A3). The issuer computes the price at which this
 * costs what borrowing otherwise costs it; the buyer computes the price at which it returns what
 * lending otherwise returns it. They overlap when the issuer's credit is worth less to it than the
 * buyer's patience is worth to the buyer, and they do not when it is not — and a book with no
 * overlap says `noOverlap` and nobody funds anybody (Appendix B: no demand added to clear).
 */
function buys(view: ParticipantView, market: MarketDecl): readonly Order[] {
  const line = lineIn(view, market);
  if (!line.some) return [];
  const { issuer, terms } = line.value;
  /**
   * C1, Clearing A2, Law 5: A NAME DOES NOT LEND TO ITSELF. Every cash investor kind was asked
   * about every paper book, its own issue included, so an issuer with spare cash bid for the paper
   * it was selling in the same session — and the solver refused the crossing fill at the site
   * (item 0, stop 20). Retiring your own paper early is a real act and a different one: it is a
   * buyback against the holder who has it, not a bid into your own primary book.
   */
  if (issuer === view.self.id) return [];
  // C3: a name it has seen fail is a name it does not lend to, whatever the price. This is the
  // clause that makes an issuer lose funding BEFORE it loses solvency — a buyer needs no insolvency
  // to refuse, only a reason to doubt, and a public failure is the plainest reason there is.
  if (doubted(view, issuer)) return [];
  const alternative = onDeposit(view, market.ccy);
  if (!alternative.some) return [];
  const on = view.calendar.startOf(view.period);
  const term = asRatio(yearFraction(terms.dayCount, on, terms.maturity), 'what is left of it');
  if (term <= 0) return [];
  // C2: the most it will pay is the price at which this returns what a deposit would.
  const most = priceCosting(alternative.value, term);
  if (most <= 0) return [];
  const room = headroomFor(view, issuer, market.ccy);
  if (room.pieces <= 0) return [];
  const afford = downTick(amountOf(room, most, 'units its room and its money reach to'));
  if (afford <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: most, qty: afford }];
}

/** The line this book is in, and who promised it. A read of the instrument (Law 19). */
function lineIn(
  view: ParticipantView,
  market: MarketDecl,
): Option<{ readonly issuer: PartyId; readonly terms: PaperTerms }> {
  if (!('instrument' in market)) return none();
  const i = view.instruments.get(market.instrument);
  if (!i.status.live || !isPaper(i.terms)) return none();
  return some({ issuer: i.terms.issuer, terms: i.terms });
}

/**
 * C2: WHAT THE MONEY WOULD EARN ON DEPOSIT — the keenest board anybody is showing this period, per
 * annum. A read of what banks said out loud (Law 19), never a rate this module decides.
 *
 * It asks each bank for its own board through the kernel's door rather than sifting the record for
 * a kind of event, which is the same discipline `bestRival` uses one module over.
 */
function onDeposit(view: ParticipantView, ccy: CurrencyCode): Option<Ratio> {
  let best: number | undefined;
  for (const bank of view.parties.ofKind(BANK)) {
    if (!bank.status.alive) continue;
    for (const paid of boardPosted(view, String(bank.id), view.period, ccy)) {
      if (best === undefined || paid > best) best = paid;
    }
  }
  return best === undefined
    ? none<Ratio>()
    : some(asRatio(best, 'what the keenest board would pay it'));
}

/**
 * C3: HOW MUCH MORE OF THIS NAME IT WILL TAKE — its own book, less what it already has out to that
 * issuer, and never more money than it holds.
 *
 * The concentration is a PREFERENCE (Law 2): how much of one name a cash investor is willing to be
 * exposed to is a taste, not a technology and not a rule anybody sets. **Every buyer in this world
 * shares one today, which is the honest limit of this build** — `Corporate Credit A4.b`'s argument
 * applies here too, that a single view held by everybody removes the dispersion a book needs — and
 * `Short-Term Debt C3` is marked PARTIAL for exactly that.
 */
function headroomFor(view: ParticipantView, issuer: PartyId, ccy: CurrencyCode): Cash {
  // Money B1, C4 (16.0): its book is a REPORT in its home money; what it lends is in the paper's.
  const book = view.inMoney(view.equity(), ccy);
  if (book.pieces <= 0) return noCash(ccy);
  const most: Cash = scale(
    book,
    view.params.ratio(PAPER_PARAMS.concentration),
    'the most of one name',
  );
  const already = exposureTo(view, issuer, ccy);
  const room: Cash = minus(most, already, 'what is left of its room for this name');
  const money = heldAsMoney(view.cash(ccy), ccy, 'the money it holds');
  return atMostCash(room, money, 'it cannot lend money it does not hold');
}

/** Law 19: what it already has out to this name, marked, read off its own holdings. */
function exposureTo(view: ParticipantView, issuer: PartyId, ccy: CurrencyCode): Cash {
  const out: Cash[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.issuer.some || i.issuer.value !== issuer) continue;
    const at = view.mark(h.instrument);
    if (!at.some) continue;
    out.push(
      view.inMoney(
        valueAt(at.value, view.quantity(h.instrument), i.ccy, 'what it has out to this name'),
        ccy,
      ),
    );
  }
  return sumCash(ccy, out, 'what it has out to this name').value;
}

/**
 * C3: WHETHER IT HAS SEEN THIS NAME FAIL, within the memory it keeps.
 *
 * A default and a breached covenant are both public, so every buyer saw them (Observer A3), and
 * either is enough: C3's point is that funding goes BEFORE solvency does, and a buyer that waited
 * for insolvency would be the forced buyer Appendix B forbids. The memory is the buyer's own and is
 * declared as the preference it is (§46 B1); a buyer that forgot nothing would never lend to a name
 * twice, and one that forgot instantly would have no credit standard at all.
 */
function doubted(view: ParticipantView, issuer: PartyId): boolean {
  const since = view.period - view.params.periods(PAPER_PARAMS.memory);
  return SOURED.some((kind) => {
    const said = view.lastPublicAbout(kind, String(issuer));
    return said.some && said.value.period >= since;
  });
}

/** C3: the two public things that make a cash investor stop lending to a name (Law 15: a table). */
const SOURED: readonly EventKind[] = ['credit.default', 'covenant.breached'];

/* --------------------------------------------------------------------------------------------
 * THE BACKSTOP
 * ------------------------------------------------------------------------------------------ */

export const BACKSTOP = agreementKindId('backstop');

/**
 * B4: A COMMITTED LINE, AND IT COSTS MONEY IN EVERY PERIOD IT IS NOT USED.
 *
 * *"A committed line with no commitment fee on undrawn headroom is a free option the lender did not
 * sell."* That sentence is the whole of why this is here and why the fee is the load-bearing half
 * of it: without the fee, every issuer in the world would hold an unlimited backstop it never paid
 * for, B3.b's run could never bite, and the liquidity risk this item exists to create would be
 * insured away for nothing by a lender that had agreed to it for nothing.
 *
 * It is an AGREEMENT and not an instrument (XI-8): nobody trades a commitment, it has no issued
 * quantity and no holder — it is a relation between a named issuer and a named bank, and it is one
 * of the things an estate has to divide.
 */
export interface BackstopTerms extends AgreementTerms {
  readonly kind: typeof BACKSTOP;
  /** The most the issuer may draw. The bank's own decision when it agreed to the line. */
  readonly limit: Cash;
  /** B4: what the undrawn headroom costs per annum, struck when the line was agreed. */
  readonly fee: Ratio;
  /**
   * 12a.7: what a DRAWING costs per annum — the rate the bank quoted the name when it committed
   * the line. What has been drawn is not a term: it is the outstanding of the loan row the drawing
   * wrote (`drawnOn`), read off the register (Law 19) — a number kept here beside it was a mirror.
   */
  readonly rate: Ratio;
}

export const isBackstop = (t: AgreementTerms): t is BackstopTerms =>
  'limit' in t && 'fee' in t && 'rate' in t;

/**
 * 12a.7, Banks Lending A1, F1.a: THE ROW A DRAWING IS. A backstop drawn is a loan — the bank's
 * money created against a claim on the issuer, at the line's rate, with a maturity — and it is
 * ONE row per line, drawn on again and again (Corporate Credit C9), named so a reader sees whose
 * it is and what it stands behind.
 */
function backstopLoanId(bank: PartyId, issuer: PartyId): InstrumentId {
  return instrumentId(`loan:${String(bank)}:${String(issuer)}:backstop`);
}

/** Law 19: what the issuer has drawn on this line — the outstanding of its row, or nothing yet. */
function drawnOn(ctx: MechanismContext, row: Agreement): Cash {
  const id = backstopLoanId(row.creditor, row.debtor);
  if (!ctx.instruments.has(id) || !ctx.instruments.get(id).status.live) return noCash(row.ccy);
  return heldAsMoney(ctx.register.heldTotal(id).value, row.ccy, 'what it has drawn and not repaid');
}

/**
 * B4: AN ISSUER KEEPS A BACKSTOP, and it is granted by a named bank rather than stated by the world.
 *
 * *"The issuer therefore keeps a backstop — a committed bank line, a liquid buffer."* It is granted
 * here as a standing decision rather than seeded (Appendix B: no seeded outcome), which also means
 * it comes BACK: an issuer whose bank failed has no line, and the next period it has one again from
 * wherever it banks now. A seeded relation could not do that.
 *
 * THE LIMIT IS THE ONE NUMBER AND IT IS A PLACEHOLDER. A committed facility is granted, priced and
 * re-sized by a lender out of its own view of the borrower and its own capital — Corporate Credit
 * C9, which this world cannot yet decide (item 17.2). Until then the line is sized off the issuer's
 * own book, declared as a SHAPE with a scheduled death and not as a fact anybody believes.
 */
/**
 * B2, XI-8, Register F2: THE LINES THAT ARE STILL LINES — performing or breached, never a row that
 * has been discharged or torn up.
 *
 * `ofKind` is the whole book and an estate has to be able to read the closed rows, so the filter
 * belongs at every read that acts on one. Without it this module charged a commitment fee every
 * period on lines the kernel had already ended with the party that made them (item 0, stop 19): a
 * firm that ceased into its estate in period 9 was invoiced in period 10, and settlement refused
 * the instruction because the payer was not there (Money E4). Read once, so the grant, the draw and
 * the fee cannot disagree about which lines exist (Law 4).
 */
function openLines(ctx: MechanismContext): readonly Agreement[] {
  return ctx.agreements
    .ofKind(BACKSTOP)
    .filter((a) => a.state === 'performing' || a.state === 'breached');
}

export function grantBackstops(ctx: MechanismContext): void {
  const held = new Set<string>();
  for (const row of openLines(ctx)) held.add(String(row.debtor));
  // 12a.7: A LINE IS ARRANGED WITH THE PAPER, not handed to every firm in the world. The issuers
  // are the ones that offered paper this period (this module's own event); an issuer that has a
  // line keeps it, and one that has none gets one from its bank the first time it comes to the
  // market — at the rate that bank quoted the name this period. A name nobody quoted has no
  // lender behind its paper, and that is the refusal (Corporate Credit C3.a), not a line at nothing.
  for (const e of ctx.journal.ofKindIn('paper.offered', ctx.period)) {
    const issuer = e.subjects[0];
    if (issuer === undefined || held.has(issuer)) continue;
    const p = ctx.parties.get(partyId(issuer));
    if (!p.status.alive) continue;
    held.add(issuer);
    const ccy = ctx.registry.currencyOf(p.region);
    const bank = ctx.accountOf(p.id, ccy).issuer;
    // A bank banks at its central bank for reserves, and a line to yourself is not a backstop.
    if (bank === p.id) continue;
    const quote = creditQuoteThisPeriod(ctx.journal, String(p.id), ctx.period);
    if (!quote.some) continue;
    const book = ctx.participant(p.id).inMoney(ctx.participant(p.id).equity(), ccy);
    if (book.pieces <= 0) continue;
    const terms: BackstopTerms = {
      kind: BACKSTOP,
      limit: scale(book, ctx.params.ratio(PAPER_PARAMS.line), 'the line it was granted'),
      fee: ctx.params.perAnnum(PAPER_PARAMS.commitmentFee),
      rate: quote.value.rate,
    };
    if (terms.limit.pieces <= 0) continue;
    ctx.owes({
      debtor: p.id,
      creditor: bank,
      ccy,
      owed: 0,
      terms,
      why: `${String(bank)} commits a line to ${String(p.id)} against its short paper`,
    });
  }
}

/**
 * B4: the fee falls every period on WHAT WAS NOT DRAWN, and it is a real payment between two named
 * parties (Law 5: both legs, same pass). An issuer that never touches its line still pays for it,
 * which is what makes holding one a decision rather than a free good.
 */
export function chargeBackstops(ctx: MechanismContext): void {
  for (const row of openLines(ctx)) {
    const t = row.terms;
    if (!isBackstop(t)) continue;
    const undrawn = minus(t.limit, drawnOn(ctx, row), 'the headroom it is paying to keep open');
    if (undrawn.pieces <= 0) continue;
    // Law 8: the fee is quoted per annum and falls per period, so it is placed on the calendar by
    // the calendar and not by a count anybody wrote down (Money G3.a).
    const on = ctx.calendar.startOf(ctx.period);
    const over = asRatio(
      yearFraction(PAPER_DAY_COUNT, on, addDays(on, ctx.calendar.periodDays)),
      'the part of a year this period is',
    );
    const due = downTick(
      scale(scale(undrawn, t.fee, 'a year of it'), over, 'this period of it').pieces,
    );
    if (due <= 0) continue;
    ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(row.debtor, row.ccy),
          to: ctx.accountOf(row.creditor, row.ccy),
          ccy: row.ccy,
          amount: due,
        },
      ],
      cause: 'transfer',
      reason: `${String(row.debtor)} pays ${String(row.creditor)} for undrawn backstop headroom`,
    });
  }
}

/**
 * B3.b: THE RUN, AND WHAT AN ISSUER DOES ABOUT IT.
 *
 * Paper matured, the book declined to lend again, and the issuer has to repay out of money it does
 * not have. This is the moment the whole item exists to produce, and NOTHING HERE RESCUES ANYBODY:
 * what it does is offer the issuer the one thing it actually agreed to in advance, its backstop,
 * and let the rest fall where it falls.
 *
 * The three paths B3.b names are all real and only one of them is this module's. It can DRAW (here).
 * It can SELL — XI-2's forced seller, which already exists and needs nothing from this module: an
 * issuer short of cash with liquid assets is exactly the party that mechanism is about. Or it can
 * FAIL, which is the kernel's: the maturity is a payment like any other, it does not happen, and
 * `commercialPaper.defaultOn` says what that means and `accelerates` carries it to every other
 * line the issuer has. There is no fourth path and no buyer of last resort (Appendix B).
 */
export function drawBackstops(ctx: MechanismContext): void {
  for (const row of openLines(ctx)) {
    const t = row.terms;
    if (!isBackstop(t)) continue;
    const owing = maturingIn(ctx, row.debtor, ctx.period, row.ccy);
    if (owing.pieces <= 0) continue;
    const has = heldAsMoney(
      ctx.register.quantity(
        row.debtor,
        moneyInstrumentId(ctx.accountOf(row.debtor, row.ccy).issuer, row.ccy),
      ),
      row.ccy,
      'the money it holds against what is due',
    );
    const short = minus(owing, has, 'what it cannot repay out of what it holds');
    if (short.pieces <= 0) continue;
    // B4: it may draw what it agreed and not a penny more. That is not a bound on an outcome — it
    // is the size of the promise somebody made it, and an issuer short of more than its line is
    // exactly the issuer that fails (Law 6).
    const room = minus(t.limit, drawnOn(ctx, row), 'what is left of the line');
    const take = downTick(atMostCash(short, room, 'it may draw what it agreed and no more').pieces);
    if (take <= 0) continue;
    // 12a.7, Banks Lending A1, B1: A DRAWING IS A LOAN ROW ON BOTH BOOKS — the bank's money
    // created into the issuer's account against a claim on the issuer, at the line's rate, with a
    // maturity; the kernel presents its interest and its maturity like any loan's. It was a
    // transfer and a number in the terms: money the bank had paid away with nothing on its book to
    // show for it, and a `drawn` nobody could read off a register (Law 19, Law 5).
    const id = backstopLoanId(row.creditor, row.debtor);
    if (!ctx.instruments.has(id) || !ctx.instruments.get(id).status.live) {
      const drawn = ctx.calendar.startOf(ctx.period);
      const terms: LoanTerms = {
        kind: LOAN,
        originator: row.creditor,
        borrower: row.debtor,
        rate: t.rate,
        drawn,
        maturity: addMonths(drawn, ctx.params.months(PAPER_PARAMS.lineMonths)),
        dayCount: 'ACT/365F',
        // Corporate Credit C9: a line is drawn and repaid at the borrower's option and falls due once.
        amortising: false,
        security: [],
      };
      ctx.issue({ id, kind: LOAN, issuer: some(row.debtor), ccy: row.ccy, terms, market: none() });
    }
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from: row.debtor,
          to: row.creditor,
          instrument: id,
          qty: asQty(take, 'what it drew, in pieces of the row'),
          pricePerUnit: some(asPerPiece(1, 'at what it promised')),
          accruedPerUnit: none(),
        },
        {
          kind: 'money',
          from: { holder: row.creditor, issuer: row.creditor },
          to: ctx.accountOf(row.debtor, row.ccy),
          ccy: row.ccy,
          amount: take,
        },
      ],
      cause: 'issuance',
      reason: `${String(row.debtor)} draws its backstop to meet maturing paper`,
    });
    if (r.outcome !== 'settled') continue;
    ctx.record(
      'backstop.drawn',
      [row.debtor, row.creditor, id],
      {
        issuer: row.debtor,
        bank: row.creditor,
        loan: id,
        drew: take,
        owing: owing.pieces,
        limit: t.limit.pieces,
        ccy: owing.ccy,
      },
      true,
    );
  }
}

/* --------------------------------------------------------------------------------------------
 * WHAT CAN BE READ ABOUT IT
 * ------------------------------------------------------------------------------------------ */

/**
 * B5 VERIFY, E3 FORBID: THE MATURITY PROFILE IS A READ, and a concentrated one is a wall somebody
 * can see coming. E3 is checked beside it because both are statements about the same outstanding:
 * no negative outstanding anywhere, and nothing matures without the cash moving.
 *
 * A VERIFY is a thing to MEASURE and never to enforce, so this reports and repairs nothing: a wall
 * is a finding about an issuer's funding, not a licence to move a maturity.
 */
function profile(): Family {
  return {
    name: 'units',
    contributor: 'short-term-debt',
    spec: 'Short-Term Debt B5 Short-Term Debt E3',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isPaper(i.terms)) continue;
        const outstanding = view.register.heldTotal(i.id).value;
        // E3: no negative outstanding. It cannot arise by arithmetic, which is why it is checked:
        // a FORBID that holds breaks silently, and this is where it would show.
        if (outstanding < 0) {
          out.push({
            family: 'units',
            spec: 'Short-Term Debt E3',
            owner: i.id,
            size: outstanding,
            unit: i.unit,
            period: view.period,
            message: `${i.id}: negative paper outstanding`,
          });
        }
        // E3: a maturity that has passed with paper still outstanding is cash that did not move.
        if (outstanding > 0 && view.calendar.periodOf(i.terms.maturity) < view.period) {
          out.push({
            family: 'units',
            spec: 'Short-Term Debt E3',
            owner: i.id,
            size: outstanding,
            unit: i.unit,
            period: view.period,
            message: `${i.id}: matured in period ${view.calendar.periodOf(i.terms.maturity)} and is still outstanding`,
          });
        }
      }
      return out;
    },
  };
}

/**
 * C1: THE CASH INVESTORS THIS MODULE SPEAKS FOR — a bank's liquidity book and a corporate treasurer.
 *
 * A MONEY FUND IS THE THIRD AND IT IS NOT HERE, which was a defect in the first draft of this item.
 * A fund may only hold what its MANDATE lets it hold (Fund Shares A4), and `funds/index.ts:eligible`
 * is the one gate that asks: the mandate's kinds, the fund's own money, the tenor its investors
 * agreed to, and whether it can value the thing at the yield it requires. A participant declared
 * here would have bid for a fund without asking any of those — a fund whose mandate says bills-only
 * buying commercial paper, which is the mandate not binding at all (Law 4: one writer of what a
 * fund may hold).
 *
 * So a money fund's appetite for this paper is expressed where a fund's appetite lives: in the
 * mandate it was launched under, which now names this kind.
 */
const CASH_INVESTORS: readonly PartyKindId[] = [BANK, FIRM];

export function shortTermDebt(): SystemModule {
  return {
    id: 'short-term-debt',
    spec: 'Short-Term Debt',
    // It reads what a firm published it is short of and what a bank published it keeps back, and it
    // prices against what banks quoted and what they pay on deposits. All four are public events,
    // never imports (Law 15) — but the modules that write them have to be there.
    requires: ['firms', 'banks', 'money-market'],
    instrumentKinds: [commercialPaper],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: PAPER_PAR, name: 'units of par', perUnit: MONEY_PIECES }],
    agreementKinds: [
      {
        id: BACKSTOP,
        what: 'a committed line an issuer may draw, and pays for whether it draws or not',
        // B2, XI-8: a line is a promise to lend on demand, and an estate lends nothing (Banks
        // Lending A1). An acquirer that bought the committing bank's book stands behind it.
        binds: 'aGoingConcern',
      },
    ],
    params: [
      {
        id: PAPER_PARAMS.tenor,
        value: 91,
        unit: 'days',
        dimension: 'days',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Short-Term Debt A1.b: how long short paper runs for — weeks to months, and three months is the tenor the market is named after. A convention rather than a choice this world makes each time, stated in DAYS because that is the grain the calendar places it on and because at this tenor a month of slack is a third of the instrument (Law 8). It is not a forecast of how long the issuer needs the money: what it needs is what it published it is short of, and the term is the market’s.',
      },
      {
        id: PAPER_PARAMS.commitmentFee,
        value: 0.0035,
        unit: 'per annum on undrawn headroom',
        dimension: 'perAnnum',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Short-Term Debt B4: what a committed line costs its holder per annum on the part it has NOT drawn. It is the price of an option the lender sold and the clause says so plainly \u2014 *\u201ca committed line with no commitment fee on undrawn headroom is a free option the lender did not sell\u201d* \u2014 which is why it is the load-bearing half of B4: without it every issuer would hold an unlimited backstop it never paid for, B3.b\u2019s run could never bite, and this world\u2019s only liquidity risk would be insured away for nothing. A convention of the market, quoted per annum and falling per period by the calendar (Money G3.a).',
      },
      {
        id: PAPER_PARAMS.line,
        value: 0.1,
        unit: 'share of the issuer\u2019s own opening book',
        dimension: 'ratio',
        // Law 2: a shape that names the item which kills it IS a placeholder, and the register says
        // so — it refused the world at assembly for as long as this said `shape` (item 0, stop 1).
        kind: 'placeholder',
        owner: 'model',
        standsInFor: {
          mechanism: 'Corporate Credit C9',
          item: '17.3',
        },
        why: 'Short-Term Debt B4: how big a line an issuer is GRANTED, as a share of its own book. A committed facility is GRANTED, priced and re-sized by a lender out of its own view of the borrower and its own capital \u2014 that is Corporate Credit C9, and this world cannot yet take the decision (item 17.3 builds it, and `banks/index.ts:draw` already has the drawing half). So this is a SHAPE with a scheduled death and not a number anybody believes. Everything else about the line IS decided: whether the issuer has one at all, what the fee takes out of its account every period, and whether it draws. When 17.3 lands, the lender sets the limit and this number is deleted in the same change.',
      },
      {
        id: PAPER_PARAMS.lineMonths,
        value: 12,
        unit: 'months',
        dimension: 'months',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Short-Term Debt B4 (12a.7): how long a drawing on a committed line runs for. A convention of the facility — a backstop is written for a year and the drawing matures with it — stated in MONTHS because that is the grain the calendar places a maturity on (Law 8, Money G3.a). It is not a forecast of how long the issuer needs the money: the row is repaid at its option (Corporate Credit C9) and falls due once.',
      },
      {
        id: PAPER_PARAMS.memory,
        value: 26,
        unit: 'periods',
        dimension: 'periods',
        kind: 'preference',
        owner: 'model',
        why: 'Short-Term Debt C3, \u00a746 B1: how long a cash investor remembers having seen a name fail. Memory is a PREFERENCE \u2014 \u00a746 gives every deciding party one and it is the only primitive an outlook is allowed \u2014 and this is the same taste pointed at a credit event rather than a price. A buyer that forgot nothing would never lend to a name twice; one that forgot instantly would have no credit standard at all. Half a year of this world\u2019s weeks, and like the concentration beside it, every buyer shares one today.',
      },
      {
        id: PAPER_PARAMS.concentration,
        value: 0.05,
        unit: 'share of its own book',
        dimension: 'ratio',
        kind: 'preference',
        owner: 'model',
        why: 'Short-Term Debt C3: the most of ONE issuer’s name a cash investor is willing to be exposed to, as a share of its own book. It is a PREFERENCE — how concentrated somebody is willing to be is a taste, not a technology and not a rule anybody sets — and it is what makes C3 true: a buyer that will not add to a name stops funding it long before anyone proves it insolvent. EVERY BUYER IN THIS WORLD SHARES ONE TODAY, which is the honest limit of this build and is why C3 is marked PARTIAL: Corporate Credit A4.b’s argument applies here too, that one view held by everybody removes the dispersion a book needs.',
      },
    ],
    phases: [
      {
        name: 'paper.issue',
        spec: 'Short-Term Debt B1 Short-Term Debt B2 Short-Term Debt B3 Short-Term Debt B3.a Short-Term Debt E1',
        // Clearing F1: after everything it reads has been published this period, and after the
        // money market has sat — so a bank's need here is what the overnight books did not fund —
        // and before the session, because an offer that arrives after the book has cleared is not
        // an offer. `before: markets` is the last position still in front of the auction.
        anchor: { before: 'markets' },
        reads: [
          { kind: 'event', name: 'bank.buffer', of: 'anyPeriod' },
          { kind: 'event', name: 'credit.quoted', of: 'thisPeriod' },
          { kind: 'event', name: 'firms.funding', of: 'thisPeriod' },
        ],
        writes: [{ kind: 'event', name: 'paper.offered' }],
        run: (ctx: MechanismContext): void => {
          // B4 with B1 (12a.7): an issuer that comes to the market gets its line with the paper,
          // periods before the maturity it is drawn to meet — a backstop arranged after the book
          // declined is not a backstop, and one handed to every firm that never issued was a free
          // option nobody had asked for.
          issuePaper(ctx);
          grantBackstops(ctx);
        },
      },
      {
        name: 'paper.backstop',
        spec: 'Short-Term Debt B3.b Short-Term Debt B4',
        /**
         * B3.b, item 0 (stop 4): BEFORE THE MATURITY IT IS DRAWN TO MEET.
         *
         * It ran `after: markets`, and the kernel presents a maturity in `corporateActions` — which
         * is the first phase of the period. So the paper failed, `defaultOn` fired and `accelerates`
         * carried it to every other line the issuer had, and the draw arrived afterwards to fund a
         * repayment that had already failed. A backstop drawn after the default is not a backstop.
         */
        anchor: { before: 'corporateActions' },
        reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
        writes: [
          { kind: 'event', name: 'backstop.drawn' },
          { kind: 'event', name: 'credit.declined' },
        ],
        run: (ctx: MechanismContext): void => {
          drawBackstops(ctx);
          chargeBackstops(ctx);
        },
      },
    ],
    // C1: a cash investor with a horizon. A money fund, a corporate treasurer, a bank's liquidity
    // book — several party kinds with ONE reason, which is why this is one participant declared
    // once per kind and not three mechanisms (Law 15).
    participants: CASH_INVESTORS.map((partyKind) => ({ partyKind, orders: buys })),
    families: [profile()],
  };
}
